# RustChaos

A rewrite of Mucky Foot's *Urban Chaos* (1999) in Rust that keeps the original's look and feel: its levels, textures, lighting, characters, sounds and mission scripts, played in a new engine.

**You need your own copy of the game.** RustChaos contains no game data. It reads everything (maps, missions, textures, sounds) from an installed copy of Urban Chaos, such as the GOG release. It is a fan project, not affiliated with or endorsed by the game's rights holders.

The engine's behaviour is ported from the game's [released source code](https://github.com/dizzy2003/MuckyFoot-UrbanChaos) (MIT licence); see [NOTICE.md](NOTICE.md). Comments in the code name the original files and functions, such as `fallen/Source/eway.cpp`: those paths are in that repository, not this one.

What's there:
- **Rendering:** the original textures and the game's own baked lighting, with distance fog into each level's sky colour.
- **Play:** start at the mission menu (the game's own list and title art). The training missions play from start to finish:
  - the combat tutorial (`ftutor1`);
  - Combat Silver and Combat Gold (`fight1`, `fight2`): timed waves of skilled and armed enemies;
  - Physical Training (`assault1`): the assault course against the clock;
  - Driving Bronze, Silver and Gold (`testdrive1a`, `testdrive2`, `testdrive3`): laps in the squad car.

  The first street missions play to the end too: RTA (`police1`), The Jump (`police2`), Nitro Car (`Bankbomb1`), the first one played as somebody other than Darci, Gun Hunt (`police3`), Rat Catch (`Gangorder2`), Headline Hostage (`Gordout1`), Seek, Sneak and Seize (`Stealtst1`), Insane Assault (`Baalrog3`), Assassin (`Gangorder1`), Trouble in the Park (`police4`), Gatecrasher (`bball2`), Arms Breaker (`wstores1`), Media Trouble (`e3`), Urban Shakedown (`westcrime1`), Auto-Destruct (`carbomb1`), Grim Gardens (`botanicc`), Psycho Park (`park2`), Transmission Terminated (`factory1`), Target UC (`snow2`) and The Fallen (`MIB`).
- **Missions:** read, edit and write them as JSON.

No game data is included; everything is read from your install.

## Crates

| Crate | What it is | Depends on |
|---|---|---|
| `uc-formats` | Readers (and a mission writer) for the game's files: maps `.iam`, missions `.ucm`, texture clumps `.txc`, prims `.prm`, wall styles `style.tma`, lighting `.lgt`, characters/animations `.all`. Also the wall-geometry, lighting, ground-height and collision rules ported from the engine. Optional `json` feature: missions as JSON. | std only (`serde` with `json`) |
| `uc-world` | Renderer-independent world: loads a mission's level and builds chunked, lit geometry (floor, kerbs, roofs, walls, props, cables, ladders); poses and animates people (every person type, with Darci's animation set); knows where people can stand (terrain, roofs, ladders, walls). Any renderer can draw it. | `uc-formats`, `glam` |
| `uc-game` | Gameplay: one state machine for every person (moves, combos, slide tackle, grapples, arrests, jumping, ledges, ladders, falling), a small fight AI (including bodyguards), cars and their locks, explosions and the mission-script runner (event points: messages, beacons, spawns, counters, win/lose). | `uc-world` |
| `uc-audio` | Sound from your install: a WAV decoder (PCM and IMA ADPCM, which every voice line uses), positional sound effects, music that crossfades between the original's dynamic tracks, and mission voice lines (`talk2\<mission>.ucm<n>.wav`, by each message's speech number). Built on `rodio`. | `uc-formats` |
| `uc-render` | wgpu renderer: mipmapped textures, alpha cut-outs, distance fog, 4× MSAA, frustum culling per 8×8-square chunk, soft blobs for shadows and smoke and added ones for fire, offscreen rendering for screenshots. | `uc-world`, `wgpu` |
| `uc-viewer` | The app (winit): plays a mission with a HUD, sound and gamepad support, or explores a level; free camera; headless screenshots. | all of the above |
| `uc-tools` | `uc` command-line tool: missions to and from JSON. | `uc-formats` |

## Build

**Windows, GNU toolchain (no Visual Studio needed):**

```
rustup default stable-x86_64-pc-windows-gnu
winget install BrechtSanders.WinLibs.POSIX.MSVCRT
```

The full MinGW-w64 provides `dlltool`/`as`, which current Windows crates need on the GNU target. Open a new terminal afterwards so its `bin` folder is on PATH.

**Windows, MSVC toolchain:** install Visual Studio Build Tools (C++), then use the default `stable-x86_64-pc-windows-msvc`.

**Linux:** stable Rust is enough.

Then, from the repository root:

```
cargo run --release -p uc-viewer                                # the mission menu
cargo run --release -p uc-viewer -- --mission testdrive1a       # straight into a mission
cargo run --release -p uc-viewer -- --mission police1 --explore # walk around any level without its script
cargo run --release -p uc-viewer -- --help
```

The install is found from `--game DIR`, then `UC_GAME_DIR`, then the default GOG location (`C:\Program Files (x86)\GOG.com\Urban Chaos`).

## Viewer controls

| Input | Third person (Darci) | Free camera |
|---|---|---|
| WASD | Walk, relative to the camera | Fly |
| Shift | Run | Fast |
| Space | Jump (grabs ledges in front) | Up |
| J or left mouse | Punch | |
| K | Kick | |
| Q (hold) | Block | |
| G | Next weapon (bare hands, knife or bat, pistol, shotgun, AK-47, explosives) | |
| T | Target the next enemy (with a gun drawn) | |
| E (or L) | Action: arrest a downed enemy, climb a ladder; hold while running to sprint | Up |
| C / Q | | Down |
| Mouse | Orbit the camera (the cursor is captured) | Look |
| M | Mouse look on/off (off: right mouse drag turns the camera) | same |
| Mouse wheel | Zoom | |
| Ctrl | | Slow |
| Tab (hold) | The mission's checklist: objectives, side objectives, badges | |
| V | Switch to free camera | Back to Darci |
| 1 – 6 | Toggle floor / walls / props / roofs / details (cables, ladders) / people | same |
| H | Hide the controls help | same |
| R | Restart the mission | same |

**Gamepad** (any controller `gilrs` knows):

| Input | Action |
|---|---|
| Left stick or d-pad | Move: push partway to walk, fully to run |
| Right stick | Camera |
| A / Cross | Jump |
| X / Square | Punch |
| LB / L2 (hold) | Block |
| RB | Next weapon |
| Right stick click | Next target |
| Y / Triangle | Kick |
| B / Circle | Action (arrest, climb); hold while running to sprint |
| Start | Restart |
| Back / Select | Free camera |
| F12 | Screenshot to `screenshots/` | same |
| Esc | Quit | same |

**Menu and missions:** Up/Down and Enter choose a mission; Esc in a mission goes back to the menu (Esc on the menu quits). After a win, Enter goes on to the next mission.

**Progress:** each win is saved, with its time, to `%APPDATA%\urban-chaos-rs\progress.txt` (`~/.config/urban-chaos-rs/` elsewhere). The mission list puts a medal before each completed mission, its colour from the hardest difficulty it was won on (bronze Easy, silver Normal, gold Hard); the menu counts them, the panel shows each one's best time, and the win banner says whether you beat it.

**100%:** besides winning, a mission has badges to find and side objectives: the jobs that earn badges, such as arresting the extra muggers in RTA or finding officer Miles' car in The Jump. Both are read from the mission's script. The menu panel shows a checklist under the mission's title: its objectives (the "-" points of the briefing, or its last sentence for training missions), each side objective (named after what's said when it's done) and the badges found. In a mission, hold Tab for the same checklist, ticked as you go, with the mission clock. The HUD also shows `BADGES x/y   SIDE x/y`, and the win banner sums them up. Like badges, they're saved when you win, and add up across your winning runs. A mission with everything done has a ring round its medal in the list.

**Badges:** street missions hide badges about the city, mostly on rooftops. Walk into one to raise one of Darci's stats: stamina, strength, reflexes or constitution, one of the four for each badge (the original picks at random each time; here each badge always gives the same stat, so they can be tracked). Constitution adds 10 health and strength makes her blows land 5% harder per badge. Stamina and reflexes are counted but don't do anything yet. The HUD shows the stats under Darci's health, in gold for those raised in this mission. Badges are kept once the mission is won and carry over to later missions. Replaying a mission can't collect the same badge twice.

**RTA walkthrough:** follow the "Car crash" beacon and talk to the cop there. Drive the abandoned taxi to the "Vehicle Drop" at the station. Answer the homicide call: the witness in the Clunton Projects points you to Kleetus Avenue. Arrest a mugger (the first one is on a rooftop). Muggers attack while you stand about, and killing both of them, or a witness, fails the mission.

**Nitro Car walkthrough:** you play Mako, a gang thug, not Darci: the mission's own `create_player` point says so (`PT_GANG`, which the original turns into a rasta thug, not Roper, who has a model of his own). He has the rasta gang's voice, taunts included, and he can't arrest anyone: the original only looks for someone to cuff when the person doing it is Darci. He starts carrying a loaded pistol — press G for it — because the mission's `create_player` point leaves its "no gun" flag clear, which every training and street mission sets. He moves like her, though, and so does everyone else — `setup_global_anim_array` (fallen/Source/Anim.cpp) points every character at Darci's animations, with the per-type overrides for Roper and the cop commented out. Drive the car bomb at the "Car pick up point" to the bank courtyard; once the charge is primed the car locks, so get out and run — the blast is three squares wide and will kill you. A second car bomb turns up, locked, with a police bomb squad guarding the drop at the department store; take the squad out and it unlocks. Deliver it, clear out again, and then go back for the cop who jumps Cliffy and Zit. Wrecking either car bomb on the way fails the mission, and so does getting one of the boys killed. You can ask them along before the second run (walk up and press E twice): they run after you, sprint when you sprint, and take on whoever comes at you, as long as they can see them: nothing here finds a route round a corner, so they let a cop they can't reach be rather than walking into the wall between. They can look after themselves while you're across town.

**Gun Hunt walkthrough:** Deeks the snitch is somewhere in the park: follow the beacon and talk to him (E). He puts the murder weapon behind some skips in the middle of town, and a new beacon points at it. Walk over it to pick it up. Its owner hears it has gone and comes looking, armed: arrest him or put him down, then take the gun back to the station. Deeks is a snitch, not a collar, so killing him or arresting him fails the mission. Where he's standing and where the gun is are picked afresh each time you play (`move_thing` points on a `random` trigger), so the beacons are worth following.

**Rat Catch walkthrough:** Mako again. Talk to the Wildcat who meets you, then collect McLean and Bish, who fall in behind you, and go to Spiv for the job: McIntyre's son is to be delivered, not hurt. Walk up to him and he bolts; run him down and beat him to half health and he gives it up, then stand over him and he comes quietly and follows you to the drop. Killing him doesn't end the mission — the Boss takes it out of you instead, and the way back into his good books is five more dead cops. Losing the Wildcats who are with you fails it.

**Headline Hostage walkthrough:** Gordansky is wanted alive and at liberty: bring him to the drop and the mission is yours. Shooting him fails it, and so does arresting him, which the radio has something to say about.

**Seek, Sneak and Seize walkthrough:** get into the Research Center and lift the encryption disk, then make the front gates. Both halves are needed: the gates on their own won't do.

**The Jump walkthrough:** Barnaby is on a ledge of the tower east of the station (the "Brooks Tower" beacon). Walk straight up to him, by the ladder from the level where the cop waits, and he talks, then runs off the edge. Come from behind instead. Climb the tall building to the west (ladders from the street by the cop who says "It's up this way"), jump for the zip line on its roof, and ride it down onto Barnaby's ledge behind him.

**Mist:** some missions lay sheets of ground mist (`create_mist`), low over the street and shimmering where they lie. The original's own `fog.tga` isn't in the retail release, so the sheet is drawn with a cloud generated at startup, as the blob shadows are.

**Explosions:** a car bomb going off throws out a flash, a fireball, a ring of dust and burning debris, and leaves a fire behind. People within three squares are knocked down or killed, the player included. The `visual_effect` points a mission sets off elsewhere are the same effects without the blast: the original's shockwave for those never fires, thanks to a typo in the 1999 code (`if (ew->ed.subtype=!16)` assigns rather than compares), and this follows it.

**Explosives:** walk over a pack to carry it (four at most). Switch to them with G and press action: a charge goes down at your feet with a five-second fuse, and the blast is five squares across and does 500 at the middle, so run. Blasts damage props as well as people, and a mission can watch for a prop being damaged (`prim_damaged`): Transmission Terminated's transmitter, the supplies in Arms Breaker, Media Trouble's props. Some props blow up when damaged and set off their neighbours. As in the 1999 code, a blast only ever looks at every other prop in each part of the map (its loop steps two at a time), so a prop right next to one that went up can survive. Cars ramming props don't damage them yet.

**Endless mode:** ENDLESS in the main menu, then a map, or `--mission NAME --endless [--seed N]`. Waves of Wildcats keep coming at Darci, each bigger and better armed than the last:
- fists first, then bats and knives, a few pistols from wave 7, shotguns and AKs later, with only so many guns out at once;
- Men in Black from wave 15;
- a boss every tenth wave, marked on the map.

Each wave comes from a different side of the arena, marked with a beacon for its first seconds. Downed thugs often carry health or ammo, so search them. Arrests score more than knockouts, and innocents hurt cost points. The run ends when Darci goes down, or when the crime rate reaches 100: it only climbs while she keeps out of the fight (hiding or running), not while she's fighting, however slowly. The same seed gives the same waves, and each map keeps its best score.

**Beat Cop:** BEAT COP in the main menu, then a map, or `--mission NAME --beat [--shift MINUTES|endless] [--seed N]`. A shift of 5, 10 (the default), 15 or 20 minutes, or an open-ended one (Left/Right on the map list picks it), with no mission script: dispatch calls in incidents, each with a beacon and a time limit:
- a mugging;
- an armed suspect;
- a gun dumped in a bin;
- a gang fight;
- a wanted man who runs when he sees you.

Handling one scores, with arrests worth extra, and brings the crime rate down. Missing one puts it up. The shift ends when its time is up, or early if the crime rate reaches 100; an open-ended one only ends that way. The best score is kept per map and shift length.

**Difficulty:** DIFFICULTY in the main menu switches between Easy, Normal and Hard. It changes:
- enemy skill;
- how often enemies shoot;
- how hard Darci is hit;
- how fast her aim settles.

Normal is the original's tuning.

**Console and cheats (for play-testing):** `~` or F9 opens the developer console, as F9 did in the original. Type a command and press Enter; Esc closes it. The commands:
- `telw N`: to event point N;
- `tels` and `telr`: store Darci's position, and go back to it;
- `wpt`: the nearest event point;
- `win` and `lose`;
- `restart`;
- `roper` and `darci`: play on as the other, where the mission has Roper's body;
- `kit`: the full kit;
- `boo`: an explosion to look at;
- `pos`: the position readout;
- `help`.

Two more shortcuts:
- **Free-camera teleport:** in the free camera (V), T puts Darci where the camera is looking. Look at a building's wall and she lands on its roof.
- **Shift+F12** toggles the position readout.

The original's cheat works too: hold punch, kick, action, jump and right together (J, K, E, Space and D) for full health, 240 rounds for every gun, and an AK-47, a shotgun and a grenade at her feet.

**Searching:** some props hide things. Stand at one, face it and hold action (E): a bar fills over about three seconds, and whatever was stashed in it drops out, as the original's search does. Let go and the search is abandoned. A mission marks the props worth searching (`make_searchable`), and a `create_item` point can stash its item in the nearest one instead of leaving it in the open: in Auto-Destruct, the UCNN video turns up in the Wildcat den this way.

**Searching people:** the dead, the knocked out and the arrested keep their weapons, and whatever their event point says they carry (often a health pack, ammo or a gun), until searched. Stand over them and hold action: after the same bar their things are put on the ground round them, and there's nothing more on them after that.

**Insane Assault walkthrough:** go to Brick Road and put down the two gang members who turn up. Then the Balrog comes: a burning giant who stomps, swipes and throws fireballs, and goes for you above anyone else. Fists don't touch him. Only bullets and blasts do, and he takes an eighth of what anything else would: you start with a pistol, and ammo boxes appear about the place once he's out. Keep your distance, keep shooting, and when he goes down the mission is yours.

**Who's talking:** every message has the speaker's face beside it, from the game's own sheets of talking heads: dispatch gets the radio, and someone you've just spoken to says their lines themselves (and turns to you). A conversation is filmed as in the original: widescreen bars, one speaker's face top right and the other's bottom left, the line beside whoever is saying it, and the camera cutting to each speaker in turn, placed where it can see them. You're held still while it goes on; punch, kick, jump or action cuts a line short. A conversation the mission marks as overheard just shows its lines. Anything happening to either speaker breaks it off.

**Changing who you play:** some missions hand you over to someone else partway through (`transfer_player`), as Cop Killers does from Darci to Roper. The person you were stays behind, can't be hurt and minds their own business.

**Moving platforms:** a mission can set a prop moving (`link_platform`): lifts in Grim Gardens, the rockets of Target UC, the platforms of Day Of Reckoning. Each goes round the points the mission gives it, easing in as it arrives and waiting at each. Stand on one and you ride it; be under one as it comes down and you're crushed. Target UC's rockets trail fire as they go.

**Warehouses:** walk into one (the police station is one, and RTA starts you in it) and the city's ambience and wind stop. If the mission has given it an ambience (`warefx`), that plays instead of the music: the station's radio chatter, a Posheeta's or an office's hum, or the club's music in Gatecrasher. The ambience is the one the warehouse had as you came in, as in the original. On its roof you're outside.

**City life:** besides the people a mission makes, the streets have the crowd and traffic it asks for (`fake_civs`, `fake_cars`: RTA has fifteen civilians and seven cars). Civilians stroll the pavements, any square within two of a road, and the zebra crossings (`wand.cpp`). Cars drive the roads in their lanes, picking a way at random at each junction, and stop for anyone in front of them. Neither is ever far from you: once one has been out of sight for a couple of seconds, it's moved to somewhere just out of sight ahead of you (`PCOM_do_regen`), on a pavement or in a lane. People out of sight are those beyond the fog (or the original's 22 squares without it). The crowd runs from gunfire within ten squares, from a gun drawn within six where they can see it, and from being hit, and they don't forget it. In the original the mission's own civilians run too; here they don't yet, since without route-finding someone running can end up stuck behind a wall, and a witness doing that could stall the mission. Walk up to one of the crowd as Darci and press action and they'll say something, a line from the mission's own chatter file (`Text\police11.txt` in RTA, else `citsez.txt`), or from `annoyed.txt` once you've given them a fright. Explosions don't frighten them: in the original only a barrel going up does. Unlike the original, nobody is put down within three squares of another of the crowd: the original takes the first pavement it finds near a spot ahead of you, so while you stand still everyone moved ends up at the same few corners.

**Creatures:** some missions have monsters (`create_creature`), run as the original runs them (`bat.cpp`):
- the **Balrog** wanders about on fire, roars when he picks a target, then follows, charges, swipes, stomps and throws fireballs that burst where they land;
- **gargoyles** sit still until you come within ten squares, then take off, circle their home, swoop on you and spit fireballs;
- **Bane** hovers, draws the lifeless bodies round him up into the air and hits you with lightning and blasts. Nothing hurts him.

Guns can aim at the Balrog and gargoyles, and a bar under the clock shows the health of whichever one is nearest. The original's bats are compiled out of the shipped game, so the missions that ask only for them (Psycho Park, Stern Revenge) get nothing, as they did in 1999. The original finds the Balrog his way about with a navigation map this engine doesn't have yet: here, when he can't see who he's after, he heads for whichever spot two squares off brings him closest.

**Steam vents:** some missions set traps (`create_trap`): vents that puff steam forwards, up or down to a routine of their own, on for a beat and off for a few. The steam scalds anyone standing in it, every puff in reach on its own account, so a full column will kill you in a second or two: time your way past. As in the original, a trap that's meant to stay on for good never puffs at all (the point is marked finished the moment it fires, and finished points aren't run), so it's the ones that come and go that you'll see. Assassin, Rat Catch, Grim Gardens, Estate of Emergency, Target UC and Day Of Reckoning have them.

**Peeing:** a mission can set someone relieving themselves against a wall (`make_person_pee`), a stream of drops from the pelvis as `DIRT_new_water` throws them, falling until they reach the ground. Semtex and Transmission Terminated have one. RTA and Arms Breaker ask for it too, but their point comes before the one that makes the person, so it finds nobody, and in the original nobody is ever seen at it either.

**Doors:** a door is shut and solid until a mission opens it, which is the only way one ever opens — the original has no walking into them and no "use" on them either, and `DOOR_open`'s only caller is the mission script.

**Falls:** landing hard hurts, and a fall of more than about five storeys kills, Darci included (as in the original).

**Driving:** E next to a car gets in (and out, once it has nearly stopped). Some are locked, as the mission sets them: the squad cars parked at the station never open, and a car locked with a coloured key opens once you've picked up a key (Media Trouble has one). Trying a locked car says so. W/S or the triggers accelerate and brake/reverse, A/D or the stick steer, Space or A is the handbrake: it locks the back wheels, so the car slides round a corner (with a skid) instead of stopping. At speed, full lock and the handbrake spin the car right round (a 180 in a little over a second), ready to drive back the way it came. Nobody can hit or shoot you while you're in the car. The camera swings round behind the car; the HUD shows speed and damage. Hit walls hard enough and the car wrecks. People get out of the way of a car coming at them, as in the original, but if you catch them they go flying. Cones and barrels get knocked about.

**Fighting:**
- Hold block to stop blows from the front. Knives and bats still hurt a little, and attacks from behind get through. Skilled enemies block too.
- Finishing moves knock people down; they get up again unless they're out of health. Arrest them while they're down. As in the original, only the living can be cuffed, and never a Man In Black, a cop who isn't guilty, someone the mission has made invulnerable or one of your own bodyguards; with several in reach, Roper and hostages come last.
- Walk over a knife or baseball bat to pick it up (enemies drop theirs when knocked out); punch then swings it. Walk over a health pack to heal.
- A bullet landing makes no sound of its own, as in the original: the shot and the victim's voice carry it.
- **Guns:** walk over a pistol, shotgun or AK-47 to pick it up (it comes loaded, and is drawn if your hands are empty), or over an ammo box for another clip. A mission can also start you carrying a pistol (its `create_player` point says so, as Nitro Car's does): that one is stowed, not drawn. The HUD lists what's in hand brightly and what else is on you dimmed beside it, as the original's weapon panel did. Armed enemies drop their guns when knocked out or arrested. G (or RB) switches weapon.
- With a gun drawn, punch fires at the target in the red brackets. Holding punch keeps the AK-47 firing. Targeting picks the nearest enemy you can see, favouring those in front, and sticks with it while it can be shot. T (or a right-stick click) moves on to the next one. Damage follows the original: pistol 70, AK-47 100, and the shotgun up to 250 close up, falling off with distance (enough to knock someone down). Enemies' shots do half damage to Darci. An empty gun clicks.
- Enemies in a fight, or hurt, show a health bar over their heads.
- Enemies with guns shoot from range when they can see you, so close in behind cover. Blocking doesn't stop bullets.
- Press punch or kick three times in time for a combo. The next press has to land in the second half of the move.
- Kick while running: sliding tackle. Kick while sprinting slides further.
- In a fight, hold back, left or right while kicking for a back or side kick.
- Punch while walking into someone: grab them, then punch (knee) or kick (throw).
- Walk into a low ledge (up to about half a square) to climb onto it. Higher ones need a jump.

**Combat tutorial walkthrough:** the trainers only go down to the move each lesson asks for, as in the original. Fight-test dummies are invulnerable to everything else.
1. Punch twice and kick twice. The first target then appears; follow the yellow beacon.
2. Slide-tackle it.
3. Use triple punches on the next two targets, then a triple kick.
4. Knock down the next target any way you like, then arrest it with E.
5. Climb the ladder onto the central building. Use side and back kicks on three of the four trainers, then grab the last one and finish him.

**Sound**, all from the game's own recordings:
- **Effects:** punches, swings, footsteps, landings, grunts and knockouts, and whatever a mission asks for by id (`sound_effect` points, through the game's own `sound_list` table). They are placed in the world, so they fade with distance and pan with the camera.
- **Darci's voice:** she calls out arrests and sometimes taunts after a knockout.
- **Enemy shouts:** thugs shout when they spot her.
- **Music:** it follows the action. Fight and sprint themes come from the mission's music world (`levels\mworlds.txt`, as in the original), the mission-complete or failure cue plays at the end, and city ambience plays underneath.
- **Mission messages and conversations:** they play their recordings (`talk2\<mission>.ucm<n>.wav`, where `n` is the message's own speech number, as in the original). They are said one at a time: skipping a fly-by on brings the next line up before the one before it has finished, so a line that arrives early waits its turn rather than talking over it. The combat tutorial has none; the other training missions and RTA have nearly all of theirs.

The HUD shows Darci's health, the weapon in hand and its rounds, enemy health bars, the target and the mission's messages. A minimap in the top right shows the city from above, north up, made once per level by rendering it from overhead (ground floors show through warehouse roofs). Darci is the white dot, the mission's beacons are gold (pinned to the edge when further away) and enemies in a fight are red. The beacons are listed underneath with their distances. It zooms out while driving. The window title shows FPS and how many chunks and draw calls the frame used.

**Options:**
- `--mission NAME`: go straight into a mission instead of the menu; `--explore` skips its script.
- `--mute`: no sound or music.
- `--weapon knife|bat|pistol|shotgun|ak`: Darci starts holding it.
- `--cam X,Y,Z,YAW,PITCH`: start in the free camera (map squares; degrees).
- `--fog START,END` or `--no-fog`: distance fog in map squares (default 20,40).
- `--fullbright`: no baked lighting.
- `--linear`: smooth texture magnification instead of crisp pixels.
- `--no-character`: leave people out.
- `--screenshot FILE`: render one frame **without opening a window** (the menu, unless `--mission` is given). Combine with `--size WxH` and `--sim SECS` (play for a while, skipping fly-bys, punching and kicking first, then look at the nearest enemy) or `--sim-walk SECS` / `--sim-run SECS` (testing aids).

## Missions as JSON

```
cargo run --release -p uc-tools -- mission export "C:\...\Urban Chaos\levels\police1.ucm" -o police1.json
cargo run --release -p uc-tools -- mission import police1.json -o police1.ucm
cargo run --release -p uc-tools -- mission check police1.json
```

- Event points get readable names (`"type": "create_player"`, `"trigger": "radius"`), with a `_summary` for common types.
- Message text and cutscenes sit on the event point that uses them.
- The default **clean** export keeps only what the game reads. `--lossless` also keeps editor leftovers, so importing reproduces the original file byte for byte.

See `docs/MISSIONS.md` for what the fields mean.

## Upscaled textures

1. Export the textures a level uses: `cargo run --release -p uc-viewer -- --mission police1 --dump-pages pages`. This writes `pages/page_NNNN.png`.
2. Upscale them with any tool, keeping the file names. Wrap tiling textures around their edges, treat alpha separately, and check text by hand (`docs/RUST_ROADMAP.md` §4).
3. Run with `--overrides pages_upscaled`. Any size works; textures get mipmaps automatically.

Keep extracted and upscaled textures out of the repository; they are the game's copyrighted art.

## Tests and CI

```
cargo test --release --workspace
```

With an install present, the tests check:
- every map, shipped mission, prim, lighting file and style table parses exactly;
- all 80 parseable missions round-trip byte for byte, directly and through JSON in both modes;
- `police1.txc` decodes completely;
- 98.9% of the police map's wall segments find their texture;
- Darci's animations are complete;
- Police 1 builds every geometry layer, and every person type can be posed;
- a bot plays the combat tutorial to the end;
- Nitro Car runs from the first car bomb to the win, with its locked cars, bodyguards and both ways of failing;
- Gun Hunt runs from Deeks to the station, and losing Deeks fails it;
- Rat Catch runs to both of its endings: the son delivered, and the Boss's punishment after he's killed;
- Headline Hostage runs to its win, and to both ways of failing it;
- Seek, Sneak and Seize runs to its win, and neither half of it counts alone;
- Insane Assault runs to its win by shooting the Balrog, and he takes damage at an eighth; Day Of Reckoning's gargoyle wakes and takes off, and its Bane draws up the bodies and can't be hurt; bats make nothing;
- Grim Gardens' lift rises with Darci standing on it;
- Cop Killers hands the player to Roper, and Darci is left unhurtable; RTA's mugger's shout is filmed with both faces, holds Darci still and can be skipped through; dispatch's messages carry the radio's face;
- every `warefx` point is inside a warehouse and gives it its ambience, and Darci is indoors under a warehouse's roof but not on it;
- steam vents keep to their routines and scald, and a trap that stays on for good never blows;
- explosives blow up Transmission Terminated's transmitter and Media Trouble's props, which their missions see;
- searching a prop turns up what Auto-Destruct stashes in it, and letting go of action, facing away or an empty prop finds nothing;
- someone knocked out keeps their gun until searched, and a search turns it up; enemies carry their point's drop;
- The Fallen gets three of its five clues (see `mib.rs` for what the other two still want).

Without an install, the data tests print a note and pass. That's what CI sees: `.github/workflows/rust.yml` builds, tests and runs Clippy on Windows and Linux.

See [CONTRIBUTING.md](CONTRIBUTING.md) for what to run before a pull request, and for the helpers the mission tests share.

## What is and isn't there

**Drawn:**
- terrain heights, floor textures with rotation, kerbs;
- walls, doors and fences with styles, painted storeys and the engine's per-wall random variants;
- rooftops and roof faces (warehouse ceilings black as on PC);
- props;
- overhead cables (with their sag) and ladders;
- baked lighting (ambient, static lights, lamppost lights);
- sky colour and distance fog;
- Darci, with idle, walk and run animations, ground following and wall collision, and whoever else a mission makes the player: Mako out of the thugs' file, Roper out of his own;
- explosions and the fires they leave.

**Not yet:**
- event points beyond the set these missions use (bombs, steam, platforms, mist...). The viewer prints which points of a mission are ignored; `cargo run -p uc-game --example unsupported` lists them for every mission at once;
- ambient city life: the original's random civilians and traffic (`fake_civs`, `fake_cars`). Scripted civilians do stroll about;
- the gusts that push ground mist aside as someone walks through it (`MIST_gust`);
- hanging from ledges and traversing them;
- stars, the skyline and moon;
- animated prims (`LoadGameThing`).

The shipped maps contain no fire-escape or interior-staircase facets, so those aren't needed. Warehouse roofs look black from outside: on PC the original takes their texture from a table only filled on PlayStation. Two-sided walls always use their front texture.

See [docs/RUST_ROADMAP.md](docs/RUST_ROADMAP.md) for the plan and progress, and [docs/MISSIONS.md](docs/MISSIONS.md) for how missions work and how to edit them.

## Licence

RustChaos is free software under the GNU General Public License, version 3 or (at your option) any later version: see [LICENSE](LICENSE). It is derived from the Urban Chaos source code released under the MIT licence; that licence and its copyright notice are kept in [NOTICE.md](NOTICE.md). Urban Chaos and its data remain the property of their owners and are not covered by either licence.
