# Graphics milestone: ramfb vs virtio-gpu

**Decision: ramfb for the QEMU milestone.** It is the simplest path we could
verify end to end, and it maps well onto the iPad, where iBoot/pongoOS also
leave a plain linear framebuffer (see research/ipad-j71.md).

| | ramfb | virtio-gpu |
|---|---|---|
| Discovery | fw_cfg file `etc/ramfb` (MMIO 0x0902_0000) | virtio-mmio slot (needs `virtio-gpu-device`) or PCI |
| Setup | one 28-byte fw_cfg DMA write (addr, fourcc, w, h, stride) | feature negotiation, control queue, RESOURCE_CREATE_2D, ATTACH_BACKING, SET_SCANOUT |
| Per frame | none, QEMU scans guest RAM | TRANSFER_TO_HOST_2D + RESOURCE_FLUSH per update |
| Code here | ~120 lines (`src/ramfb.rs`) | estimated 400+ lines plus a request/response queue |
| Resolution change | fixed at init | runtime (GET_DISPLAY_INFO, EDID) |
| Acceleration / cursor | no | cursor queue, virgl later |

What virtio-gpu would add (runtime modes, hardware cursor, damage tracking)
doesn't matter until after the console works. The virtio-mmio code written for
the tablet (`src/tablet.rs`) is the base for a later virtio-gpu driver.

## Pointer

`virtio-tablet-device` on virtio-mmio, polled, modern transport
(`-global virtio-mmio.force-legacy=false`). Absolute 0..32767 axes are scaled
to 800x600. `BTN_LEFT` and `BTN_TOUCH` both count as a press, so later touch
input can use the same path.

## Constraints we hit

- MMU and caches are off, so VA == PA. The target is built with strict
  alignment.
- fw_cfg DMA descriptors must be written with volatile stores into static
  memory before the doorbell write. Stack buffers let the compiler sink the
  stores and QEMU read zeros (our first boot showed "ramfb not present").
- Ubuntu's QEMU prints `Could not open option rom 'vgabios-ramfb.bin'`. It's
  harmless on ARM virt.
