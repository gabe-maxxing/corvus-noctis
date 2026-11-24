//! Polled virtio-input driver for QEMU's `virtio-tablet-device` on the virt
//! board's virtio-mmio transports (modern / version 2 interface, so QEMU runs
//! with `-global virtio-mmio.force-legacy=false`).
//!
//! The tablet reports absolute coordinates in 0..=32767 on both axes, which
//! maps to touch semantics later. No interrupts: the main loop polls.

use core::ptr::{addr_of, addr_of_mut};

const MMIO_BASE: usize = 0x0a00_0000;
const MMIO_STRIDE: usize = 0x200;
const MMIO_SLOTS: usize = 32;
const MAGIC: u32 = 0x7472_6976; // "virt"
const DEVICE_INPUT: u32 = 18;

const MAGIC_VALUE: usize = 0x000;
const VERSION: usize = 0x004;
const DEVICE_ID: usize = 0x008;
const DEVICE_FEATURES_SEL: usize = 0x014;
const DRIVER_FEATURES: usize = 0x020;
const DRIVER_FEATURES_SEL: usize = 0x024;
const QUEUE_SEL: usize = 0x030;
const QUEUE_NUM_MAX: usize = 0x034;
const QUEUE_NUM: usize = 0x038;
const QUEUE_READY: usize = 0x044;
const QUEUE_NOTIFY: usize = 0x050;
const INTERRUPT_STATUS: usize = 0x060;
const INTERRUPT_ACK: usize = 0x064;
const STATUS: usize = 0x070;
const QUEUE_DESC_LOW: usize = 0x080;
const QUEUE_DESC_HIGH: usize = 0x084;
const QUEUE_DRIVER_LOW: usize = 0x090;
const QUEUE_DRIVER_HIGH: usize = 0x094;
const QUEUE_DEVICE_LOW: usize = 0x0a0;
const QUEUE_DEVICE_HIGH: usize = 0x0a4;
const CONFIG: usize = 0x100;

const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_DRIVER_OK: u32 = 4;
const STATUS_FEATURES_OK: u32 = 8;
const DESC_F_WRITE: u16 = 2;
const CFG_ID_NAME: u8 = 1;

pub const EV_KEY: u16 = 1;
pub const EV_ABS: u16 = 3;
pub const ABS_X: u16 = 0;
pub const ABS_Y: u16 = 1;
pub const BTN_LEFT: u16 = 0x110;
pub const BTN_TOUCH: u16 = 0x14a;
pub const ABS_MAX: u32 = 0x7fff;

const QUEUE_SIZE: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
struct Descriptor {
    address: u64,
    length: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct Available {
    flags: u16,
    index: u16,
    ring: [u16; QUEUE_SIZE],
    used_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UsedElement {
    id: u32,
    length: u32,
}

#[repr(C, align(4))]
struct Used {
    flags: u16,
    index: u16,
    ring: [UsedElement; QUEUE_SIZE],
    avail_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Event {
    pub kind: u16,
    pub code: u16,
    pub value: u32,
}

#[repr(C, align(4096))]
struct Queue {
    descriptors: [Descriptor; QUEUE_SIZE],
    available: Available,
    _pad: [u8; 1024],
    used: Used,
    events: [Event; QUEUE_SIZE],
}

static mut QUEUE: Queue = Queue {
    descriptors: [Descriptor { address: 0, length: 0, flags: 0, next: 0 }; QUEUE_SIZE],
    available: Available { flags: 0, index: 0, ring: [0; QUEUE_SIZE], used_event: 0 },
    _pad: [0; 1024],
    used: Used { flags: 0, index: 0, ring: [UsedElement { id: 0, length: 0 }; QUEUE_SIZE], avail_event: 0 },
    events: [Event { kind: 0, code: 0, value: 0 }; QUEUE_SIZE],
};

pub struct Tablet {
    base: usize,
    last_used: u16,
    pub name: [u8; 32],
    pub name_len: usize,
}

fn read(base: usize, offset: usize) -> u32 {
    unsafe { ((base + offset) as *const u32).read_volatile() }
}

fn write(base: usize, offset: usize, value: u32) {
    unsafe { ((base + offset) as *mut u32).write_volatile(value) }
}

fn barrier() {
    unsafe { core::arch::asm!("dsb sy") }
}

impl Tablet {
    /// Finds and starts the first virtio-input device. `None` if absent.
    pub fn init() -> Option<Tablet> {
        let base = (0..MMIO_SLOTS).map(|slot| MMIO_BASE + slot * MMIO_STRIDE).find(|&base| {
            read(base, MAGIC_VALUE) == MAGIC && read(base, VERSION) == 2 && read(base, DEVICE_ID) == DEVICE_INPUT
        })?;

        write(base, STATUS, 0);
        write(base, STATUS, STATUS_ACKNOWLEDGE);
        write(base, STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);
        // Accept only VIRTIO_F_VERSION_1 (feature bit 32).
        write(base, DEVICE_FEATURES_SEL, 1);
        write(base, DRIVER_FEATURES_SEL, 0);
        write(base, DRIVER_FEATURES, 0);
        write(base, DRIVER_FEATURES_SEL, 1);
        write(base, DRIVER_FEATURES, 1);
        write(base, STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK);
        if read(base, STATUS) & STATUS_FEATURES_OK == 0 {
            return None;
        }

        write(base, QUEUE_SEL, 0);
        if (read(base, QUEUE_NUM_MAX) as usize) < QUEUE_SIZE {
            return None;
        }
        write(base, QUEUE_NUM, QUEUE_SIZE as u32);
        unsafe {
            let queue = addr_of_mut!(QUEUE);
            for i in 0..QUEUE_SIZE {
                (*queue).descriptors[i] = Descriptor {
                    address: addr_of!((*queue).events[i]) as u64,
                    length: core::mem::size_of::<Event>() as u32,
                    flags: DESC_F_WRITE,
                    next: 0,
                };
                (*queue).available.ring[i] = i as u16;
            }
            barrier();
            addr_of_mut!((*queue).available.index).write_volatile(QUEUE_SIZE as u16);
            let split = |address: u64| (address as u32, (address >> 32) as u32);
            let (lo, hi) = split(addr_of!((*queue).descriptors) as u64);
            write(base, QUEUE_DESC_LOW, lo);
            write(base, QUEUE_DESC_HIGH, hi);
            let (lo, hi) = split(addr_of!((*queue).available) as u64);
            write(base, QUEUE_DRIVER_LOW, lo);
            write(base, QUEUE_DRIVER_HIGH, hi);
            let (lo, hi) = split(addr_of!((*queue).used) as u64);
            write(base, QUEUE_DEVICE_LOW, lo);
            write(base, QUEUE_DEVICE_HIGH, hi);
        }
        write(base, QUEUE_READY, 1);
        write(base, STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK);
        barrier();
        write(base, QUEUE_NOTIFY, 0);

        let mut tablet = Tablet { base, last_used: 0, name: [0; 32], name_len: 0 };
        tablet.read_name();
        Some(tablet)
    }

    fn read_name(&mut self) {
        let config = self.base + CONFIG;
        unsafe {
            (config as *mut u8).write_volatile(CFG_ID_NAME);
            ((config + 1) as *mut u8).write_volatile(0);
            let size = ((config + 2) as *const u8).read_volatile() as usize;
            self.name_len = size.min(self.name.len());
            for i in 0..self.name_len {
                self.name[i] = ((config + 8 + i) as *const u8).read_volatile();
            }
            while self.name_len > 0 && self.name[self.name_len - 1] == 0 {
                self.name_len -= 1;
            }
        }
    }

    /// Returns the next input event, recycling its buffer to the device.
    pub fn poll(&mut self) -> Option<Event> {
        unsafe {
            let queue = addr_of_mut!(QUEUE);
            let used_index = addr_of!((*queue).used.index).read_volatile();
            if used_index == self.last_used {
                return None;
            }
            barrier();
            let slot = self.last_used as usize % QUEUE_SIZE;
            let id = addr_of!((*queue).used.ring[slot].id).read_volatile() as usize % QUEUE_SIZE;
            let event = addr_of!((*queue).events[id]).read_volatile();
            self.last_used = self.last_used.wrapping_add(1);

            let available = addr_of!((*queue).available.index).read_volatile();
            addr_of_mut!((*queue).available.ring[available as usize % QUEUE_SIZE]).write_volatile(id as u16);
            barrier();
            addr_of_mut!((*queue).available.index).write_volatile(available.wrapping_add(1));
            barrier();
            write(self.base, QUEUE_NOTIFY, 0);
            let pending = read(self.base, INTERRUPT_STATUS);
            if pending != 0 {
                write(self.base, INTERRUPT_ACK, pending);
            }
            Some(event)
        }
    }
}
