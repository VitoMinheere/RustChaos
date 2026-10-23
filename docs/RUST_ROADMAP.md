# Rust rewrite and texture upscaling: a starting plan

This plan was written alongside the original source release
([dizzy2003/MuckyFoot-UrbanChaos](https://github.com/dizzy2003/MuckyFoot-UrbanChaos)). Paths such as
`fallen/Source/eway.cpp` are in that repository. So are the notes and Python prototypes it
mentions (`ARCHITECTURE.md`, `GOALS.md`, `tools/uc_formats/`): they belonged to a fork of
it and aren't part of this project. Claims marked **(unverified)** are untested.

## Status

**Built so far (see the [README](../README.md)):** Phase 0, Phase 1's mission tooling, Phase 2 and the start of Phase 3 below. The workspace is now split the way a rewrite needs it:

- **`uc-formats`** reads every file type a level uses, and **writes missions**. Every parseable retail `.ucm` round-trips byte for byte, and missions convert to and from JSON (clean or lossless) through the `uc` tool in `uc-tools`.
- **`uc-world`** is the renderer-independent game side. It loads a level and builds chunked, lit geometry (floor, kerbs, roofs, walls, props, cables, ladders), plus Darci: animated, controllable, following the ground and blocked by walls.
- **`uc-render`** is a wgpu renderer with:
  - mipmaps and alpha cut-outs;
  - distance fog into the level's sky colour;
  - 4× MSAA;
  - frustum culling (typically 50–110 of 256 chunks drawn);
  - offscreen rendering.
- **`uc-game`** is the gameplay, as a compact redesign rather than a port of `Person.cpp`, `pcom.cpp`, `combat.cpp` and `eway.cpp`:
  - one state machine for every person, driven by input or a small fight AI;
  - moves: punch/kick combos, side and back kicks, the sliding tackle, grapples, arrests, jumping, ledge pull-ups, ladders and falling;
  - a mission-script runner for the event-point subset the combat tutorial uses: messages, beacons, spawns, counters, kill and adjust points, win and lose.
- **`uc-viewer`** (winit) ties these together: plays a mission with a HUD (health, messages, beacon markers), a free camera, headless screenshots.
- **CI** (`.github/workflows/rust.yml`) builds, tests and lints on Windows and Linux.

All of it was checked against the GOG release: the police station lobby, the Union City streets of Police 1, and the Southside map of "The Fallen" at night. **The combat tutorial (`ftutor1`) is playable from start to finish.** A test bot (`uc-game/tests/ftutor1.rs`) completes it headlessly in about two minutes of game time.

**Missions that play to the end:**
- the combat tutorial (`ftutor1`);
- Combat Silver and Gold (`fight1`, `fight2`): enemy waves with guard, MIB, fleeing and assassin AI, skill levels, knives, bats, pistols, shotguns and AK-47s, health packs and weapon pickups;
- Physical Training (`assault1`): fly-bys, the instructor's conversation, the course clock, crate stairs, the zip line and the stop-the-clock switch;
- Driving Bronze, Silver and Gold (`testdrive1a`, `testdrive2`, `testdrive3`): the squad car (arcade physics, the original's meshes), lap checkpoints, parking and direction checks, road signs, cones. People dodge cars coming at them, and the handbrake slides the car;
- RTA (`police1`), the first street mission: the abandoned car, the homicide call, witnesses and conversations, rooftop muggers, "person seen" triggers (`can_a_see_b`), assassins with their own mark (`ai_other`: the crash cops go after the drunk driver), wandering civilians (`PCOM_MOVE_WANDER`) and badges (`create_treasure`, with their stat bonuses carried between missions);
- The Jump (`police2`): the jumper who runs off the roof if approached head on (patrols, `PCOM_MOVE_PATROL`, follow the points of their colour and group; fall damage after `person_hit_ground`), and the zip line that brings Darci down behind him;
- Nitro Car (`Bankbomb1`), the first mission played as somebody else: `create_player`'s `PT_GANG` makes Mako a rasta thug (`PLAYER_THUG`, Player.cpp), with the rasta gang's voice, a pistol to start with (`create_player`'s `Data[1]`) and no power of arrest (`set_person_croutch` asks for `PERSON_DARCI` first). His model is `rthug.all`'s, not Roper's: `anim_type[PERSON_THUG_RASTA]` is `ANIM_TYPE_CIV` while Roper has his own `ANIM_TYPE_ROPER`. The animations really are Darci's, for everybody: `setup_global_anim_array` fills all four slots from `game_chunk[ANIM_TYPE_DARCI]` and every per-type override is commented out. Two car bombs driven across town and left to go off (`lock_vehicle` and `create_vehicle`'s `VK_*` keys; a `kill_waypoint` on a car wrecks it where it stands and sets off a shockwave that catches the player, then forgets it was ever created so the "car destroyed" lose points stop watching), explosions and fire (`visual_effect` to `PYRO_construct`), bodyguards who follow Mako and see off whoever comes at him (`PCOM_AI_BODYGUARD`, `PCOM_MOVE_FOLLOW`, invulnerable twenty blocks away), and the `specific_item_held` trigger;
- Gun Hunt (`police3`), on the Southside map: Deeks the snitch and the stashed weapon behind the skips (Deeks is dropped somewhere different each run by `move_thing` points on a `random` trigger; the four aimed at the gun move nothing, as in the original), the armed suspect who comes looking for it once it's picked up (`specific_item_held` again), the station cuboid that takes it back in, and losing the mission by killing or arresting Deeks. Its one unsupported point is `warefx`, a warehouse's own ambience;
- Rat Catch (`Gangorder2`), Mako's again: the Wildcats who fall in behind him, McIntyre's son running (`PCOM_AI_FLEE_PLAYER`), surrendering at half health and then following (`PCOM_MOVE_FOLLOW`), and both endings. Killing the son turns the job into the Boss's punishment, five more dead cops, which the original tallies in counter 7 wherever a `PERSON_COP` dies (Person.cpp); `reset_counter` went in with it. Traps and mist are still to do.

The viewer saves completed missions, best times and badges, and the menu shows them. The HUD has a minimap rendered from overhead.

Darci picks up and fires the pistol, shotgun and AK-47 (the original's damage, ammo per clip and animations), with a sticky auto-target she can cycle, and enemies show health bars.

Tests in `uc-game/tests/` check each of them (`missions.rs` runs the scripts to the end), plus RTA's, The Jump's, Nitro Car's and Gun Hunt's scripts (`rta.rs`, `jump.rs`, `bankbomb1.rs`, `police3.rs`), the moves (`moves.rs`) and the guns (`guns.rs`: pickups, ammo, switching, and a bot shooting its way through Combat Gold's waves).

**Next steps, toward a game that keeps the original's atmosphere:**
- **The city's life:** the crowd and traffic are in (`fake_civs`, `fake_cars`, `PCOM_AI_DRIVER`); still to do are the crime rate, the bored passers-by who turn into muggers (`BOREDOM_RATE` in `PCOM_do_regen`), drivers who flee their cars, route-finding for people on foot (MAV) and the remaining cutscene types;
- the MIB's minigun, grenades and muzzle flashes;
- fence vaulting, ledge hanging and traversing;
- sky details (stars, moon, skyline).

## 1. Size of the job

| Part | Lines (C/C++, compiled for PC) | Biggest files |
|---|---|---|
| Game logic (`fallen/Source`, 103 files) | ~202,000 | `Person.cpp` 22.5k, `pcom.cpp` (AI) 15.1k, `building.cpp` 11.7k, `collide.cpp` 11.4k, `interfac.cpp` (input) 8.5k, `eway.cpp` (missions) 8.2k, `frontend.cpp` 7.8k |
| Engine (`fallen/DDEngine`, non-Dreamcast) | ~99,000 | `aeng.cpp` 17.1k, `figure.cpp` 10.2k, `panel.cpp` 6.7k, `facet.cpp` 5.7k, `poly.cpp` 5.0k |
| Platform layer (`fallen/DDLibrary`) | ~28,000 | DirectX/Miles/Bink wrappers. Replaced wholesale rather than ported. |

A good share of that is `#ifdef PSX` / `TARGET_DC` / commented-out code, but even so
a faithful gameplay port is a multi-month project. The way to keep it tractable is
to get value at every step, which this plan is built around.

## 2. Two strategies, and a recommendation

**A. Clean reimplementation in Rust, using the C as the spec.** This is the OpenMW / OpenRA style. Data formats first, then rendering, then gameplay, system by system.
- Pros: idiomatic Rust; no DirectX/Win32 baggage; free choice of renderer.
- Cons: gameplay "feel" (fixed-point maths, tick rates, AI quirks) is easy to get subtly wrong, and there is no running reference to diff against.

**B. Port the C first (goal A), then replace it module by module from Rust over FFI.** This is the "strangler fig" approach.
- Pros: the game is playable the whole time, and every replaced module can be compared against the original.
- Cons: goal A has to happen first; FFI boundaries through global-heavy C code (things, `PAP_hi`, `EWAY_way`) are awkward.

**Recommendation: start with A's first two phases (data crates and a viewer), which are useful on their own and needed by both strategies.** Decide between A and B for gameplay once you've seen how the viewer work goes. B also has a built-in test harness worth keeping in mind: the game records and replays input (`GS_RECORD`/`GS_PLAYBACK`, `do_packets` in `fallen/Source/Thing.cpp`). A deterministic port could replay the same input file and compare positions frame by frame against the C build **(unverified that the C simulation is fully deterministic)**.

## 3. Phased plan

### Phase 0: workspace and data crate (days)

```
urban-chaos-rs/
  crates/uc-formats   # parsers for .iam .ucm .all .prm .lgt .txc .sty (+ writers later)
  crates/uc-tools     # CLI: verify, render, dump-json, extract-textures (goal C)
  crates/uc-viewer    # later: 3D viewer
```

- Port `tools/uc_formats/ucformats.py` to `uc-formats`. Suggested crates: `binrw` (or plain byte-cursor code), `thiserror`, `serde` for JSON.
- **Tests against real data, outside the repo.** Read the install path from an env var (e.g. `UC_GAME_DIR`) and skip when it's unset. Port the assertion that every shipped file ends exactly at EOF; `verify_formats.py` gives the expected pass/fail list (ARCHITECTURE §5, "Retail data check").
- Add **writers** (`.ucm` first, see `docs/MISSIONS.md`) with byte-exact round-trip tests over every shipped file.
- Keep the game's numeric conventions as explicit types:
  - world coordinates are integers (`SLONG`, often `<< 8` fixed point inside `Thing.WorldPos`);
  - angles are 0–2047 (`yaw << 3` in object records, 2048 per turn);
  - one map square = 256 world units.

### Phase 1: goal C in Rust (days)

`uc-tools render` and `dump-json`, matching `iam_render.py` / `dump_json.py`. Use the
`image` crate for PNG, `tiny-skia` for lines, `clap` for the CLI.
`docs/images/disturb1_topdown.png` is the reference output. Generate the textured version locally with `iam_render.py --clump`; don't commit it (it contains game art).

### Phase 2: static 3D viewer (weeks)

Pick a stack: `winit` + `wgpu` + `glam` is flexible and portable. `bevy` gets you
further faster but pushes an ECS shape onto code that is not ECS-shaped (the
original `Thing` model is closer to a tagged union with per-class state functions).
Steps:

1. **Terrain**: a mesh from `PAP_Hi` (128×128 squares; corner height `Alt << 3`; kerbs via `PAP_FLAG_SINK_*`, see `fallen/Source/pap.cpp`), textured from the mission's clump with the rotation table in `ucformats.FLOOR_ROTATION_UVS`.
2. **Props**: `.prm` meshes placed from `OB_Ob` records (prim ID, position, yaw).
3. **Buildings**: the hard part. Walls are generated at load time from `DFacet` + `DStorey` + style/paint data (`fallen/Source/building.cpp`, `build2.cpp`, `supermap.cpp`; drawing in `fallen/DDEngine/Source/facet.cpp`, `superfacet.cpp`). Roofs come from `RoofFace4`. Port `FACET_draw`'s geometry generation first, ignoring lighting.
4. **Lighting**: the `.lgt` lights plus ambient (`NIGHT_*`, `fallen/Source/night.cpp`). The original lights vertices on the CPU; a modern renderer can use real per-pixel lights.
5. **Sky**: `stars\poo`, `sky.tga`, `SKY_*` in `fallen/DDEngine/Source/sky.cpp`.

### Phase 3: characters and animation (weeks)

- Load `.all` files: body meshes (15-part rigid skeleton, see ARCHITECTURE §5) and keyframes.
- Decode the compressed per-part matrices: `GameKeyFrameElementBig` → `CMatrix33` → `SetCMatrix` / `ULTRA_COMPRESSED_ANIMATIONS` in `fallen/Editor/Headers/Anim.h` and `fallen/Source/Anim.cpp`. This decode hasn't been traced yet.
- Play back tweens like `FIGURE_draw_prim_tween` (`fallen/DDEngine/Source/figure.cpp`).
- A model/animation browser is a good milestone: it would also answer goal B's visual questions, such as how Darci's climbing animations look on the MIB body.

### Phase 4: gameplay (months)

Suggested order, each module ported against the C source as the spec:

1. Thing allocation, map-who and tick timing (`Thing.cpp`, `process_things_tick`).
2. Collision and walkable surfaces (`collide.cpp`, `walkable.cpp`).
3. The player person (`Person.cpp` states + `interfac.cpp` input).
4. Mission engine (`elev.cpp` load + `eway.cpp` runtime). Playable "walk around and trigger messages" missions follow.
5. Combat and weapons (`combat.cpp`, `guns.cpp`, `Special.cpp`).
6. AI (`pcom.cpp`, pathfinding `mav.cpp`).
7. Vehicles (`vehicle.cpp`).
8. Frontend and HUD (`frontend.cpp`, `panel.cpp`).
9. Audio: plain `.wav` through e.g. `kira` or `rodio`. Music-world switching follows `music.cpp`.
10. Video: Bink 1 `.bik`. Use an FFmpeg binding, or skip **(unverified that FFmpeg handles these files)**.

Keep the original's fixed-point integer maths where it affects behaviour (movement,
collision, AI distances). Floating point makes the feel drift.

## 4. Texture upscaling

### What you start from

Every world, prim and character texture is inside `clumps\<mission>.txc` (the
retail install has no loose copies). They are stored as **16-bit** colour: RGB565, or
ARGB4444 with alpha. Sizes are mostly 32×32 and 64×64, with some 128 and 256.
`police1.txc` has 940 entries:

| Size | Count |
|---|---|
| 32×32 | 352 |
| 64×64 | 449 |
| 128×128 | 9 |
| 256×256 | 21 |

Extract them losslessly with:

```
python tools/uc_formats/txc_extract.py <GAME>/clumps/police1.txc police1_textures
```

This writes one `page_NNNN.png` per texture page; NNNN is the page ID used everywhere else.

### Upscaling them

- **Tool**: use an ESRGAN-family upscaler (e.g. Real-ESRGAN or a "pixel art / game texture" model in a front end such as chaiNNer or Upscayl) at 4×. Expect to touch up results by hand. The 16-bit sources have colour banding, which a denoising model or a light dither-removal pass handles better than plain 4×.
- **Tiling textures** (roads, pavements, walls, most world textures) repeat across squares. Upscale them *wrapped*: pad each image with its own opposite edges (or tile it 3×3), upscale, then crop the centre, otherwise seams appear.
- **Alpha** (ARGB4444 entries: windows, fences, foliage) has only 16 levels. Upscale the alpha channel separately and threshold or clean it, or the cut-outs get mushy.
- **Keep the page IDs and sizes square, power-of-two.**
- Some pages are text or decals (signs, graffiti), where AI upscaling tends to invent detail. Review those manually.

### Getting them into a game

**Into the Rust renderer**: no size limit. Load PNG/KTX2 overrides by page ID (e.g. `overrides/page_0123.png`) with the clump as fallback, generate mipmaps, and use BCn compression for VRAM.

`uc-viewer --overrides DIR` does the PNG part. It tries `DIR/<mission>/page_NNNN.png`, then `DIR/world<N>/page_NNNN.png` (N is the map's texture set), then `DIR/page_NNNN.png`, then the clump.

**Missions with no clump** (the unfinished ones) borrow pages by number from other missions' clumps. Pages that no finished mission in their world uses get another world's picture or none at all. `uc textures missing [--overrides DIR]` lists those pages by world, the ones to paint as `world<N>/` overrides, and marks the ones already done.

The worst cases are:

- Worlds 2 (FreeCD1, train1), 6 (subdock1) and 12 (oldsmoke1): almost every page.
- World 6 also has no style table, so its walls follow world 3's layout.
- `uc-viewer --mission NAME --dump-pages DIR` writes the pictures a level draws with now, at the right sizes, as a starting point.

The PlayStation build doesn't help here. Its 34 levels (`levels[]`, `fallen/Source/supermap.cpp:2582`) are all maps the PC ships with clumps. Its `textures.tex` files are raw 4-bit VRAM dumps per level number, with pages renumbered from the PC's (`fallen/psxlib/Source/GDisplay.cpp:174`).

**Into the original 1999 engine (retail exe or a source build)** **(unverified in the retail exe; try on a backup)**:

- **Size limit**: the engine accepts only square power-of-two textures up to **256×256**. `D3DTexture::Reload_TGA` loads into a 256×256 buffer and accepts widths of 32, 64, 128 and 256 (`fallen/DDLibrary/Source/D3DTexture.cpp:~1505-1545`). So 32→128 or 64→256 (4×) fits; larger needs engine changes.
- **File format**: uncompressed TGA (image type 2, 24- or 32-bit, or palettised); RLE TGAs are rejected (`TGA_load_from_file`, `fallen/DDLibrary/Source/Tga.cpp`).
- **Turn clumps off**: set `enable_clumps=0` under `[TextureClumps]` in `config.ini`. The engine then loads loose files per page: `tex%03dto.tga`, else `tex%03dhi.tga`, else `tex%03d.tga` (`TEXTURE_load_page`, `fallen/DDEngine/Source/texture.cpp:742`; size is taken from the TGA header, not the name). There is **no per-page fallback to the clump**, so *every* page a level uses must then exist as a loose file.
- **Where pages go**: `TEXTURE_load_page`; directories from `TEXTURE_choose_set`; N = the map's `texture_set`.

  | Pages | Directory | File number |
  |---|---|---|
  | 0–255 | `server\textures\world<N>\` | page |
  | 256–511 | `server\textures\shared\` | page |
  | 512–575 | `...\shared\people2\` or `...\world<N>\insides\` | page − 512 |
  | 576–703 | `...\shared\people\` | page − 576 |
  | 704–1151 | `...\shared\prims\` | page − 704 |
  | 1152–1343 | `...\shared\people2\` | page − 1152 |

- **Next step**: export every page of every clump into that tree as TGA, at original or upscaled size. This is a small addition to `txc_extract.py`. It needs the mission → map → `texture_set` link from `dump_json.py`. Different missions on the same world should contain identical pages; worth asserting while exporting.

### Licensing note

Upscaled textures are derivatives of the game's copyrighted art. Keep them, and any
extracted data, out of this repository. Share the scripts, not the outputs.
