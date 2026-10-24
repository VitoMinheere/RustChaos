"""What the missions use, by the original's names, and which of those the Rust code never names.

`uc-game`'s `unsupported` example already checks every event point's type and trigger; this
goes a level down, into the numbers inside each point: the enemy types, AI types, moves and
personalities a `create_enemies` uses, the items, vehicles, keys, barrels, creatures and so
on. Each value is looked up in the original's headers and checked against the Rust code by
name, in full (`PCOM_MOVE_WANDER`) or by the part after the prefix (`MOVE_WANDER`). The Rust
code often handles these by number, so a value it never names isn't necessarily missing: the
report is a list of things to check, most-used first.

Export the missions to JSON first (see tools/story/README.md), then:

    python tools/parity/missions.py <json dir> [--game <Urban Chaos folder>] [-o report.md]
                                    [--all] [--original <fallen folder>]

Only the missions the game lists (`data/urban.sty`) count, unless `--all`.
"""

import argparse
import json
import os
import sys
from collections import defaultdict
from pathlib import Path

from source import Rust, by_value, family, original_root

GOG = Path(r"C:\Program Files (x86)\GOG.com\Urban Chaos")


def released(game):
    """(file name, title) of the missions the game lists, from data/urban.sty."""
    sty = game / "data" / "urban.sty"
    if not sty.exists():
        return []
    out = []
    for line in sty.read_text(encoding="latin-1").splitlines():
        if line.lstrip().startswith("//"):
            continue
        f = [x.strip() for x in line.split(" : ")]
        if len(f) >= 9 and f[7].lower().endswith(".ucm"):
            name = f[7][:-4]
            if not any(n.lower() == name.lower() for n, _ in out):
                out.append((name, f[8]))
    return out


class Families:
    """The original's names for the numbers inside event points."""

    def __init__(self, root):
        m, p, e, s, pe = "Headers/Mission.h", "Headers/pcom.h", "Headers/eway.h", "Headers/Special.h", "Headers/Person.h"
        self.prefix = {}
        self.names = {}
        for key, header, pat, excl in [
            ("PT", m, r"PT_", None), ("ET", m, r"ET_", None), ("IT", m, r"IT_", None),
            ("VT", m, r"VT_", None), ("VMT", m, r"VMT_", None), ("VK", m, r"VK_", None),
            ("BT", m, r"BT_", None), ("AP", m, r"AP_", None), ("VFX", m, r"VFX_", None),
            ("CT", m, r"CT_", r"CT_(NORMAL|ATTACHED|NEAREST_LIVING)$"), ("OT", m, r"OT_", None),
            ("WPT_FLAGS", m, r"WPT_FLAGS_", None), ("LP", m, r"LP_", None), ("ZF", m, r"ZF_", None),
            ("HAS", m, r"HAS_", None), ("PCOM_AI", p, r"PCOM_AI_", r"PCOM_AI_(STATE|SUBSTATE)_"),
            ("PCOM_MOVE", p, r"PCOM_MOVE_", r"PCOM_MOVE_(STATE|SUBSTATE)_"), ("PCOM_BENT", p, r"PCOM_BENT_", None),
            ("SPECIAL", s, r"SPECIAL_", r"SPECIAL_(AMMO_IN|GROUP|SUBSTATE)_"),
            ("ITEM_ARG", e, r"EWAY_ARG_ITEM_", None), ("FLAG2_PERSON", pe, r"FLAG2_PERSON_", None),
        ]:
            self.prefix[key] = pat
            self.names[key] = by_value(family(root, header, pat, excl))
        # The flags above the zone in `create_enemies`' Data[4] >> 8 (EWAY_create_enemy, eway.cpp).
        self.names["ENEMY_FLAG"] = {16: "FLAG2_PERSON_INVULNERABLE", 32: "FLAG2_PERSON_GUILTY", 64: "FLAG2_PERSON_FAKE_WANDER"}
        self.prefix["ENEMY_FLAG"] = r"FLAG2_PERSON_"

    def name(self, key, value):
        return self.names[key].get(value, f"{key} {value}?")


class Tally:
    """Which missions use each (field, value)."""

    def __init__(self):
        self.uses = defaultdict(lambda: defaultdict(set))
        self.points = defaultdict(lambda: defaultdict(int))

    def add(self, field, key, value, mission):
        self.uses[(field, key)][value].add(mission)
        self.points[(field, key)][value] += 1

    def bits(self, field, key, value, mission, width=8):
        for b in range(width):
            if value >> b & 1:
                self.add(field, key, 1 << b, mission)


def tally_mission(t, title, j):
    for ep in j.get("event_points", []):
        kind, d = ep.get("type"), ep.get("data", [0] * 10)
        t.add("any point's on_trigger", "OT", {"none": 0, "active": 1, "active_while": 2, "active_time": 3, "active_die": 4}.get(ep.get("on_trigger"), ep.get("on_trigger")), title)
        t.bits("any point's flags", "WPT_FLAGS", ep.get("flags", 0), title)
        if kind == "create_player":
            t.add("create_player: player type", "PT", d[0], title)
        elif kind == "create_enemies":
            t.add("create_enemies: enemy type", "ET", d[0] & 0xFFFF, title)
            t.add("create_enemies: AI", "PCOM_AI", d[5] & 0xFFFF, title)
            t.add("create_enemies: move", "PCOM_MOVE", d[3] + 1, title)
            t.bits("create_enemies: personality", "PCOM_BENT", d[4] & 0xFF, title)
            t.bits("create_enemies: weapons", "HAS", (d[2] >> 16) & 0xFF, title)
            t.bits("create_enemies: flags", "ENEMY_FLAG", (d[4] >> 8) & 0xF0, title)
            for b in range(32):
                if d[8] >> b & 1:
                    t.add("create_enemies: drops", "SPECIAL", b + 1, title)
        elif kind == "adjust_enemy":
            t.add("adjust_enemy: AI", "PCOM_AI", d[5] & 0xFFFF, title)
            t.add("adjust_enemy: move", "PCOM_MOVE", d[3] + 1, title)
            t.bits("adjust_enemy: personality", "PCOM_BENT", d[4] & 0xFF, title)
        elif kind == "create_vehicle":
            t.add("create_vehicle: vehicle", "VT", d[0], title)
            t.add("create_vehicle: move", "VMT", d[1], title)
            t.add("create_vehicle: key", "VK", d[3], title)
        elif kind == "create_item":
            t.add("create_item: item", "IT", d[0], title)
            t.bits("create_item: flags", "ITEM_ARG", d[2], title)
        elif kind == "create_creature":
            t.add("create_creature: creature", "CT", d[0], title)
        elif kind == "create_barrel":
            t.add("create_barrel: barrel", "BT", d[0], title)
        elif kind == "activate_prim":
            t.add("activate_prim: prim", "AP", d[0], title)
        elif kind == "visual_effect":
            t.bits("visual_effect: effects", "VFX", d[0], title)
        elif kind == "link_platform":
            t.bits("link_platform: flags", "LP", d[1], title)
    zones = set()
    for row in j.get("zones", []):
        for k in range(0, len(row), 2):
            zones.add(int(row[k:k + 2], 16))
    for z in zones:
        for b in (1, 2, 4, 128):
            if z & b:
                t.add("zone map", "ZF", b, title)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("json_dir", help="the missions exported with `uc mission export`")
    ap.add_argument("--game", help="the Urban Chaos folder (for the mission list)")
    ap.add_argument("--original", help="the original's `fallen` folder")
    ap.add_argument("--all", action="store_true", help="every exported mission, not only the ones the game lists")
    ap.add_argument("-o", "--output", help="write the report here instead of printing it")
    args = ap.parse_args()

    root = original_root(args.original)
    game = Path(args.game or os.environ.get("UC_GAME_DIR") or GOG)
    fam = Families(root)
    rust = Rust()

    files = {p.stem.lower(): p for p in Path(args.json_dir).glob("*.json")}
    listed = released(game)
    if args.all or not listed:
        missions = [(p.stem, p.stem) for p in sorted(files.values())]
    else:
        missions = [(n, title) for n, title in listed if n.lower() in files]
    t = Tally()
    for name, title in missions:
        j = json.loads(files[name.lower()].read_text(encoding="utf-8"))
        tally_mission(t, title, j)

    out = ["# Parity: what the missions use\n",
           f"{len(missions)} missions. A value is **named** when the Rust code mentions its original name "
           "in full, **short** when only the part after the prefix turns up (to check), **no** when "
           "neither does: those are worth a look first. Numbers are points using it.\n"]
    todo = []
    for (field, key), values in sorted(t.uses.items()):
        rows = []
        for value, users in values.items():
            name = fam.name(key, value)
            state = "named" if rust.cites(name) else "short" if name in fam.names[key].values() and rust.cites_short(name, fam.prefix[key]) else "no"
            rows.append((state, name, value, users, t.points[(field, key)][value]))
            if state == "no":
                todo.append((len(users), field, name, users))
        rows.sort(key=lambda r: ({"no": 0, "short": 1, "named": 2}[r[0]], -len(r[3])))
        out.append(f"## {field}\n")
        out.append("| Value | In Rust | Points | Missions |")
        out.append("|---|---|---|---|")
        for state, name, value, users, points in rows:
            shown = sorted(users)
            more = f" and {len(shown) - 8} more" if len(shown) > 8 else ""
            out.append(f"| `{name}` ({value}) | {state} | {points} | {len(users)}: {', '.join(shown[:8])}{more} |")
        out.append("")
    todo.sort(key=lambda r: -r[0])
    out.insert(2, "## Not named anywhere, most-used first\n\n" + "\n".join(
        f"- `{name}` ({field}): {n} missions" for n, field, name, _ in todo) + "\n")
    text = "\n".join(out) + "\n"
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
