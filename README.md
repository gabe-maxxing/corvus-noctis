<a href="https://gabe-maxxing.github.io/corvus-noctis/">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/gabe-maxxing/gabe-maxxing/main/assets/corvus-flight-dark.gif">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/gabe-maxxing/gabe-maxxing/main/assets/corvus-flight-light.gif">
    <img align="right" src="https://raw.githubusercontent.com/gabe-maxxing/gabe-maxxing/main/assets/corvus-flight-light.gif" width="144" alt="Corvus in flight">
  </picture>
</a>

<h1>Corvus Noctis</h1>

<p><b>An operating system you can talk to.</b> Written from scratch in Rust in the spirit of TempleOS: immediate, playful, and live all the way down. Boot it, tap it, type to it.</p>

[![CI](https://github.com/gabe-maxxing/corvus-noctis/actions/workflows/ci.yml/badge.svg)](https://github.com/gabe-maxxing/corvus-noctis/actions/workflows/ci.yml)
[![Pages](https://github.com/gabe-maxxing/corvus-noctis/actions/workflows/pages.yml/badge.svg)](https://gabe-maxxing.github.io/corvus-noctis/)
[![License: MIT](https://img.shields.io/badge/license-MIT-5b3f8c)](LICENSE)
[![Rust no_std](https://img.shields.io/badge/rust-no__std%20ARM64-ffd166)](https://doc.rust-lang.org/rustc/platform-support/aarch64-unknown-none.html)

**[▶ Try it in your browser](https://gabe-maxxing.github.io/corvus-noctis/)** · [Download](https://github.com/gabe-maxxing/corvus-noctis/releases/tag/nightly) · [Corvus](https://corvus.gent) · [#jointhearmy](https://x.com/gabe_maxxing)

| You have | You get |
| :--- | :--- |
| iPhone, iPad, Android | The web edition. Share → Add to Home Screen. |
| macOS, Windows, Linux | The real kernel in a VM (QEMU). Build steps below; one-click downloads are coming. |
| A spare PC | Dual boot. Experimental and coming; the x86_64 port is planned. |

**Why:** a computer you can shape yourself, for builders. Not a Linux distro: its own kernel, its own shell, its own crow.

---

## About

A small, live-programming-inspired operating system in Rust. The ARM64 kernel
starts in a Corvus serial shell with `help`, `about`, `crow`, `clear` and
`echo`. It boots to a graphical console (ramfb, 800x600) with a large HELP button
driven by a virtio tablet, mirroring the serial shell. The current board is
QEMU virt. Dependencies: THIRD_PARTY.md. Design: docs/graphics.md.

## Web edition

Get Corvus: https://gabe-maxxing.github.io/corvus-noctis/ picks the right
download for your device (web app on iPad/Android, Corvus-mac.zip,
Corvus-windows.zip, or `curl -fsSL .../install.sh | sh` on Linux/Unix). All
desktop builds run Corvus in a QEMU VM; downloads come from the `nightly`
release, rebuilt by `.github/workflows/release.yml` after every green CI run.

The web app (`/app/`) is the same console, shell and `calc` compiled to wasm
(iPad Safari: Share -> Add to Home Screen for fullscreen). Build locally:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version =0.2.130 --locked
cd web && cargo build --release
wasm-bindgen --target web --no-typescript --out-dir www/app/pkg target/wasm32-unknown-unknown/release/corvus_web.wasm
python3 -m http.server -d www 8000
```

CI (`.github/workflows/ci.yml`) builds the kernel, runs the pointer proof and
checks the installers; Pages
(`.github/workflows/pages.yml`) deploys `web/` only after CI is green.

## Boot

Install Rust via rustup and QEMU (Homebrew: `brew install qemu`; Debian:
`sudo apt install qemu-system-arm`).

```sh
rustup target add aarch64-unknown-none-softfloat
cargo run --release
```

A QEMU window shows the console; the shell stays on this terminal. Use
Ctrl-C to exit. `CORVUS_DISPLAY=none cargo run --release` runs headless.

Optional logo animation from your own GIF (gitignored, needs Pillow):

```sh
python3 tools/gif2frames.py assets/corvus.gif assets/corvus.cvf
```

Acceptance proof (typed `help` == HELP button click via QMP, saves
screendumps to target/proof/):

```sh
python3 tools/prove_pointer.py
```

## Try the touch console

With Python 3 and the same requirements:

```sh
python3 tools/touch.py
```

Open http://127.0.0.1:8080 on the host. To try Safari on your restored iPad,
start the bridge with `python3 tools/touch.py --bind YOUR_MAC_LAN_IP` and
open `http://YOUR_MAC_LAN_IP:8080` on the same trusted Wi-Fi network.
The bridge is an unauthenticated interface to the shared guest shell.
It passes input to QEMU's serial port. Commands execute inside the Rust
kernel. The web page uses plain HTML, CSS and XMLHttpRequest.

Create the `assets` directory, put your animation at `assets/corvus.gif` and reload. This path is gitignored
so personal artwork stays local. Safari decodes the GIF in this milestone.

## From emulator to iPad

1. QEMU graphical console with bitmap text, then absolute pointer input and
   large controls. Keep the existing serial path for debugging.
2. An interactive command evaluator and a small built-in editor inspired by
   the immediacy of HolyC. Develop original syntax and Corvus visuals.
3. Convert approved GIFs into bounded frame buffers at build time. Prove
   playback without a browser in QEMU before targeting hardware.
4. Track the A1474 / iPad4,1 / J71 separately. Investigate documented handoff,
   framebuffer, touch and power support. Orchard lists J71 as experimental.
   Native boot and hardware input have yet to be verified for this kernel.
5. Connect DeepSeek via a separate host service after the console works.
   Model inference belongs on the host for this iPad.

## Upstream references

- [TempleOS Foundation](https://templeos.net/) for the public-domain HolyC
  project and its interactive programming model.
- [QEMU virt board](https://www.qemu.org/docs/master/system/arm/virt.html)
  for the virtual platform and its serial, graphics and input options.
- [Rust bare-metal ARM64 targets](https://doc.rust-lang.org/rustc/platform-support/aarch64-unknown-none.html)
  for the compiler target.
- [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics),
  a MIT/Apache-2.0 candidate for text and graphics.
- [Orchard compatibility](https://github.com/OrchardLinux/orchard/blob/main/docs/compatibility.md)
  for tracking J71 device status.

These are references for the initial implementation. Before importing upstream
code, record its pinned revision and license and include required notices.

## License

MIT, see LICENSE. Personal Corvus artwork in `assets/` is not part of the repo.
