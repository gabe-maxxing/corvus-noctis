#![no_std]
#![no_main]

mod console;
mod frames;
mod ramfb;
mod tablet;
mod uart;

use core::arch::{asm, global_asm};
use core::cell::UnsafeCell;
use core::panic::PanicInfo;

use console::Console;
use frames::Frames;
use tablet::{Tablet, ABS_MAX, ABS_X, ABS_Y, BTN_LEFT, BTN_TOUCH, EV_ABS, EV_KEY};

global_asm!(
    ".section .text.boot,\"ax\"",
    ".global _start",
    "_start:",
    "ldr x0, =__stack_top",
    "mov sp, x0",
    "ldr x0, =__bss_start",
    "ldr x1, =__bss_end",
    "1:",
    "cmp x0, x1",
    "b.hs 2f",
    "str xzr, [x0], #8",
    "b 1b",
    "2:",
    "bl kernel_main",
);

/// Single-core, interrupt-free kernel: plain global state is sound here.
struct Global<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}

static CONSOLE: Global<Option<Console>> = Global(UnsafeCell::new(None));

fn screen() -> Option<&'static mut Console> {
    unsafe { (*CONSOLE.0.get()).as_mut() }
}

/// Shell output: serial (for QEMU debugging and the browser console) and the
/// bitmap terminal receive the same bytes.
fn print(text: &str) {
    uart::print(text);
    if let Some(console) = screen() {
        for byte in text.bytes() {
            console.write_byte(byte);
        }
    }
}

fn banner() {
    print("\x1b[35m   .__\n   (o )>   C O R V U S\n   / )     ARM64 / SERIAL CONSOLE\n  /_/      type help\n\x1b[0m");
}

/// The one command dispatcher. Serial input and the on-screen HELP button
/// both end up here.
fn command(line: &str) {
    let line = line.trim();
    match line {
        "" => {}
        "help" => print("help  about  crow  clear  echo <text>\n"),
        "about" => print("Corvus OS 0.2 | Rust bare metal | QEMU virt | ramfb + virtio-tablet\n"),
        "crow" => banner(),
        "clear" => print("\x1b[2J\x1b[H"),
        _ => {
            if let Some(message) = line.strip_prefix("echo ") {
                print(message);
                print("\n");
            } else {
                print("Unknown command. Try help.\n");
            }
        }
    }
}

fn ticks() -> (u64, u64) {
    let (count, frequency): (u64, u64);
    unsafe {
        asm!("mrs {}, cntpct_el0", out(reg) count, options(nomem, nostack));
        asm!("mrs {}, cntfrq_el0", out(reg) frequency, options(nomem, nostack));
    }
    (count, frequency)
}

struct Shell {
    line: [u8; 128],
    length: usize,
    previous_was_cr: bool,
}

impl Shell {
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

    fn feed(&mut self, byte: u8) {
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
    fn run_as_typed(&mut self, text: &str) {
        while self.length > 0 {
            self.feed(8);
        }
        for byte in text.bytes() {
            self.feed(byte);
        }
        self.feed(b'\r');
    }
}

#[no_mangle]
extern "C" fn kernel_main() -> ! {
    let display = ramfb::init();
    let logo = if display { Frames::load() } else { None };
    if display {
        let mut console = Console::new();
        console.draw_chrome(logo.as_ref());
        unsafe { *CONSOLE.0.get() = Some(console) };
    }
    let mut pointer = Tablet::init();

    banner();
    uart::print(if display { "[ramfb] 800x600 XRGB8888 up\n" } else { "[ramfb] not present, serial only\n" });
    match &pointer {
        Some(tablet) => {
            uart::print("[input] ");
            uart::print(core::str::from_utf8(&tablet.name[..tablet.name_len]).unwrap_or("virtio-input"));
            uart::print("\n");
        }
        None => uart::print("[input] no virtio tablet\n"),
    }
    if let Some(frames) = &logo {
        uart::print("[logo] ");
        uart::print_u32(frames.count as u32);
        uart::print(" bounded frames\n");
    }
    print("corvus> ");

    let mut shell = Shell { line: [0; 128], length: 0, previous_was_cr: false };
    let (mut x, mut y) = (0u32, 0u32);
    let mut pressed = false;
    let mut frame = 0usize;
    let mut next_frame = 0u64;

    loop {
        if let Some(byte) = uart::try_receive() {
            shell.feed(byte);
        }

        if let Some(event) = pointer.as_mut().and_then(Tablet::poll) {
            match (event.kind, event.code) {
                (EV_ABS, ABS_X) => x = event.value.min(ABS_MAX) * ramfb::WIDTH as u32 / (ABS_MAX + 1),
                (EV_ABS, ABS_Y) => y = event.value.min(ABS_MAX) * ramfb::HEIGHT as u32 / (ABS_MAX + 1),
                (EV_KEY, BTN_LEFT | BTN_TOUCH) => {
                    let inside = Console::hits_help(x as usize, y as usize);
                    if event.value == 1 && inside {
                        pressed = true;
                        if let Some(console) = screen() {
                            console.draw_help_button(true);
                        }
                        uart::print("\n[pointer] HELP pressed at ");
                        uart::print_u32(x);
                        uart::print(",");
                        uart::print_u32(y);
                        uart::print("\ncorvus> ");
                        shell.run_as_typed("help");
                    } else if event.value == 0 && pressed {
                        pressed = false;
                        if let Some(console) = screen() {
                            console.draw_help_button(false);
                        }
                    }
                }
                _ => {}
            }
        }

        if let (Some(frames), Some(console)) = (&logo, screen()) {
            if frames.count > 1 {
                let (now, frequency) = ticks();
                if now >= next_frame {
                    frame = (frame + 1) % frames.count;
                    console.frame(frames, frame);
                    next_frame = now + frequency * frames.delay_ms as u64 / 1000;
                }
            }
        }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    uart::print("\nKernel panic.\n");
    loop {
        unsafe { asm!("wfe", options(nomem, nostack)) };
    }
}
