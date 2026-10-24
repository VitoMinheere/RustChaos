# Parity reports

Two scripts that list what the original game has and the Rust code doesn't mention yet, as a
starting point for finding features to build. They need the original source
([MuckyFoot-UrbanChaos](https://github.com/dizzy2003/MuckyFoot-UrbanChaos)) next to this
repository, or its `fallen` folder passed with `--original` (or `UC_SOURCE`). They use the
Python standard library only.

Both work by name. The Rust code cites the original's names in its comments, so a name it
never mentions is something nobody has looked at yet. That makes them lists to check, not
verdicts. Something can be done without being named (the Rust code handles many values by
number), and dead code in the original turns up too.

- `constants.py`: the original's families of constants and how many of each the Rust code
  names. These include mission actions, AI types, moves and personalities, people's states,
  deaths, effects, dirt, music and camera modes. `--files` adds, for each file the PC build
  compiles, how many of its functions the Rust code names, least covered first.
- `missions.py`: what the missions actually use, a level below the point types and triggers
  that `cargo run --release -p uc-game --example unsupported` already checks. That means the
  enemy types, AI, moves, personalities and weapons a `create_enemies` uses, plus the items,
  vehicles, keys, barrels, creatures, effects, point flags and zone flags. Each value comes
  with the number of points and the missions that use it, and the unnamed ones are listed
  most-used first.

```sh
python tools/parity/constants.py --files -o constants.md

# missions.py reads the missions as JSON: export them with the uc tool first.
GAME="C:/Program Files (x86)/GOG.com/Urban Chaos"
mkdir -p "$TEMP/ucall"
for f in "$GAME"/levels/*.ucm; do
    cargo run -q --release -p uc-tools -- mission export "$f" -o "$TEMP/ucall/$(basename "$f" .ucm).json"
done
python tools/parity/missions.py "$TEMP/ucall" -o missions.md
```

`missions.py` counts only the missions the game lists (`data/urban.sty`) unless you pass
`--all`.
