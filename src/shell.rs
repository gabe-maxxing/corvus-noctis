//! The Corvus shell: one command dispatcher and the line editor that feeds it.
//! Shared by the kernel (src/main.rs) and the web edition (web/src/lib.rs);
//! each provides `crate::print` and `crate::calc`.

use crate::{calc, print};

pub fn banner() {
    print("\x1b[35m   .__\n   (o )>   C O R V U S\n   / )     ARM64 / SERIAL CONSOLE\n  /_/      type help\n\x1b[0m");
}

/// The one command dispatcher. Serial input and the on-screen HELP button
/// both end up here.
fn command(line: &str) {
    let line = line.trim();
    match line {
        "" => {}
        "help" => print("help  about  crow  clear  echo <text>  calc <expr>\n"),
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
                print("Unknown command. Try help.\n");
            }
        }
    }
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
    line: [u8; 128],
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
}

impl Shell {
    pub const fn new() -> Shell {
        Shell { line: [0; 128], length: 0, previous_was_cr: false,
            llm_mode: false, waiting: false, receiving: false, reply: [0; 2048],
            reply_len: 0, now_ms: 0, deadline: 0, dio_online: false }
    }

    fn prompt(&self) {
        print(if self.llm_mode { "diogenes> " } else { "corvus> " });
    }

    /// Called from the event loop; never block pointer input on a host model.
    pub fn tick(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        if self.waiting && now_ms >= self.deadline {
            self.waiting = false;
            self.dio_online = false;
            print("Diogenes: even a philosopher has limits. Host timed out; try again.\n");
            self.prompt();
        }
    }

    fn ask(&mut self, text: &str) {
        if text.is_empty() {
            self.llm_mode = true;
            print("Talk plainly. Type exit to return; Ctrl-C cancels a reply.\n");
            return;
        }
        self.waiting = true;
        self.deadline = self.now_ms.saturating_add(45_000);
        // Protocol bytes go only to UART, never to the framebuffer terminal.
        #[cfg(all(target_arch = "aarch64", target_os = "none"))]
        let output: fn(&str) = crate::uart::protocol;
        #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
        let output: fn(&str) = print;
        output("\x1eDIO "); output(text); output("\n");
    }

    fn submit(&mut self) {
        print("\n");
        let mut copy = [0u8; 128];
        copy[..self.length].copy_from_slice(&self.line[..self.length]);
        if let Ok(text) = core::str::from_utf8(&copy[..self.length]) {
            let text = text.trim();
            if text == "exit" && self.llm_mode {
                self.llm_mode = false;
            } else if text == "dio" {
                self.ask("");
            } else if let Some(question) = text.strip_prefix("dio ").or_else(|| text.strip_prefix('?')) {
                self.ask(question.trim());
            } else if self.llm_mode && !text.is_empty() {
                self.ask(text);
            } else {
                command(text);
            }
        }
        self.length = 0;
        if !self.waiting { self.prompt(); }
    }

    pub fn feed(&mut self, byte: u8) {
        if byte == 3 {
            self.waiting = false;
            self.length = 0;
            self.previous_was_cr = false;
            print("^C\n"); self.prompt(); return;
        }
        if byte == 0x1e { self.receiving = true; self.reply_len = 0; return; }
        if self.receiving {
            if byte == 0x1f {
                self.receiving = false;
                if self.waiting {
                    self.waiting = false;
                    let reply = core::str::from_utf8(&self.reply[..self.reply_len]).unwrap_or("Invalid host reply.");
                    self.dio_online = !reply.starts_with("Diogenes offline:");
                    print(reply); print("\n"); self.prompt();
                }
            } else if (byte == b'\n' || (b' '..=b'~').contains(&byte)) && self.reply_len < self.reply.len() {
                self.reply[self.reply_len] = byte; self.reply_len += 1;
            }
            return;
        }
        if self.waiting { return; }
        if byte == b'\n' && self.previous_was_cr {
            self.previous_was_cr = false;
            return;
        }
        self.previous_was_cr = byte == b'\r';
        match byte {
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

    /// Runs a command exactly as if it had been typed at the prompt.
    pub fn run_as_typed(&mut self, text: &str) {
        while self.length > 0 {
            self.feed(8);
        }
        for byte in text.bytes() {
            self.feed(byte);
        }
        self.feed(b'\r');
    }
}
