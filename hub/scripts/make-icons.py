"""Generate the hub's icon set: PNGs plus a Windows .ico.

Tauri wants a fixed list of icon files before it will bundle, and the toolkit has
no image toolchain -- no Pillow, no ImageMagick, no design source. Rather than
add a dependency for six small squares, this writes the PNG and ICO containers
directly: both are simple enough that `zlib` and `struct` cover them.

The mark is the same shape the sidebar draws in SVG -- a gold diamond over the
obsidian ground from `theme.css` -- so the taskbar icon and the window agree.

    py -3 scripts/make-icons.py

Re-run it only when the mark changes; the output is committed.
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

ICON_DIR = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"

# theme.css: --ground-3 behind, --gold-1 for the mark, --edge-4 for the rim.
GROUND = (0x09, 0x10, 0x1A, 0xFF)
GOLD = (0xD6, 0xA6, 0x4C, 0xFF)
ARCANE = (0x59, 0xD6, 0xC0, 0xFF)
EDGE = (0x31, 0x51, 0x6A, 0xFF)
CLEAR = (0, 0, 0, 0)


def blend(bottom, top, alpha):
    """`top` over `bottom` at `alpha` in 0..1, both straight RGBA."""
    return tuple(
        round(bottom[i] * (1 - alpha) + top[i] * alpha) for i in range(3)
    ) + (max(bottom[3], round(255 * alpha)),)


def render(size: int):
    """The mark at `size`x`size`, as a list of rows of RGBA tuples.

    Supersampled 3x3 per pixel: at 32 pixels a diamond drawn with hard edges is
    a staircase, and the icon is mostly edge.
    """
    pixels = [[CLEAR] * size for _ in range(size)]
    centre = (size - 1) / 2
    outer = size * 0.47
    rim = size * 0.43
    diamond = size * 0.30
    inner = size * 0.13
    samples = [(-1 / 3, -1 / 3), (0, -1 / 3), (1 / 3, -1 / 3),
               (-1 / 3, 0), (0, 0), (1 / 3, 0),
               (-1 / 3, 1 / 3), (0, 1 / 3), (1 / 3, 1 / 3)]

    for y in range(size):
        for x in range(size):
            hits = {"ground": 0, "rim": 0, "gold": 0, "arcane": 0}
            for dx, dy in samples:
                px, py = x + dx - centre, y + dy - centre
                radius = (px * px + py * py) ** 0.5
                manhattan = abs(px) + abs(py)
                if radius > outer:
                    continue
                hits["ground"] += 1
                if radius > rim:
                    hits["rim"] += 1
                elif manhattan <= inner:
                    hits["arcane"] += 1
                elif manhattan <= diamond:
                    hits["gold"] += 1

            if not hits["ground"]:
                continue
            total = len(samples)
            colour = blend(CLEAR, GROUND, hits["ground"] / total)
            for key, paint in (("rim", EDGE), ("gold", GOLD), ("arcane", ARCANE)):
                if hits[key]:
                    colour = blend(colour, paint, hits[key] / total)
            pixels[y][x] = colour
    return pixels


def png_bytes(pixels) -> bytes:
    height, width = len(pixels), len(pixels[0])
    raw = bytearray()
    for row in pixels:
        raw.append(0)  # filter type 0: no filtering, which compresses fine here
        for r, g, b, a in row:
            raw += bytes((r, g, b, a))

    def chunk(tag: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + tag
            + payload
            + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)  # 8-bit RGBA
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def ico_bytes(images) -> bytes:
    """An .ico wrapping PNG-encoded entries, which Vista and later accept."""
    count = len(images)
    directory = b"\x00\x00\x01\x00" + struct.pack("<H", count)
    offset = 6 + 16 * count
    entries, blobs = b"", b""
    for size, payload in images:
        entries += struct.pack(
            "<BBBBHHII",
            0 if size >= 256 else size,  # 0 means 256
            0 if size >= 256 else size,
            0,  # no palette
            0,
            1,  # colour planes
            32,  # bits per pixel
            len(payload),
            offset,
        )
        blobs += payload
        offset += len(payload)
    return directory + entries + blobs


def main() -> int:
    ICON_DIR.mkdir(parents=True, exist_ok=True)
    rendered = {size: png_bytes(render(size)) for size in (16, 32, 48, 64, 128, 256, 512)}

    written = {
        "32x32.png": rendered[32],
        "64x64.png": rendered[64],
        "128x128.png": rendered[128],
        "128x128@2x.png": rendered[256],
        "icon.png": rendered[512],
        "icon.ico": ico_bytes([(s, rendered[s]) for s in (16, 32, 48, 64, 128, 256)]),
    }
    for name, payload in written.items():
        (ICON_DIR / name).write_bytes(payload)
        print(f"{name}: {len(payload)} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
