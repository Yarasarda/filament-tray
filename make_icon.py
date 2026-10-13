"""Basit bir makara ikonu üretir (bağımlılıksız RGBA PNG ve ICO)."""
import math
import struct
import zlib

def make_png(n):
    rows = []
    scale = n / 128.0
    for y in range(n):
        row = bytearray([0])
        for x in range(n):
            d = math.hypot(x - (n / 2 - 0.5), y - (n / 2 - 0.5)) / scale
            if d < 14:
                px = (0, 0, 0, 0)          # makara deliği
            elif d < 60:
                px = (76, 195, 138, 255)   # filament
            elif d < 63:
                px = (232, 232, 234, 255)  # kenar
            else:
                px = (0, 0, 0, 0)
            row += bytes(px)
        rows.append(bytes(row))

    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", n, n, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(b"".join(rows)))
        + chunk(b"IEND", b"")
    )

png128 = make_png(128)
png32 = make_png(32)

with open("icons/icon.png", "wb") as f:
    f.write(png128)
with open("icons/128x128.png", "wb") as f:
    f.write(png128)
with open("icons/32x32.png", "wb") as f:
    f.write(png32)

# Pack multi-resolution ICO (32x32 and 128x128)
images = [(32, png32), (128, png128)]
header = struct.pack("<HHH", 0, 1, len(images))
offset = 6 + len(images) * 16
entries = b""
for size, data in images:
    entries += struct.pack(
        "<BBBBHHII",
        size if size < 256 else 0,
        size if size < 256 else 0,
        0,
        0,
        1,
        32,
        len(data),
        offset,
    )
    offset += len(data)

with open("icons/icon.ico", "wb") as f:
    f.write(header + entries + png32 + png128)

print("Icons generated successfully in icons/")

