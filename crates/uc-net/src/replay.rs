//! Replays (#184): a mission saved as how it started and every seat's input for every turn,
//! and played back by feeding those inputs to a game started the same way. The game plays
//! the same turns the same way given the same inputs (what co-op stands on too), so a replay
//! of a long mission is a few hundred KB, not a video.
//!
//! What a replay can't hold, it has to be told about or refuse: the console changes the game
//! outside the inputs, so a run that used it isn't saved (the viewer drops the [`Recorder`]),
//! and the one thing the viewer itself does to the game, losing a won mission over too many
//! civilians dead, goes in as an [`Event`]. The state hash goes with every [`HASH_EVERY`]th
//! turn, so a playback that has drifted (another build, or something missed) can say when.
//!
//! The file: [`MAGIC`], then little-endian fields as the co-op messages have them. The
//! inputs go as runs of identical turns, since most turns repeat the last.

use crate::wire::{In, Out};
use crate::{Input, Setup, BUILD, HASH_EVERY};
use std::io;
use uc_game::{Difficulty, Game, Outcome, Weapon, TURN};
use uc_world::Level;

/// The file's first bytes, with the format's version in the last.
pub const MAGIC: &[u8; 4] = b"UCR4";

/// The versions before inputs held looking round, and before that the strafe key (#309),
/// still read.
const MAGIC_3: &[u8; 4] = b"UCR3";
const MAGIC_2: &[u8; 4] = b"UCR2";

/// The weapons a game can start with, by their number in a file.
const WEAPONS: [Weapon; 10] = [
    Weapon::None, Weapon::Knife, Weapon::Bat, Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47,
    Weapon::Explosives, Weapon::Sniper, Weapon::Grenade, Weapon::Hook,
];

/// What kind of game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// The mission, its script running.
    Mission,
    /// The map without the script (`--explore`).
    Explore,
    /// Endless mode on the mission's map (#129): the seed, the arena's middle (world units;
    /// the start if none) and its radius in squares.
    Endless { seed: u32, centre: Option<[f32; 2]>, radius: f32 },
    /// A Beat Cop shift (#130): the seed, the arena's middle, and the shift's minutes (none
    /// for an open-ended one).
    Beat { seed: u32, centre: Option<[f32; 2]>, shift: Option<u32> },
    /// The mission in co-op, Roper next to Darci.
    Coop,
}

/// How a game started: everything that changes it before turn 0. [`Start::game`] makes it,
/// for the run and again for its replay, so the two can't start differently.
#[derive(Debug, Clone, PartialEq)]
pub struct Start {
    pub mission: String,
    pub mode: Mode,
    /// [`Difficulty`], as its place in `Difficulty::ALL`.
    pub difficulty: u8,
    /// The player's badge stats ([`Game::set_stats`]).
    pub stats: [u8; 4],
    /// The weapon in hand (`--weapon`).
    pub weapon: Weapon,
    /// Dogs let loose round the start (`--dogs`).
    pub dogs: u32,
    /// How far the camera sees, at least, in world units: where the city's extra people come
    /// and go (`--fog`).
    pub view_dist: Option<f32>,
}

impl Start {
    /// The mission at `difficulty` with badge stats `stats`, and nothing else.
    pub fn new(mission: &str, mode: Mode, difficulty: Difficulty, stats: [u8; 4]) -> Start {
        let d = Difficulty::ALL.iter().position(|&x| x == difficulty).unwrap_or(1);
        Start { mission: mission.to_string(), mode, difficulty: d as u8, stats, weapon: Weapon::None, dogs: 0, view_dist: None }
    }

    /// What a co-op session plays.
    pub fn coop(s: &Setup) -> Start {
        Start::new(&s.mission, Mode::Coop, s.difficulty(), s.stats)
    }

    pub fn difficulty(&self) -> Difficulty {
        Difficulty::ALL.get(self.difficulty as usize).copied().unwrap_or(Difficulty::Normal)
    }

    /// The game, at turn 0.
    pub fn game(&self, level: &Level) -> Game {
        let centre = |c: Option<[f32; 2]>| c.map(glam::Vec2::from_array);
        let mut g = match self.mode {
            Mode::Mission | Mode::Coop => Game::new(level, true),
            Mode::Explore => Game::new(level, false),
            Mode::Endless { seed, centre: c, radius } => Game::new_endless(level, seed, centre(c), radius),
            Mode::Beat { seed, centre: c, shift } => Game::new_beat(level, seed, centre(c), shift),
        };
        if let Some(d) = self.view_dist {
            g.view_dist = g.view_dist.max(d);
        }
        g.set_stats(self.stats);
        g.set_difficulty(self.difficulty());
        if self.mode == Mode::Coop {
            g.add_partner(level);
        }
        if let Some(p) = g.player {
            g.people[p].give(self.weapon);
            g.people[p].weapon = self.weapon;
        }
        if self.dogs > 0 {
            g.let_dogs_out(level, self.dogs as usize);
        }
        g
    }
}

/// Something done to the game between two turns that isn't an input.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The mission lost, as the viewer loses a win with too many civilians dead.
    Lose(String),
}

impl Event {
    fn apply(&self, g: &mut Game) {
        match self {
            Event::Lose(why) => g.outcome = Some(Outcome::Lost(why.clone())),
        }
    }
}

/// How a run went besides its outcome, for a list of replays.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Tally {
    /// The mission's badges found and side objectives done in this run.
    pub badges: u8,
    pub sides: u8,
    /// An endless run's or Beat Cop shift's score, and the wave reached or incidents handled.
    pub score: i64,
    pub count: u32,
}

/// A saved run.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    pub start: Start,
    /// The build that played it ([`BUILD`]): only that build is sure to play it back the same.
    pub build: String,
    /// When it was played: seconds since 1970.
    pub date: u64,
    pub seats: u8,
    /// How it ended: "won", "lost: why", or "abandoned".
    pub outcome: String,
    /// The mission clock at the end.
    pub time: f32,
    pub tally: Tally,
    /// Every turn's input, seat 0 first.
    pub turns: Vec<Vec<Input>>,
    /// The state hash after so many turns.
    pub hashes: Vec<(u32, u64)>,
    /// What happened between turns: before the turn with this number (0 first).
    pub events: Vec<(u32, Event)>,
}

impl Replay {
    /// Whether this build is the one that played it.
    pub fn same_build(&self) -> bool {
        self.build == BUILD
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Out(MAGIC.to_vec());
        o.str(&self.build);
        o.u64(self.date);
        let s = &self.start;
        o.str(&s.mission);
        let centre = |o: &mut Out, c: Option<[f32; 2]>| {
            o.u8(c.is_some() as u8);
            let c = c.unwrap_or_default();
            o.f32(c[0]);
            o.f32(c[1]);
        };
        match s.mode {
            Mode::Mission => o.u8(0),
            Mode::Explore => o.u8(1),
            Mode::Endless { seed, centre: c, radius } => {
                o.u8(2);
                o.u32(seed);
                centre(&mut o, c);
                o.f32(radius);
            }
            Mode::Beat { seed, centre: c, shift } => {
                o.u8(3);
                o.u32(seed);
                centre(&mut o, c);
                o.u32(shift.map_or(0, |m| m + 1));
            }
            Mode::Coop => o.u8(4),
        }
        o.u8(s.difficulty);
        o.0.extend_from_slice(&s.stats);
        o.u8(WEAPONS.iter().position(|&w| w == s.weapon).unwrap_or(0) as u8);
        o.u32(s.dogs);
        o.f32(s.view_dist.unwrap_or(-1.0));
        o.u8(self.seats);
        o.str(&self.outcome);
        o.f32(self.time);
        let t = &self.tally;
        o.u8(t.badges);
        o.u8(t.sides);
        o.u64(t.score as u64);
        o.u32(t.count);
        o.u32(self.events.len() as u32);
        for (turn, e) in &self.events {
            o.u32(*turn);
            match e {
                Event::Lose(why) => {
                    o.u8(0);
                    o.str(why);
                }
            }
        }
        o.u32(self.hashes.len() as u32);
        for &(turn, hash) in &self.hashes {
            o.u32(turn);
            o.u64(hash);
        }
        let runs: Vec<(u32, &Vec<Input>)> = self.turns.chunk_by(|a, b| a == b).map(|r| (r.len() as u32, &r[0])).collect();
        o.u32(self.turns.len() as u32);
        o.u32(runs.len() as u32);
        for (n, inputs) in runs {
            o.u32(n);
            for s in 0..self.seats as usize {
                o.input(&inputs.get(s).copied().unwrap_or_default());
            }
        }
        o.0
    }

    pub fn from_bytes(b: &[u8]) -> io::Result<Replay> {
        let bad = |why: &str| io::Error::new(io::ErrorKind::InvalidData, why.to_string());
        let (b, version) = match (b.strip_prefix(MAGIC), b.strip_prefix(MAGIC_3), b.strip_prefix(MAGIC_2)) {
            (Some(b), ..) => (b, 4),
            (_, Some(b), _) => (b, 3),
            (.., Some(b)) => (b, 2),
            _ => return Err(bad("not a replay, or another version")),
        };
        let mut i = In(b);
        let build = i.str()?;
        let date = i.u64()?;
        let mission = i.str()?;
        let centre = |i: &mut In| -> io::Result<Option<[f32; 2]>> {
            let some = i.u8()? != 0;
            let c = [i.f32()?, i.f32()?];
            Ok(some.then_some(c))
        };
        let mode = match i.u8()? {
            0 => Mode::Mission,
            1 => Mode::Explore,
            2 => Mode::Endless { seed: i.u32()?, centre: centre(&mut i)?, radius: i.f32()? },
            3 => Mode::Beat { seed: i.u32()?, centre: centre(&mut i)?, shift: i.u32()?.checked_sub(1) },
            4 => Mode::Coop,
            _ => return Err(bad("unknown kind of game")),
        };
        let difficulty = i.u8()?;
        let stats = i.bytes(4)?.try_into().unwrap();
        let weapon = WEAPONS.get(i.u8()? as usize).copied().unwrap_or(Weapon::None);
        let dogs = i.u32()?;
        let view_dist = Some(i.f32()?).filter(|&d| d >= 0.0);
        let start = Start { mission, mode, difficulty, stats, weapon, dogs, view_dist };
        let seats = i.u8()?;
        let outcome = i.str()?;
        let time = i.f32()?;
        let tally = Tally { badges: i.u8()?, sides: i.u8()?, score: i.u64()? as i64, count: i.u32()? };
        let mut events = Vec::new();
        for _ in 0..i.u32()? {
            let turn = i.u32()?;
            match i.u8()? {
                0 => events.push((turn, Event::Lose(i.str()?))),
                _ => return Err(bad("unknown event")),
            }
        }
        let n = i.u32()? as usize;
        // Each takes 12 bytes, so a count past what's left is garbage.
        if n > i.0.len() / 12 {
            return Err(bad("too many hashes"));
        }
        let mut hashes = Vec::with_capacity(n);
        for _ in 0..n {
            hashes.push((i.u32()?, i.u64()?));
        }
        let total = i.u32()? as usize;
        let mut turns = Vec::with_capacity(total.min(1 << 20));
        for _ in 0..i.u32()? {
            let n = i.u32()? as usize;
            let inputs = (0..seats).map(|_| i.input_of(version)).collect::<io::Result<Vec<_>>>()?;
            if turns.len() + n > total {
                return Err(bad("more turns than it says"));
            }
            turns.extend(std::iter::repeat_n(inputs, n));
        }
        if turns.len() != total {
            return Err(bad("fewer turns than it says"));
        }
        Ok(Replay { start, build, date, seats, outcome, time, tally, turns, hashes, events })
    }
}

/// Keeps a run as it's played: the game keeps its turns ([`Game::keep_played`]), and they're
/// taken into the replay every frame.
#[derive(Debug, Clone)]
pub struct Recorder {
    replay: Replay,
}

impl Recorder {
    /// Starts keeping `game`, made by `start` and not yet played.
    pub fn new(game: &mut Game, start: Start) -> Recorder {
        game.keep_played(HASH_EVERY as u64);
        let date = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let replay = Replay {
            start, build: BUILD.to_string(), date, seats: game.seats() as u8, outcome: String::new(), time: 0.0,
            tally: Tally::default(), turns: Vec::new(), hashes: Vec::new(), events: Vec::new(),
        };
        Recorder { replay }
    }

    /// The turns played since the last time.
    pub fn take(&mut self, game: &mut Game) {
        for p in game.take_played() {
            let turn = self.replay.turns.len() as u32;
            let inputs = p.inputs.iter().enumerate().map(|(s, c)| Input::new(c, p.skip && s == 0)).collect();
            self.replay.turns.push(inputs);
            if let Some(h) = p.hash {
                self.replay.hashes.push((turn + 1, h));
            }
        }
    }

    /// `e` done to `game` now, after the turns played so far.
    pub fn event(&mut self, game: &mut Game, e: Event) {
        self.take(game);
        self.replay.events.push((self.replay.turns.len() as u32, e));
    }

    /// The turns kept so far.
    pub fn turns(&self) -> usize {
        self.replay.turns.len()
    }

    /// The replay, as `game` stands now: its outcome, time, and an endless run's or shift's
    /// score (the badges and side objectives are the caller's to fill in, who knows the
    /// mission's).
    pub fn finish(&mut self, game: &mut Game) -> Replay {
        self.take(game);
        let mut r = self.replay.clone();
        r.outcome = match &game.outcome {
            Some(Outcome::Won) => "won".into(),
            Some(Outcome::Lost(why)) => format!("lost: {why}"),
            None => "abandoned".into(),
        };
        r.time = game.time;
        if let Some(e) = game.endless() {
            (r.tally.score, r.tally.count) = (e.score, e.wave);
        }
        if let Some(b) = game.beat() {
            (r.tally.score, r.tally.count) = (b.score, b.handled);
        }
        r
    }
}

/// A replay being played back into a game made by its [`Start::game`].
#[derive(Debug, Clone)]
pub struct Playback {
    pub replay: Replay,
    /// The turns played.
    next: usize,
    /// The hashes and events already come to.
    hash_at: usize,
    event_at: usize,
    /// Real time not yet played: less than a [`TURN`].
    clock: f32,
    /// The first turn after which the game wasn't as recorded.
    pub drift: Option<u32>,
}

impl Playback {
    pub fn new(replay: Replay) -> Playback {
        Playback { replay, next: 0, hash_at: 0, event_at: 0, clock: 0.0, drift: None }
    }

    /// Back to turn 0, for a game made again.
    pub fn rewind(&mut self) {
        (self.next, self.hash_at, self.event_at, self.clock, self.drift) = (0, 0, 0, 0.0, None);
    }

    /// The turns played so far.
    pub fn turn(&self) -> usize {
        self.next
    }

    pub fn len(&self) -> usize {
        self.replay.turns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.replay.turns.is_empty()
    }

    /// Every turn played.
    pub fn done(&self) -> bool {
        self.next >= self.replay.turns.len()
    }

    /// The next turn into `g`, with whatever happened before it, and its hash compared.
    /// False at the end.
    pub fn step(&mut self, g: &mut Game, level: &Level) -> bool {
        let r = &self.replay;
        while let Some((_, e)) = r.events.get(self.event_at).filter(|e| e.0 as usize <= self.next) {
            e.apply(g);
            self.event_at += 1;
        }
        let Some(inputs) = r.turns.get(self.next) else { return false };
        crate::play_turn(g, level, inputs);
        self.next += 1;
        while let Some(&(turn, hash)) = r.hashes.get(self.hash_at).filter(|h| h.0 as usize <= self.next) {
            if turn as usize == self.next && self.drift.is_none() && g.state_hash() != hash {
                self.drift = Some(turn);
            }
            self.hash_at += 1;
        }
        true
    }

    /// `dt` seconds of the replay (already sped up or slowed down): the turns due.
    pub fn frame(&mut self, g: &mut Game, level: &Level, dt: f32) {
        self.clock += dt;
        while self.clock >= TURN {
            self.clock -= TURN;
            if !self.step(g, level) {
                self.clock = 0.0;
                break;
            }
        }
        g.set_between(self.clock / TURN);
    }

    /// Straight on `turns` turns, as fast as they play.
    pub fn skip(&mut self, g: &mut Game, level: &Level, turns: usize) {
        for _ in 0..turns {
            if !self.step(g, level) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty replay's bytes.
    fn empty() -> Vec<u8> {
        Replay {
            start: Start::new("", Mode::Mission, Difficulty::Normal, [0; 4]), build: String::new(), date: 0, seats: 1,
            outcome: String::new(), time: 0.0, tally: Tally::default(), turns: Vec::new(), hashes: Vec::new(), events: Vec::new(),
        }.to_bytes()
    }

    #[test]
    fn a_replay_round_trips() {
        let i = |x: f32, b: u16| Input { wish: [x, -x], drive: [0.5, x], buttons: b, strafe: (b & 1 != 0).then_some([x, 1.0]), look: (b & 2 != 0).then_some([x, 0.25]) };
        let r = Replay {
            start: Start {
                mission: "Assault1".into(), mode: Mode::Beat { seed: 7, centre: Some([256.0, -512.0]), shift: None },
                difficulty: 2, stats: [1, 2, 3, 4], weapon: Weapon::Shotgun, dogs: 3, view_dist: Some(10240.0),
            },
            build: "abc".into(), date: 1_700_000_000, seats: 2, outcome: "lost: Darci died".into(), time: 12.5,
            tally: Tally { badges: 2, sides: 1, score: -40, count: 7 },
            turns: vec![vec![i(0.0, 0), i(1.0, 4)], vec![i(0.0, 0), i(1.0, 4)], vec![i(0.25, 1), i(0.0, 0)]],
            hashes: vec![(1, 0xdead_beef), (3, 42)],
            events: vec![(2, Event::Lose("Too many civilian casualties".into()))],
        };
        let b = r.to_bytes();
        assert_eq!(Replay::from_bytes(&b).unwrap(), r);
        assert!(Replay::from_bytes(&b[..b.len() - 1]).is_err());
        assert!(Replay::from_bytes(b"UCG1").is_err());
    }

    #[test]
    fn every_mode_round_trips() {
        for mode in [Mode::Mission, Mode::Explore, Mode::Coop, Mode::Endless { seed: 1, centre: None, radius: 10.0 },
                     Mode::Beat { seed: 2, centre: None, shift: Some(10) }] {
            let mut r = Replay::from_bytes(&empty()).unwrap();
            r.start.mode = mode;
            assert_eq!(Replay::from_bytes(&r.to_bytes()).unwrap().start.mode, mode);
        }
    }
}
