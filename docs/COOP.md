# Co-op multiplayer: a plan for a server

Two to four players play the story missions together, joined through a server. This
plan builds on the multiplayer ticket (#114) and on what's already in place:
- fixed game turns (#128);
- `Game::state_hash` and the test that two runs agree on it (#115);
- the second player in Capture the Flag (#133).

Nothing here is built yet.

## The choice: lockstep, with a server that also runs the game

**How it works:**
- Every machine runs the same deterministic game.
- Only the players' inputs cross the network.
- The server collects each turn's inputs and sends the bundle to everyone.
- The server also runs the game itself, headless. That makes it the referee: it checks
  everyone's state hashes and keeps snapshots for players who join late or fall out of step.

**Why not a server that sends state (snapshots, as most action games do):**

| | Lockstep (inputs only) | Server state (snapshots) |
|---|---|---|
| **Bandwidth** | A few bytes per player per turn. | Everything that moves: hundreds of people, the cars, barrels, the script's state. Many KB a second for every client. |
| **Work in `uc-game`** | Make it deterministic across machines (most of it is done), and give it several players. | Separate what's predicted from what's corrected, interpolate other people, and sync the script, cutscenes and messages. That's a second game architecture. |
| **Latency** | Each input is delayed by a few turns (about 100 ms) for everyone. That's fine for co-op against the AI. | Your own moves respond at once; others are corrected afterwards. |
| **Weak spots** | Everyone needs the same build. A desync has to be detected and repaired. | Bandwidth and complexity. |

The original game was built lockstep (`cnet.cpp`, see #114). The game is now a fixed-turn
simulation with a state hash, so lockstep is the shorter road. The headless server covers
lockstep's two weak spots: joining mid-mission, and recovering from a desync.

## Architecture

```
 client (uc-viewer)                 server (uc-server)                client (uc-viewer)
 ┌──────────────────┐   input t+D   ┌──────────────────────┐  input t+D ┌──────────────────┐
 │ local Controls ──┼──────────────►│ seat inputs per turn │◄───────────┼── local Controls │
 │                  │◄──────────────┼─ turn bundle t       ├───────────►│                  │
 │ Game::step(t)    │   bundle t    │ Game::step(t)        │  bundle t  │ Game::step(t)    │
 │ render, sound    │── hash t ────►│ compare hashes       │◄── hash t ─┤ render, sound    │
 └──────────────────┘               │ snapshots, lobby     │            └──────────────────┘
                                    └──────────────────────┘
```

### New crates
- **`uc-net`:**
  - the message types;
  - their encoding, small and versioned (`postcard` or hand-rolled);
  - the session logic shared by client and server (turn buffers, input delay, hash checks).
  - It has no rendering and no sockets of its own: it can be tested with in-memory pipes.
- **`uc-server`:** a headless binary.
  - It loads the install (it needs the level data to run the game), hosts a lobby, and runs
    sessions.
  - It runs on Windows and Linux, the same platforms CI tests.

### `uc-viewer`
- **New menu entries:** CO-OP, then HOST (start a server in the same process and play on it)
  or JOIN (by address).
- **The main loop:** a session hands the game its turns, instead of `Game::update` counting
  out turns from the frame time.

### Transport
- **QUIC through `quinn`:**
  - reliable, ordered streams for turn bundles and lobby messages;
  - unreliable datagrams for the redundant input resends;
  - encryption and connection IDs for free.
- **A first cut can be plain TCP.** Lockstep needs every input anyway, so TCP's
  head-of-line blocking costs little at two to four players.
- **Connecting:** host and join by address. There's no master server or NAT punch-through
  at first: the host forwards a port, or players use a VPN such as Tailscale.

## Changes to `uc-game`

### 1. Several players
Today `Game::player: Option<usize>` (122 uses of `self.player` in 28 files) means "the"
player. It becomes:

- **`players: Vec<Seat>`.** A seat has:
  - the person it controls;
  - its own targeting (`update_target`, `aim_at_people` are per player now);
  - its pending presses (`pending`);
  - its stats and badges;
  - its camera hints.
- **`Game::step(turn_inputs: &[Controls])`:** one turn with one input per seat. `update(dt,
  input)` stays as the single-player wrapper.
- **Going through every use of "the player":**
  - **Who's hurt and how much** (difficulty, god mode): per seat.
  - **The AI's "go for the player":** the nearest player who can be seen, as the original's
    `NET_PERSON` loops did (`PCOM` mostly looks for the nearest player).
  - **The script's player triggers** (in `mission.rs`):
    - "player in radius" and "player uses": **any** player, as the original did for player 1
      or player 2;
    - "player dead": the mission is lost when **all** players are down for good (see revive,
      below);
    - `transfer_player` (Cop Killers, Day Of Reckoning): moves the seat it was written for,
      seat 1. The other seats keep their characters.
  - **Cutscenes and messages:** shared. Everyone sees the same cutscene, and any player can
    skip it once everyone has pressed, or the host skips it.
  - **Beacons, the HUD and the crime rate:** shared. Health, stamina and weapons are per seat.
  - **Cheats:** off in co-op, or host-only.
- **The second player's character:**
  - Missions have one `create_player`, so seat 2 and up spawn next to seat 1.
  - **The character:** Roper in the missions where he's a playable person, otherwise a police
    partner made from a cop variant (the `CIV`/`COP` person types). A mission file can name a
    second `create_player` later, as `Capflag1` does.
- **Down, not out:**
  - A player knocked to zero health is down for a while.
  - A partner holding action next to them revives them, with half health.
  - Bleeding out (30 seconds) is out for the rest of the mission, and that player watches
    their partner's camera.
  - The mission is lost only when everyone is out. That keeps one player's slip from ending
    a long mission.

### 2. Determinism across machines
- **Already there:**
  - Fixed turns of 1/30 s.
  - A single seeded set of dice (`rngs`).
  - The `HashMap`s are only looked up by key, never iterated for logic. The 16 uses in
    `lib.rs`, `cars.rs`, `litter.rs`, `people.rs`, `trip.rs` and `wand.rs` must be audited
    once and then kept that way. A clippy lint (`disallowed_methods` on `HashMap::iter`,
    `values`, `keys` in `uc-game`) enforces it.
- **Floats:**
  - **Same build, same CPU family:** `+ - * / sqrt` are exact IEEE on x86-64 and AArch64, and
    Rust doesn't fuse multiplies into FMAs on its own.
  - **The real risk is `sin`, `cos`, `atan2`, `powf`, `exp`.** These go to the platform's C
    library, and MSVC and glibc can disagree in the last bit. That matters as soon as the
    server runs on Linux and the clients on Windows. Route them through the `libm` crate (pure
    Rust, the same everywhere) with a small `uc_game::math` module, and lint against the
    `f32` methods in `uc-game`. There are about 70 calls today.
  - **`glam`:** uses SSE2 on x86-64 and NEON on ARM, and its dot products may add up in a
    different order. Either support x86-64 only at first, or turn on `glam`'s `scalar-math`
    feature for `uc-game` and measure the cost.
- **The frame stays out of the game:** nothing the game decides may depend on the frame
  time, the camera or the window. The camera and sound only read the game. That's mostly so
  already. The sniper scope and the camera-relative aim must turn into inputs (`aim`,
  `target` in `Controls`), and they already are.
- **Inputs on the wire are quantized:** `wish` becomes an `i8` pair, `aim` becomes a
  fixed-point position. Every machine then plays the quantized value, the sender included.
- **Tests:**
  - **Replays:** record the inputs of a mission run (the mission bots in `tests/` produce
    them), play them back, and compare `state_hash` every turn.
  - **Cross-platform:** CI plays the same recordings on Windows and Linux and compares the
    hash sequences. This is the test that keeps co-op working.

### 3. Snapshots
- **Uses:** a player joining mid-mission, and repairing a desync.
- **Format:** the whole `Game` serialized (`serde` on `Game` and what it holds) and compressed.
- **The level isn't in it:** that's loaded from the install, and checked by a hash of the
  mission and map files at join.
- **Size:** probably 100–500 KB compressed; sent once, over the reliable stream.

## The session

### Lobby
1. **The client says hello:** its protocol version, its build ID (the git hash) and its
   player name. A different build is turned away with a clear message.
2. **The server sends the lobby:** the seats, the host's choice of mission and difficulty, and
   who's ready.
3. **Content check:** each client sends a hash of the chosen mission's `.ucm`, `.iam` and
   lighting files. Mismatches are refused (a modded or different-language install).
4. **The start:** the server sends the mission, the seed, the difficulty, the seat assignment
   and the input delay. Everyone loads, says "loaded", and turn 0 begins once all have.

### Every turn
- **The client:** at local turn `t`, it samples its `Controls` and sends them for turn
  `t + D`, where `D` is the input delay, 3 turns (100 ms) to start with. Each packet also
  carries the last few inputs again, so one lost datagram costs nothing.
- **The server:**
  - waits for every seat's input for turn `t`;
  - after a timeout (`D` turns' worth), uses that seat's last input with the presses cleared,
    and marks the seat as lagging;
  - broadcasts the bundle, then steps its own game.
- **The clients:** step turn `t` when its bundle is in. A client without the bundle holds
  the frame, and the HUD shows "waiting for <name>".
- **Adapting the delay:** the server raises `D` when round trips grow, and lowers it slowly
  when they settle. The change is announced for a future turn so everyone switches on the
  same turn.
- **Rendering:** draws between turns as now (`Game::blend`). Sound plays from the stepped
  turns.

### Hash checks and desync
- Every 30 turns (once a second), each client sends `(turn, state_hash)`.
- The server compares them with its own.
- **A client that differs has desynced:**
  1. it is paused;
  2. it is sent a snapshot of the server's game at a turn it has bundles from;
  3. it catches up by stepping the bundles since then, faster than real time.
- The other players play on. Each desync is logged with the turn and the first differing
  part of the state (a debug `state_dump` to compare).

### Joining, leaving, pausing
- **Join mid-mission:** a free seat gets a snapshot, catches up, and spawns next to a living
  player.
- **Leave or drop:** after a timeout the seat's person is taken off (or handed to the AI as a
  partner who follows, later), and the session goes on.
- **Pause:** any player can ask for it, and the server pauses on an agreed turn. In the menu
  the game keeps running with no input from that seat, as in other co-op games. A full pause
  is the host's to call.
- **The host leaves:** with the dedicated server the session lives on. With HOST in the
  viewer it ends, and saving the session for a later resume is a later step.

### Progress
- Each client saves its own progress: missions won, badges, stats, as now.
- A co-op win counts for everyone who was in it at the end.

## Missions
- **First: the West District and Gangland missions.** They already play through with bots in
  the tests, and their scripts lean on the player triggers least. Then the rest of the
  campaign, mission by mission, with a co-op test bot for each (two seats, one input script).
- **Mako's missions (Nitro Car, Rat Catch):** seat 2 is a Wildcat partner. Story-wise they're
  odd in co-op, so they're allowed but not required.
- **The flagship missions:** Cop Killers and Day Of Reckoning were written with both Darci
  and Roper. Here seat 2 is Roper from the start, and `transfer_player` only moves seat 1.
- **Beat Cop and Endless:** co-op as well, at almost no cost once seats exist. The incident
  rate and the waves scale with the number of players.
- **Capture the Flag (#133):** becomes player against player with the same machinery. That's
  the original's own mission for it.

## Order of work

Each step lands on its own and keeps single player working.

1. **Determinism hardening:**
   - the `libm` math module;
   - the `HashMap` lint;
   - input recording and replay;
   - the cross-platform hash test in CI.
2. **Seats in `Game`:**
   - `players: Vec<Seat>` and `Game::step(&[Controls])`;
   - the "the player" audit;
   - revive;
   - script triggers for any player.
3. **Split-screen co-op on one machine:** two pads, two viewports (`uc-render` draws the scene
   twice into halves). This tests everything in step 2 with no network at all, and it's
   playable on its own.
4. **`uc-net` with in-memory transport:**
   - the turn buffers, input delay, hash checks and catch-up;
   - tested with two clients and a server in one test, inputs from recordings, and simulated
     lag and loss.
5. **Snapshots:** `serde` on the game state, and save and restore tested against
   `state_hash`.
6. **`uc-server` and the QUIC transport:** the lobby, the content check, join and leave.
7. **The viewer's co-op menu:** HOST and JOIN, the lobby screen, and the "waiting for" and
   desync notices.
8. **Missions:** co-op test bots per mission, and play-testing on two machines.
9. **Later:**
   - a server list or relay;
   - AI partners for empty seats;
   - voice (the original had none);
   - player against player in CTF and deathmatch on `multi1.iam`.

## Risks
- **Float determinism across Windows and Linux.** Step 1 exists to find this out early.
  The fallback is the same OS for the server and the clients.
- **Hidden single-player assumptions** in the script runner and the cutscenes. The co-op
  test bots per mission are the safety net.
- **Performance:** the server runs the full game headless. It does already in the tests, at
  many times real time, so one server can run several sessions.
- **Cheating:** lockstep shows every client the whole state, and inputs can be forged. For
  co-op among friends that's acceptable. The server's own simulation is the referee, so a
  modified client only desyncs itself.
