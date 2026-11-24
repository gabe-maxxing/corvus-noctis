# Corvus OS

A small, live-programming-inspired operating system in Rust. The ARM64 kernel
starts in a Corvus serial shell with `help`, `about`, `crow`, `clear` and
`echo`. It boots to a graphical console (ramfb, 800x600) with a large HELP button
driven by a virtio tablet, mirroring the serial shell. The current board is
QEMU virt. Dependencies: THIRD_PARTY.md. Design: docs/graphics.md.

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
