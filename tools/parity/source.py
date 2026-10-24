"""Shared helpers for the parity reports: reading the original's headers and the Rust tree.

The original source is the MuckyFoot-UrbanChaos release; point at its `fallen` folder with
`--original` (or the `UC_SOURCE` environment variable).
"""

import os
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
RUST_ROOT = HERE.parent.parent
DEFAULT_ORIGINAL = RUST_ROOT.parent / "MuckyFoot-UrbanChaos" / "fallen"


def original_root(arg=None):
    root = Path(arg or os.environ.get("UC_SOURCE") or DEFAULT_ORIGINAL)
    if not (root / "Headers").is_dir():
        raise SystemExit(f"no original source at {root} (pass --original <path to fallen>)")
    return root


def strip_comments(text):
    """Drops /* */ and // comments, keeping the line count so line numbers still match."""
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


# Builds the shipped PC game doesn't have: code inside these is left out.
DEAD_IFS = (r"#if\s+0\b", r"#ifdef\s+PSX\b", r"#ifdef\s+TARGET_DC\b", r"#if\s+defined\s*\(\s*PSX\s*\)")


def live_lines(text):
    """The source's lines with the compiled-out ones blanked: `#if 0`, and the PSX and
    Dreamcast builds. Each `#else` flips its block. Naive, but good enough for a report."""
    out, stack = [], []
    for line in text.split("\n"):
        s = line.strip()
        if s.startswith("#if"):
            stack.append(any(re.match(p, s) for p in DEAD_IFS))
            out.append("")
            continue
        if s.startswith("#else") and stack:
            stack[-1] = not stack[-1]
            out.append("")
            continue
        if s.startswith("#endif") and stack:
            stack.pop()
            out.append("")
            continue
        out.append("" if any(stack) else line)
    return out


def read_source(path):
    return live_lines(strip_comments(Path(path).read_text(encoding="latin-1")))


DEFINE = re.compile(r"^\s*#define\s+([A-Z][A-Z0-9_]*)\s+(.+?)\s*$")


def number(expr):
    """The value of a define that's a plain number or a `(1 << n)`, or None."""
    expr = expr.strip()
    m = re.fullmatch(r"\(?\s*1\s*<<\s*(\d+)\s*\)?", expr)
    if m:
        return 1 << int(m.group(1))
    m = re.fullmatch(r"\(?\s*(-?(?:0x[0-9a-fA-F]+|\d+))\s*\)?", expr)
    if m:
        return int(m.group(1), 0)
    return None


def defines(path):
    """Every numeric #define in a header that the PC build sees: [(name, value, line)]."""
    out = []
    for n, line in enumerate(read_source(path), 1):
        m = DEFINE.match(line)
        if m:
            v = number(m.group(2))
            if v is not None:
                out.append((m.group(1), v, n))
    return out


def family(root, header, pattern, exclude=None):
    """The constants of one family: their names matching `pattern` in `header`, first
    definition of each name wins. Counts (`*_NUMBER`, `*_NUM`, `*_RANGE`, `MAX_*`) are dropped."""
    seen, out = set(), []
    for name, value, line in defines(root / header):
        if not re.match(pattern, name) or (exclude and re.match(exclude, name)):
            continue
        if re.search(r"_(NUMBER|NUM|RANGE|NUM_TYPES|COUNT)$", name) or name in seen:
            continue
        seen.add(name)
        out.append((name, value, line))
    return out


def by_value(consts):
    """value -> name, the first name for each value."""
    out = {}
    for name, value, _ in consts:
        out.setdefault(value, name)
    return out


class Rust:
    """The Rust tree's text, for checking whether it mentions a name from the original."""

    def __init__(self, root=RUST_ROOT):
        self.files = {}
        for p in sorted((root / "crates").rglob("*.rs")):
            if "target" not in p.parts:
                self.files[p.relative_to(root).as_posix()] = p.read_text(encoding="utf-8", errors="replace")
        self.words = set()
        for text in self.files.values():
            self.words.update(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", text))
        self.lower = "\n".join(self.files.values()).lower()

    def cites(self, name):
        return name in self.words

    def cites_short(self, name, prefix):
        """Whether the Rust code names it by the part after its family's prefix, as a constant
        (`trigger::RADIUS` for `TT_RADIUS`). Short or generic parts don't count."""
        short = name[len(re.match(prefix, name).group(0)):]
        return len(short) >= 5 and short not in ("FALSE", "NONE", "NORMAL") and short in self.words

    def where(self, name, limit=3):
        """The first few files that mention `name`."""
        pat = re.compile(r"\b" + re.escape(name) + r"\b")
        return [f for f, t in self.files.items() if pat.search(t)][:limit]

    def cites_file(self, filename):
        return filename.lower() in self.lower
