# Corvus OS: GIF to bounded native frames (design note)

Goal: at build time, turn a brand GIF into a small `.cvan` blob that is `include_bytes!`-ed and
played by the kernel with no heap, O(1) decoder state, writing straight into the ramfb
(XRGB8888) framebuffer. The source GIFs (4-6 MB, LZW, 256 colors, full-res) are far too big to
embed or decode as-is, so all the hard work happens on the host.

## 1. Host tool recommendation
| Tool | Current (checked 2026-11-24) | License | Verdict |
|---|---|---|---|
| Python **Pillow** | 13.0.0 (PyPI) | MIT-CMU (HPND) | **Use this.** Iterates frames, gives per-frame `duration`, composites disposal for you (`ImageSequence` + `convert("RGB")`), `quantize(colors=N, palette=...)` builds one global palette. Permissive license, ~150 lines of script. |
| **ffmpeg** | 9.1 "Gauss" (released 2026-11-20) | LGPL-2.1+ (GPL if built with GPL parts) | Optional pre-step: best scaler and `palettegen/paletteuse` dithering. Only a build-time CLI, so the license does not touch the kernel. |
| **gifsicle** | 1.98 (lcdf.org) | GPL-2.0 | Optional pre-step to crop, `--resize`, `--colors 16`, drop frames. Fine as CLI, no frame export API. |

Pipeline: `gifsicle/ffmpeg (optional resize+crop)` -> `tools/gif2cvan.py` (Pillow) -> `assets/*.cvan` ->
`build.rs` checks budgets and fails the build if exceeded. Pin `Pillow==13.0.0` in a
`requirements.txt` for reproducible output. The script must: (1) composite each frame to full RGB
(GIF disposal), (2) resize to fit MAX_W x MAX_H (LANCZOS, keep aspect, letterbox), (3) quantize
ALL frames against one shared palette (quantize a montage, then `quantize(palette=pal, dither=NONE)`
per frame; dithering kills RLE), (4) drop/merge frames to reach MAX_FRAMES by summing delays,
(5) encode keyframe + deltas, (6) assert total <= MAX_BYTES.

## 2. Binary format `.cvan` v1 (all little-endian)
```
off size field
0   4    magic  "CVAN"
4   1    version = 1
5   1    flags   bit0 = RLE, bit1 = delta frames, bit2 = has transparent index
6   2    width   (<= MAX_W)
8   2    height  (<= MAX_H)
10  2    frame_count (<= MAX_FRAMES)
12  2    loop_count (0 = forever)
14  1    palette_len (1..=256)
15  1    transparent_index (valid if flags.bit2)
16  4    total_len (whole file, for bounds checks)
20  4    reserved = 0
24  4*P  palette: P x u32 XRGB8888 (already in framebuffer order)
..  12*F frame table (below), then frame data
```
Frame table entry (12 bytes): `data_off u32, data_len u32, delay_ms u16, kind u8 (0=key,1=delta), pad u8`.
Frame 0 is always a keyframe; a keyframe every K frames (e.g. 16) keeps restart cheap.

Frame stream: byte-oriented opcodes over 8-bit palette indices (simple; 4-bit packing only pays
off for raw storage):
```
0b00nnnnnn            SKIP  n+1 pixels (unchanged since previous frame; delta frames only)
0b01nnnnnn i0..in     LIT   n+1 literal indices follow
0b1nnnnnnn i          RUN   n+1 copies of index i      (n+1 in 1..=128)
```
A frame is complete when width*height pixels have been produced; the decoder rejects overruns.

## 3. Size budget math
Pixels: 160x160 = 25,600; 256x256 = 65,536. Palette cost is negligible (16c = 64 B, 64c = 256 B).

| Setting | Raw/frame | 48 frames raw | Est. RLE+delta (flat brand art, 4-10x) |
|---|---|---|---|
| 160x160, 16c packed 4bpp | 12.8 KB | 614 KB | 60-150 KB |
| 160x160, 64c 8bpp | 25.6 KB | 1.23 MB | 120-300 KB |
| 256x256, 16c 4bpp | 32.8 KB | 1.57 MB | 160-390 KB |
| 256x256, 64c 8bpp | 65.5 KB | 3.15 MB | 0.3-0.8 MB |

Suggested defaults: `MAX_W=MAX_H=160, MAX_FRAMES=64, MAX_COLORS=16, MAX_BYTES=256 KiB` per asset
(kernel image stays small, QEMU boots fast). Worst case RLE expansion is 1 byte per 64 literals
(+1.6%), so an RLE stream never exceeds `ceil(W*H*65/64)` bytes; `build.rs` can rely on that bound.
Integer-scale at draw time (160 -> 640 at 4x) instead of storing large frames. Measure real numbers by running the script on
`corvus-logo-week.gif` (350 KB) first; the 4-6 MB launch GIF will likely need crop + frame drop.

## 4. Decode loop sketch (no_std, no alloc, O(1) state)
```rust
pub struct Player<'a> { f: &'a [u8], frame: u16 }   // whole state: slice + counter

/// Draws one frame into fb (XRGB8888), scaled by `s`, at (ox, oy). Returns delay in ms.
pub fn draw_frame(a: &Anim, idx: usize, fb: &mut [u32], stride: usize,
                  ox: usize, oy: usize, s: usize) -> Result<u16, Err> {
    let e = a.entry(idx)?;                       // bounds-checked frame table read
    let mut d = a.data(e)?.iter().copied();      // &[u8] iterator, no copies
    let (w, n) = (a.w as usize, a.w as usize * a.h as usize);
    let mut p = 0usize;                          // pixel cursor
    let mut put = |p: usize, ix: u8| {
        let c = a.pal(ix);                       // u32 XRGB from palette slice
        let (x, y) = (ox + (p % w) * s, oy + (p / w) * s);
        for dy in 0..s { let row = (y + dy) * stride + x;
            fb[row..row + s].fill(c); }
    };
    while p < n {
        let op = d.next().ok_or(Err::Trunc)?;
        let len = ((op & 0x3f) as usize) + 1;
        match op >> 6 {
            0b00 => p += len,                                         // SKIP
            0b01 => for _ in 0..len { put(p, d.next().ok_or(Err::Trunc)?); p += 1 }, // LIT
            _    => { let l = ((op & 0x7f) as usize) + 1;             // RUN
                      let ix = d.next().ok_or(Err::Trunc)?;
                      for _ in 0..l { put(p, ix); p += 1 } }
        }
        if p > n { return Err(Err::Overrun) }
    }
    Ok(e.delay_ms)
}
```
Caller loop: `delay = draw_frame(..)?; timer::sleep_ms(delay.max(20)); idx = (idx+1) % count;`
(after the last frame, restart at frame 0 which is a keyframe). Validate the header once at
boot (magic, version, `total_len == bytes.len()`, offsets in range) and `const`-assert sizes.
Embed: `static LOGO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/logo.cvan"));`.

## 5. Reusing existing no_std crates
- **tinygif 0.0.4** (MIT/Apache-2.0, last release 2023-07): `#![no_std]`, no alloc; uses
  `heapless` (LZW table ~20 KB of state, on the stack/struct) and draws via embedded-graphics
  `DrawTarget`. Plays real GIFs directly with per-frame `delay_centis`. Caveats from its README:
  no interlace, "fails on some highly compressed gifs", unmaintained. Good as a quick demo with a
  pre-shrunk GIF (gifsicle `--resize 160x160 --colors 16`), but not O(1) state and pulls in
  embedded-graphics 0.8 + heapless 0.7.
- **embedded-graphics 0.8.2** (MIT OR Apache-2.0, 2026-02): no_std, no alloc. Implementing
  `DrawTarget` for the ramfb (Rgb888) is ~30 lines and unlocks text, shapes, `ImageRaw`
  (raw uncompressed pixels, no palette type) and the sibling crates below. Worth adding anyway
  for the console/UI.
- **tinybmp 0.7.0** (2026-01) and **tinyqoi 0.2.0** (2023-05), both MIT OR Apache-2.0, no_std,
  still images only. QOI per frame is an alternative to custom RLE (good compression for flat
  art, no palette needed), but no animation container and no delta frames.
- **gif 0.14.2** (image-rs, MIT OR Apache-2.0): std/alloc-based; use it host-side (e.g. in
  `build.rs`) if you want the pipeline in Rust instead of Python, not in the kernel.

Recommendation: custom `.cvan` (this note) for kernel playback: smallest, bounded, auditable,
~80 lines of decoder. Add embedded-graphics for drawing primitives; keep tinygif only as a
throwaway prototype.

## Sources
- Pillow on PyPI (version 13.0.0, license MIT-CMU): https://pypi.org/project/pillow/
- FFmpeg downloads/releases (9.1 "Gauss", 2026-11-20): https://ffmpeg.org/download.html
- FFmpeg license: https://ffmpeg.org/doxygen/8.0/md_LICENSE.html
- gifsicle (1.98, GPL-2.0): https://www.lcdf.org/gifsicle/ , changes: https://lcdf.org/gifsicle/changes.html
- tinygif: https://crates.io/crates/tinygif , https://github.com/andelf/tinygif
- embedded-graphics: https://crates.io/crates/embedded-graphics , https://github.com/embedded-graphics/embedded-graphics
- tinybmp: https://crates.io/crates/tinybmp ; tinyqoi: https://crates.io/crates/tinyqoi
- gif (image-rs): https://crates.io/crates/gif , https://github.com/image-rs/image-gif
