#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

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

const UART_DATA: *mut u32 = 0x0900_0000 as *mut u32;
const UART_FLAGS: *const u32 = 0x0900_0018 as *const u32;

fn send(byte: u8) {
    // These are the PL011 registers on QEMU's virt board.
    unsafe {
        while UART_FLAGS.read_volatile() & (1 << 5) != 0 {
            core::hint::spin_loop();
        }
        UART_DATA.write_volatile(byte.into());
    }
}

fn receive() -> u8 {
    unsafe {
        while UART_FLAGS.read_volatile() & (1 << 4) != 0 {
            core::hint::spin_loop();
        }
        UART_DATA.read_volatile() as u8
    }
}

fn print(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            send(b'\r');
        }
        send(byte);
    }
}

fn banner() {
    print("\x1b[35m   .__\n   (o )>   C O R V U S\n   / )     ARM64 / SERIAL CONSOLE\n  /_/      type help\n\x1b[0m");
}

fn command(line: &str) {
    let line = line.trim();
    match line {
        "" => {}
        "help" => print("help  about  crow  clear  echo <text>\n"),
        "about" => print("Corvus OS 0.1 | Rust bare metal | QEMU virt\n"),
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

#[no_mangle]
extern "C" fn kernel_main() -> ! {
    banner();
    print("corvus> ");
    let mut line = [0; 128];
    let mut length = 0;
    let mut previous_was_cr = false;
    loop {
        let byte = receive();
        if byte == b'\n' && previous_was_cr {
            previous_was_cr = false;
            continue;
        }
        previous_was_cr = byte == b'\r';
        match byte {
            b'\n' | b'\r' => {
                print("\n");
                if let Ok(text) = core::str::from_utf8(&line[..length]) {
                    command(text);
                }
                length = 0;
                print("corvus> ");
            }
            8 | 127 if length > 0 => {
                length -= 1;
                print("\x08 \x08");
            }
            b' '..=b'~' if length < line.len() => {
                line[length] = byte;
                length += 1;
                send(byte);
            }
            _ => {}
        }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    print("\nKernel panic.\n");
    loop {
        unsafe { asm!("wfe", options(nomem, nostack)) };
    }
}
