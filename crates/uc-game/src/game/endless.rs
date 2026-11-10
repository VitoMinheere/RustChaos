//! Endless mode, "Riot" (#129): no mission script, just waves of thugs on the map, each
//! bigger and better armed than the last, until the player goes down or the city falls (the
//! crime rate reaches 100, which it only climbs towards while the player keeps out of the
//! fight). Each wave comes from one side of the arena, a different one from the last, marked
//! with a beacon; every tenth brings a boss. Every wave comes from the run's own seed, so the
//! same seed gives the same waves.
//!
//! A run can be played with modifiers ([`Mods`], #518), rules chosen before it starts that
//! change how it plays, each with a score multiplier. None of them are in the original, as
//! none of endless mode is.

use crate::*;
use uc_world::character::person as ptype;

/// Seconds between waves.
pub const BREATHER: f32 = 15.0;
/// No more than this many of a wave about at once; the rest come as others go down.
fn at_once(wave: u32) -> usize {
    (6 + wave as usize / 2).min(12)
}
/// The sides of the arena waves come from, by the angle round its middle.
const SIDES: u32 = 4;
/// Seconds the beacon on the side a wave comes from stays up.
const SIDE_BEACON: f32 = 10.0;
/// Seconds of keeping out of the fight before the crime rate starts to climb.
const GRACE: f32 = 12.0;
/// Seconds a foe can stand about getting nowhere, out of a fight, before he's made again
/// somewhere he can get to her from.
const STUCK: f32 = 8.0;
/// Spawned no nearer the player than this (world units), and out of their sight if it can be.
const NOT_NEARER: f32 = 6.0 * 256.0;
/// Every tenth wave has a boss.
const BOSS_EVERY: u32 = 10;
/// Seconds before the first wave: long enough for the intro, and Darci's answer.
pub const FIRST_BREATHER: f32 = 11.0;
/// The extra seconds before the first wave with modifiers on, for dispatch to say so.
const RULES_TIME: f32 = 4.0;

/// How fast a run with `mods` goes: one and a half times with the speed modifier.
fn pace(mods: Mods) -> f32 {
    if mods.has(Mod::Speed) { 1.5 } else { 1.0 }
}

/// What dispatch says of the run's rules before the first wave: the one modifier's line, or
/// the stacked deck and then each of theirs.
fn rules(mods: Mods) -> Vec<Riot> {
    let each = mods.list().into_iter().map(|m| Riot::Rules(Mod::ALL.iter().position(|&x| x == m).unwrap() as u8));
    match mods.list().len() {
        0 | 1 => each.collect(),
        _ => std::iter::once(Riot::Stacked).chain(each).collect(),
    }
}

/// A rule an endless run can be played by (#518).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mod {
    /// No weapons on the map or the waves: fists and feet.
    Fists,
    /// Darci goes down in one hit.
    OneHit,
    /// Every one of the wave as tough as a boss (four times the health).
    Heavy,
    /// Every one of the wave armed with a gun, ammo dropped more often, and Darci starts with
    /// a pistol.
    GunRun,
    /// Everyone falls at half the weight: jumps twice as high, falls twice as far unhurt.
    LowGravity,
    /// The game runs at one and a half times the speed.
    Speed,
    /// No health back from sitting on a bench: only from what's picked up.
    NoRegen,
}

impl Mod {
    pub const ALL: [Mod; 7] = [Mod::Fists, Mod::OneHit, Mod::Heavy, Mod::GunRun, Mod::LowGravity, Mod::Speed, Mod::NoRegen];

    fn bit(self) -> u8 {
        1 << Mod::ALL.iter().position(|&m| m == self).unwrap()
    }

    /// One word for it, as it's kept in the progress file and given on the command line.
    pub fn id(self) -> &'static str {
        match self {
            Mod::Fists => "fists",
            Mod::OneHit => "onehit",
            Mod::Heavy => "heavy",
            Mod::GunRun => "gunrun",
            Mod::LowGravity => "lowgravity",
            Mod::Speed => "speed",
            Mod::NoRegen => "noregen",
        }
    }

    pub fn named(id: &str) -> Option<Mod> {
        Mod::ALL.into_iter().find(|m| m.id().eq_ignore_ascii_case(id))
    }

    /// As the menu and HUD say it.
    pub fn name(self) -> &'static str {
        match self {
            Mod::Fists => "FISTS ONLY",
            Mod::OneHit => "ONE HIT",
            Mod::Heavy => "HEAVY HITTERS",
            Mod::GunRun => "GUN RUN",
            Mod::LowGravity => "LOW GRAVITY",
            Mod::Speed => "SPEED",
            Mod::NoRegen => "NO REGENERATION",
        }
    }

    /// What it does, for the menu.
    pub fn text(self) -> &'static str {
        match self {
            Mod::Fists => "No weapons on the streets or the thugs",
            Mod::OneHit => "Darci goes down in one hit",
            Mod::Heavy => "Every thug as tough as a boss",
            Mod::GunRun => "Every thug has a gun, more ammo drops, and you start with a pistol",
            Mod::LowGravity => "Jump twice as high, fall twice as far unhurt",
            Mod::Speed => "The game runs at 1.5x",
            Mod::NoRegen => "No health from benches, only pickups",
        }
    }

    /// Its score multiplier, in hundredths.
    pub fn percent(self) -> u32 {
        match self {
            Mod::Fists => 120,
            Mod::OneHit => 200,
            Mod::Heavy => 150,
            Mod::GunRun => 130,
            Mod::LowGravity => 100,
            Mod::Speed => 125,
            Mod::NoRegen => 110,
        }
    }

    /// The modifier it can't be played with.
    pub fn clash(self) -> Option<Mod> {
        match self {
            Mod::Fists => Some(Mod::GunRun),
            Mod::GunRun => Some(Mod::Fists),
            _ => None,
        }
    }

    /// What dispatch says of it before the first wave.
    fn line(self) -> &'static str {
        match self {
            Mod::Fists => "Word is they've left the guns at home tonight. Fists only, Darci.",
            Mod::OneHit => "One hit and you're down tonight, Darci. Don't let them touch you.",
            Mod::Heavy => "These boys are built like tanks tonight. Hit them hard.",
            Mod::GunRun => "Everyone's packing tonight, Darci. Shoot first.",
            Mod::LowGravity => "Strange reports from the street tonight. People are floating off the kerb.",
            Mod::Speed => "It's all happening fast tonight, Darci. Keep up.",
            Mod::NoRegen => "No medics on call tonight, Darci. Make every pickup count.",
        }
    }
}

/// The modifiers a run is played with (#518): a set of [`Mod`]s.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Mods(u8);

impl Mods {
    pub const NONE: Mods = Mods(0);

    pub fn has(self, m: Mod) -> bool {
        self.0 & m.bit() != 0
    }

    /// With `m` switched: on (and what it clashes with off), or off.
    pub fn toggled(self, m: Mod) -> Mods {
        if self.has(m) {
            Mods(self.0 & !m.bit())
        } else {
            let off = m.clash().map_or(0, Mod::bit);
            Mods((self.0 & !off) | m.bit())
        }
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The modifiers on, in [`Mod::ALL`]'s order.
    pub fn list(self) -> Vec<Mod> {
        Mod::ALL.into_iter().filter(|&m| self.has(m)).collect()
    }

    /// The score multiplier, in hundredths: each one's, multiplied together.
    pub fn percent(self) -> u32 {
        self.list().into_iter().fold(100, |p, m| p * m.percent() / 100)
    }

    /// As a file stores it, and back (unknown bits dropped).
    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn from_bits(b: u8) -> Mods {
        Mods(b & ((1 << Mod::ALL.len()) - 1))
    }

    /// Their ids joined with `-`, in [`Mod::ALL`]'s order (so the same set is always the same
    /// words), or "" for none; and back, None for a word that isn't one.
    pub fn ids(self) -> String {
        self.list().into_iter().map(Mod::id).collect::<Vec<_>>().join("-")
    }

    /// `points`, by the multiplier.
    pub fn scaled(self, points: i64) -> i64 {
        points * self.percent() as i64 / 100
    }

    pub fn named(ids: &str) -> Option<Mods> {
        ids.split(['-', ',', '+']).filter(|s| !s.is_empty()).try_fold(Mods::NONE, |m, id| Some(Mods(m.0 | Mod::named(id)?.bit())))
    }
}

/// The waves dispatch marks with a line of its own when they're cleared.
const MILESTONES: [u32; 8] = [5, 10, 15, 20, 25, 30, 40, 50];

/// What's said over the radio (the lines are made voices, `talk2\endless\`, [`lines`]):
/// dispatch's, and Darci's answers. The game never had endless mode, so never these. The
/// ones with a number are takes of the same moment, picked at random, so a long run doesn't
/// hear the same words every wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Riot {
    /// The run starts.
    Intro,
    /// Darci's answer to it.
    Ready,
    /// The next wave's on its way.
    Incoming(u8),
    /// The next wave has a boss.
    BossIncoming(u8),
    /// A wave's down.
    Cleared(u8),
    /// A boss wave's down.
    BossDown(u8),
    /// A wave in [`MILESTONES`] is down: the milestone's place in it.
    Milestone(u8),
    /// Darci between waves, now and then.
    Quip(u8),
    /// Keeping out of the fight: the crime rate's climbing.
    Slacking(u8),
    /// The crime rate's reached 100.
    Lost,
    /// Before the first wave, the run's one modifier: its place in [`Mod::ALL`].
    Rules(u8),
    /// Before the first wave, with more than one modifier.
    Stacked,
}

impl Riot {
    /// How many takes of each line there are.
    const INCOMING: u8 = 6;
    const BOSS_INCOMING: u8 = 3;
    const CLEARED: u8 = 6;
    const BOSS_DOWN: u8 = 3;
    const QUIPS: u8 = 6;
    const SLACKING: u8 = 3;

    /// Every line.
    pub fn all() -> Vec<Riot> {
        let mut v = vec![Riot::Intro, Riot::Ready, Riot::Lost, Riot::Stacked];
        v.extend((0..Mod::ALL.len() as u8).map(Riot::Rules));
        v.extend((0..Riot::INCOMING).map(Riot::Incoming));
        v.extend((0..Riot::BOSS_INCOMING).map(Riot::BossIncoming));
        v.extend((0..Riot::CLEARED).map(Riot::Cleared));
        v.extend((0..Riot::BOSS_DOWN).map(Riot::BossDown));
        v.extend((0..MILESTONES.len() as u8).map(Riot::Milestone));
        v.extend((0..Riot::QUIPS).map(Riot::Quip));
        v.extend((0..Riot::SLACKING).map(Riot::Slacking));
        v
    }

    /// Darci says it, not dispatch.
    pub fn darci(self) -> bool {
        matches!(self, Riot::Ready | Riot::Quip(_))
    }

    /// The made voice's file, under `talk2\`.
    pub fn file(self) -> String {
        let name = match self {
            Riot::Intro => "intro".to_string(),
            Riot::Ready => "ready".to_string(),
            Riot::Incoming(k) => format!("incoming{k}"),
            Riot::BossIncoming(k) => format!("boss_incoming{k}"),
            Riot::Cleared(k) => format!("cleared{k}"),
            Riot::BossDown(k) => format!("boss_down{k}"),
            Riot::Milestone(k) => format!("wave{}", MILESTONES[k as usize]),
            Riot::Quip(k) => format!("quip{k}"),
            Riot::Slacking(k) => format!("slacking{k}"),
            Riot::Lost => "lost".to_string(),
            Riot::Rules(k) => format!("rules_{}", Mod::ALL[k as usize].id()),
            Riot::Stacked => "rules_stacked".to_string(),
        };
        format!("endless/{name}.wav")
    }

    /// What's said.
    pub fn text(self) -> &'static str {
        match self {
            Riot::Intro => "This is dispatch. The Wildcats have declared war on these streets, and you're all we've got out there. Hold them back as long as you can.",
            Riot::Ready => "Copy that. Let them come.",
            Riot::Incoming(0) => "Heads up, more of them heading your way.",
            Riot::Incoming(1) => "We've got another crew moving in on your position.",
            Riot::Incoming(2) => "Here comes the next lot. Stay sharp.",
            Riot::Incoming(3) => "Reports of a gang massing nearby. They're coming for you.",
            Riot::Incoming(4) => "Another wave inbound. Don't let them take the street.",
            Riot::Incoming(_) => "They're not done yet. Get ready.",
            Riot::BossIncoming(0) => "Watch yourself. A big one's leading the next lot.",
            Riot::BossIncoming(1) => "Word is their top enforcer's coming down himself. Be careful.",
            Riot::BossIncoming(_) => "Something big's heading your way. Take him down and the rest will scatter.",
            Riot::Cleared(0) => "That's the last of them. Catch your breath.",
            Riot::Cleared(1) => "Street's clear. Nice work.",
            Riot::Cleared(2) => "All down. Take a moment, there'll be more.",
            Riot::Cleared(3) => "Good job. The crime rate's coming down.",
            Riot::Cleared(4) => "Area secure, for now.",
            Riot::Cleared(_) => "That's them dealt with. Stay on your toes.",
            Riot::BossDown(0) => "The big one's down! That'll rattle them.",
            Riot::BossDown(1) => "You took out their enforcer. Outstanding.",
            Riot::BossDown(_) => "Boss down. The Wildcats won't forget that in a hurry.",
            Riot::Milestone(0) => "Five waves and still standing. The Chief's watching.",
            Riot::Milestone(1) => "Ten waves. Half the precinct's listening in on this.",
            Riot::Milestone(2) => "Fifteen waves. And we're hearing reports of men in black suits. Stay alert.",
            Riot::Milestone(3) => "Twenty waves. Nobody's held a street this long.",
            Riot::Milestone(4) => "Twenty five. You're making history out there.",
            Riot::Milestone(5) => "Thirty waves. They're throwing everything they've got at you.",
            Riot::Milestone(6) => "Forty. I don't know how you're still on your feet.",
            Riot::Milestone(_) => "Fifty waves. They'll be writing songs about this.",
            Riot::Quip(0) => "Is that all you've got?",
            Riot::Quip(1) => "Next.",
            Riot::Quip(2) => "Somebody call the meat wagon.",
            Riot::Quip(3) => "I'm just getting warmed up.",
            Riot::Quip(4) => "They just keep coming.",
            Riot::Quip(_) => "Who's next?",
            Riot::Slacking(0) => "The crime rate's climbing. Get in there!",
            Riot::Slacking(1) => "You're losing the street. Get stuck in!",
            Riot::Slacking(_) => "They're running riot out there. Do something!",
            Riot::Lost => "That's it, we've lost the streets. Pull out!",
            Riot::Rules(k) => Mod::ALL[k as usize].line(),
            Riot::Stacked => "They've stacked the deck against you tonight, Darci. Good luck.",
        }
    }
}

/// Every line said over the radio, for making their voices: (file, words, Darci's).
pub fn lines() -> Vec<(String, String, bool)> {
    Riot::all().into_iter().map(|l| (l.file(), l.text().to_string(), l.darci())).collect()
}

/// Where a wave is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Seconds until the next wave.
    Breather(f32),
    /// A wave is on.
    Wave,
}

/// One of the wave's people, and whether they've been counted yet.
#[derive(Debug, Clone, Copy)]
struct Foe {
    person: usize,
    counted: bool,
    boss: bool,
    /// Where he was when he last got anywhere, and the seconds since.
    last: Vec3,
    still: f32,
}

/// An endless run.
#[derive(Debug, Clone)]
pub struct Endless {
    pub seed: u32,
    /// The rules it's played by.
    pub mods: Mods,
    /// Seconds before the first wave: [`FIRST_BREATHER`], longer with modifiers to say.
    first: f32,
    /// The wave on, or the last one done (0 before the first).
    pub wave: u32,
    pub phase: Phase,
    pub score: i64,
    /// Arrests and knockouts, all waves.
    pub arrests: u32,
    pub downed: u32,
    rng: u32,
    /// The middle of the arena, and how far out from it (world units) waves are made.
    centre: Vec3,
    /// The arena: the middle of every square that can be walked to from where the player
    /// started, within its radius, on their level ([`arena`]).
    pub spots: Vec<Vec3>,
    /// The arena's squares: each one's middle and the squares it leads to.
    grid: Grid,
    /// Still to come in this wave.
    to_come: u32,
    foes: Vec<Foe>,
    /// The red marks already charged for.
    red_marks: u8,
    /// Seconds towards the crime rate's next rise.
    crime_clock: f32,
    /// Seconds since the player last hurt anyone of the wave.
    pub idle: f32,
    /// What the wave's people had left between them last turn.
    wave_health: i64,
    /// The side of the arena this wave comes from (of four, by the angle round its middle),
    /// and the seconds left of its beacon.
    pub side: u32,
    side_beacon: f32,
    /// This wave's boss is still to be made.
    boss_due: bool,
    /// What picks the radio's takes: apart from `rng`, so the same seed still makes the
    /// same waves.
    talk_rng: u32,
    /// The last few lines said, not to be said again straight away.
    heard: Vec<Riot>,
    /// The player's health last turn, and whether she's lost any this wave.
    my_health: i32,
    hurt: bool,
}

impl Endless {
    /// The wave's people still on their feet, and still to come.
    pub fn left(&self, g: &Game) -> u32 {
        self.foes.iter().filter(|f| !f.counted && !g.people[f.person].removed).count() as u32 + self.to_come
    }

    /// Seconds to the next wave, between waves.
    pub fn breather(&self) -> Option<f32> {
        match self.phase {
            Phase::Breather(t) => Some(t),
            Phase::Wave => None,
        }
    }

    fn rand(&mut self) -> u32 {
        rand(&mut self.rng)
    }

    /// Which side of the arena `at` is on.
    fn side_of(&self, at: Vec3) -> u32 {
        let a = (at.z - self.centre.z).atan2(at.x - self.centre.x) + std::f32::consts::PI;
        ((a / std::f32::consts::TAU * SIDES as f32) as u32).min(SIDES - 1)
    }

    /// The middle of the arena's squares on `side`.
    fn side_middle(&self, side: u32) -> Option<Vec3> {
        let on: Vec<Vec3> = self.spots.iter().copied().filter(|&s| self.side_of(s) == side).collect();
        let mean = on.iter().copied().sum::<Vec3>() / on.len().max(1) as f32;
        // The spot nearest the average, so it's on the ground somewhere real.
        on.into_iter().min_by(|a, b| a.distance(mean).total_cmp(&b.distance(mean)))
    }

    /// Whether this wave's boss is up, not yet down.
    pub fn boss(&self, g: &Game) -> Option<usize> {
        self.foes.iter().find(|f| f.boss && !f.counted && !g.people[f.person].removed).map(|f| f.person)
    }
}

/// What wave `n` sends: how many, and how tough.
pub fn wave_size(n: u32) -> u32 {
    3 + n * 3 / 2
}

impl Game {
    /// Starts an endless run on `level` with `seed`, in the arena `radius` squares round
    /// `centre` (world xz; the player's start if none), the player put there: no mission
    /// script, the city's crowd about as usual, the crime rate on the HUD.
    pub fn new_endless(level: &Level, seed: u32, centre: Option<Vec2>, radius: f32) -> Game {
        Game::new_endless_with(level, seed, centre, radius, Mods::NONE)
    }

    /// [`Game::new_endless`], played with modifiers `mods` (#518).
    pub fn new_endless_with(level: &Level, seed: u32, centre: Option<Vec2>, radius: f32, mods: Mods) -> Game {
        let mut g = Game::new(level, false);
        if let Some(c) = centre {
            g.teleport(Vec3::new(c.x, 1.0e5, c.y), level);
            // From the ground there, not the roof above it: the highest surface no more than
            // a storey over the street.
            let y = level.ground.surface(&level.map, c.x, c.y, level.ground.surface(&level.map, c.x, c.y, 0.0) + 128.0);
            g.teleport(Vec3::new(c.x, y, c.y), level);
        }
        // Out in the street: RTA's start is inside the police station.
        let start = g.player().map_or(Vec3::ZERO, |p| p.fig.pos);
        let centre = out_of_doors(level, start);
        if centre != start {
            g.teleport(centre, level);
        }
        let grid = arena(level, centre, radius * 256.0);
        let spots = grid.values().map(|(at, _)| *at).collect();
        g.show_crime_rate = true;
        g.crime_rate = 20;
        // Fists only: none of the map's weapons. A gun run: a pistol to start with.
        if mods.has(Mod::Fists) {
            g.items.retain(|it| Weapon::from_item(it.kind).is_none());
        }
        if let (true, Some(p)) = (mods.has(Mod::GunRun), g.player) {
            g.people[p].give(Weapon::Pistol);
            g.people[p].give(Weapon::Pistol);
        }
        // Room for each of dispatch's rules lines, in game seconds (more of them at speed).
        let first = (FIRST_BREATHER + RULES_TIME * rules(mods).len() as f32) * pace(mods);
        g.endless = Some(Endless {
            seed, mods, first, wave: 0, phase: Phase::Breather(first), score: 0, arrests: 0, downed: 0,
            rng: seed | 1, centre, spots, grid, to_come: 0, foes: Vec::new(), red_marks: 0, crime_clock: 0.0,
            idle: 0.0, wave_health: 0, side: SIDES, side_beacon: 0.0, boss_due: false,
            talk_rng: (seed ^ 0x2545_f491).wrapping_mul(0x9e37_79b9), heard: Vec::new(),
            my_health: 0, hurt: false,
        });
        g
    }

    /// The endless run, if this is one.
    pub fn endless(&self) -> Option<&Endless> {
        self.endless.as_ref()
    }

    /// Whether this is an endless run played with modifier `m`.
    pub fn has_mod(&self, m: Mod) -> bool {
        self.endless.as_ref().is_some_and(|e| e.mods.has(m))
    }

    /// How fast the game runs: one and a half times with the speed modifier.
    pub fn pace(&self) -> f32 {
        self.endless.as_ref().map_or(1.0, |e| pace(e.mods))
    }

    /// For tests: the run, to skip ahead.
    pub fn endless_mut(&mut self) -> Option<&mut Endless> {
        self.endless.as_mut()
    }

    /// One turn of the run: waves start, are counted and filled in, and the crime rate
    /// rises while they're about.
    pub(crate) fn run_endless(&mut self, dt: f32, level: &Level) {
        let Some(mut e) = self.endless.take() else { return };
        if self.outcome.is_none() {
            self.endless_turn(&mut e, dt, level);
        }
        self.endless = Some(e);
    }

    /// Says `line` over the radio: shown with its speaker's face, and heard.
    fn riot(&mut self, e: &mut Endless, line: Riot) {
        let (who, face) = if line.darci() { ("Darci", talk::face::DARCI) } else { ("Despatch", talk::face::RADIO) };
        // Up for as long as it takes to say: about two and a half words a second.
        let words = line.text().split_whitespace().count() as f32;
        self.messages.push(Message { text: format!("{who}: {}", line.text()), left: (words / 2.6 + 1.0).max(3.0), face: Some(face), filmed: false });
        let at = self.player().map_or(e.centre, |p| p.fig.pos);
        if !line.darci() {
            self.sounds.push((Sfx::Radio, at));
        }
        self.sounds.push((Sfx::Riot(line), at));
        e.heard.push(line);
        if e.heard.len() > 4 {
            e.heard.remove(0);
        }
    }

    /// One of the `n` takes `take` makes, not one of the last few said.
    fn take(e: &mut Endless, n: u8, take: fn(u8) -> Riot) -> Riot {
        let mut line = take((rand(&mut e.talk_rng) % n as u32) as u8);
        for _ in 0..4 {
            if !e.heard.contains(&line) {
                break;
            }
            line = take((rand(&mut e.talk_rng) % n as u32) as u8);
        }
        line
    }

    fn endless_turn(&mut self, e: &mut Endless, dt: f32, level: &Level) {
        // Innocents hurt: a red mark costs 200.
        if self.red_marks > e.red_marks {
            e.score -= 200 * (self.red_marks - e.red_marks) as i64;
            e.red_marks = self.red_marks;
        }
        // One hit: she has the one point of health, whatever she picks up.
        if let (true, Some(p)) = (e.mods.has(Mod::OneHit), self.player) {
            let p = &mut self.people[p];
            p.max_health = 1;
            p.health = p.health.min(1);
        }
        match e.phase {
            Phase::Breather(t) => {
                let crossed = |at: f32| t > at && t - dt <= at;
                if e.wave == 0 {
                    // The run's start: dispatch sets the scene, and Darci answers.
                    if crossed(e.first - 0.3) {
                        self.riot(e, Riot::Intro);
                    }
                    // And what tonight's rules are, if there are any: one to a few seconds,
                    // the last ending before Darci answers.
                    let (lines, pace) = (rules(e.mods), pace(e.mods));
                    for (k, &line) in lines.iter().enumerate() {
                        if crossed((2.5 + RULES_TIME * (lines.len() - k) as f32) * pace) {
                            self.riot(e, line);
                        }
                    }
                    if crossed(3.5 * pace) {
                        self.riot(e, Riot::Ready);
                    }
                } else {
                    // Now and then Darci has something to say between waves.
                    if crossed(BREATHER - 5.0) && rand(&mut e.talk_rng).is_multiple_of(3) {
                        let line = Game::take(e, Riot::QUIPS, Riot::Quip);
                        self.riot(e, line);
                    }
                    // Three seconds before the next, dispatch says what's coming.
                    if crossed(3.0) {
                        let line = if (e.wave + 1).is_multiple_of(BOSS_EVERY) {
                            Game::take(e, Riot::BOSS_INCOMING, Riot::BossIncoming)
                        } else {
                            Game::take(e, Riot::INCOMING, Riot::Incoming)
                        };
                        self.riot(e, line);
                    }
                }
                let t = t - dt;
                if t > 0.0 {
                    e.phase = Phase::Breather(t);
                    return;
                }
                e.wave += 1;
                e.to_come = wave_size(e.wave);
                e.foes.clear();
                e.phase = Phase::Wave;
                e.idle = 0.0;
                e.wave_health = 0;
                e.boss_due = e.wave.is_multiple_of(BOSS_EVERY);
                e.hurt = false;
                e.my_health = self.player().map_or(0, |p| p.health);
                // A side it didn't come from last time.
                let last = e.side;
                let sides: Vec<u32> = (0..SIDES).filter(|&s| s != last && e.side_middle(s).is_some()).collect();
                e.side = if sides.is_empty() { last % SIDES } else { sides[(e.rand() as usize) % sides.len()] };
                e.side_beacon = SIDE_BEACON;
                let at = self.player().map_or(e.centre, |p| p.fig.pos);
                let text = if e.boss_due { format!("Wave {}: a boss is coming", e.wave) } else { format!("Wave {}", e.wave) };
                self.messages.push(Message { text, left: 3.0, face: None, filmed: false });
                self.sounds.push((Sfx::Alert { kind: ptype::THUG_GREY }, at));
                // A boss comes with the sirens, from his side.
                if e.boss_due {
                    self.sounds.push((Sfx::Siren, e.side_middle(e.side).unwrap_or(e.centre)));
                }
            }
            Phase::Wave => {}
        }
        e.side_beacon = (e.side_beacon - dt).max(0.0);
        // Any health the player's lost this wave (for the stats: a boss wave cleared unhurt).
        let health = self.player().map_or(0, |p| p.health);
        e.hurt |= health < e.my_health;
        e.my_health = health;
        // Count those who've gone down: an arrest is worth more than a knockout.
        let mult = e.wave as i64;
        for f in e.foes.iter_mut().filter(|f| !f.counted) {
            let p = &self.people[f.person];
            if p.arrested {
                f.counted = true;
                e.arrests += 1;
                e.score += e.mods.scaled(150 * mult);
            } else if p.knocked_out || p.is_dead() || p.removed {
                f.counted = true;
                e.downed += 1;
                e.score += e.mods.scaled(100 * mult);
            }
            if f.counted {
                self.crime_rate = self.crime_rate.saturating_sub(2);
            }
        }
        // Whether the player's in the fight: any of the wave hurt, or downed, this turn.
        let health: i64 = e.foes.iter().filter(|f| !f.counted).map(|f| self.people[f.person].health.max(0) as i64).sum();
        let downed = e.foes.iter().filter(|f| f.counted).count() as i64;
        let tally = health - downed * 100_000;
        if tally < e.wave_health {
            e.idle = 0.0;
        } else {
            e.idle += dt;
        }
        e.wave_health = tally;
        // Anyone getting nowhere, out of the fight, is made again where he can get to her.
        let me = self.player().map(|p| p.fig.pos);
        for k in 0..e.foes.len() {
            let f = e.foes[k];
            let p = &self.people[f.person];
            if f.counted || p.removed || p.is_down() {
                continue;
            }
            let near = me.is_some_and(|m| flat_dist(p.fig.pos, m) < 3.0 * 256.0);
            if near || p.fig.pos.distance(f.last) > 128.0 {
                e.foes[k].last = p.fig.pos;
                e.foes[k].still = 0.0;
                continue;
            }
            e.foes[k].still += dt;
            if e.foes[k].still > STUCK {
                if let Some(at) = self.spawn_spot(e, level) {
                    self.people[f.person].fig.pos = at;
                    e.foes[k].last = at;
                }
                e.foes[k].still = 0.0;
            }
        }
        // The rest keep coming for the player, by the arena's own ways (the navigation map
        // can think a wall's open).
        let ways = self.player().and_then(|me| towards(&e.grid, me.fig.pos));
        let me = self.player().map(|p| p.fig.pos);
        for f in &e.foes {
            let p = &mut self.people[f.person];
            if f.counted {
                p.steer = None;
                continue;
            }
            // (They shouted coming on, `spawn_foe`'s `go_for`: not again each time they
            // come back to her.)
            if !p.aggro() && !p.is_down() {
                p.keep_after();
            }
            p.steer = match (&ways, me) {
                (Some(ways), Some(me)) => next_step(&e.grid, ways, p.fig.pos, me, level),
                _ => None,
            };
        }
        // More of the wave, as there's room: the boss first, if it's his wave.
        let about = e.foes.iter().filter(|f| !f.counted).count();
        for _ in about..at_once(e.wave) {
            if e.to_come == 0 {
                break;
            }
            let boss = e.boss_due;
            if !self.spawn_foe(e, level, boss) {
                break;
            }
            e.boss_due = false;
            e.to_come -= 1;
        }
        // The crime rate climbs only while the player keeps out of the fight (hiding, or
        // running about): after a grace, a point every three seconds, faster the longer it
        // goes on, to one every half second. At 100 the city's lost.
        let standing = e.foes.iter().any(|f| !f.counted) || e.to_come > 0;
        if standing && e.idle > GRACE {
            if e.crime_clock == 0.0 {
                let line = Game::take(e, Riot::SLACKING, Riot::Slacking);
                self.riot(e, line);
            }
            let every = (3.0 - (e.idle - GRACE) / 10.0).max(0.5);
            e.crime_clock += dt / every;
            while e.crime_clock >= 1.0 {
                e.crime_clock -= 1.0;
                self.crime_rate = (self.crime_rate + 1).min(100);
            }
            e.crime_clock = e.crime_clock.max(f32::MIN_POSITIVE);
        } else {
            e.crime_clock = 0.0;
        }
        if self.crime_rate >= 100 {
            self.riot(e, Riot::Lost);
            self.finish(Outcome::Lost("The Wildcats have taken the streets".into()));
            return;
        }
        // All down: a bonus, and a breather.
        if e.to_come == 0 && e.foes.iter().all(|f| f.counted) {
            e.score += e.mods.scaled(250 * mult);
            e.phase = Phase::Breather(BREATHER);
            if e.wave.is_multiple_of(BOSS_EVERY) && !e.hurt {
                self.record.boss_unhurt += 1;
            }
            self.messages.push(Message { text: format!("Wave {} cleared", e.wave), left: 3.0, face: None, filmed: false });
            // A milestone's own line, a boss's, or one of the takes.
            let line = match MILESTONES.iter().position(|&m| m == e.wave) {
                Some(k) => Riot::Milestone(k as u8),
                None if e.wave.is_multiple_of(BOSS_EVERY) => Game::take(e, Riot::BOSS_DOWN, Riot::BossDown),
                None => Game::take(e, Riot::CLEARED, Riot::Cleared),
            };
            self.riot(e, line);
        }
    }

    /// Makes one of the wave somewhere out of the player's way. Whether there was room.
    fn spawn_foe(&mut self, e: &mut Endless, level: &Level, boss: bool) -> bool {
        let Some(pos) = self.spawn_spot(e, level) else { return false };
        let n = e.wave;
        // The boss is a big thug: the Men in Black jump clear of every blow, so one as a boss
        // could only be shot.
        let kind = if boss {
            ptype::THUG_RED
        // (Not the Men in Black with fists only: they always have their AKs.)
        } else if n >= 15 && !e.mods.has(Mod::Fists) && e.rand().is_multiple_of(4) {
            [ptype::MIB1, ptype::MIB2, ptype::MIB3][(e.rand() % 3) as usize]
        } else {
            [ptype::THUG_GREY, ptype::THUG_RED, ptype::THUG_RASTA][(e.rand() % 3) as usize]
        };
        // Fists first, then bats and knives, then a few pistols, then shotguns and AKs.
        let arms: &[Weapon] = match n {
            0..=3 => &[Weapon::None],
            4..=6 => &[Weapon::None, Weapon::Bat, Weapon::Knife],
            7..=10 => &[Weapon::None, Weapon::Bat, Weapon::Knife, Weapon::Pistol],
            11..=14 => &[Weapon::Bat, Weapon::Knife, Weapon::Pistol, Weapon::Shotgun],
            _ => &[Weapon::Knife, Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47],
        };
        // A gun run: pistols, then shotguns, then AKs, as many as there are.
        let gun_run: &[Weapon] = match n {
            0..=6 => &[Weapon::Pistol],
            7..=14 => &[Weapon::Pistol, Weapon::Shotgun],
            _ => &[Weapon::Pistol, Weapon::Shotgun, Weapon::Ak47],
        };
        let arms = if e.mods.has(Mod::GunRun) { gun_run } else { arms };
        let mut weapon = arms[(e.rand() as usize) % arms.len()];
        // No more guns about at once than one, and another every four waves.
        let guns = e.foes.iter().filter(|f| !f.counted && self.people[f.person].weapon.is_gun()).count() as u32;
        if weapon.is_gun() && guns > n / 4 && !e.mods.has(Mod::GunRun) {
            weapon = if n >= 11 { Weapon::Bat } else { Weapon::Knife };
        }
        if boss {
            weapon = if n >= 20 || e.mods.has(Mod::GunRun) { Weapon::Ak47 } else { Weapon::Bat };
        }
        if e.mods.has(Mod::Fists) {
            weapon = Weapon::None;
        }
        // Something to find on them: health or ammo, less often as the waves go on, but
        // always some chance of health.
        let find = e.rand() % 100;
        let health_chance = (30 - n as i32 + self.difficulty().finds()).max(15) as u32;
        // (Plenty of ammo on a gun run; none with fists only, with nothing to put it in.)
        let ammo_chance = (25 + self.difficulty().finds()).max(0) as u32 + if e.mods.has(Mod::GunRun) { 30 } else { 0 };
        let drop = if find < health_chance {
            Some(3)
        } else if e.mods.has(Mod::Fists) {
            None
        } else if find < health_chance + ammo_chance {
            match weapon {
                Weapon::Shotgun => Some(11),
                Weapon::Ak47 => Some(12),
                _ => Some(10),
            }
        } else {
            None
        };
        let angle = (e.rand() % 2048) as f32 * std::f32::consts::TAU / 2048.0;
        let Some(i) = self.spawn(level, kind, false, 0, pos, angle) else { return false };
        let p = &mut self.people[i];
        p.ai = person::AI_GANG;
        // Following the player (`PCOM_MOVE_FOLLOW`): after her wherever she is, and never
        // losing interest (a follower's sphere is 64 squares).
        (p.mv, p.follow) = (5, 0);
        p.skill = if boss { (8 + n / 3).min(15) as u8 } else { (2 + n / 3).min(15) as u8 };
        p.fig.jacket = p.skill / 4;
        p.weapon = weapon;
        p.give(weapon);
        p.drop = drop;
        // A boss, or any of them with heavy hitters, takes four times the beating.
        if boss || e.mods.has(Mod::Heavy) {
            p.max_health *= 4;
            p.health = p.max_health;
        }
        if boss {
            p.drop = Some(3);
        }
        p.go_for();
        let change = self.difficulty().skill_change();
        self.tune(i, change);
        e.foes.push(Foe { person: i, counted: false, boss, last: pos, still: 0.0 });
        true
    }

    /// A spot for the next of the wave: one of the arena's squares, no nearer the player
    /// than [`NOT_NEARER`] (or half the arena, if that's smaller) and out of their sight if
    /// any such turns up.
    fn spawn_spot(&mut self, e: &mut Endless, level: &Level) -> Option<Vec3> {
        let me = self.player().map(|p| p.fig.pos)?;
        let eye = me + Vec3::new(0.0, 120.0, 0.0);
        let far = e.spots.iter().map(|s| flat_dist(*s, e.centre)).fold(0.0, f32::max);
        let not_nearer = NOT_NEARER.min(far * 0.5);
        let mut seen = None;
        for tries in 0..90 {
            if e.spots.is_empty() {
                return None;
            }
            let k = (e.rand() as usize) % e.spots.len();
            let at = e.spots[k];
            // The first tries keep to the wave's side.
            if flat_dist(at, me) < not_nearer || (tries < 60 && e.side_of(at) != e.side) {
                continue;
            }
            if !level.ground.line_of_sight(eye, at + Vec3::new(0.0, 120.0, 0.0)) {
                return Some(at);
            }
            seen.get_or_insert(at);
        }
        seen
    }
}

fn flat_dist(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

/// The arena round `start` (world units): the middle of each square within `radius` that
/// can be walked to from `start`'s square on its level. A square leads to the next if the
/// ground there is within a step of it (the highest surface no more than a step over this
/// one, so the inside of a building isn't its roof) and no wall stands between their
/// middles at waist height, as the walker finds them, and it isn't under a building's roof.
/// So everyone made there can get to the player, and nobody is made outside the playable
/// area, on a roof or shut inside a building.
pub fn arena(level: &Level, start: Vec3, radius: f32) -> Grid {
    let n = uc_formats::iam::MAP_SIZE as i32;
    let sq = |v: Vec3| ((v.x / 256.0) as i32, (v.z / 256.0) as i32);
    let middle = |x: i32, z: i32, from: f32| {
        let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
        Vec3::new(cx, level.ground.surface(&level.map, cx, cz, from + uc_world::ground::STEP_HEIGHT), cz)
    };
    let waist = Vec3::new(0.0, uc_world::ground::STEP_HEIGHT - 4.0, 0.0);
    let inside = |x: i32, z: i32| level.map.square(x as usize, z as usize).has(uc_formats::iam::flags::HIDDEN);
    let (sx, sz) = sq(start);
    let mut at = std::collections::BTreeMap::new();
    at.insert((sx, sz), Vec3::new(sx as f32 * 256.0 + 128.0, start.y, sz as f32 * 256.0 + 128.0));
    let mut queue = std::collections::VecDeque::from([(sx, sz)]);
    let mut out = Grid::new();
    while let Some((x, z)) = queue.pop_front() {
        let here = at[&(x, z)];
        let mut leads = Vec::new();
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, nz) = (x + dx, z + dz);
            if !(1..n - 1).contains(&nx) || !(1..n - 1).contains(&nz) {
                continue;
            }
            // Not into a building (`PAP_FLAG_HIDDEN`: under its roof).
            if inside(nx, nz) {
                continue;
            }
            let next = at.get(&(nx, nz)).copied().unwrap_or_else(|| middle(nx, nz, here.y));
            if flat_dist(next, start) > radius || (next.y - here.y).abs() > uc_world::ground::STEP_HEIGHT {
                continue;
            }
            // Nor over a fence that can't be climbed over (it's a wall to the AI, mav.cpp:797).
            if !level.ground.line_of_sight(here + waist, next + waist) || level.ground.unscalable_fence_between(here, next) {
                continue;
            }
            leads.push((nx, nz));
            if let std::collections::btree_map::Entry::Vacant(v) = at.entry((nx, nz)) {
                v.insert(next);
                queue.push_back((nx, nz));
            }
        }
        out.insert((x, z), (here, leads));
    }
    out
}

/// The nearest square to `at` that isn't inside a building (`PAP_FLAG_HIDDEN`), on the
/// ground at about `at`'s height: `at` itself if it's out of doors already.
pub fn out_of_doors(level: &Level, at: Vec3) -> Vec3 {
    let inside = |x: i32, z: i32| level.map.square(x as usize, z as usize).has(uc_formats::iam::flags::HIDDEN);
    let (sx, sz) = square(at);
    if !inside(sx, sz) {
        return at;
    }
    let n = uc_formats::iam::MAP_SIZE as i32;
    for r in 1..32 {
        let mut best: Option<(f32, Vec3)> = None;
        for dz in -r..=r {
            for dx in -r..=r {
                let (x, z) = (sx + dx, sz + dz);
                if dx.abs().max(dz.abs()) != r || !(1..n - 1).contains(&x) || !(1..n - 1).contains(&z) || inside(x, z) {
                    continue;
                }
                let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
                let y = level.ground.surface(&level.map, cx, cz, at.y + uc_world::ground::STEP_HEIGHT);
                if (y - at.y).abs() > uc_world::ground::STEP_HEIGHT {
                    continue;
                }
                let spot = Vec3::new(cx, y, cz);
                let d = flat_dist(spot, at);
                if best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, spot));
                }
            }
        }
        if let Some((_, spot)) = best {
            return spot;
        }
    }
    at
}

/// An arena: each square's middle, and the squares it leads to.
pub type Grid = std::collections::BTreeMap<(i32, i32), (Vec3, Vec<(i32, i32)>)>;

fn square(v: Vec3) -> (i32, i32) {
    ((v.x / 256.0) as i32, (v.z / 256.0) as i32)
}

/// For every square of the arena, the next one on the way to `to`'s square (a search out
/// from it), or None if `to` isn't in the arena.
pub(crate) fn towards(grid: &Grid, to: Vec3) -> Option<std::collections::BTreeMap<(i32, i32), (i32, i32)>> {
    let goal = square(to);
    grid.get(&goal)?;
    let mut next = std::collections::BTreeMap::from([(goal, goal)]);
    let mut queue = std::collections::VecDeque::from([goal]);
    while let Some(s) = queue.pop_front() {
        for &n in &grid[&s].1 {
            if let std::collections::btree_map::Entry::Vacant(v) = next.entry(n) {
                v.insert(s);
                queue.push_back(n);
            }
        }
    }
    Some(next)
}

/// Where someone at `from` should head for to get to `me` by the arena: the furthest of the
/// next three squares on the way that can be walked to straight, or None to go straight
/// for her (already near, or off the arena).
pub(crate) fn next_step(grid: &Grid, ways: &std::collections::BTreeMap<(i32, i32), (i32, i32)>, from: Vec3, me: Vec3, level: &Level) -> Option<Vec3> {
    let waist = Vec3::new(0.0, uc_world::ground::STEP_HEIGHT - 4.0, 0.0);
    if level.ground.line_of_sight(from + waist, me + waist) && flat_dist(from, me) < 6.0 * 256.0 {
        return None;
    }
    let mut s = square(from);
    let mut best = None;
    for _ in 0..3 {
        let &n = ways.get(&s)?;
        if n == s {
            break;
        }
        let at = grid[&n].0;
        if best.is_none() || level.ground.line_of_sight(from + waist, at + waist) {
            best = Some(at);
        } else {
            break;
        }
        s = n;
    }
    best
}

impl Game {
    /// An endless run's beacons: the side of the arena the wave's coming from (for its first
    /// seconds), and the boss while he's up.
    pub(crate) fn endless_beacons(&self) -> Vec<(String, Vec3)> {
        let Some(e) = &self.endless else { return Vec::new() };
        let mut out = Vec::new();
        if e.side_beacon > 0.0 && e.phase == Phase::Wave {
            if let Some(at) = e.side_middle(e.side) {
                out.push((format!("Wave {} coming", e.wave), at));
            }
        }
        if let Some(b) = e.boss(self) {
            out.push(("Boss".to_string(), self.people[b].fig.pos));
        }
        out
    }
}
