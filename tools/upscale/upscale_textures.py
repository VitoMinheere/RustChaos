#!/usr/bin/env python3
"""
Batch-upscale Urban Chaos textures with realesrgan-ncnn-vulkan.

- Resumable: a file whose output exists is skipped, and every output is written to a temp
  file and renamed, so a crash never leaves half a file. Rerun the same command to go on.
- Deduplicated: the clumps share most pages, so each distinct picture is upscaled once and
  written to every path that has it.
- Seam-safe: each texture is wrap-padded before upscaling and cropped after, so tiling
  walls and roads stay seamless. --no-wrap PATTERN pads non-tiling pictures by repeating
  their edge instead.
- Transparency: alpha is upscaled on its own. Binary (cut-out) alpha is re-thresholded so
  it stays crisp; graded alpha is kept as upscaled. Fully transparent pixels, and colour-key
  pixels (--colorkey), are filled before upscaling so they don't bleed into the edges; the
  key is restored exactly afterwards through a nearest-neighbour mask.
- Mirrors the source tree and keeps each file's format (PNG stays PNG, TGA stays TGA).
- Calls the upscaler once per --chunk textures, so the model loads once per batch.
- Logs totals, progress after every batch, and every failure, to --log. A bad texture is
  logged and left for the next run; it never stops this one.

  upscale_textures.py --src ~/uc/textures --dst ~/uc/textures_4x --limit 20
"""
import argparse
import fnmatch
import hashlib
import logging
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import numpy as np
from PIL import Image

EXTS = {".png", ".tga", ".bmp"}
NEUTRAL = (128, 128, 128)


def parse_args():
    home = Path.home() / "tools" / "realesrgan-ncnn-vulkan"
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--src", required=True, type=Path, help="extracted textures (extract_textures.py)")
    p.add_argument("--dst", required=True, type=Path, help="output; the source tree is mirrored")
    p.add_argument("--bin", type=Path, default=home / "realesrgan-ncnn-vulkan")
    p.add_argument("--model", default="realesrgan-x4plus", help="model name (-n)")
    p.add_argument("--model-dir", type=Path, default=home / "models", help="folder of .param/.bin (-m)")
    p.add_argument("--scale", type=int, default=4)
    p.add_argument("--gpu", default="0", help="Vulkan GPU index; the T1000 is 0 here")
    p.add_argument("--tile", type=int, default=256, help="tile size; 256 is safe on 4 GB")
    p.add_argument("--threads", default="1:1:2",
                   help="upscaler -j load:proc:save; its default 1:2:2 peaks at 3.6 GB at tile "
                        "256 and runs out of memory on a 4 GB card, 1:1:2 at 2.2 GB")
    p.add_argument("--pad", type=int, default=16, help="padding in source pixels")
    p.add_argument("--no-wrap", action="append", default=[], metavar="PATTERN",
                   help="glob on the relative path of non-tiling pictures, padded by edge "
                        "instead of wrapped; '*' for all; repeatable")
    p.add_argument("--fill", choices=["edge", "neutral"], default="edge",
                   help="colour for transparent/keyed pixels: bled from the nearest opaque "
                        "ones, or flat grey")
    p.add_argument("--colorkey", default="", help="transparent colour R,G,B, e.g. 255,0,255")
    p.add_argument("--chunk", type=int, default=50, help="textures per upscaler call")
    p.add_argument("--include", action="append", default=[], metavar="PATTERN",
                   help="only relative paths matching this glob; repeatable")
    p.add_argument("--limit", type=int, default=0, help="only N distinct textures (test runs)")
    p.add_argument("--log", type=Path, default=Path("upscale.log"))
    return p.parse_args()


def setup_logging(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    fmt = logging.Formatter("%(asctime)s %(levelname)s %(message)s", "%Y-%m-%d %H:%M:%S")
    log = logging.getLogger("upscale")
    log.setLevel(logging.INFO)
    for h in (logging.FileHandler(path), logging.StreamHandler(sys.stdout)):
        h.setFormatter(fmt)
        log.addHandler(h)
    return log


def hms(s: float) -> str:
    s = int(s)
    return f"{s // 3600}h{s % 3600 // 60:02d}m" if s >= 3600 else f"{s // 60}m{s % 60:02d}s"


# ---------------------------------------------------------------- preparing

def load_rgba(path: Path):
    """The picture as HxWx4 uint8, and whether it has an alpha channel."""
    im = Image.open(path)
    im.load()
    has_alpha = im.mode in ("RGBA", "LA", "PA") or (im.mode == "P" and "transparency" in im.info)
    return np.array(im.convert("RGBA")), has_alpha, im.mode


def bleed(rgb: np.ndarray, hole: np.ndarray, wrap: bool) -> np.ndarray:
    """Fills `hole` pixels with the average of their filled 4-neighbours, ring by ring."""
    rgb = rgb.astype(np.float32).copy()
    filled = ~hole
    if not filled.any():
        rgb[:] = NEUTRAL
        return rgb.astype(np.uint8)
    rgb[hole] = 0
    for _ in range(max(rgb.shape[:2])):
        if filled.all():
            break
        acc = np.zeros_like(rgb)
        cnt = np.zeros(filled.shape, np.float32)
        for axis, step in ((0, 1), (0, -1), (1, 1), (1, -1)):
            if wrap:
                c, f = np.roll(rgb, step, axis), np.roll(filled, step, axis)
            else:
                c = np.roll(rgb, step, axis)
                f = np.roll(filled, step, axis)
                edge = [slice(None)] * 2
                edge[axis] = 0 if step == 1 else -1
                f[tuple(edge)] = False
            acc += c * f[..., None]
            cnt += f
        new = ~filled & (cnt > 0)
        rgb[new] = acc[new] / cnt[new][:, None]
        filled |= new
    return rgb.round().astype(np.uint8)


class Job:
    """One distinct picture and every output path that should get it."""

    def __init__(self, key, rel, srcs, dsts, pixels):
        self.key, self.rel, self.srcs, self.dsts, self.pixels = key, rel, srcs, dsts, pixels


def prepare(job: Job, a, indir: Path):
    """Writes the padded colour (and alpha) inputs; returns what finishing needs."""
    rgba, has_alpha, mode = load_rgba(job.srcs[0])
    wrap = not any(fnmatch.fnmatch(job.rel, p) for p in a.no_wrap)
    rgb, alpha = rgba[..., :3], rgba[..., 3]
    key = np.zeros(alpha.shape, bool)
    if a.colorkey:
        key = (rgb == np.array(a.colorkey, np.uint8)).all(-1)
    hole = key | (alpha == 0) if has_alpha else key
    if hole.any():
        rgb = bleed(rgb, hole, wrap) if a.fill == "edge" else np.where(hole[..., None], np.uint8(NEUTRAL), rgb)
    pad = ((a.pad, a.pad), (a.pad, a.pad))
    mode_pad = "wrap" if wrap else "edge"
    Image.fromarray(np.pad(rgb, pad + ((0, 0),), mode=mode_pad)).save(indir / f"{job.key}.png")
    use_alpha = has_alpha and (alpha < 255).any()
    if use_alpha:
        a3 = np.repeat(np.pad(alpha, pad, mode=mode_pad)[..., None], 3, -1)
        Image.fromarray(a3).save(indir / f"{job.key}_a.png")
    binary = use_alpha and np.isin(alpha, (0, 255)).all()
    return dict(mode=mode, has_alpha=has_alpha, use_alpha=use_alpha, binary=binary,
                key=key if key.any() else None, shape=alpha.shape)


def finish(job: Job, info, a, outdir: Path):
    h, w = info["shape"]
    s, p = a.scale, a.pad * a.scale

    def up(name):
        im = np.array(Image.open(outdir / name).convert("RGB"))
        im = im[p:p + h * s, p:p + w * s]
        if im.shape[:2] != (h * s, w * s):
            raise RuntimeError(f"upscaled {name} is {im.shape[1]}x{im.shape[0]}, expected {w * s}x{h * s}")
        return im

    rgb = up(f"{job.key}.png")
    if info["key"] is not None:
        mask = info["key"].repeat(s, 0).repeat(s, 1)
        rgb[mask] = a.colorkey
    if info["use_alpha"]:
        alpha = up(f"{job.key}_a.png").mean(-1)
        alpha = np.where(alpha >= 128, 255, 0) if info["binary"] else alpha.round()
        img = Image.fromarray(np.dstack([rgb, alpha.clip(0, 255).astype(np.uint8)]), "RGBA")
    elif info["has_alpha"]:
        img = Image.fromarray(np.dstack([rgb, np.full((h * s, w * s), 255, np.uint8)]), "RGBA")
    else:
        img = Image.fromarray(rgb, "RGB")
    return img


def save_atomic(img: Image.Image, dst: Path):
    dst.parent.mkdir(parents=True, exist_ok=True)
    fmt = {".png": "PNG", ".tga": "TGA", ".bmp": "BMP"}[dst.suffix.lower()]
    tmp = dst.with_name(f".{dst.name}.part")
    img.save(tmp, format=fmt)
    os.replace(tmp, dst)


def copy_atomic(src: Path, dst: Path):
    dst.parent.mkdir(parents=True, exist_ok=True)
    tmp = dst.with_name(f".{dst.name}.part")
    shutil.copyfile(src, tmp)
    os.replace(tmp, dst)


# ---------------------------------------------------------------- running

def run_upscaler(a, indir: Path, outdir: Path, log, tile=None) -> bool:
    cmd = [str(a.bin), "-i", str(indir), "-o", str(outdir), "-n", a.model, "-m", str(a.model_dir),
           "-s", str(a.scale), "-t", str(tile or a.tile), "-g", str(a.gpu), "-j", a.threads, "-f", "png"]
    r = subprocess.run(cmd, cwd=a.bin.parent, capture_output=True, text=True)
    if r.returncode != 0:
        tail = " | ".join((r.stderr or r.stdout).strip().splitlines()[-3:])
        log.error("upscaler exited %s on %s: %s", r.returncode, indir.name, tail)
        return False
    return True


def collect(a, log):
    """Groups the source files by picture into jobs, and copies what an earlier run did."""
    files = sorted(f for f in a.src.rglob("*") if f.is_file() and f.suffix.lower() in EXTS
                   and not f.name.startswith("."))
    rels = [f.relative_to(a.src).as_posix() for f in files]
    if a.include:
        keep = [any(fnmatch.fnmatch(r, p) for p in a.include) for r in rels]
        files = [f for f, k in zip(files, keep) if k]
        rels = [r for r, k in zip(rels, keep) if k]
    groups = {}
    for f, rel in zip(files, rels):
        try:
            im = Image.open(f)
            im.load()
            k = hashlib.sha1(f"{im.mode}{im.size}{f.suffix.lower()}".encode() + im.tobytes()).hexdigest()[:16]
        except Exception as e:
            log.error("FAILED %s: can't read: %s", rel, e)
            continue
        g = groups.setdefault(k, {"rel": rel, "srcs": [], "dsts": [], "px": im.size[0] * im.size[1]})
        g["srcs"].append(f)
        g["dsts"].append(a.dst / rel)
    jobs, copied, done_files = [], 0, 0
    for k, g in groups.items():
        have = [d for d in g["dsts"] if d.exists()]
        done_files += len(have)
        missing = [d for d in g["dsts"] if not d.exists()]
        if missing and have:
            # Upscaled before under another name: copy it, no GPU needed.
            for d in missing:
                copy_atomic(have[0], d)
            copied += len(missing)
        elif missing:
            jobs.append(Job(k, g["rel"], g["srcs"], missing, g["px"]))
    return len(files), len(groups), done_files, copied, jobs


def main():
    a = parse_args()
    a.src, a.dst = a.src.expanduser().resolve(), a.dst.expanduser().resolve()
    if a.dst == a.src or a.src in a.dst.parents:
        sys.exit("--dst must not be inside --src")
    if a.colorkey:
        a.colorkey = tuple(int(c) for c in a.colorkey.split(","))
    log = setup_logging(a.log)
    log.info("start: model %s x%d, gpu %s, tile %d, threads %s, pad %d, fill %s, colorkey %s, no-wrap %s",
             a.model, a.scale, a.gpu, a.tile, a.threads, a.pad, a.fill, a.colorkey or "none", a.no_wrap or "none")
    if not a.bin.is_file():
        sys.exit(f"upscaler not found: {a.bin}")

    nfiles, nuniq, done_files, copied, jobs = collect(a, log)
    jobs.sort(key=lambda j: j.rel)
    if a.limit:
        jobs = jobs[: a.limit]
    total_px = sum(j.pixels for j in jobs)
    log.info("totals: %d files, %d distinct textures; %d files already done, %d copied from "
             "earlier output; %d textures (%d files, %.1f Mpx) to upscale",
             nfiles, nuniq, done_files, copied, len(jobs), sum(len(j.dsts) for j in jobs), total_px / 1e6)

    t0 = time.time()
    ok = failed = files_written = 0
    px_done = 0
    for b in range(0, len(jobs), a.chunk):
        batch = jobs[b:b + a.chunk]
        with tempfile.TemporaryDirectory(prefix="uc-upscale-") as tmp:
            indir, outdir = Path(tmp, "in"), Path(tmp, "out")
            indir.mkdir()
            outdir.mkdir()
            infos = {}
            for j in batch:
                try:
                    infos[j.key] = prepare(j, a, indir)
                except Exception as e:
                    log.error("FAILED %s: prepare: %s", j.rel, e)
                    failed += 1
            if infos and not run_upscaler(a, indir, outdir, log):
                # Find the bad one: one call per texture for this batch, and if that fails
                # too (out of GPU memory, say) once more with half the tile.
                for j in batch:
                    if j.key not in infos:
                        continue
                    one_in = Path(tmp, f"in_{j.key}")
                    one_in.mkdir()
                    for f in indir.glob(f"{j.key}*.png"):
                        f.rename(one_in / f.name)
                    if not run_upscaler(a, one_in, outdir, log):
                        log.info("retrying %s with tile %d", j.rel, a.tile // 2)
                        run_upscaler(a, one_in, outdir, log, tile=a.tile // 2)
            for j in batch:
                if j.key not in infos:
                    continue
                try:
                    img = finish(j, infos[j.key], a, outdir)
                    save_atomic(img, j.dsts[0])
                    for d in j.dsts[1:]:
                        copy_atomic(j.dsts[0], d)
                    ok += 1
                    files_written += len(j.dsts)
                except Exception as e:
                    log.error("FAILED %s (%d files): %s", j.rel, len(j.dsts), e)
                    failed += 1
                px_done += j.pixels
        el = time.time() - t0
        n = ok + failed
        eta = el / px_done * (total_px - px_done) if px_done else 0
        log.info("%d/%d done, %d failed, %.2f s/texture, %d files written, elapsed %s, ETA %s",
                 n, len(jobs), failed, el / max(n, 1), files_written, hms(el), hms(eta))
    log.info("finished: %d upscaled, %d failed, %d files written in %s%s", ok, failed, files_written,
             hms(time.time() - t0), "; rerun to retry the failures" if failed else "")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
