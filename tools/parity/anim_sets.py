"""The people's animations the PC game plays, by set, as the Rust tables `uc unused` checks the
install against (crates/uc-tools/src/unused.rs: `PLAYED_ANIMS`, `PLAYED_ROPER`,
`PLAYED_ROPER2`, `PLAYED_CIV`).

- Everyone's set (`darci1.all`, `ANIM_*` in Headers/animate.h): every id the game code
  (Source, DDEngine) names, less the four named only in code that never runs (#342).
- Roper's set (`roper.all`, with `police1.all`'s cops at 200 and the trainer's or Bane's suit
  at 240), Roper2's (`roper2.all`) and the civilians' (`rthug.all`, with `newciv.all` at 100
  and `newcivf.all` at 122): the ids `setup_global_anim_array` (Anim.cpp) copies into a slot
  of everyone's array that the code plays, and the ones started straight from the set
  (`set_anim_of_type` and the like, or `AnimList[...]` read straight from the set), and
  any of the set's own constants put in a variable (`anim = COP_ROPER_ANIM_IDLE1`, for
  `AnimList[anim]`). The `build_psx` and `save_psx` branches are left out:
  both are false on the PC. One call starts Roper's set with a variable: it's always
  `get_yomp_anim`'s `COP_ROPER_ANIM_RUN`, which a copy already counts.

Standard library only. Usage: python tools/parity/anim_sets.py [--original PATH]
"""

import argparse
import re
from pathlib import Path

from source import live_lines, original_root, strip_comments

# Named only in code that never runs (#342): `HIT_WALL` under `if(0)`, `STOP_RUN` in the
# `else` of an `if(1)` (interfac.cpp:2483), `NECK_SNAP` and `WARM_HANDS` never started.
DEAD = {"ANIM_HIT_WALL", "ANIM_STOP_RUN", "ANIM_NECK_SNAP", "ANIM_WARM_HANDS"}
SETS = {"ANIM_TYPE_ROPER": "roper", "1": "roper", "ANIM_TYPE_ROPER2": "roper2", "2": "roper2",
        "ANIM_TYPE_CIV": "civ", "3": "civ"}


def drop_pc_false(code):
    """Removes `if(build_psx) {...}` and `if(save_psx) {...}`, keeping their `else`."""
    res, i = "", 0
    pat = re.compile(r"if\s*\(\s*(build_psx|save_psx)\s*\)")
    while m := pat.search(code, i):
        res += code[i:m.start()]
        j = m.end()
        while code[j].isspace():
            j += 1
        if code[j] == "{":
            depth = 0
            while True:
                depth += {"{": 1, "}": -1}.get(code[j], 0)
                j += 1
                if depth == 0:
                    break
        else:
            j = code.index(";", j) + 1
        k = j
        while k < len(code) and code[k].isspace():
            k += 1
        i = k + 4 if code.startswith("else", k) else j
    return res + code[i:]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--original")
    root = original_root(ap.parse_args().original)
    files = sorted(root.glob("Source/*.cpp")) + sorted(root.glob("DDEngine/**/*.cpp"))
    code = "\n".join(drop_pc_false("\n".join(live_lines(strip_comments(f.read_text(encoding="latin-1"))))) for f in files)
    # animate.h first, so its values win where another header reuses a name.
    headers = "\n".join("\n".join(live_lines(strip_comments(f.read_text(encoding="latin-1"))))
                        for f in [root / "Headers" / "animate.h"] + sorted(root.glob("Headers/*.h")) + sorted(root.glob("Source/*.cpp")))

    defs = {}
    for m in re.finditer(r"^\s*#\s*define\s+(\w+)\s+(\(?[\w\s+\-()]+\)?)\s*$", headers, flags=re.M):
        defs.setdefault(m.group(1), m.group(2).strip())

    def val(name, depth=0):
        if re.fullmatch(r"\d+", name):
            return int(name)
        e = defs.get(name)
        if e is None or depth > 10:
            return None
        e = re.sub(r"[A-Za-z_]\w*", lambda m: str(val(m.group(0), depth + 1)), e)
        try:
            return int(eval(e, {"__builtins__": {}}))
        except Exception:
            return None

    # Everyone's set.
    # Without its comments: it ends with a commented-out block of older numbers.
    anim_h = "\n".join(live_lines(strip_comments((root / "Headers" / "animate.h").read_text(encoding="latin-1"))))
    named = set(re.findall(r"\bANIM_[A-Z0-9_]+\b", code)) - DEAD
    # animate.h's own values: other headers reuse some of the names.
    numbered = re.findall(r"^\s*#define\s+(ANIM_\w+)\s+\((\d+)\)", anim_h, flags=re.M)
    darci = sorted({int(v) for n, v in numbered if n in named})
    names = {}
    for n, v in numbered:
        names.setdefault(int(v), n)

    # The other sets: copies into a slot that's played, and straight starts.
    played = {"roper": set(), "roper2": set(), "civ": set()}
    copy = re.compile(r"global_anim_array\[[^\]]+\]\[\s*(\w+)\s*\]\s*=\s*game_chunk\[\s*(\w+)\s*\]\.AnimList\[\s*(\w+)\s*\]")
    outside = re.sub(r"global_anim_array\[[^\]]+\]\[\s*\w+\s*\]\s*=[^;]*;", "", code)
    for slot, chunk, const in copy.findall(code):
        if chunk in SETS and val(const) is not None and re.search(rf"\b{slot}\b", outside):
            played[SETS[chunk]].add(val(const))
    for args, t in re.findall(r"(?:set_anim_of_type|locked_anim_change_of_type|set_locked_anim_of_type)\s*\(([^;]*?),\s*(ANIM_TYPE_\w+)\s*\)", code):
        v = val(args.split(",")[-1].strip())
        if t in SETS and v is not None:
            played[SETS[t]].add(v)

    # Read straight from a set (`game_chunk[ANIM_TYPE_ROPER].AnimList[COP_ROPER_ANIM_RUN]`,
    # outside the copies), and the set's own constants put in a variable that's then used so
    # (the cops' idles, Person.cpp:11691: `anim = COP_ROPER_ANIM_IDLE1` ... `AnimList[anim]`).
    for chunk, const in re.findall(r"game_chunk\[\s*(\w+)\s*\]\.AnimList\[\s*(\w+)\s*\]", outside):
        if chunk in SETS and val(const) is not None:
            played[SETS[chunk]].add(val(const))
    families = {"roper": ("NROPER_", "COP_ROPER_"), "civ": ("CIV_",)}
    for set_, prefixes in families.items():
        for const in re.findall(r"[^=!<>]=\s*([A-Z_][A-Z0-9_]*)\s*;", code):
            if const.startswith(prefixes) and val(const) is not None:
                played[set_].add(val(const))

    def table(name, ids):
        rows = [", ".join(map(str, ids[i:i + 20])) for i in range(0, len(ids), 20)]
        return f"const {name}: &[usize] = &[\n" + "".join(f"    {r},\n" for r in rows) + "];"

    print(table("PLAYED_ANIMS", darci))
    print("const ANIM_NAMES: &[(usize, &str)] = &[")
    for v, n in sorted(names.items()):
        if v not in darci:
            print(f'    ({v}, "{n[5:]}"),')
    print("];")
    print(table("PLAYED_ROPER", sorted(played["roper"])))
    print(table("PLAYED_ROPER2", sorted(played["roper2"])))
    print(table("PLAYED_CIV", sorted(played["civ"])))

    # The names of the other sets' ids nothing plays: Roper's own (`NROPER_*`), the cops'
    # (`COP_ROPER_*`, from 200) and the civilians' (`CIV_*`, `CIV_M_*`, `CIV_F_*`).
    print("const SET_NAMES: &[(&str, usize, &str)] = &[")
    for set_, prefixes in (("roper", ("NROPER_", "COP_ROPER_")), ("civ", ("CIV_",))):
        seen = {}
        for n in defs:
            if n.startswith(prefixes) and not n.endswith(("_START", "_END")):
                v = val(n)
                if v is not None and v not in played[set_]:
                    seen.setdefault(v, n)
        for v, n in sorted(seen.items()):
            print(f'    ("{set_}", {v}, "{n}"),')
    print("];")


if __name__ == "__main__":
    main()
