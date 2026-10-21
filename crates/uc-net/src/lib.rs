//! Co-op through a server (#182, `docs/COOP.md`): lockstep over the game's fixed turns.
//!
//! ```text
//!  client ── Input(turn t + D) ──►  server  ◄── Input(turn t + D) ── client
//!  client ◄──── Turn(t: every seat's input) ──── server ────────────► client
//!  client ── Hash(t, state_hash) ──► server compares ◄── Hash ─────── client
//! ```
//!
//! Every machine runs the same game, and only the players' inputs cross the network, as the
//! original's `cnet.cpp` did. Each input is for a turn `D` turns ahead (the input delay), so
//! it has time to reach everyone before that turn is played. The server collects each turn's
//! inputs from every seat and sends them out together; a client plays a turn once it has
//! them. Once a second each client sends its [`uc_game::Game::state_hash`], and the server
//! tells everyone if they've parted.
//!
//! [`Server`] is the session's logic with no sockets of its own; [`server::serve`] runs it
//! over TCP, and [`Client`] is a player's end.

pub mod client;
pub mod server;
mod wire;

pub use client::Client;
pub use server::Server;
pub use wire::{read_msg, write_msg};

use uc_game::{Controls, Difficulty, Drive, Game};
use uc_world::Level;

/// Bumped whenever a message changes.
pub const PROTOCOL: u16 = 1;

/// The build: its git commit. Players on different builds are turned away at hello, since
/// their games may not play out the same.
pub const BUILD: &str = env!("UC_BUILD_ID");

/// The port a server listens on unless told otherwise.
pub const PORT: u16 = 7777;

/// Seats in a session: Darci and Roper.
pub const SEATS: usize = 2;

/// The fewest turns between a player's input and the turn it's played on: 100 ms at 30 turns
/// a second. A game's own delay ([`Msg::Start`]) is longer when a player's round trip to the
/// server is: long enough for an input to get there and the turn back, with some to spare.
pub const DELAY: u32 = 3;

/// The most: a third of a second. Past that the game slows down rather than stalls.
pub const MOST_DELAY: u32 = 10;

/// The input delay for players whose slowest round trip to the server is `rtt_ms`: that and a
/// margin for the odd slow packet, in turns.
pub fn delay_for(rtt_ms: u64) -> u32 {
    let turns = ((rtt_ms + 40) as f32 / (1000.0 / 30.0)).ceil() as u32;
    turns.clamp(DELAY, MOST_DELAY)
}

/// Turns between state hashes: once a second.
pub const HASH_EVERY: u32 = 30;

/// A message between a client and the server.
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    /// Client: who it is, and what it's built from.
    Hello { protocol: u16, build: String, name: String },
    /// Client, seat 0 only: the mission to play, how hard, and seat 0's badge stats. Again
    /// later: start over, or play another.
    Choose(Setup),
    /// Client: its input for a turn of game `game` (the number [`Msg::Start`] gave it).
    Input { game: u32, turn: u32, input: Input },
    /// Client: its game's state hash after a turn.
    Hash { game: u32, turn: u32, hash: u64 },
    /// Client: the answer to [`Msg::Ping`], straight away.
    Pong { id: u32 },
    /// Server: in, at this seat.
    Welcome { seat: u8 },
    /// Server: not let in, and why.
    Refused { why: String },
    /// Server: who's in, by seat (empty for a free seat).
    Lobby { names: Vec<String> },
    /// Server: start the mission, as game number `game` (anything still on its way from the
    /// game before is for that one), each input going for the turn `delay` turns on. Turn 0
    /// follows.
    Start { game: u32, delay: u8, setup: Setup },
    /// Server: every seat's input for a turn, seat 0 first.
    Turn { turn: u32, inputs: Vec<Input> },
    /// Server: something to show the players (someone left, the games have parted).
    Notice { text: String },
    /// Server: to time the round trip by.
    Ping { id: u32 },
}

/// What a session plays.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    pub mission: String,
    /// [`uc_game::Difficulty`], as its place in `Difficulty::ALL`.
    pub difficulty: u8,
    /// Seat 0's badge stats ([`uc_game::Game::set_stats`]).
    pub stats: [u8; 4],
}

impl Setup {
    /// The game every machine starts: the mission's, with seat 0's stats, the difficulty,
    /// and Roper next to her. Everything that changes the game before turn 0 is here, so that
    /// every machine does the same.
    pub fn game(&self, level: &Level) -> Game {
        let mut g = Game::new(level, true);
        g.set_stats(self.stats);
        g.set_difficulty(self.difficulty());
        g.add_partner(level);
        g
    }

    pub fn difficulty(&self) -> Difficulty {
        Difficulty::ALL.get(self.difficulty as usize).copied().unwrap_or(Difficulty::Normal)
    }

    pub fn new(mission: &str, difficulty: Difficulty, stats: [u8; 4]) -> Setup {
        let d = Difficulty::ALL.iter().position(|&x| x == difficulty).unwrap_or(1);
        Setup { mission: mission.to_string(), difficulty: d as u8, stats }
    }
}

/// Plays one turn of a co-op game with every seat's input: a cutscene any of them skips is
/// skipped first.
pub fn play_turn(g: &mut Game, level: &Level, inputs: &[Input]) {
    if inputs.iter().any(Input::skip) && g.cutscene.is_some() {
        g.skip_cutscene();
    }
    let controls: Vec<Controls> = inputs.iter().map(Input::controls).collect();
    g.step(&controls, level);
}

/// Testing: a made-up player's input at second `t` of the game, walking about, running,
/// fighting, crouching and firing, a different pattern for each seat.
pub fn bot_input(seat: usize, t: usize) -> Controls {
    let mut c = Controls::default();
    match (t + 3 * seat) % 8 {
        0 | 1 => c.wish = glam::vec2(0.0, 1.0),
        2 => {
            c.wish = glam::vec2(1.0, 0.5);
            c.run = true;
        }
        3 => c.punch = true,
        4 => c.kick = true,
        5 => c.crouch = true,
        6 => {
            c.wish = glam::vec2(-0.7, -0.7);
            c.jump = true;
        }
        _ => c.fire = true,
    }
    c
}

/// A player's input for a turn, as it goes over the wire: [`Controls`] without what the
/// game works out for itself (the target and the aim), plus skipping a cutscene. The floats
/// go as they are, bit for bit, so every machine plays exactly what the player sent.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Input {
    pub wish: [f32; 2],
    pub drive: [f32; 2],
    pub buttons: u16,
}

/// [`Input::buttons`].
pub mod button {
    pub const RUN: u16 = 1;
    pub const SPRINT: u16 = 1 << 1;
    pub const PUNCH: u16 = 1 << 2;
    pub const KICK: u16 = 1 << 3;
    pub const ACTION: u16 = 1 << 4;
    pub const JUMP: u16 = 1 << 5;
    pub const BLOCK: u16 = 1 << 6;
    pub const FIRE: u16 = 1 << 7;
    pub const SWITCH: u16 = 1 << 8;
    pub const CYCLE_TARGET: u16 = 1 << 9;
    pub const SCOPE: u16 = 1 << 10;
    pub const TAUNT: u16 = 1 << 11;
    pub const CROUCH: u16 = 1 << 12;
    pub const HANDBRAKE: u16 = 1 << 13;
    /// Skip the cutscene.
    pub const SKIP: u16 = 1 << 14;
    /// Those that are pressed rather than held: kept until a turn takes them.
    pub const PRESSES: u16 = PUNCH | KICK | ACTION | JUMP | SWITCH | CYCLE_TARGET | SCOPE | TAUNT | SKIP;
}

impl Input {
    pub fn new(c: &Controls, skip: bool) -> Input {
        use button::*;
        let bits = [
            (c.run, RUN), (c.sprint, SPRINT), (c.punch, PUNCH), (c.kick, KICK), (c.action, ACTION),
            (c.jump, JUMP), (c.block, BLOCK), (c.fire, FIRE), (c.switch, SWITCH),
            (c.cycle_target, CYCLE_TARGET), (c.scope, SCOPE), (c.taunt, TAUNT), (c.crouch, CROUCH),
            (c.drive.handbrake, HANDBRAKE), (skip, SKIP),
        ];
        let buttons = bits.iter().filter(|b| b.0).fold(0, |a, b| a | b.1);
        Input { wish: c.wish.to_array(), drive: [c.drive.throttle, c.drive.steer], buttons }
    }

    pub fn controls(&self) -> Controls {
        use button::*;
        let on = |b: u16| self.buttons & b != 0;
        Controls {
            wish: glam::Vec2::from_array(self.wish),
            run: on(RUN),
            sprint: on(SPRINT),
            punch: on(PUNCH),
            kick: on(KICK),
            action: on(ACTION),
            jump: on(JUMP),
            block: on(BLOCK),
            fire: on(FIRE),
            switch: on(SWITCH),
            cycle_target: on(CYCLE_TARGET),
            scope: on(SCOPE),
            taunt: on(TAUNT),
            crouch: on(CROUCH),
            drive: Drive { throttle: self.drive[0], steer: self.drive[1], handbrake: on(HANDBRAKE) },
            target: None,
            aim: None,
        }
    }

    pub fn skip(&self) -> bool {
        self.buttons & button::SKIP != 0
    }

    /// A later frame's input on top of this one: what's held as it is now, and what was
    /// pressed in either.
    pub fn then(&self, later: &Input) -> Input {
        Input { buttons: later.buttons | (self.buttons & button::PRESSES), ..*later }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_round_trip() {
        let c = Controls {
            wish: glam::vec2(0.25, -0.75), run: true, punch: true, crouch: true, scope: true,
            drive: Drive { throttle: 1.0, steer: -0.5, handbrake: true }, ..Default::default()
        };
        let i = Input::new(&c, true);
        let back = i.controls();
        assert_eq!(format!("{back:?}"), format!("{c:?}"));
        assert!(i.skip());
    }

    #[test]
    fn presses_are_kept_and_holds_are_the_latest() {
        let a = Input::new(&Controls { punch: true, run: true, ..Default::default() }, false);
        let b = Input::new(&Controls { wish: glam::vec2(1.0, 0.0), ..Default::default() }, false);
        let c = a.then(&b).controls();
        assert!(c.punch && !c.run);
        assert_eq!(c.wish, glam::vec2(1.0, 0.0));
    }
}
