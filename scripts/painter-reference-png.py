#!/usr/bin/env python3
"""Renders the reference `.ans` files into PNGs (`docs/coupler-handoff.md`, B1).

Reads every `*.utf8.ans` in a folder (default `painter-data/reference`,
made by `examples/painter_reference.rs`) and writes a PNG beside each:
8 by 16 pixels a cell, IBM VGA 8x16 (`public/fonts/ansiapps/`) for the
characters, the 16 colors as `spanColors` in `lib/mud.ts` shows them.
Needs Pillow. Run: `python3 scripts/painter-reference-png.py [folder]`.
"""
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT = ROOT / "public/fonts/ansiapps/WebPlus_IBM_VGA_8x16.woff"
VGA = [
    (0, 0, 0), (170, 0, 0), (0, 170, 0), (170, 85, 0), (0, 0, 170), (170, 0, 170), (0, 170, 170), (170, 170, 170),
    (85, 85, 85), (255, 85, 85), (85, 255, 85), (255, 255, 85), (85, 85, 255), (255, 85, 255), (85, 255, 255), (255, 255, 255),
]
CELL_W, CELL_H = 8, 16
SGR = re.compile(r"\x1b\[([0-9;]*)m")


def indexed(n):
    """The 256-color palette: 16 VGA, a 6x6x6 cube, 24 grays."""
    if n < 16:
        return VGA[n]
    if n < 232:
        n -= 16
        level = lambda v: 0 if v == 0 else 55 + 40 * v
        return (level(n // 36), level(n // 6 % 6), level(n % 6))
    g = 8 + 10 * (n - 232)
    return (g, g, g)


def parse(data: bytes):
    """Rows of (char, fg, bg) from an ANSI file: SGR only, as Coupler writes it."""
    text = data.split(b"\x1a")[0].decode("utf-8")
    rows = []
    for line in text.split("\r\n"):
        if not line:
            continue
        fg, bg = VGA[7], VGA[0]
        row = []
        pos = 0
        for m in SGR.finditer(line):
            row.extend((c, fg, bg) for c in line[pos:m.start()])
            pos = m.end()
            codes = [int(c) if c else 0 for c in m.group(1).split(";")]
            i = 0
            while i < len(codes):
                c = codes[i]
                if c == 0:
                    fg, bg = VGA[7], VGA[0]
                elif 30 <= c <= 37:
                    fg = VGA[c - 30]
                elif 90 <= c <= 97:
                    fg = VGA[c - 90 + 8]
                elif 40 <= c <= 47:
                    bg = VGA[c - 40]
                elif 100 <= c <= 107:
                    bg = VGA[c - 100 + 8]
                elif c == 39:
                    fg = VGA[7]
                elif c == 49:
                    bg = VGA[0]
                elif c in (38, 48):
                    if codes[i + 1] == 5:
                        color, i = indexed(codes[i + 2]), i + 2
                    else:
                        color, i = tuple(codes[i + 2:i + 5]), i + 4
                    if c == 38:
                        fg = color
                    else:
                        bg = color
                i += 1
        row.extend((c, fg, bg) for c in line[pos:])
        rows.append(row)
    return rows


def render(rows):
    columns = max(len(r) for r in rows)
    image = Image.new("RGB", (columns * CELL_W, len(rows) * CELL_H))
    draw = ImageDraw.Draw(image)
    font = ImageFont.truetype(str(FONT), 16)
    for y, row in enumerate(rows):
        for x, (char, fg, bg) in enumerate(row):
            box = (x * CELL_W, y * CELL_H, x * CELL_W + CELL_W - 1, y * CELL_H + CELL_H - 1)
            draw.rectangle(box, fill=bg)
            if char != " ":
                draw.text((x * CELL_W, y * CELL_H), char, font=font, fill=fg)
    return image


def main():
    folder = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "painter-data/reference"
    files = sorted(folder.glob("*.utf8.ans"))
    for path in files:
        render(parse(path.read_bytes())).save(path.with_name(path.name.removesuffix(".utf8.ans") + ".png"))
    print(f"{len(files)} PNGs in {folder}")


if __name__ == "__main__":
    main()
