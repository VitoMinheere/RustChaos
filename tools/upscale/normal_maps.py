#!/usr/bin/env python3
"""
Normal maps for the upscaled world pages (#474): page_NNNN.png -> page_NNNN_n.png beside it.

The renderer lights a wall, floor or roof page that has one per pixel by it (Options > RELIEF).
Made from the picture alone, no model:

- Height is brightness: mortar, cracks and the gaps between paving slabs are darker than
  what stands out round them. The pages have shading painted in as well (a lit top edge, a
  dark band under a sill), which as height would emboss the whole page the wrong way, so
  only the fine detail is kept: the brightness less a blur of it (--keep pixels across, at
  the upscaled size; about a brick's height at 4x).
- The slopes of that height, wrapped round the edges as the pages tile, make the normal.
  --strength sets how steep a given step in brightness is.
- Stored as the renderer reads it: x along the picture's width, y down it, z out of the
  page; each 0..255 for -1..1 (128,128,255 is flat).

Only the world's own and shared pages (below 704: the prims' pages are people and props,
which the renderer doesn't light this way), and only opaque ones (a fence or a window that
can be seen through gets none). Made again when the picture is newer than its map.

  normal_maps.py --dst ~/uc/textures_4x
"""
import argparse
import os
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image

PRIM_PAGES = 704
PAGE = re.compile(r"page_(\d{4})\.png$")


def blur(a: np.ndarray, sigma: float) -> np.ndarray:
    """A Gaussian blur, wrapped round the edges (the pages tile), through the FFT."""
    h, w = a.shape
    fy, fx = np.fft.fftfreq(h)[:, None], np.fft.fftfreq(w)[None, :]
    g = np.exp(-2.0 * (np.pi * sigma) ** 2 * (fx * fx + fy * fy))
    return np.real(np.fft.ifft2(np.fft.fft2(a) * g))


def normal_map(rgb: np.ndarray, keep: float, strength: float) -> np.ndarray:
    """The normal map (HxWx3 uint8) of a picture (HxWx3 uint8)."""
    lum = rgb.astype(np.float32) @ np.array([0.299, 0.587, 0.114], np.float32) / 255.0
    # A light blur first: the upscaler's grain isn't relief.
    lum = blur(lum, 0.8)
    height = lum - blur(lum, keep)
    # Slopes per texel, scaled to the picture's size so a map reads the same at any scale
    # (a step across a 256-wide page is as steep as across a 64-wide one).
    scale = rgb.shape[1] / 64.0
    dx = (np.roll(height, -1, 1) - np.roll(height, 1, 1)) * 0.5 * scale
    dy = (np.roll(height, -1, 0) - np.roll(height, 1, 0)) * 0.5 * scale
    n = np.dstack([-dx * strength, -dy * strength, np.ones_like(dx)])
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    return np.clip(n * 127.5 + 127.5, 0, 255).round().astype(np.uint8)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--dst", required=True, type=Path, help="the upscaled textures (upscale_textures.py's --dst)")
    p.add_argument("--keep", type=float, default=6.0, help="detail kept as height: blur radius, upscaled pixels")
    p.add_argument("--strength", type=float, default=4.0, help="how steep the relief is")
    p.add_argument("--force", action="store_true", help="make every map again")
    a = p.parse_args()
    pages = sorted(f for f in a.dst.expanduser().rglob("page_*.png") if PAGE.search(f.name))
    made = skipped = 0
    cache = {}
    for f in pages:
        if int(PAGE.search(f.name).group(1)) >= PRIM_PAGES:
            continue
        out = f.with_name(f.name.replace(".png", "_n.png"))
        if not a.force and out.exists() and out.stat().st_mtime >= f.stat().st_mtime:
            skipped += 1
            continue
        im = Image.open(f)
        im.load()
        rgba = np.array(im.convert("RGBA"))
        if (rgba[..., 3] < 255).any():
            out.unlink(missing_ok=True)
            continue
        # The clumps share most pictures: each made once.
        key = hash(rgba.tobytes())
        if key not in cache:
            cache[key] = normal_map(rgba[..., :3], a.keep * rgba.shape[1] / 256.0, a.strength)
        tmp = out.with_name(f".{out.name}.part")
        Image.fromarray(cache[key], "RGB").save(tmp, format="PNG")
        os.replace(tmp, out)
        made += 1
    print(f"{made} normal maps made ({len(cache)} distinct), {skipped} up to date")


if __name__ == "__main__":
    sys.exit(main())
