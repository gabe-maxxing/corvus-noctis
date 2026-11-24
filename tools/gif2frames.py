"""Convert an approved Corvus GIF into bounded CVF1 frames for the kernel.

    python3 tools/gif2frames.py assets/corvus.gif assets/corvus.cvf

Picks evenly spaced frames, composites them (Pillow applies GIF disposal),
scales to a square, and quantizes all frames to one shared palette. Fails if
the result breaks the kernel's bounds (src/frames.rs). Needs Pillow.
Format notes: src/frames.rs. Design notes: docs/research/gif-frames.md.
"""

import argparse
import struct
import sys

from PIL import Image

MAX_SIDE = 128
MAX_FRAMES = 32
MAX_BYTES = 512 * 1024


def convert(source, size, frames, colors, background):
    image = Image.open(source)
    total = getattr(image, "n_frames", 1)
    count = min(frames, total)
    picks = [round(i * total / count) for i in range(count)]
    durations = []
    for i in range(total):
        image.seek(i)
        durations.append(image.info.get("duration", 100) or 100)
    rendered = []
    for index in picks:
        image.seek(index)
        rgba = image.convert("RGBA")
        canvas = Image.new("RGBA", rgba.size, background + (255,))
        canvas.alpha_composite(rgba)
        rendered.append(canvas.convert("RGB").resize((size, size), Image.Resampling.LANCZOS))

    # One palette for every frame: quantize a vertical strip of all frames.
    strip = Image.new("RGB", (size, size * count))
    for i, frame in enumerate(rendered):
        strip.paste(frame, (0, i * size))
    indexed = strip.quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    palette = indexed.getpalette()[: colors * 3]
    used = max(indexed.tobytes()) + 1
    delay = round(sum(durations) / count)

    out = bytearray(b"CVF1")
    out += struct.pack("<6H", size, size, count, used, delay, 0)
    for i in range(used):
        r, g, b = palette[i * 3 : i * 3 + 3]
        out += struct.pack("<I", r << 16 | g << 8 | b)
    out += indexed.tobytes()
    return out, count, used, delay


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("source")
    parser.add_argument("output")
    parser.add_argument("--size", type=int, default=128)
    parser.add_argument("--frames", type=int, default=30)
    parser.add_argument("--colors", type=int, default=16)
    parser.add_argument("--background", default="0c0a10", help="hex RGB behind transparent pixels")
    args = parser.parse_args()
    if not 1 <= args.size <= MAX_SIDE or not 1 <= args.frames <= MAX_FRAMES or not 2 <= args.colors <= 256:
        sys.exit(f"bounds: size <= {MAX_SIDE}, frames <= {MAX_FRAMES}, 2 <= colors <= 256")
    background = tuple(bytes.fromhex(args.background))
    data, count, used, delay = convert(args.source, args.size, args.frames, args.colors, background)
    if len(data) > MAX_BYTES:
        sys.exit(f"{len(data)} bytes exceeds the {MAX_BYTES} byte budget; use fewer frames or a smaller size")
    with open(args.output, "wb") as handle:
        handle.write(data)
    print(f"{args.output}: {args.size}x{args.size}, {count} frames, {used} colors, {delay} ms/frame, {len(data)} bytes")


if __name__ == "__main__":
    main()
