//! Web stand-in for src/ramfb.rs: same drawing API (rgb, Framebuffer, Scaled,
//! WIDTH, HEIGHT), but pixels live in a static RGBA buffer that JavaScript
//! copies to a <canvas>. The fw_cfg/ramfb hardware part stays kernel-only.

use core::ptr::addr_of_mut;
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

static mut PIXELS: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

/// Start of the RGBA bytes (canvas ImageData layout).
pub fn pixels_ptr() -> *const u8 {
    addr_of_mut!(PIXELS) as *const u8
}

pub const fn rgb(value: u32) -> Rgb888 {
    Rgb888::new((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

fn pack(color: Rgb888) -> u32 {
    (color.r() as u32) << 16 | (color.g() as u32) << 8 | color.b() as u32
}

pub struct Framebuffer;

impl Framebuffer {
    /// Takes 0x00RRGGBB like the kernel and stores it as little-endian RGBA.
    #[inline]
    pub fn put(&mut self, x: usize, y: usize, value: u32) {
        if x < WIDTH && y < HEIGHT {
            let rgba = 0xff00_0000 | (value & 0xff) << 16 | (value & 0xff00) | (value >> 16 & 0xff);
            unsafe { (*addr_of_mut!(PIXELS))[y * WIDTH + x] = rgba }
        }
    }

    pub fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, value: u32) {
        for row in y..(y + h).min(HEIGHT) {
            for col in x..(x + w).min(WIDTH) {
                self.put(col, row, value);
            }
        }
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb888>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(point, color) in pixels {
            if point.x >= 0 && point.y >= 0 {
                self.put(point.x as usize, point.y as usize, pack(color));
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb888) -> Result<(), Self::Error> {
        let area = area.intersection(&self.bounding_box());
        self.fill(area.top_left.x as usize, area.top_left.y as usize,
                  area.size.width as usize, area.size.height as usize, pack(color));
        Ok(())
    }
}
/// Draws into `inner` with every pixel scaled to an `s`×`s` block, used for
/// the large CORVUS title and the HELP label.
pub struct Scaled<'a> {
    pub inner: &'a mut Framebuffer,
    pub scale: u32,
    pub origin: Point,
}

impl OriginDimensions for Scaled<'_> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32 / self.scale, HEIGHT as u32 / self.scale)
    }
}

impl DrawTarget for Scaled<'_> {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb888>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        let s = self.scale as i32;
        for Pixel(point, color) in pixels {
            let x = self.origin.x + point.x * s;
            let y = self.origin.y + point.y * s;
            if x >= 0 && y >= 0 {
                self.inner.fill(x as usize, y as usize, s as usize, s as usize, pack(color));
            }
        }
        Ok(())
    }
}
