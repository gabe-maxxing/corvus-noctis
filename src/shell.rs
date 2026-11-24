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
}

impl Shell {
    pub const fn new() -> Shell {
        Shell { line: [0; 128], length: 0, previous_was_cr: false }
    }

    fn submit(&mut self) {
        print("\n");
        let mut copy = [0u8; 128];
        copy[..self.length].copy_from_slice(&self.line[..self.length]);
        if let Ok(text) = core::str::from_utf8(&copy[..self.length]) {
            command(text);
        }
        self.length = 0;
        print("corvus> ");
    }

    pub fn feed(&mut self, byte: u8) {
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
