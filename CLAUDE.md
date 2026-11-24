# Corvus OS handoff

You are the technical lead for Corvus OS, an original TempleOS-inspired ARM64
operating system. Start by reading README.md, inspecting the existing Git tree,
and booting the actual kernel with cargo run --release. Keep that path working.

Ship a graphical QEMU milestone next. Compare QEMU ramfb and virtio-gpu and
choose the smallest reproducible implementation. Research reusable no_std
work (start with embedded-graphics), pin any dependency you adopt, and record
the license and origin of imported code. Implement a bitmap terminal with a
dark Corvus palette, legible glyphs, and large on-screen controls. Follow up
with emulated absolute pointer events so buttons invoke the same dispatcher
as serial commands. Keep QEMU debugging available over serial.

In parallel where useful, assign independent research tasks for (1) a
host-side GIF converter with bounded frames and timing, (2) an original
interactive evaluator inspired by HolyC, and (3) the A1474/J71 board's
documented graphics and touch support. Keep iPad research read-only until
there's a verified hardware plan. Give each implementer ownership of distinct
files or branches. Use approved Corvus assets supplied by Gabriel and keep
personal files out of commits. Keep DeepSeek on a host service.

Integrate one small runnable change at a time, commit it, and show proof from
QEMU. First acceptance: boot, draw CORVUS, activate Help with a pointer, and
get the expected output on screen. Report exact commands and observed outputs.
Keep emulator results, Safari results, and native iPad results explicit.

## Status (2026-11-24)

Done: ramfb console, CORVUS title, bounded logo frames, HELP button via
virtio tablet into the same dispatcher (`tools/prove_pointer.py` PASS in QEMU
8.2). Research notes: docs/research/ (gif-frames, holyc-evaluator, ipad-j71).
Shell: `calc <expr>` (src/calc.rs); dispatcher + line editor moved to
src/shell.rs so the kernel and web/ share them. Web edition: web/ (wasm,
canvas, touch HELP, iPad meta), CI + Pages workflows, MIT LICENSE.
Get page (device-aware downloads), install.sh / Corvus-mac.zip /
Corvus-windows.zip on the nightly release (release.yml), `dio` in the web app
(WebLLM, Ollama fallback), diogenes/ persona + voice + X import.
Rule: src/ is Codex's; web-only features live in web/ and JS.
Next: Codex's CVF2 main (a3bad0a) onto this history, Pages enabled.
