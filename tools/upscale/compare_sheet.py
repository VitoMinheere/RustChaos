#!/usr/bin/env python3
"""
A comparison sheet for checking upscaled textures by eye.

Each row: the original scaled up nearest-neighbour, the upscaled one, and for tiling
textures (--tiling) the upscaled one tiled 2x2 to show any seam. Transparency is shown on a
checkerboard. Pictures are drawn at --cell pixels.

  compare_sheet.py --src ~/uc/textures --dst ~/uc/textures_4x --out sheet.png \
      clumps/testdrive1a/page_0001.png ... --tiling clumps/testdrive1a/page_0001.png
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw


def checker(w, h, n=16):
    y, x = np.mgrid[0:h, 0:w]
    c = np.where(((x // n) + (y // n)) % 2 == 0, 200, 140).astype(np.uint8)
    return Image.fromarray(np.dstack([c, c, c, np.full_like(c, 255)]), "RGBA")


def on_checker(img):
    bg = checker(*img.size)
    bg.alpha_composite(img.convert("RGBA"))
    return bg.convert("RGB")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--src", required=True, type=Path)
    ap.add_argument("--dst", required=True, type=Path)
    ap.add_argument("--dst2", type=Path, help="a second output to show beside the first")
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--cell", type=int, default=384)
    ap.add_argument("--tiling", nargs="*", default=[])
    ap.add_argument("files", nargs="+")
    a = ap.parse_args()
    c, label = a.cell, 18
    cols = 2 + (1 if a.dst2 else 0) + (1 if a.tiling else 0)
    sheet = Image.new("RGB", (cols * (c + 8) + 8, len(a.files) * (c + label + 8) + 8), (40, 40, 40))
    d = ImageDraw.Draw(sheet)
    for r, rel in enumerate(a.files):
        y = 8 + r * (c + label + 8)
        src = Image.open(a.src / rel)
        ups = [Image.open(x / rel) for x in [a.dst] + ([a.dst2] if a.dst2 else [])]
        cells = [("original (nearest)", src.resize((c, c * src.height // src.width), Image.NEAREST))]
        names = ["upscaled"] if not a.dst2 else [a.dst.name, a.dst2.name]
        cells += [(n, u.resize((c, c * u.height // u.width), Image.LANCZOS)) for n, u in zip(names, ups)]
        if rel in a.tiling:
            u = ups[0]
            t = Image.new(u.mode, (u.width * 2, u.height * 2))
            for i in range(4):
                t.paste(u, (i % 2 * u.width, i // 2 * u.height))
            cells.append(("upscaled 2x2", t.resize((c, c * t.height // t.width), Image.LANCZOS)))
        d.text((8, y), f"{rel}  {src.width}x{src.height} {src.mode} -> {ups[0].width}x{ups[0].height} {ups[0].mode}",
               fill=(230, 230, 230))
        for i, (name, im) in enumerate(cells):
            x = 8 + i * (c + 8)
            sheet.paste(on_checker(im), (x, y + label))
            d.text((x + 4, y + label + 4), name, fill=(255, 255, 0))
    a.out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(a.out)
    print(a.out, sheet.size)


if __name__ == "__main__":
    main()
