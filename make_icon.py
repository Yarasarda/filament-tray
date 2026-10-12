"""Basit bir makara ikonu üretir (bağımlılıksız RGBA PNG)."""
import struct, zlib, math

N = 128
rows = []
for y in range(N):
    row = bytearray([0])
    for x in range(N):
        d = math.hypot(x - 63.5, y - 63.5)
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

png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0)) \
    + chunk(b"IDAT", zlib.compress(b"".join(rows))) + chunk(b"IEND", b"")
open("icons/icon.png", "wb").write(png)
