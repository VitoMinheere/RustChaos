# Missions: where they live and how to make your own

This guide is for people who want to modify or author Urban Chaos missions. It is
based on the source in this repository and on the GOG release. Anything marked
**(unverified)** has not been tried in the running game.

## 1. There is no mission scripting language

Missions are **data, not code**. A mission is a graph of up to 512 *event points*
("waypoints"), each a fixed 74-byte record with:

- a type (what happens),
- a trigger (when it happens),
- an on-trigger rule (whether it stays active or resets),
- a position, a radius, and ten integer parameters.

GEdit, the in-house mission editor, placed these on the map. At load time
`ELEV_load_level` (`fallen/Source/elev.cpp:193`) converts each one into an `EWAY_Way`.
Every frame `EWAY_process` (`fallen/Source/eway.cpp:6542`) evaluates the conditions
and runs the actions.

`MuckyBasic/` is an unrelated in-house scripting language; the game never uses it.

## 2. Where a mission's pieces are stored

Taking Police 1 ("RTA") in the GOG install as the example:

| What | Where | Format / notes |
|---|---|---|
| The mission itself: event points, message and conversation text, cutscenes, zone map, crime rate, civilian/car density, music world | `levels\police1.ucm` | Binary (read and written by `crates/uc-formats/src/ucm.rs`). Export to JSON with the `uc` tool, below. |
| Which map to play on | `MapName` inside the `.ucm` → `Data\disturb1.iam` | Several missions share a map. |
| Lighting | `LightMapName` inside the `.ucm` → `Data\Lighting\disturb1.lgt` | |
| Ambient civilian chatter | `CitSezMapName` inside the `.ucm` → `Text\police11.txt` | Plain text: a section header, then numbered lines. |
| Menu entry: title, briefing, unlock order, city-map district | `data\urban.sty` (English). Other languages: `data\<Language>.sty`, chosen by the language file (`frontend.cpp:7039-7048`) | Text, one line per mission (§4). |
| Textures | `clumps\police1.txc` | **Chosen by the mission file name**, not the map (`TEXTURE_initialise_clumping`, `fallen/DDEngine/Source/texture.cpp:1073`). |
| Speech | `talk2\police1.ucm<N>.wav`, where N = event point index; conversations add a letter suffix | `eway.cpp:447`, `:519`. |
| Music | `MusicWorld` byte in the `.ucm` selects `data\sfx\1622\music<NN>\` | 0 is treated as 1. |
| Which character bodies to load | Hard-coded per mission name in `fallen/Source/frontend.cpp:~7633-7667` (`dontload`) | Unknown mission names get the default for their slot **(unverified)**. |

Shipped missions also have name-specific behaviour hard-coded in the source, e.g.:

- `playing_level("skymiss2.ucm")` adds a shotgun;
- `Finale1.ucm` and `park2.ucm` trigger FMVs;
- `semtex.ucm`, `estate2.ucm` and `Album1.ucm` change which animations load.

Search the source for `playing_level(` and `.ucm"` before reusing those names.

## 3. How an event point works

Fields (`EventPoint`, `fallen/Headers/Mission.h:18` in the original source; JSON names as the `uc` tool writes them):

| Field | Meaning |
|---|---|
| `type` | `WPT_*`: what the point *does* when it activates. Examples: `create_player`, `create_enemies`, `create_vehicle`, `create_item`, `message`, `conversation`, `nav_beacon`, `cut_scene`, `end_game_win`, `end_game_lose`, `kill_waypoint`, `group_life`/`group_death`, `bonus_points`, `autosave`… (58 types; `names::WAYPOINT` in `crates/uc-formats/src/ucm.rs`). |
| `trigger` | `TT_*`: *when* it activates. Examples: `none` (immediately), `dependency` (when the point in `ep_ref` is active), `radius` (player within `radius` of x/z), `timer`, `killed`, `person_arrested`, `group_dead`, `boolean_and`/`boolean_or` (of `ep_ref` and `ep_ref_bool`), `item_held`, `conversation_over`, `counter`… (45 triggers). |
| `on_trigger` | `OT_*`: `active` (stays on once triggered), `active_while` (only while the condition holds), `active_time` (for `after_timer`), `active_die`. |
| `colour` + `group` | Ties points into sets (e.g. all the waypoints of one patrol route, or a group of enemies). |
| `data[0..9]` | Type-specific parameters (below). |
| `x, y, z` | World position (one map square = 256 units). |
| `flags` | `WPT_FLAGS_INVERSE` (negate the trigger), `INSIDE`, `WARE`, `OPTIONAL`, … |

Missions chain these together. In Police 1, for example:

1. A `create_player` point (trigger `none`) spawns the player: Darci in nearly every mission, but `data[0]` chooses (Nitro Car's `PT_GANG` makes you a thug).
2. `message` points with `dependency` triggers deliver the radio briefing in sequence.
3. `create_enemies` points spawn the mugging crew.
4. A `group_death`/`killed`-triggered chain ends in `end_game_win`.

Export a shipped mission with `uc mission export` to see real chains.

`data[]` meanings that have been traced in the loader (`elev.cpp`, from about line 700):

| Type | Parameters |
|---|---|
| `create_player` (2) | `data[0]` = `PT_*` (1 Darci, 2 Roper, 3 cop, 4 gang); `data[1]` ≠ 0 means they start with nothing, 0 hands them a loaded pistol (`has = (subtype & 8) ? 0 : PCOM_HAS_GUN` in `EWAY_DO_CREATE_PLAYER`) |
| `create_enemies` (3) | `data[0]` = `ET_*` type in low 16 bits, count in high 16 bits; `data[1]` = follow target; `data[3]` = `PCOM_MOVE_*` − 1; `data[4]` = bent flags, zone in bits 8+; `data[5]` = `PCOM_AI_*` low 16 bits, skill high 16 bits; `data[7]` = bodyguard/assassin target event point |
| `message` (12) | `data[1]` = duration, `data[2]` = who says it |
| `nav_beacon` (27) | `data[1]` = target |
| `conversation` (35) | `data[1]`, `data[2]` = the two people; `data[3]` ≠ 0 means ambient |
| `cut_scene` (15) | Uses the next cutscene block in the file |
| `visual_effect` (14) | `data[0]` = `PYRO_construct`'s types (1 flash, 2 fireball, 4 dust, 8 streamers, 16 fire, 32 dome, 64/128 firebomb); `data[1]` = scale |
| `create_vehicle` (4) | `data[0]` = `VT_*`; `data[3]` = `VK_*` key (0 unlocked, 1–5 a coloured key, 6 locked for good) |
| `lock_vehicle` (43) | `data[0]` = the `create_vehicle` point; `data[1]` ≠ 0 locks, 0 unlocks |

For any other type, the `switch (event_point.WaypointType)` in `ELEV_load_level` is
the reference. So are the editor dialogs: each `GEdit/Source/<Type>Setup.cpp` file
shows which `Data[]` slot each dialog control writes.

**How text is stored.** Types that carry text (`message`, `nav_beacon`,
`bonus_points`, `conversation`) don't store it in the event point. In the editor,
`Data[0]` held a pointer to the text and `Data[9]` a text ID
(`GEdit/Source/Mission.cpp`, `HasText`/`GetEPTextID`). In the file, the strings follow
the event-point table, in event-point index order, interleaved with cutscene blocks.
That's why the loader has to count those types to know how many strings to read.

## 4. Adding a mission to the menus (`urban.sty`)

Mission lines (Version 4 format, parsed by `FRONTEND_ParseMissionData`, `fallen/Source/frontend.cpp:1385`):

```
ObjID : GroupID : Parent : ParentIsGroup : Type : Flags : District : Filename : Title : Briefing
6     : 0       : 3      : 0             : 1    : 0     : 4        : police1.ucm : RTA : Okay Stern, I guess you're ready...
```

- **ObjID**: unique, and below 60. Save games store a 60-byte `mission_hierarchy` (`FRONTEND_save_savegame`, `frontend.cpp:2517`). Shipped IDs run 2–34.
- **Parent**: the ObjID whose completion unlocks this mission. ID 1 is the implicit root and is always complete (`mission_hierarchy[1]=3`, `frontend.cpp:3398`), so `Parent = 1` should make a mission available from the start **(unverified)**.
- **Type**: 1 for a normal mission (every shipped line).
- **District**: 0-based index into the `[districts]` list at the bottom of the file, which gives the marker position on the city map.
- **Briefing**: `@` becomes a line break.
- The whole file must stay under 20,480 bytes (`SCRIPT_MEMORY`, `frontend.cpp:312`); the English one is 14,165.
- `frontend.cpp` also special-cases some shipped mission names (`ANNOYING_HACK_FOR_SIMON`), so avoid reusing those names for unrelated missions.

## 5. Ways to make your own mission

### Route 1: edit an existing mission (works with the retail game today) (unverified)

The lowest-risk route, because the clump, speech, menu entry and body-loading table
all stay valid:

1. Back up the original `.ucm`.
2. Export it to JSON with the `uc` tool (`crates/uc-tools`):

   ```
   cargo run --release -p uc-tools -- mission export "<GAME>\levels\police1.ucm" -o police1.json
   ```

3. Edit the JSON. Examples:
   - move the player spawn (`pos` of the `create_player` point);
   - swap enemy types (`create_enemies` `data[0]`: type in the low 16 bits, e.g. 19 = `mib1`, count in the high 16 bits);
   - change AI and skill (`data[5]`);
   - edit any message's `text`;
   - change `crime_rate`, `fake_civs` or `music_world`.
4. Convert it back under the original name and play it from the normal menu:

   ```
   cargo run --release -p uc-tools -- mission import police1.json -o "<GAME>\levels\police1.ucm"
   ```

The writer (`Mission::write` in `crates/uc-formats/src/ucm.rs`) is proven by round-trip tests: all 80 parseable retail missions are reproduced byte for byte, both directly and through lossless JSON.

What the JSON looks like:
- Each event point has `type`, `trigger` and `on_trigger` as names (or numbers), and its parameters as `data[0..9]`.
- Positions are `pos: [x, y, z]` in world units.
- A message's text is in its `text` field; a cut-scene point carries its `cutscene`.
- Zones are 128 hex rows.

`uc mission import` enforces the loader's rules:
- every message, nav-beacon, bonus-points and conversation point has text;
- every cut-scene point has a cutscene;
- event-point indices are unique and below 512.

The default clean export sets `data[0]` of text points to `1`. On disk that slot held an editor pointer, and the game only needs it to be nonzero.

Stay within the engine's limits: 512 event points, 20 cutscenes (`MAX_CUTSCENES`, `fallen/Headers/playcuts.h`), and about 4 KB of cutscene text. Testing an edited mission in the retail game is still **(unverified)**.

### Route 2: add a new mission alongside the shipped ones (unverified)

As route 1, but saved under a new name, plus:

1. A new line in `data\urban.sty` (and the other language `.sty` files, if you play in another language). See §4.
2. A copy of an existing clump renamed to your mission, e.g. `clumps\police1.txc` → `clumps\mymission.txc`. Use a mission on the same map. Textures are looked up by mission name, and with clumps enabled there is no fallback to loose files, so without the clump the level would load untextured **(unverified what exactly happens)**.
3. Optionally, speech files `talk2\mymission.ucm<N>.wav` and a citizen-chatter text file.
4. Expect the per-mission-name defaults in `frontend.cpp` (body loading) to treat it as an unknown mission. If you use MIBs or other optional bodies, check that they load, or add your mission to that table in a source build.

### Route 3: the original editor, GEdit (needs a source build)

GEdit is compiled into the game itself as a static library (`fallen/GEdit.dsp`; debug configuration with `/D "EDITOR"`). It's entered from the frontend's Editor menu item, which only exists in debug builds (`frontend.cpp`, `FE_EDITOR` under `!NDEBUG`), via `GS_EDITOR` → `editor_loop()` (`Game.cpp:1157`).

- Its menus and the dialogs for its 37 `*Setup.cpp` waypoint editors survive in `fallen/DDLibrary/DDlib.rc` (111 `IDD_`/`IDR_` resources).
- It works on workspaces (`*.ucw`) that combine a map (`.iam`), lighting (`.lgt`) and missions, and it can open and export `.ucm` files (`GEdit/Source/Mission.cpp` `export_mission`, `:568`).
- This is the full-featured route, with visual placement on the map and per-type dialogs, but it depends on goal A, a working Windows build. Because GEdit is a Win32 dialog application, an SDL2 port of the game would need to keep a Win32 editor build around, or GEdit's job moves into a new tool.

### Route 4: a new mission editor (fits the Rust plan)

The Rust crates already provide most of a modern editor's data model:
- read and write missions (`uc-formats`, `ucm` and `ucm_json`);
- build the level in 3D (`uc-world`) and render it (`uc-render`).

What's left is the editor UI: show event points in the 3D view, select and move them, and edit their parameters. See `docs/RUST_ROADMAP.md`.

## 6. New maps are a much bigger job

A `.iam` contains more than terrain:

- the heightmap and floor textures;
- every building as facets (walls, fences, ladders, doors, cables, fire escapes) with storeys, styles and painted texture data;
- the interiors and roof faces;
- prop placements.

All of this was produced by the original map editor (`fallen/Editor/`) and
`building.cpp`/`build2.cpp`. Lighting (`.lgt`) came from the light editor (`fallen/Ledit/`), and textures come from the clumps. Realistic options:

1. Reuse the 20 maps the shipped missions use (many missions already share maps).
2. Make small edits to existing maps (heights, floor textures, prop positions) with a `.iam` writer. That needs the same round-trip discipline as `.ucm`.
3. Build or port a real map editor, a large project.
