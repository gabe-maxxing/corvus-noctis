//! Bounded, palette-indexed animation frames ("CVF1"), produced at build time
//! by tools/gif2frames.py from an approved Corvus GIF and embedded with
//! include_bytes!. No heap: the kernel reads the bytes in place.
//!
//! Layout (little-endian):
//!   0  "CVF1"
//!   4  width u16, height u16, frames u16, colors u16, delay_ms u16, reserved u16
//!   16 palette: colors × u32 (0x00RRGGBB)
//!   .. pixels: frames × width × height palette indices (u8)

use crate::ramfb::Framebuffer;

pub const MAX_SIDE: usize = 128;
pub const MAX_FRAMES: usize = 32;
pub const MAX_BYTES: usize = 512 * 1024;

static DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/corvus.cvf"));

pub struct Frames {
    pub width: usize,
    pub height: usize,
    pub count: usize,
    pub delay_ms: u32,
    colors: usize,
}

fn u16_at(offset: usize) -> usize {
    u16::from_le_bytes([DATA[offset], DATA[offset + 1]]) as usize
}

impl Frames {
    /// Validates the embedded asset against the bounds; `None` if missing or invalid.
    pub fn load() -> Option<Frames> {
        if DATA.len() < 16 || &DATA[..4] != b"CVF1" {
            return None;
        }
        let frames = Frames {
            width: u16_at(4),
            height: u16_at(6),
            count: u16_at(8),
            colors: u16_at(10),
            delay_ms: u16_at(12).max(20) as u32,
        };
        let expected = 16 + frames.colors * 4 + frames.count * frames.width * frames.height;
        let bounded = frames.width <= MAX_SIDE && frames.height <= MAX_SIDE && frames.count >= 1
            && frames.count <= MAX_FRAMES && (1..=256).contains(&frames.colors)
            && expected == DATA.len() && expected <= MAX_BYTES;
        bounded.then_some(frames)
    }

    pub fn draw(&self, index: usize, fb: &mut Framebuffer, x: usize, y: usize) {
        let palette = 16;
        let pixels = palette + self.colors * 4 + (index % self.count) * self.width * self.height;
        for row in 0..self.height {
            for col in 0..self.width {
                let entry = (DATA[pixels + row * self.width + col] as usize).min(self.colors - 1);
                let p = palette + entry * 4;
                let color = u32::from_le_bytes([DATA[p], DATA[p + 1], DATA[p + 2], DATA[p + 3]]);
                fb.put(x + col, y + row, color);
            }
        }
    }
}
