//! The Corvus shell: one command dispatcher and the line editor that feeds it.
//! Shared by the kernel (src/main.rs) and the web edition (web/src/lib.rs);
//! each provides `crate::print` and `crate::calc`.

use crate::{calc, print};
#[path = "shell_platform.rs"]
pub mod platform;
#[path = "crowc.rs"]
mod crowc;

const COMMANDS: &[&str] = &["help", "about", "crow", "clear", "echo", "calc", "dio", "history", "exit", "time", "uptime", "mem", "color", "banner", "repeat", "sleep", "logo", "lantern", "Draw", "Print", "Vars", "Reset", "Ticks", "let", "if", "while"];
const HISTORY: usize = 16;

/// One row of Levenshtein state; bounded by the command vocabulary.
fn closest(word: &str) -> &'static str {
    let mut best = COMMANDS[0];
    let mut distance = usize::MAX;
    for &candidate in COMMANDS {
        let mut row = [0usize; 32];
        for (i, cell) in row.iter_mut().enumerate() { *cell = i; }
        for (i, a) in word.bytes().enumerate() {
            let mut diagonal = row[0]; row[0] = i + 1;
            for (j, b) in candidate.bytes().enumerate() {
                let old = row[j + 1];
                row[j + 1] = (row[j] + 1).min(old + 1).min(diagonal + usize::from(!a.eq_ignore_ascii_case(&b)));
                diagonal = old;
            }
        }
        if row[candidate.len()] < distance { distance = row[candidate.len()]; best = candidate; }
    }
    best
}

pub fn banner() {
    print("\x1b[35m   .__\n   (o )>   C O R V U S\n   / )     ARM64 / SERIAL CONSOLE\n  /_/      type help\n\x1b[0m");
}

/// The one command dispatcher. Serial input and the on-screen HELP button
/// both end up here.
fn command(line: &str) -> bool {
    let line = line.trim();
    match line {
        "" => {}
        "help" => print("help  about  crow  clear  echo <text>  calc <expr>  dio [question]  history\ntime  uptime  mem  color <0-15>  banner <text>\nrepeat <1-32> <command>  sleep <ms>  logo  lantern\n"),
        "about" => print("Corvus OS 0.2 | Rust bare metal | QEMU virt | ramfb + virtio-tablet\n"),
        "crow" => banner(),
        "clear" => print("\x1b[2J\x1b[H"),
        _ => {
            if let Some(message) = line.strip_prefix("echo ") {
                print(message);
                print("\n");
            } else if line == "calc" || line.starts_with("calc ") {
                calculate(line[4..].trim_start());
            } else {
                return false;
            }
        }
    }
    true
}

/// `calc <expr>`: 64-bit integer arithmetic with + - * / % and parentheses.
fn calculate(expression: &str) {
    match calc::eval(expression) {
        Ok(value) => {
            let mut digits = [0u8; 20];
            print(calc::format(value, &mut digits));
            print("\n");
        }
        Err(error) => {
            print("calc: ");
            print(error.message());
            if let Some(column) = error.column() {
                let mut digits = [0u8; 20];
                print(" at column ");
                print(calc::format(column as i64, &mut digits));
            }
            print("\n");
            if error == calc::Error::Empty {
                print("usage: calc (2 + 3) * 4\n");
            }
        }
    }
}

pub struct Shell {
    line: [u8; 256],
    length: usize,
    previous_was_cr: bool,
    llm_mode: bool,
    waiting: bool,
    receiving: bool,
    reply: [u8; 2048],
    reply_len: usize,
    now_ms: u64,
    deadline: u64,
    pub dio_online: bool,
    unknown: u8,
    history: [[u8; 256]; HISTORY],
    history_lens: [usize; HISTORY],
    history_count: usize,
    history_cursor: usize,
    draft: [u8; 256],
    draft_len: usize,
    escape: u8,
    sleeping: Option<u64>,
    evaluator: crowc::CrowC,
    source: [u8; 1024],
    source_len: usize,
    pub graphics: for<'a> fn(platform::Effect<'a>),
}

impl Shell {
    pub const fn new() -> Shell {
        Shell { line: [0; 256], length: 0, previous_was_cr: false,
            llm_mode: false, waiting: false, receiving: false, reply: [0; 2048],
            reply_len: 0, now_ms: 0, deadline: 0, dio_online: false, unknown: 0,
            history: [[0; 256]; HISTORY], history_lens: [0; HISTORY], history_count: 0,
            history_cursor: 0, draft: [0; 256], draft_len: 0, escape: 0, sleeping: None, evaluator: crowc::CrowC::new(), source: [0; 1024], source_len: 0, graphics: platform::render }
    }

    fn prompt(&self) {
        print(if self.source_len > 0 { ".. " } else if self.llm_mode { "diogenes> " } else { "corvus> " });
    }

    /// Called from the event loop; never block pointer input on a host model.
    pub fn tick(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        if self.sleeping.is_some_and(|until| now_ms >= until) {
            self.sleeping = None; self.prompt();
        }
        if self.waiting && now_ms >= self.deadline {
            self.waiting = false;
            self.dio_online = false;
            print("Diogenes: even a philosopher has limits. Host timed out; try again.\n");
            self.prompt();
        }
    }

    fn ask(&mut self, text: &str) {
        self.unknown = 0;
        if text.is_empty() {
            self.llm_mode = true;
            print("Talk plainly. Type exit to return; Ctrl-C cancels a reply.\n");
            return;
        }
        self.waiting = true;
        self.deadline = platform::clock(self.now_ms).saturating_add(45_000);
        platform::wake_after(45_000);
        // Protocol bytes go only to UART, never to the framebuffer terminal.
        #[cfg(all(target_arch = "aarch64", target_os = "none"))]
        let output: fn(&str) = crate::uart::protocol;
        #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
        let output: fn(&str) = print;
        output("\x1eDIO "); output(text); output("\n");
    }

    fn remember(&mut self) {
        if self.length == 0 { return; }
        if self.history_count == HISTORY {
            self.history.copy_within(1.., 0); self.history_lens.copy_within(1.., 0);
            self.history_count -= 1;
        }
        self.history[self.history_count] = self.line;
        self.history_lens[self.history_count] = self.length;
        self.history_count += 1;
        self.history_cursor = self.history_count;
    }

    fn replace_line(&mut self, text: &[u8]) {
        for _ in 0..self.length { print("\x08 \x08"); }
        self.length = text.len().min(self.line.len());
        self.line[..self.length].copy_from_slice(&text[..self.length]);
        print(core::str::from_utf8(&self.line[..self.length]).unwrap_or(""));
    }

    fn recall(&mut self, up: bool) {
        if self.history_count == 0 { return; }
        if up && self.history_cursor == self.history_count {
            self.draft = self.line; self.draft_len = self.length;
        }
        self.history_cursor = if up { self.history_cursor.saturating_sub(1) }
            else { (self.history_cursor + 1).min(self.history_count) };
        let (text, len) = if self.history_cursor == self.history_count { (self.draft, self.draft_len) }
            else { (self.history[self.history_cursor], self.history_lens[self.history_cursor]) };
        self.replace_line(&text[..len]);
    }

    fn complete(&mut self) {
        if self.llm_mode || self.source_len > 0 || self.line[..self.length].contains(&b' ') { return; }
        let prefix = core::str::from_utf8(&self.line[..self.length]).unwrap_or("");
        let mut found = ""; let mut count = 0;
        for &candidate in COMMANDS {
            if candidate.starts_with(prefix) { found = candidate; count += 1; }
        }
        if count == 1 {
            self.replace_line(found.as_bytes()); self.feed(b' ');
        } else if count > 1 {
            print("\n");
            for &candidate in COMMANDS { if candidate.starts_with(prefix) { print(candidate); print("  "); } }
            print("\n"); self.prompt(); print(prefix);
        }
    }

    fn dispatch(&mut self, text: &str) {
        if self.source_len > 0 || self.evaluator.accepts(text) {
            if self.source_len + text.len() + 1 > self.source.len() {
                self.source_len = 0; print("CrowC: block exceeds 1024 bytes\n"); return;
            }
            self.source[self.source_len..self.source_len + text.len()].copy_from_slice(text.as_bytes());
            self.source_len += text.len(); self.source[self.source_len] = b'\n'; self.source_len += 1;
            let copy = self.source;
            let source = core::str::from_utf8(&copy[..self.source_len]).unwrap_or("");
            if crowc::incomplete(source) { return; }
            self.source_len = 0;
            let context = crowc::Context { graphics: self.graphics, command, ticks: platform::clock(self.now_ms), cancelled: platform::cancelled };
            match self.evaluator.eval(source, context) {
                Ok(()) => { print("\n"); self.unknown = 0; }
                Err(error) => { print("\nCrowC: "); print(error.0); print("\n"); }
            }
            return;
        }
        let (name, args) = text.split_once(' ').unwrap_or((text, ""));
        let args = args.trim();
        match name {
            "time" | "uptime" => {
                let mut digits = [0; 20]; print("uptime: ");
                print(calc::format(platform::clock(self.now_ms).min(i64::MAX as u64) as i64, &mut digits));
                print(" ms (generic monotonic timer)\n"); self.unknown = 0; return;
            }
            "mem" => { platform::memory(core::mem::size_of::<Self>()); self.unknown = 0; return; }
            "color" => {
                const ANSI: [&str; 16] = ["\x1b[30m", "\x1b[34m", "\x1b[32m", "\x1b[36m", "\x1b[31m", "\x1b[35m", "\x1b[33m", "\x1b[37m", "\x1b[90m", "\x1b[94m", "\x1b[92m", "\x1b[96m", "\x1b[91m", "\x1b[95m", "\x1b[93m", "\x1b[97m"];
                if let Ok(n @ 0..=15) = args.parse::<usize>() { print(ANSI[n]); }
                else { print("usage: color <0-15>\n"); }
                self.unknown = 0; return;
            }
            "banner" => {
                if args.is_empty() || args.len() > 38 { print("usage: banner <1-38 characters>\n"); }
                else { (self.graphics)(platform::Effect::Banner(args)); print(args); print("\n"); }
                self.unknown = 0; return;
            }
            "logo" => { (self.graphics)(platform::Effect::Logo); banner(); self.unknown = 0; return; }
            "lantern" => {
                print("     .----.\n     | /\\ |   DIOGENES\n    /|/  \x5c|\\  looking for an honest human\n    +------+\n    |  **  |\n    | *  * |\n    +------+\n       ||\n    ___||___\n");
                self.unknown = 0; return;
            }
            "sleep" => {
                if let Ok(ms @ 0..=60_000) = args.parse::<u64>() {
                    self.sleeping = Some(platform::clock(self.now_ms).saturating_add(ms));
                    platform::wake_after(ms);
                } else { print("usage: sleep <0-60000 ms>; Ctrl-C cancels\n"); }
                self.unknown = 0; return;
            }
            "repeat" => {
                if let Some((count, command)) = args.split_once(' ') {
                    if let Ok(n @ 1..=32) = count.parse::<usize>() {
                        let first = command.trim().split_ascii_whitespace().next().unwrap_or("");
                        if !["repeat", "sleep", "dio"].contains(&first) && !first.starts_with('?') && !first.is_empty() {
                            for _ in 0..n { self.dispatch(command.trim()); }
                            return;
                        }
                    }
                }
                print("usage: repeat <1-32> <command>; no nested repeat, sleep or dio\n"); return;
            }
            _ => {}
        }
        if text == "history" {
            for i in 0..self.history_count {
                let mut digits = [0; 20];
                print(calc::format((i + 1) as i64, &mut digits)); print("  ");
                print(core::str::from_utf8(&self.history[i][..self.history_lens[i]]).unwrap_or("")); print("\n");
            }
            self.unknown = 0;
        } else if command(text) {
            if !text.is_empty() { self.unknown = 0; }
        } else {
            self.unknown = self.unknown.saturating_add(1);
            let word = text.split_ascii_whitespace().next().unwrap_or(text);
            print("`"); print(word); print("` isn't a command. Did you mean `");
            print(closest(word)); print("`? Or just ask Diogenes: dio <your question>\n");
            if self.unknown >= 2 { print("Not technical? Type `dio` and just talk.\n"); }
        }
    }

    fn submit(&mut self) {
        self.remember();
        print("\n");
        let mut copy = [0u8; 128];
        copy[..self.length].copy_from_slice(&self.line[..self.length]);
        if let Ok(text) = core::str::from_utf8(&copy[..self.length]) {
            let text = text.trim();
            if self.source_len > 0 { self.dispatch(text);
            } else if text == "exit" && self.llm_mode {
                self.llm_mode = false; self.unknown = 0;
            } else if text == "dio" {
                self.ask("");
            } else if let Some(question) = text.strip_prefix("dio ").or_else(|| text.strip_prefix('?')) {
                self.ask(question.trim());
            } else if self.llm_mode && !text.is_empty() {
                self.ask(text);
            } else {
                self.dispatch(text);
            }
        }
        self.length = 0;
        if !self.waiting && self.sleeping.is_none() { self.prompt(); }
    }

    pub fn feed(&mut self, byte: u8) {
        if byte == 3 {
            self.waiting = false;
            self.sleeping = None;
            self.source_len = 0;
            self.length = 0;
            self.previous_was_cr = false;
            self.escape = 0;
            print("^C\n"); self.prompt(); return;
        }
        if byte == 0x1e { self.receiving = true; self.reply_len = 0; return; }
        if self.receiving {
            if byte == 0x1f {
                self.receiving = false;
                let reply = core::str::from_utf8(&self.reply[..self.reply_len]).unwrap_or("Invalid host reply.");
                if reply == "DIO_STATUS ready" || reply == "DIO_STATUS offline" {
                    self.dio_online = reply == "DIO_STATUS ready";
                } else if self.waiting {
                    self.waiting = false;
                    self.dio_online = !reply.starts_with("Diogenes offline:");
                    print(reply); print("\n"); self.prompt();
                }
            } else if (byte == b'\n' || (b' '..=b'~').contains(&byte)) && self.reply_len < self.reply.len() {
                self.reply[self.reply_len] = byte; self.reply_len += 1;
            }
            return;
        }
        if self.waiting || self.sleeping.is_some() { return; }
        if self.escape != 0 {
            match (self.escape, byte) {
                (1, b'[' | b'O') => self.escape = 2,
                (2, b'A') => { self.escape = 0; self.recall(true); }
                (2, b'B') => { self.escape = 0; self.recall(false); }
                (2, b'0'..=b'9' | b';') => {},
                _ => self.escape = 0,
            }
            return;
        }
        if byte == b'\n' && self.previous_was_cr {
            self.previous_was_cr = false;
            return;
        }
        self.previous_was_cr = byte == b'\r';
        match byte {
            0x1b => self.escape = 1,
            b'\t' => self.complete(),
            b'\n' | b'\r' => self.submit(),
            8 | 127 if self.length > 0 => {
                self.length -= 1;
                print("\x08 \x08");
            }
            b' '..=b'~' if self.length < self.line.len() => {
                self.line[self.length] = byte;
                self.length += 1;
                let echo = [byte];
                print(core::str::from_utf8(&echo).unwrap_or(""));
            }
            _ => {}
        }
    }

    /// Physical buttons leave conversation/continuation before dispatching.
    pub fn action(&mut self, command: &str) {
        self.llm_mode = false; self.source_len = 0;
        self.run_as_typed(command);
    }

    /// Runs a command exactly as if it had been typed at the prompt.
    pub fn run_as_typed(&mut self, text: &str) {
        if self.waiting || self.sleeping.is_some() { self.feed(3); }
        while self.length > 0 {
            self.feed(8);
        }
        for byte in text.bytes() {
            self.feed(byte);
        }
        self.feed(b'\r');
    }
}
