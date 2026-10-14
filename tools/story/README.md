# Story generator

Builds `docs/STORY.md` (the world, the cast and a list of the missions) and `docs/scripts/<mission>.md` (one full write-up per mission) from the game's mission files.

- `story_gen.py`: reads the missions and writes the documents.
- `merge_notes.py`: merges the hand-written notes into `notes.json` (not kept in git).
- `notes/intro.md`: the front of `STORY.md`: the world, factions, cast, story arc and open threads.
- `notes/part*.json`: per mission, the story summary, the characters, names for unnamed people, and titles for the unreleased missions.
- `goals.json`: per mission, the badges and side objectives as this engine's checklist shows them. It was dumped once from `uc-game`; dump it again if the checklist changes.
- `view.sh`: a compact view of a `--draft` file, for reading a mission while writing its notes.

## Rebuilding

Export every mission to JSON with the `uc` tool, then run the two scripts (from Git Bash):

```sh
GAME="C:/Program Files (x86)/GOG.com/Urban Chaos"
mkdir -p "$TEMP/ucall"
for f in "$GAME"/levels/*.ucm; do
    n=$(basename "$f" .ucm)
    cargo run -q -p uc-tools -- mission export "$f" -o "$TEMP/ucall/$n.json" 2> "$TEMP/ucall/$n.err"
done
cd tools/story
python merge_notes.py "$TEMP/ucall"
PYTHONIOENCODING=utf-8 python story_gen.py "$TEMP/ucall" goals.json "$GAME" notes.json ../../docs/STORY.md
```

Sixteen older-format missions don't export yet; they're listed in `STORY.md` as not included.

`--draft <dir>` writes every mission, with no notes, to `<dir>`: the starting point for writing notes for a mission.
