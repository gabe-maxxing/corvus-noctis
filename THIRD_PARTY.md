# Third-party code

All dependencies are exact-pinned in `Cargo.toml` (`=x.y.z`) and locked in
`Cargo.lock`. Nothing is vendored or copied into `src/`.

| Crate | Version | License | Origin | Why |
|---|---|---|---|---|
| embedded-graphics | =0.8.2 | MIT OR Apache-2.0 | https://github.com/embedded-graphics/embedded-graphics (crates.io) | `DrawTarget` trait, text layout, `FONT_10X20` |
| embedded-graphics-core | 0.4.1 | MIT OR Apache-2.0 | same repo | pulled in by embedded-graphics |
| az | 1.2.1 | MIT OR Apache-2.0 | https://gitlab.com/tspiteri/az | transitive |
| byteorder | 1.5.0 | Unlicense OR MIT | https://github.com/BurntSushi/byteorder | transitive |
| float-cmp | 0.9.0 | MIT | https://github.com/mikedilger/float-cmp | transitive |
| micromath | 2.2.0 | Apache-2.0 OR MIT | https://github.com/tarcieri/micromath | transitive |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | https://github.com/rust-num/num-traits | transitive |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | https://github.com/cuviper/autocfg | build-time only |

Bitmap font: embedded-graphics' built-in mono fonts are generated from public
domain BDF fonts in the XOrg project (embedded-graphics `MIGRATING-0.6-0.7.md`).

Host tools: `tools/gif2frames.py` uses Pillow (MIT-CMU) at build time only;
it never ships in the kernel.

Upstream specs used for drivers written from scratch (no code copied):
QEMU fw_cfg and ramfb (`docs/specs/fw_cfg.rst`, `hw/display/ramfb.c`),
OASIS VIRTIO 1.x (virtio-mmio transport, virtio-input), Arm PL011 TRM.

Artwork: `assets/corvus.gif` and `assets/corvus.cvf` are Gabriel's Corvus
artwork, kept local and gitignored.
