"""What the original has that the Rust code never mentions.

Two checks, both by name: the Rust code cites the original's names in its comments
(`PCOM_MOVE_WANDER`, `EWAY_DO_ELECTRIFY_FENCE`, `fallen/Source/eway.cpp`), so a name it never
mentions is a feature nobody has looked at yet, or one skipped on purpose.

- Constant families: the original's lists of things (mission actions, AI types, people's
  states, effects, music modes...), each constant marked done if the Rust code names it, in
  full or by the part after the family's prefix (`waypoint::CREATE_ENEMIES` for
  `WPT_CREATE_ENEMIES`; these are listed apart, to check by hand). The things missions are
  made of (enemy types, items, vehicles, triggers) are numbers in the Rust code rather than
  names: `missions.py` reports those, by what the missions actually use.
- Source files: for each file the PC build compiles, how many of its functions the Rust code
  names, and whether it names the file at all.

    python tools/parity/constants.py [--original <fallen folder>] [-o report.md] [--files]

It's a starting point for a person to check, not a verdict: something can be done without
being named, or named only to say it's still missing.
"""

import argparse
import re
import sys
from pathlib import Path

from source import Rust, family, original_root, read_source

# (title, header, name pattern, exclude pattern): the families worth tracking.
FAMILIES = [
    ("Mission actions (EWAY_DO_*)", "Headers/eway.h", r"EWAY_DO_", None),
    ("Mission conditions (EWAY_COND_*)", "Headers/eway.h", r"EWAY_COND_", None),
    ("AI types (PCOM_AI_*)", "Headers/pcom.h", r"PCOM_AI_", r"PCOM_AI_(STATE|SUBSTATE)_"),
    ("AI states (PCOM_AI_STATE_*)", "Headers/pcom.h", r"PCOM_AI_STATE_", None),
    ("AI substates (PCOM_AI_SUBSTATE_*)", "Headers/pcom.h", r"PCOM_AI_SUBSTATE_", None),
    ("AI moves (PCOM_MOVE_*)", "Headers/pcom.h", r"PCOM_MOVE_", r"PCOM_MOVE_(STATE|SUBSTATE)_"),
    ("AI move states (PCOM_MOVE_STATE_*)", "Headers/pcom.h", r"PCOM_MOVE_STATE_", None),
    ("AI personality (PCOM_BENT_*)", "Headers/pcom.h", r"PCOM_BENT_", None),
    ("AI voice lines (PCOM_SOUND_*)", "Headers/pcom.h", r"PCOM_SOUND_", None),
    ("People (PERSON_*)", "Headers/Person.h", r"PERSON_", r"PERSON_(DEATH|ON|FLAG)_"),
    ("Deaths (PERSON_DEATH_*)", "Headers/Person.h", r"PERSON_DEATH_", None),
    ("Person flags (FLAG_PERSON_*)", "Headers/Person.h", r"FLAG_PERSON_", None),
    ("Person flags 2 (FLAG2_PERSON_*)", "Headers/Person.h", r"FLAG2_PERSON_", None),
    ("States (STATE_*)", "Headers/statedef.h", r"STATE_", None),
    ("Substates (SUB_STATE_*)", "Headers/statedef.h", r"SUB_STATE_", None),
    ("Specials (SPECIAL_*)", "Headers/Special.h", r"SPECIAL_", r"SPECIAL_(AMMO_IN|GROUP|SUBSTATE)_"),
    ("Hit types (HIT_TYPE_*)", "Headers/combat.h", r"HIT_TYPE_", None),
    ("Effects (PYRO_*)", "Headers/pyro.h", r"PYRO_", r"PYRO_FLAGS_"),
    ("Explosions (POW_TYPE_*)", "Headers/pow.h", r"POW_TYPE_", None),
    ("Dirt (DIRT_TYPE_*)", "Headers/dirt.h", r"DIRT_TYPE_", None),
    ("Music modes (MUSIC_MODE_*)", "Headers/music.h", r"MUSIC_MODE_", None),
    ("Camera modes (CAM_MODE_*)", "Headers/cam.h", r"CAM_MODE_", None),
    ("Vehicle types (VEH_TYPE_*)", "Headers/vehicle.h", r"VEH_TYPE_", None),
    ("Fog (FOG_TYPE_*)", "Headers/fog.h", r"FOG_TYPE_", None),
]

FUNCTION = re.compile(
    r"^(?!(?:if|for|while|switch|return|else|do|case|typedef|struct|class)\b)"
    r"[A-Za-z_][\w \t\*&]*?[\s\*&]([A-Za-z_]\w*)\s*\(([^;{}]*)\)\s*\{",
    re.M,
)


def functions(path):
    """The functions a .cpp defines (that the PC build compiles), in order."""
    text = "\n".join(read_source(path))
    names = []
    for m in FUNCTION.finditer(text):
        name = m.group(1)
        if name not in names and name not in ("main", "WinMain"):
            names.append(name)
    return names


def pc_sources(root):
    """The Source/*.cpp files the PC build compiles (Fallen.vcxproj)."""
    proj = (root / "Fallen.vcxproj").read_text(encoding="utf-8", errors="replace")
    files = re.findall(r'ClCompile Include="Source\\([^"]+\.cpp)"', proj)
    return [root / "Source" / f for f in files if (root / "Source" / f).exists()]


def constants_report(root, rust, out):
    rows = []
    for title, header, pat, excl in FAMILIES:
        if not (root / header).exists():
            continue
        consts = family(root, header, pat, excl)
        if not consts:
            continue
        missing = [(n, v) for n, v, _ in consts if not rust.cites(n) and not rust.cites_short(n, pat)]
        short = [(n, v) for n, v, _ in consts if not rust.cites(n) and rust.cites_short(n, pat)]
        rows.append((title, header, consts, missing, short))
    out.append("## Constant families\n")
    out.append("| Family | Named in Rust | Only by short name | Header |")
    out.append("|---|---|---|---|")
    for title, header, consts, missing, short in rows:
        done = len(consts) - len(missing) - len(short)
        out.append(f"| {title} | {done} / {len(consts)} | {len(short)} | `{header}` |")
    out.append("")
    for title, header, consts, missing, short in rows:
        if not missing and not short:
            continue
        out.append(f"### {title}: {len(missing)} not named\n")
        out.append(", ".join(f"`{n}` ({v})" for n, v in missing) or "(none)")
        if short:
            out.append("\nNamed only by the short name, to check: " + ", ".join(f"`{n}`" for n, _ in short))
        out.append("")


def files_report(root, rust, out):
    rows = []
    for path in pc_sources(root):
        fns = functions(path)
        if not fns:
            continue
        cited = [f for f in fns if rust.cites(f)]
        rows.append((path.name, len(fns), cited, [f for f in fns if f not in cited], rust.cites_file(path.name)))
    rows.sort(key=lambda r: (len(r[2]) / r[1], -r[1]))
    out.append("## Source files (PC build)\n")
    out.append("Least covered first. A function counts when the Rust code names it.\n")
    out.append("| File | Functions named | File named | Not named (first few) |")
    out.append("|---|---|---|---|")
    for name, total, cited, missing, file_named in rows:
        pct = 100 * len(cited) // total
        sample = ", ".join(f"`{f}`" for f in missing[:6]) + (" ..." if len(missing) > 6 else "")
        out.append(f"| {name} | {len(cited)} / {total} ({pct}%) | {'yes' if file_named else 'no'} | {sample} |")
    out.append("")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--original", help="the original's `fallen` folder")
    ap.add_argument("-o", "--output", help="write the report here instead of printing it")
    ap.add_argument("--files", action="store_true", help="also list per-file function coverage")
    args = ap.parse_args()
    root = original_root(args.original)
    rust = Rust()
    out = ["# Parity: the original's names the Rust code never mentions\n"]
    constants_report(root, rust, out)
    if args.files:
        files_report(root, rust, out)
    text = "\n".join(out) + "\n"
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
