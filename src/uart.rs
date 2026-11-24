//! PL011 UART on QEMU's virt board (0x0900_0000). The serial shell and all
//! QEMU debugging go through here.

const UART_DATA: *mut u32 = 0x0900_0000 as *mut u32;
const UART_FLAGS: *const u32 = 0x0900_0018 as *const u32;
const TX_FULL: u32 = 1 << 5;
const RX_EMPTY: u32 = 1 << 4;

pub fn send(byte: u8) {
    unsafe {
        while UART_FLAGS.read_volatile() & TX_FULL != 0 {
            core::hint::spin_loop();
        }
        UART_DATA.write_volatile(byte.into());
    }
}

/// Non-blocking receive so the main loop can also poll the pointer.
pub fn try_receive() -> Option<u8> {
    unsafe {
        if UART_FLAGS.read_volatile() & RX_EMPTY != 0 {
            None
        } else {
            Some(UART_DATA.read_volatile() as u8)
        }
    }
}

/// Serial-only text (debug traces). Translates `\n` to `\r\n`.
pub fn print(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            send(b'\r');
        }
        send(byte);
    }
}

pub fn print_u32(mut value: u32) {
    let mut digits = [0u8; 10];
    let mut n = 0;
    loop {
        digits[n] = b'0' + (value % 10) as u8;
        n += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while n > 0 {
        n -= 1;
        send(digits[n]);
    }
}

/// Framing uses LF verbatim; unlike terminal text, do not inject CR.
pub fn protocol(text: &str) {
    for byte in text.bytes() { send(byte); }
}
