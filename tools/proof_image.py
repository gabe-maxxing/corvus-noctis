"""Encode QEMU's P6 screendump as PNG using only the standard library."""
import base64
from pathlib import Path
import struct
import zlib


def png(path):
    header, size, maximum, pixels = Path(path).read_bytes().split(b"\n", 3)
    width, height = map(int, size.split())
    assert header == b"P6" and maximum == b"255" and len(pixels) == width * height * 3
    def chunk(name, data):
        return struct.pack(">I", len(data)) + name + data + struct.pack(">I", zlib.crc32(name + data))
    scanlines = b"".join(b"\0" + pixels[y * width * 3:(y + 1) * width * 3] for y in range(height))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(scanlines, 9)) + chunk(b"IEND", b""))


def record(path):
    image = png(path)
    Path(path).with_suffix(".png").write_bytes(image)
    # CI already collects logs. This keeps visual proof retrievable without
    # editing the separately owned artifact/release workflow.
    print("CORVUS_PNG " + Path(path).stem + " " + base64.b64encode(image).decode())
