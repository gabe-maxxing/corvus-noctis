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

Web edition (`web/`, not linked into the kernel): wasm-bindgen =0.2.130
(MIT OR Apache-2.0, https://github.com/wasm-bindgen/wasm-bindgen) and its
transitive crates, all MIT OR Apache-2.0. `web/www/apple-touch-icon.png` is an
original mark generated for this repo.

`dio` in the web app loads at runtime, on first use only (not in this repo):
WebLLM @mlc-ai/web-llm 0.2.89 (Apache-2.0, https://github.com/mlc-ai/web-llm)
from esm.run, and the model Qwen2.5-0.5B-Instruct-q4f16_1-MLC (Apache-2.0,
Qwen team; MLC build from https://huggingface.co/mlc-ai). Fallback: the user's
own Ollama on localhost. Diogenes quotes: Hicks 1925 translation, public domain.

Bitmap font: embedded-graphics' built-in mono fonts are generated from public
domain BDF fonts in the XOrg project (embedded-graphics `MIGRATING-0.6-0.7.md`).

Host tools: `tools/gif2frames.py` uses Pillow (MIT-CMU) at build time only;
it never ships in the kernel.

Upstream specs used for drivers written from scratch (no code copied):
QEMU fw_cfg and ramfb (`docs/specs/fw_cfg.rst`, `hw/display/ramfb.c`),
OASIS VIRTIO 1.x (virtio-mmio transport, virtio-input), Arm PL011 TRM.

Artwork: `assets/corvus.gif` and `assets/corvus.cvf` are Gabriel's Corvus
artwork, kept local and gitignored.
