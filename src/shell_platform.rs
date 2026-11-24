//! Small target-specific effects; the command language itself stays shared.
use crate::{calc, print};

pub enum Effect<'a> { Banner(&'a str), Logo, Draw(usize, usize, usize, usize, u32) }

pub fn render(effect: Effect<'_>) {
    #[cfg(any(target_os = "none", target_arch = "wasm32"))]
    if let Some(console) = crate::screen() {
        match effect {
            Effect::Draw(x,y,w,h,color) => console.draw_rect(x,y,w,h,color),
            Effect::Banner(text) => console.large_banner(text),
            Effect::Logo => {
                console.logo_requested = true;
                #[cfg(target_arch = "wasm32")]
                { crate::state().frame = 0; crate::state().next_frame_ms = 0.0; }
            }
        }
    }
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
    match effect { Effect::Banner(text) => { let _ = text; }, Effect::Logo => {}, Effect::Draw(x,y,w,h,c) => { let _ = (x,y,w,h,c); } }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = performance, js_name = now)]
    fn browser_now() -> f64;

}

pub fn clock(fallback: u64) -> u64 {
    #[cfg(target_arch = "wasm32")]
    { let _ = fallback; browser_now().max(0.0) as u64 }
    #[cfg(not(target_arch = "wasm32"))]
    { fallback }
}

// A primitive-only host import preserves MVP wasm; Rust Closure/JsValue
// callbacks introduce reference-type tables that old Safari cannot execute.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = "import { corvus_timer_tick } from '../../corvus_web.js'; export function corvus_wake(ms) { setTimeout(corvus_timer_tick, ms); }")]
extern "C" { fn corvus_wake(ms: f64); }

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn corvus_timer_tick() { crate::shell().tick(clock(0)); }

pub fn wake_after(ms: u64) {
    #[cfg(target_arch = "wasm32")]
    corvus_wake(ms as f64);
    #[cfg(not(target_arch = "wasm32"))]
    { let _ = ms; }
}

pub fn memory(shell_bytes: usize) {
    print("mem: shell state "); let mut digits = [0; 20];
    print(calc::format(shell_bytes as i64, &mut digits)); print(" bytes; ");
    #[cfg(target_os = "none")]
    {
        unsafe extern "C" { static __bss_start: u8; static __bss_end: u8; }
        let bytes = core::ptr::addr_of!(__bss_end) as usize - core::ptr::addr_of!(__bss_start) as usize;
        print("BSS "); print(calc::format(bytes as i64, &mut digits));
        print(" bytes; stack 65536 bytes reserved; heap none. Free RAM is not tracked.\n");
    }
    #[cfg(not(target_os = "none"))]
    print("host-managed memory; kernel heap none. Free RAM is not tracked.\n");
}

/// Preserve serial type-ahead while polling Ctrl-C between interpreter steps.
pub fn cancelled() -> bool {
    #[cfg(target_os = "none")]
    { crate::uart::interrupt_requested() }
    #[cfg(not(target_os = "none"))]
    { false }
}
