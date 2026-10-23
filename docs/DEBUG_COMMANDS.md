# Debug commands

The developer console (`crates/uc-viewer/src/console.rs`, after the original's F9
`parse_console`, Controls.cpp:238). Open it with **`~`** (backquote) or **F9**, type a
command, **Enter** runs it, **Esc** (or `~`/F9 again) closes it. Commands are not
case-sensitive. While it's open the console takes the keyboard.

It isn't available while paused, while watching a replay, or in co-op.

> Commands marked **✱** change the game, so a run that used one isn't kept as a replay (#184).

## Teleporting

| Command | What it does |
|---|---|
| `telw N` ✱ | Teleport to event point `N`. |
| `tels` | Store the current position. |
| `telr` ✱ | Return to the stored position (`tels` first). |
| `wpt` | Say the nearest event point and how many squares away it is. |
| `bonus` | List the open bonus objectives and badges (up to 5, with distance) and toggle showing them on the map in light blue. |
| `telb N` ✱ | Teleport to bonus objective `N` from the `bonus` list. |

Teleports don't work while driving.

## Mission

| Command | What it does |
|---|---|
| `win` ✱ | End the mission as won. |
| `lose` ✱ | End the mission as lost. |
| `restart` | Restart the mission. |
| `crime N` ✱ | Set the crime rate (0–100). |
| `wave N` ✱ | Jump to wave `N` (endless runs only). |
| `call WHAT` ✱ | Call in a Beat Cop incident now (Beat Cop shifts only): `mugging`, `armed`, `dumped`, `gang`, `wanted`, `drunk`, `getaway`, `jumper`, `bomb`. |
| `autopilot` ✱ | Put Darci on AI control, bot against bot (capture the flag only). |

## Player & cheats

| Command | What it does |
|---|---|
| `god` ✱ | Toggle god mode (no damage, health kept full). |
| `ammo` ✱ | Toggle unlimited ammo, grenades and explosives. |
| `freeze` ✱ | Toggle freezing everyone else. |
| `kit` ✱ | Give the full kit. |
| `give WHAT` ✱ | Give one thing: `pistol`, `shotgun`, `ak`/`ak47`, `sniper`/`rifle`, `knife`, `bat`, `grenade(s)` (+3), `explosives`/`c4` (+3), `hook`/`grapple`, `health` (full), `key` (car key). |
| `kill` ✱ | Knock out everyone after the player. |
| `kill near` ✱ | Same, but only within 10 squares. |
| `roper` ✱ | Play as Roper (only where the mission loaded his body). |
| `darci` ✱ | Play as Darci. |

## Spawning

| Command | What it does |
|---|---|
| `car` ✱ | An empty sedan just ahead (on foot only). |
| `dog` / `spawn dog` ✱ | A dog just ahead (on foot only, needs `DOG2.all`). |

## Effects & physics tests

| Command | What it does |
|---|---|
| `boo` ✱ | The game-over explosion (`PYRO_GAMEOVER`) where the player stands. |
| `pow` ✱ | A POW bang a square in front of the player. |
| `sm` | Clear the sphere-matter and drop a cube of it above the player (Controls.cpp:4233). |
| `dike` | A new dike where the player stands; keypad 8 to go, `/` and `*` to steer (Controls.cpp:2217). |
| `hm` | An advertising sign (prim 6) as hypermatter above the player; up to eight (Controls.cpp:4269). |
| `flag` | A pink cloth flag near the player (Controls.cpp:4070). |

## Display

| Command | What it does |
|---|---|
| `pos` | Toggle the position readout (x/y/z, nearest event point, game time). |
| `time X` | Game speed, 0.1 to 4 (1 is normal). |
| `crinkles` | Toggle crinkles (walls in relief up close, Controls.cpp:450). |
| `help` / `?` | List the commands. |

## Editor only

| Command | What it does |
|---|---|
| `ep {json}` | Change the selected event point's fields, using the names from its JSON. `help` in the editor describes this. |

## Keys

| Key | What it does |
|---|---|
| `~` / F9 | Open/close the console. |
| Shift+F12 | Toggle the position readout (same as `pos`; the original's `cheat = 2`). |
| J + K + E + Space + D held together | Full kit ✱ (the original's button combination). |
| V (pad: Select) | Toggle free camera / third person. |
| T (in free camera) | Teleport Darci to where the camera is looking ✱. |
| F5 (in the editor) | Play the edit from its start; Shift+F5 starts Darci where the camera looks. Esc comes back. |
| F12 | Screenshot. |
| F11 / Alt+Enter | Toggle fullscreen. |
