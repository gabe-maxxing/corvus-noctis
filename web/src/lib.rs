//! Corvus OS, web edition. Compiles the kernel's own console (src/console.rs),
//! shell (src/shell.rs), calculator (src/calc.rs) and logo frames
//! (src/frames.rs) to wasm. JavaScript only moves pixels and input:
//! framebuffer -> <canvas>, keys -> Shell::feed, taps -> the same HELP path
//! the kernel takes for virtio-tablet events.

#![allow(dead_code)]

#[path = "../../src/calc.rs"]
mod calc;
#[path = "../../src/console.rs"]
mod console;
#[path = "../../src/frames.rs"]
mod frames;
mod ramfb;
#[path = "../../src/shell.rs"]
mod shell;

use core::cell::UnsafeCell;
use console::Console;
use frames::Frames;
use shell::{banner, Shell};
use wasm_bindgen::prelude::*;

/// wasm32-unknown-unknown is single-threaded: plain global state is sound.
struct Global<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}

static CONSOLE: Global<Option<Console>> = Global(UnsafeCell::new(None));
static SHELL: Global<Shell> = Global(UnsafeCell::new(Shell::new()));
static STATE: Global<State> = Global(UnsafeCell::new(State { logo: None, frame: 0, next_frame_ms: 0.0, pressed: false }));

struct State {
    logo: Option<Frames>,
    frame: usize,
    next_frame_ms: f64,
    pressed: bool,
}

fn screen() -> Option<&'static mut Console> {
    unsafe { (*CONSOLE.0.get()).as_mut() }
}

fn shell() -> &'static mut Shell {
    unsafe { &mut *SHELL.0.get() }
}

fn state() -> &'static mut State {
    unsafe { &mut *STATE.0.get() }
}

/// Shell output. The kernel also mirrors this to serial; here the canvas is
/// the only terminal.
pub(crate) fn print(text: &str) {
    if let Some(console) = screen() {
        for byte in text.bytes() {
            console.write_byte(byte);
        }
    }
}

#[wasm_bindgen]
pub fn boot() {
    let logo = Frames::load();
    let mut console = Console::new();
    console.draw_chrome(logo.as_ref());
    unsafe { *CONSOLE.0.get() = Some(console) };
    state().logo = logo;
    banner();
    print("web edition: try  calc 6*7  or  dio what is virtue?\n");
    print("corvus> ");
}

/// Prints text straight to the console, for web-only commands such as `dio`
/// that run in JavaScript. Non-ASCII is the caller's job.
#[wasm_bindgen]
pub fn write(text: &str) {
    print(text);
}

#[wasm_bindgen]
pub fn width() -> u32 {
    ramfb::WIDTH as u32
}

#[wasm_bindgen]
pub fn height() -> u32 {
    ramfb::HEIGHT as u32
}

/// Address of the RGBA framebuffer inside wasm memory.
#[wasm_bindgen]
pub fn pixels() -> u32 {
    ramfb::pixels_ptr() as u32
}

/// One byte of keyboard input, exactly as the serial port would deliver it.
#[wasm_bindgen]
pub fn key(byte: u8) {
    shell().feed(byte);
}

/// Pointer/touch in framebuffer pixels. Mirrors the kernel's tablet handler:
/// press inside HELP draws it pressed and runs `help` as if typed.
#[wasm_bindgen]
pub fn pointer(x: u32, y: u32, down: bool) {
    let s = state();
    if down && Console::hits_help(x as usize, y as usize) {
        s.pressed = true;
        if let Some(console) = screen() {
            console.draw_help_button(true);
        }
        shell().run_as_typed("help");
    } else if !down && s.pressed {
        s.pressed = false;
        if let Some(console) = screen() {
            console.draw_help_button(false);
        }
    }
}

/// Advances the logo animation; returns true when pixels changed.
#[wasm_bindgen]
pub fn tick(now_ms: f64) -> bool {
    let s = state();
    let (Some(frames), Some(console)) = (s.logo.as_ref(), screen()) else {
        return false;
    };
    if frames.count < 2 || now_ms < s.next_frame_ms {
        return false;
    }
    s.frame = (s.frame + 1) % frames.count;
    console.frame(frames, s.frame);
    s.next_frame_ms = now_ms + frames.delay_ms as f64;
    true
}
