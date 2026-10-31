#!/usr/bin/env python3
"""
Extract Urban Chaos textures to plain image files, for upscaling.

- clumps/<name>.txc  ->  <out>/clumps/<name>/page_NNNN.png
  A Python port of uc_formats::txc (crates/uc-formats/src/txc.rs): Clump::parse and decode.
  Textures with an alpha channel (ARGB4444) become RGBA PNGs, the rest (RGB565) RGB PNGs.
  Crinkles (clump ids 1568..2079) are relief meshes, not pictures, and are skipped.
  A prim page shared/prims/textype.txt doesn't make see-through loses its alpha channel, as
  the game draws it solid (uc_world::level::Level::texture) but takes an override's alpha as
  it is: the traffic cones' page, among others, has an alpha channel that's mostly clear.
  The layout is the one --overrides reads per mission (uc_world::level::override_path),
  so the upscaled clumps/ folder can be passed to the game as is.
- Every *.tga in the install is copied unchanged to the same relative path under <out>.

The install is only read. Rerunning skips files that already exist.

  python3 extract_textures.py --game ~/Games/"Urban Chaos" --out ~/uc/textures
"""
import argparse
import re
import shutil
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

CRINKLE_BASE = 1568
CRINKLE_PAGES = 512
PRIM_PAGES, EXTRA_PAGES = 11 * 64, 22 * 64


def see_through_prims(game: Path) -> set[int]:
    """`see_through_pages`: the prim pages textype.txt flags T, M, A or D, and the page after
    one flagged I."""
    path = next((p for p in game.rglob("*") if p.as_posix().lower().endswith(
        "server/textures/shared/prims/textype.txt")), None)
    out = set()
    if path is None:
        return out
    for line in path.read_text(errors="ignore").splitlines():
        m = re.match(r"\s*Page\s+(\d+)\s*:(.*)", line)
        if not m:
            continue
        n, flags = int(m.group(1)), m.group(2).upper()
        if any(c in flags for c in "TMAD"):
            out.add(n)
        if "I" in flags:
            out.add(n + 1)
    return out


def parse_clump(data: bytes) -> dict[int, bytes]:
    """`Clump::parse`: max_id, max_id offsets, max_id lengths; offset 0 means absent."""
    (max_id,) = struct.unpack_from("<I", data, 0)
    offsets = struct.unpack_from(f"<{max_id}I", data, 4)
    lengths = struct.unpack_from(f"<{max_id}I", data, 4 + 4 * max_id)
    out = {}
    for i, (off, n) in enumerate(zip(offsets, lengths)):
        if off == 0:
            continue
        if off + n > len(data):
            raise ValueError(f"entry {i} runs past the end of the file")
        out[i] = data[off:off + n]
    return out


def decode(blob: bytes):
    """`decode` (ReadSquished + TGA_read_compressed). Returns (has_alpha, HxWx4 uint8) or None."""
    words = np.frombuffer(blob[: len(blob) // 2 * 2], dtype="<u2")
    if len(words) < 3:
        raise ValueError("texture blob too short")
    if words[0] == 0xFFFF:
        if len(words) < 5:
            raise ValueError("squished texture header too short")
        header = words[1:4]
        total = int(words[4])
        mapping = words[5:5 + total]
        if len(mapping) < total:
            raise ValueError("palette runs past end")
        bits = 1
        while total > (1 << bits):
            bits += 1
        n = int(header[1]) * int(header[2])
        # Indices are packed MSB first across consecutive 16-bit words.
        packed = words[5 + total:]
        need = (n * bits + 15) // 16
        if len(packed) < need:
            raise ValueError("packed indices run past end")
        bitstream = np.unpackbits(packed[:need].astype(">u2").view(np.uint8))
        idx = bitstream[: n * bits].reshape(n, bits)
        idx = idx.dot(1 << np.arange(bits - 1, -1, -1)).astype(np.int64)
        if n and idx.max() >= total:
            raise ValueError("palette index out of range")
        pixels = mapping[idx]
    else:
        header = words[0:3]
        pixels = words[3:]
    alpha, w, h = header[0] != 0, int(header[1]), int(header[2])
    if w == 0 or h == 0:
        return None
    if len(pixels) < w * h:
        raise ValueError("not enough pixel data")
    p = pixels[: w * h].astype(np.uint16)
    if alpha:
        # ARGB4444, expanded like C4to8.
        x4 = lambda v: ((v & 15) * 17).astype(np.uint8)
        rgba = np.stack([x4(p >> 8), x4(p >> 4), x4(p), x4(p >> 12)], axis=-1)
    else:
        # RGB565, expanded like C5to8/C6to8.
        r5, g6, b5 = (p >> 11) & 31, (p >> 5) & 63, p & 31
        rgba = np.stack([(r5 << 3) | (r5 >> 2), (g6 << 2) | (g6 >> 4), (b5 << 3) | (b5 >> 2),
                         np.full_like(p, 255)], axis=-1).astype(np.uint8)
    return bool(alpha), rgba.reshape(h, w, 4)


def save_png(path: Path, has_alpha: bool, rgba: np.ndarray):
    tmp = path.with_name(path.name + ".part")
    img = Image.fromarray(rgba, "RGBA") if has_alpha else Image.fromarray(rgba[..., :3], "RGB")
    img.save(tmp, format="PNG")
    tmp.replace(path)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--game", required=True, type=Path, help="Urban Chaos install (read only)")
    ap.add_argument("--out", required=True, type=Path)
    a = ap.parse_args()
    game, out = a.game.resolve(), a.out.resolve()
    if out == game or game in out.parents:
        sys.exit("--out must not be inside the install")

    see_through = see_through_prims(game)
    written = skipped = failed = 0
    clumps = sorted(p for p in (game / "clumps").iterdir() if p.suffix.lower() == ".txc")
    for c in clumps:
        dst = out / "clumps" / c.stem
        dst.mkdir(parents=True, exist_ok=True)
        try:
            entries = parse_clump(c.read_bytes())
        except Exception as e:
            print(f"{c.name}: {e}", file=sys.stderr)
            failed += 1
            continue
        for i, blob in entries.items():
            if CRINKLE_BASE <= i < CRINKLE_BASE + CRINKLE_PAGES:
                continue
            f = dst / f"page_{i:04}.png"
            if f.exists():
                skipped += 1
                continue
            try:
                t = decode(blob)
            except Exception as e:
                print(f"{c.name} entry {i}: {e}", file=sys.stderr)
                failed += 1
                continue
            if t is not None:
                has_alpha, rgba = t
                if PRIM_PAGES <= i < EXTRA_PAGES and i - PRIM_PAGES not in see_through:
                    has_alpha = False
                save_png(f, has_alpha, rgba)
                written += 1
        print(f"{c.name}: {len(entries)} entries", flush=True)

    for t in sorted(game.rglob("*")):
        if t.is_file() and t.suffix.lower() == ".tga":
            f = out / t.relative_to(game)
            if f.exists():
                skipped += 1
                continue
            f.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(t, f)
            written += 1
    print(f"written {written}, already there {skipped}, failed {failed}")


if __name__ == "__main__":
    main()
