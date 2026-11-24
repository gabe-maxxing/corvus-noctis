//! QEMU ramfb: a plain guest-RAM framebuffer configured through fw_cfg.
//!
//! Chosen over virtio-gpu for this milestone because it needs no PCI
//! enumeration, no virtqueues and no flush commands: one 28-byte fw_cfg DMA
//! write and the guest draws straight into RAM. See docs/graphics.md.
//!
//! The MMU is off, so virtual == physical and the static buffer's address
//! can be handed to QEMU as is.

use core::ptr::{addr_of, addr_of_mut};
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

const FW_CFG: usize = 0x0902_0000;
const FW_CFG_DATA: *const u8 = FW_CFG as *const u8;
const FW_CFG_SELECTOR: *mut u16 = (FW_CFG + 0x08) as *mut u16;
const FW_CFG_DMA: *mut u64 = (FW_CFG + 0x10) as *mut u64;
const FW_CFG_FILE_DIR: u16 = 0x19;
const DMA_ERROR: u32 = 0x01;
const DMA_SELECT: u32 = 0x08;
const DMA_WRITE: u32 = 0x10;
const FOURCC_XRGB8888: u32 = u32::from_le_bytes(*b"XR24");

#[repr(C, align(4096))]
struct Pixels([u32; WIDTH * HEIGHT]);

static mut PIXELS: Pixels = Pixels([0; WIDTH * HEIGHT]);

#[repr(C, align(16))]
struct DmaAccess {
    control: u32,
    length: u32,
    address: u64,
}

fn select(key: u16) {
    unsafe { FW_CFG_SELECTOR.write_volatile(key.to_be()) }
}

fn read_be(bytes: usize) -> u32 {
    let mut value = 0;
    for _ in 0..bytes {
        value = value << 8 | unsafe { FW_CFG_DATA.read_volatile() } as u32;
    }
    value
}

fn find_file(name: &[u8]) -> Option<u16> {
    select(FW_CFG_FILE_DIR);
    let count = read_be(4);
    for _ in 0..count {
        let _size = read_be(4);
        let key = read_be(2) as u16;
        let _reserved = read_be(2);
        let mut entry = [0u8; 56];
        for byte in entry.iter_mut() {
            *byte = unsafe { FW_CFG_DATA.read_volatile() };
        }
        let len = entry.iter().position(|&b| b == 0).unwrap_or(entry.len());
        if &entry[..len] == name {
            return Some(key);
        }
    }
    None
}

static mut DMA_ACCESS: DmaAccess = DmaAccess { control: 0, length: 0, address: 0 };
static mut RAMFB_CONFIG: [u8; 28] = [0; 28];

/// fw_cfg DMA write. Descriptor and payload are statics written with volatile
/// stores, so the device sees them before the doorbell write.
fn dma_write(key: u16, data: *const u8, length: usize) -> bool {
    unsafe {
        let access = addr_of_mut!(DMA_ACCESS);
        addr_of_mut!((*access).control).write_volatile(((key as u32) << 16 | DMA_SELECT | DMA_WRITE).to_be());
        addr_of_mut!((*access).length).write_volatile((length as u32).to_be());
        addr_of_mut!((*access).address).write_volatile((data as u64).to_be());
        core::arch::asm!("dsb sy");
        FW_CFG_DMA.write_volatile((access as u64).to_be());
        loop {
            let control = u32::from_be(addr_of!((*access).control).read_volatile());
            if control & DMA_ERROR != 0 {
                return false;
            }
            if control == 0 {
                return true;
            }
        }
    }
}

/// Hands the framebuffer to QEMU. Returns false when `-device ramfb` is absent.
pub fn init() -> bool {
    let Some(key) = find_file(b"etc/ramfb") else {
        return false;
    };
    let address = addr_of!(PIXELS) as u64;
    let mut config = [0u8; 28];
    config[0..8].copy_from_slice(&address.to_be_bytes());
    config[8..12].copy_from_slice(&FOURCC_XRGB8888.to_be_bytes());
    config[12..16].copy_from_slice(&0u32.to_be_bytes());
    config[16..20].copy_from_slice(&(WIDTH as u32).to_be_bytes());
    config[20..24].copy_from_slice(&(HEIGHT as u32).to_be_bytes());
    config[24..28].copy_from_slice(&((WIDTH * 4) as u32).to_be_bytes());
    unsafe {
        let target = addr_of_mut!(RAMFB_CONFIG) as *mut u8;
        for (i, byte) in config.iter().enumerate() {
            target.add(i).write_volatile(*byte);
        }
        dma_write(key, target, config.len())
    }
}

pub const fn rgb(value: u32) -> Rgb888 {
    Rgb888::new((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

fn pack(color: Rgb888) -> u32 {
    (color.r() as u32) << 16 | (color.g() as u32) << 8 | color.b() as u32
}

/// embedded-graphics drawing target backed by the ramfb pixels.
pub struct Framebuffer;

impl Framebuffer {
    #[inline]
    pub fn put(&mut self, x: usize, y: usize, value: u32) {
        if x < WIDTH && y < HEIGHT {
            unsafe { (addr_of_mut!(PIXELS.0) as *mut u32).add(y * WIDTH + x).write_volatile(value) }
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
