//! Beat Cop (#130): a shift on a district with no mission script. Dispatch calls in
//! incidents one or two at a time (a mugging, an armed suspect, a dumped weapon, a gang
//! fight, a wanted man running, a drunk driver, a getaway car, a jumper on a roof, a bomb,
//! a bag snatch, a drug deal, a troublemaker, a man down, a gunman on a roof), each with a
//! beacon and a time limit, never the same kind twice running nor where the last few were;
//! handling them scores
//! and brings the crime rate down, missing them puts it up. The shift ends when its time's
//! up (a win; one of [`SHIFTS`], or open-ended), or when the crime rate reaches 100 or the
//! player goes down. An unlimited shift (#520) has no clock and goes on past 100, the calls
//! coming faster, until the player goes down or ends it ([`Game::end_shift`]). Everything
//! comes from the shift's seed.

use crate::*;
use uc_world::character::person as ptype;

/// A shift this long (seconds) ended with the crime rate under [`QUIET_RATE`] counts in
/// the stats. Not in the original, as none of Beat Cop is.
const QUIET_SHIFT: f32 = 20.0 * 60.0;
const QUIET_RATE: u8 = 20;

/// How long a shift is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shift {
    /// This many minutes, then it's over (a win).
    Minutes(u32),
    /// No clock: until the crime rate reaches 100 or the player drops.
    Open,
    /// No clock, and the crime rate reaching 100 doesn't end it either (#520): a patrol at
    /// the player's own pace, ended from the pause menu. No best is kept for it.
    Unlimited,
}

impl Shift {
    /// As the menu and HUD say it: "10 MIN", "OPEN-ENDED" or "UNLIMITED".
    pub fn name(self) -> String {
        match self {
            Shift::Minutes(m) => format!("{m} MIN"),
            Shift::Open => "OPEN-ENDED".into(),
            Shift::Unlimited => "UNLIMITED".into(),
        }
    }

    /// The shift's length in seconds, if it has a clock.
    pub fn seconds(self) -> Option<f32> {
        match self {
            Shift::Minutes(m) => Some(m as f32 * 60.0),
            Shift::Open | Shift::Unlimited => None,
        }
    }

    /// Whether a best score is kept for it (and it counts towards the best-shift stat).
    pub fn scored(self) -> bool {
        self != Shift::Unlimited
    }
}

impl Default for Shift {
    fn default() -> Shift {
        DEFAULT_SHIFT
    }
}

/// The shift lengths on offer, in the menu's order.
pub const SHIFTS: [Shift; 6] =
    [Shift::Minutes(5), Shift::Minutes(10), Shift::Minutes(15), Shift::Minutes(20), Shift::Open, Shift::Unlimited];
/// The shift length a new player gets.
pub const DEFAULT_SHIFT: Shift = Shift::Minutes(10);
/// How far round the start incidents happen, in squares, and no nearer the player than
/// this when they're called in. A district with fewer than [`ROOMY`] spots that far from
/// the start (Gangland's streets are hemmed in by the river) gets a wider beat, up to
/// [`WIDEST`].
const BEAT: f32 = 20.0;
const WIDEST: f32 = 40.0;
const ROOMY: usize = 200;
const NOT_NEARER: f32 = 8.0 * 256.0;
/// A new incident is no nearer than this to one on or to any of the last [`RECENT`] (world
/// units), while there's anywhere else for it.
const APART: f32 = 6.0 * 256.0;
const RECENT: usize = 5;
/// What a dumped gun goes in: the dustbins (`PRIM_OBJ_BIN`, and the bigger 134) and the
/// skips (221), the props the missions stash things in.
const BINS: [u8; 3] = [33, 134, 221];
/// A bag snatcher, a dealer and his buyer run from a cop this near (world units).
const RUNS_FROM: f32 = 8.0 * 256.0;
/// A gunman on a roof turns on a cop this near, up there with him.
const OPENS_UP: f32 = 12.0 * 256.0;
/// No more than this many incidents at once.
const AT_ONCE: usize = 2;
/// The event point numbers given to the people incidents make, so the AI's "go for the
/// person point N made" can point at them; well above any mission's.
const FIRST_WAY: usize = 50_000;
/// Talking a jumper down, defusing a bomb: this near (and on the same level) for this long.
const BESIDE: f32 = 1.5 * 256.0;
/// A siren this near a drunk driver (world units) has him pull over.
const PULL_OVER: f32 = 8.0 * 256.0;
/// A getaway car stuck this long (seconds), or down to this much of its health, and the
/// driver gets out to fight.
const STUCK: f32 = 1.5;
const BAIL_HEALTH: i32 = 150;
const TALK_DOWN: f32 = 2.0;
const DEFUSE: f32 = 3.0;
/// Seeing to a man down: this long beside him, till the ambulance comes.
const FIRST_AID: f32 = 4.0;
/// The seconds between calls (this, and up to as long again at random); with the city out
/// of control on an unlimited shift, these.
const CALLS: f32 = 25.0;
const CALLS_RIOT: f32 = 10.0;

/// What dispatch says aloud (the lines are made voices, `talk2\beat\`, [`lines`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Despatch {
    /// The call for an incident.
    Call(Kind),
    /// It's handled.
    Done(Kind),
    /// It's missed.
    TooLate(Kind),
    /// The shift's over.
    ShiftOver,
    /// The crime rate's hit 100.
    OutOfControl,
}

impl Despatch {
    /// The made voice's file, under `talk2\`.
    pub fn file(self) -> String {
        let slug = |k: Kind| k.label().to_ascii_lowercase().replace(' ', "_");
        match self {
            Despatch::Call(k) => format!("beat/call_{}.wav", slug(k)),
            Despatch::Done(k) => format!("beat/done_{}.wav", slug(k)),
            Despatch::TooLate(k) => format!("beat/late_{}.wav", slug(k)),
            Despatch::ShiftOver => "beat/shift_over.wav".into(),
            Despatch::OutOfControl => "beat/out_of_control.wav".into(),
        }
    }

    /// What's said: what's shown, without "Despatch: ".
    pub fn text(self) -> String {
        match self {
            Despatch::Call(k) => k.call().into(),
            Despatch::Done(k) => format!("{} dealt with. Good work.", k.label()),
            Despatch::TooLate(k) => format!("Too late for the {}.", k.label().to_ascii_lowercase()),
            Despatch::ShiftOver => "Shift over. Good work out there.".into(),
            Despatch::OutOfControl => "The city's out of control.".into(),
        }
    }
}

/// Every line dispatch says, for making their voices: (file, words).
pub fn lines() -> Vec<(String, String)> {
    let each = Kind::ALL.iter().flat_map(|&k| [Despatch::Call(k), Despatch::Done(k), Despatch::TooLate(k)]);
    each.chain([Despatch::ShiftOver, Despatch::OutOfControl]).map(|d| (d.file(), d.text())).collect()
}

/// What's going on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A thug going for a passer-by: get him before the victim's down.
    Mugging,
    /// Someone with a gun about the streets: disarm and take him.
    ArmedSuspect,
    /// A gun dumped in a bin or a skip near the spot: find it (search it out).
    DumpedWeapon,
    /// Two lots of Wildcats fighting each other: take them all down.
    GangFight,
    /// A wanted man, who runs on seeing a cop: catch him.
    WantedMan,
    /// A car weaving through the traffic: get the driver out of it. He pulls over for a
    /// siren, stops (late) for someone in the road and gets out to her at his door; a shot
    /// frightens him out, and a wreck throws him out, to be taken in.
    DrunkDriver,
    /// A robber driving off fast: stop the car (box it in or smash it up and he bails out),
    /// then take him, gun and all.
    Getaway,
    /// Somebody up on a roof by a ladder, about to jump: get up there and talk them down
    /// (stay with them a moment).
    Jumper,
    /// A bomb ticking in the street: get to it and defuse it (stay with it a moment).
    Bomb,
    /// A thief already running off with a bag: run him down.
    BagSnatch,
    /// A dealer and his buyer on a corner, who scatter when they see a cop: take both.
    DrugDeal,
    /// A thug picking fights with whoever walks by (`PCOM_AI_BULLY`): take him down.
    Troublemaker,
    /// Someone lying hurt in the street: get to them and stay with them till the ambulance
    /// comes.
    ManDown,
    /// A gunman up on a roof by a ladder: get up there and take him down (he turns on her
    /// once she's up).
    Gunman,
}

impl Kind {
    pub const ALL: [Kind; 14] = [
        Kind::Mugging, Kind::ArmedSuspect, Kind::DumpedWeapon, Kind::GangFight, Kind::WantedMan,
        Kind::DrunkDriver, Kind::Getaway, Kind::Jumper, Kind::Bomb, Kind::BagSnatch, Kind::DrugDeal,
        Kind::Troublemaker, Kind::ManDown, Kind::Gunman,
    ];

    /// The beacon's label.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Mugging => "Mugging",
            Kind::ArmedSuspect => "Armed suspect",
            Kind::DumpedWeapon => "Dumped weapon",
            Kind::GangFight => "Gang fight",
            Kind::WantedMan => "Wanted man",
            Kind::DrunkDriver => "Drunk driver",
            Kind::Getaway => "Getaway",
            Kind::Jumper => "Jumper",
            Kind::Bomb => "Bomb",
            Kind::BagSnatch => "Bag snatch",
            Kind::DrugDeal => "Drug deal",
            Kind::Troublemaker => "Troublemaker",
            Kind::ManDown => "Man down",
            Kind::Gunman => "Rooftop gunman",
        }
    }

    /// The one called `name` (the console's `call`): its label's first word, any case.
    pub fn named(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.label().split(' ').next().is_some_and(|w| w.eq_ignore_ascii_case(name)))
    }

    /// What dispatch says.
    fn call(self) -> &'static str {
        match self {
            Kind::Mugging => "Report of a mugging in progress. Get over there before somebody gets hurt!",
            Kind::ArmedSuspect => "We got an armed man walking the streets. Disarm him and bring him in.",
            Kind::DumpedWeapon => "A citizen saw somebody dump a gun in one of the bins round there. We need it as evidence: search them.",
            Kind::GangFight => "Wildcats fighting it out in the street. Break it up!",
            Kind::WantedMan => "A wanted man's been spotted. He'll run if he sees you coming.",
            Kind::DrunkDriver => "Somebody's driving all over the road round there. Get him out of that car before he kills someone.",
            Kind::Getaway => "Robbery! The suspect's making off in a car, and he's armed. Stop him.",
            Kind::Jumper => "We got a jumper up on a roof. Get up there and talk them down, quick.",
            Kind::Bomb => "Bomb threat, and this one's real. Get to it and defuse it before it goes up!",
            Kind::BagSnatch => "Bag snatch! The thief's on foot and running. Run him down.",
            Kind::DrugDeal => "There's a deal going down on a corner. They'll scatter when they see you: bring them both in.",
            Kind::Troublemaker => "Some lunatic's picking fights with anybody who walks past. Put a stop to it.",
            Kind::ManDown => "Man down in the street, hurt bad. Get to him and stay with him till the ambulance gets there.",
            Kind::Gunman => "Shots fired from a rooftop! Get up there and take the shooter down.",
        }
    }

    /// Seconds to deal with it.
    fn time(self) -> f32 {
        match self {
            Kind::Mugging => 75.0,
            Kind::ArmedSuspect => 120.0,
            Kind::DumpedWeapon => 150.0,
            Kind::GangFight => 120.0,
            Kind::WantedMan => 150.0,
            Kind::DrunkDriver => 150.0,
            Kind::Getaway => 150.0,
            Kind::Jumper => 120.0,
            Kind::Bomb => 90.0,
            Kind::BagSnatch => 90.0,
            Kind::DrugDeal => 150.0,
            Kind::Troublemaker => 120.0,
            Kind::ManDown => 90.0,
            Kind::Gunman => 150.0,
        }
    }

    /// Points for handling it.
    fn points(self) -> i64 {
        match self {
            Kind::Mugging => 200,
            Kind::ArmedSuspect => 250,
            Kind::DumpedWeapon => 150,
            Kind::GangFight => 300,
            Kind::WantedMan => 250,
            Kind::DrunkDriver => 250,
            Kind::Getaway => 350,
            Kind::Jumper => 250,
            Kind::Bomb => 400,
            Kind::BagSnatch => 200,
            Kind::DrugDeal => 300,
            Kind::Troublemaker => 200,
            Kind::ManDown => 200,
            Kind::Gunman => 350,
        }
    }
}

/// An incident going on.
#[derive(Debug, Clone)]
pub struct Incident {
    pub kind: Kind,
    /// Where it was called in.
    pub at: Vec3,
    /// Seconds left to deal with it.
    pub left: f32,
    /// The people it's about (the ones to take down), and any it's for (a victim).
    culprits: Vec<usize>,
    victims: Vec<usize>,
    /// For a dumped weapon: the item's point number.
    item: Option<usize>,
    /// For a jumper: who; for a man down: who; for a bomb: which mine it is.
    pub jumper: Option<usize>,
    pub patient: Option<usize>,
    pub bomb: Option<usize>,
    /// Seconds Darci's been with the jumper or the man down, or at the bomb.
    pub with: f32,
    /// Arrests among the culprits, counted as they're made.
    arrests: u32,
}

/// How a shift is going.
#[derive(Debug, Clone)]
pub struct Beat {
    pub seed: u32,
    /// How long the shift is.
    pub length: Shift,
    /// Seconds left of the shift; `None` when it has no clock.
    pub shift: Option<f32>,
    /// Seconds on duty so far.
    pub on: f32,
    pub score: i64,
    pub handled: u32,
    pub missed: u32,
    /// Arrests made in the incidents handled.
    pub arrests: u32,
    /// Ended from the pause menu ([`Game::end_shift`]).
    pub ended: bool,
    /// The crime rate is at 100 on an unlimited shift, and dispatch has said so.
    riot: bool,
    pub incidents: Vec<Incident>,
    rng: u32,
    /// Seconds to the next call.
    next: f32,
    next_way: usize,
    /// Where incidents can happen: the beat's squares, walkable from the start.
    spots: Vec<Vec3>,
    /// How far round the start the beat goes (world units).
    radius: f32,
    /// The bins and skips on the beat (map object numbers), for a dumped gun.
    bins: Vec<usize>,
    /// Where the last few incidents were, the latest last.
    recent: Vec<Vec3>,
    /// The kind called in last.
    last: Option<Kind>,
    /// The way the spot last picked faces (a car's lane, a jumper's ledge).
    yaw: f32,
    red_marks: u8,
}

impl Beat {
    fn rand(&mut self) -> u32 {
        rand(&mut self.rng)
    }

    /// Incident `inc` is on, and remembered, so the next few happen elsewhere.
    fn called(&mut self, inc: Incident) {
        self.last = Some(inc.kind);
        self.recent.push(inc.at);
        if self.recent.len() > RECENT {
            self.recent.remove(0);
        }
        self.incidents.push(inc);
    }

    /// Whether `at` is clear of the incidents on and the last few.
    fn fresh(&self, at: Vec3) -> bool {
        let far = |o: Vec3| crate::flat(at, o).length() >= APART;
        self.recent.iter().all(|&o| far(o)) && self.incidents.iter().all(|i| far(i.at))
    }
}

/// The end of a shift: what the summary says.
pub fn summary(b: &Beat) -> String {
    format!("Shift over. {} incidents handled, {} missed, {} arrests. Score {}", b.handled, b.missed, b.arrests, b.score)
}

/// Seconds as the HUD says them: "4:05", or "1:04:05" past the hour.
pub fn clock(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

impl Game {
    /// Starts a shift of `length` on `level` with `seed`, round `centre` (world xz; the
    /// start if none): no mission script, the city's crowd and traffic as usual, the crime
    /// rate on the HUD, the first call a few seconds in.
    pub fn new_beat(level: &Level, seed: u32, centre: Option<Vec2>, length: Shift) -> Game {
        let mut g = Game::new(level, false);
        if let Some(c) = centre {
            let y = level.ground.surface(&level.map, c.x, c.y, level.ground.surface(&level.map, c.x, c.y, 0.0) + 128.0);
            g.teleport(Vec3::new(c.x, y, c.y), level);
        }
        let start = g.player().map_or(Vec3::ZERO, |p| p.fig.pos);
        // Out in the street: RTA's start is inside the police station.
        let mut outside = game::endless::out_of_doors(level, start);
        // And down in it: Southside Beach's start is up on a roof, with nowhere to go.
        if game::endless::arena(level, outside, BEAT * 256.0).len() < ROOMY {
            let ground = level.ground.surface(&level.map, start.x, start.z, 128.0);
            let down = game::endless::out_of_doors(level, Vec3::new(start.x, ground, start.z));
            if game::endless::arena(level, down, BEAT * 256.0).len() >= ROOMY {
                outside = down;
            }
        }
        if outside != start {
            g.teleport(outside, level);
        }
        let start = outside;
        // Wide enough for a beat's worth of places to send her.
        let mut radius = BEAT * 256.0;
        let mut grid = game::endless::arena(level, start, radius);
        let far = |g: &game::endless::Grid| g.values().filter(|(at, _)| crate::flat(*at, start).length() >= NOT_NEARER).count();
        while far(&grid) < ROOMY && radius < WIDEST * 256.0 {
            radius += 5.0 * 256.0;
            grid = game::endless::arena(level, start, radius);
        }
        let spots = grid.values().map(|(at, _)| *at).collect();
        // A bin or skip on (or beside) a square she can walk to, out to the widest beat: they
        // are few and far between (West District has a skip by the start, and the next
        // twenty squares off).
        let wide = if radius < WIDEST * 256.0 { game::endless::arena(level, start, WIDEST * 256.0) } else { grid.clone() };
        let bins = level.map.objects.iter().enumerate()
            .filter(|(_, o)| BINS.contains(&o.prim))
            .filter(|(_, o)| {
                let (x, z) = (o.x >> 8, o.z >> 8);
                (-1..=1).any(|dx| (-1..=1).any(|dz| wide.contains_key(&(x + dx, z + dz))))
            })
            .map(|(i, _)| i)
            .collect();
        g.show_crime_rate = true;
        g.crime_rate = 40;
        g.beat = Some(Beat {
            // Stirred, so seeds a step apart don't make the same shift.
            seed, length, shift: length.seconds(), on: 0.0, score: 0, handled: 0, missed: 0, arrests: 0, ended: false, riot: false,
            incidents: Vec::new(),
            rng: (seed ^ 0x5bd1_e995).wrapping_mul(0x9e37_79b9),
            next: 4.0, next_way: FIRST_WAY, spots, radius, bins, recent: Vec::new(), last: None, yaw: 0.0, red_marks: 0,
        });
        g
    }

    /// The shift, if this is one.
    pub fn beat(&self) -> Option<&Beat> {
        self.beat.as_ref()
    }

    /// Ends the shift now, as from the pause menu (#520): over, a win, with dispatch's
    /// summary. Nothing if this isn't a shift or it's over already.
    pub fn end_shift(&mut self) {
        let Some(mut b) = self.beat.take() else { return };
        if self.outcome.is_none() {
            b.ended = true;
            self.messages.push(Message { text: summary(&b), left: 10.0, face: Some(talk::face::RADIO), filmed: false });
            self.voice(Despatch::ShiftOver);
            self.finish(Outcome::Won);
        }
        self.beat = Some(b);
    }

    /// Dispatch calls `kind` in now, whatever else is on (tests, and the console's `call`);
    /// whether there was somewhere for it.
    pub fn call_in_now(&mut self, kind: Kind, level: &Level) -> bool {
        let Some(mut b) = self.beat.take() else { return false };
        let inc = self.call_in(&mut b, kind, level);
        let called = inc.is_some();
        if let Some(inc) = inc {
            self.say(Despatch::Call(kind));
            b.called(inc);
        }
        self.beat = Some(b);
        called
    }

    /// For tests: the shift's incidents, to settle by hand.
    pub fn beat_mut(&mut self) -> Option<&mut Beat> {
        self.beat.as_mut()
    }

    pub(crate) fn run_beat(&mut self, dt: f32, level: &Level) {
        let Some(mut b) = self.beat.take() else { return };
        if self.outcome.is_none() {
            self.beat_turn(&mut b, dt, level);
        }
        self.beat = Some(b);
    }

    fn beat_turn(&mut self, b: &mut Beat, dt: f32, level: &Level) {
        if self.red_marks > b.red_marks {
            b.score -= 200 * (self.red_marks - b.red_marks) as i64;
            b.red_marks = self.red_marks;
        }
        b.on += dt;
        if let Some(left) = &mut b.shift {
            *left -= dt;
        }
        if b.shift.is_some_and(|left| left <= 0.0) {
            // A 20-minute shift (or longer) with the city quiet at the end, for the stats.
            if b.on >= QUIET_SHIFT - 1.0 && self.crime_rate < QUIET_RATE {
                self.record.quiet_shifts += 1;
            }
            self.messages.push(Message { text: summary(b), left: 10.0, face: Some(talk::face::RADIO), filmed: false });
            self.voice(Despatch::ShiftOver);
            self.finish(Outcome::Won);
            return;
        }
        // The incidents on: handled, missed or still going.
        let mut k = 0;
        while k < b.incidents.len() {
            let inc = &mut b.incidents[k];
            inc.left -= dt;
            match self.settle(inc, dt) {
                Some(true) => {
                    let inc = b.incidents.remove(k);
                    let bonus = 50 * inc.arrests as i64;
                    b.score += inc.kind.points() + bonus;
                    b.handled += 1;
                    b.arrests += inc.arrests;
                    self.crime_rate = self.crime_rate.saturating_sub(5);
                    self.say(Despatch::Done(inc.kind));
                    self.sounds.push((Sfx::Found, inc.at));
                    self.clear_up(&inc);
                }
                Some(false) => {
                    let inc = b.incidents.remove(k);
                    b.score -= 50;
                    b.missed += 1;
                    self.crime_rate = (self.crime_rate + 8).min(100);
                    self.say(Despatch::TooLate(inc.kind));
                    self.clear_up(&inc);
                }
                None => k += 1,
            }
        }
        if self.crime_rate >= 100 && b.length != Shift::Unlimited {
            self.voice(Despatch::OutOfControl);
            self.finish(Outcome::Lost("The city's out of control".into()));
            return;
        }
        // On an unlimited shift it goes on: dispatch says so (once each time it gets there),
        // and the calls come faster till it's back under.
        if self.crime_rate >= 100 && !b.riot {
            b.riot = true;
            self.say(Despatch::OutOfControl);
            b.next = b.next.min(CALLS_RIOT);
        } else if self.crime_rate < 100 {
            b.riot = false;
        }
        // The next call, when there's room for one.
        b.next -= dt;
        if b.next <= 0.0 && b.incidents.len() < AT_ONCE {
            // Not the kind just called, nor one that's on.
            let mut kind = Kind::ALL[(b.rand() as usize) % Kind::ALL.len()];
            for _ in 0..8 {
                if Some(kind) != b.last && !b.incidents.iter().any(|i| i.kind == kind) {
                    break;
                }
                kind = Kind::ALL[(b.rand() as usize) % Kind::ALL.len()];
            }
            // One there's no place for on this beat (a jumper, with no ladder up onto a roof)
            // makes way for another.
            for _ in 0..4 {
                if let Some(inc) = self.call_in(b, kind, level) {
                    self.say(Despatch::Call(kind));
                    b.called(inc);
                    break;
                }
                kind = Kind::ALL[(b.rand() as usize) % Kind::ALL.len()];
            }
            let gap = if b.riot { CALLS_RIOT } else { CALLS };
            b.next = gap + (b.rand() % gap as u32) as f32;
        }
    }

    /// A message from dispatch, shown and said.
    fn say(&mut self, line: Despatch) {
        self.messages.push(Message { text: format!("Despatch: {}", line.text()), left: 6.0, face: Some(talk::face::RADIO), filmed: false });
        let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
        self.sounds.push((Sfx::Radio, at));
        self.voice(line);
    }

    /// Dispatch says `line` aloud.
    fn voice(&mut self, line: Despatch) {
        let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
        self.sounds.push((Sfx::Despatch(line), at));
    }

    /// Whether incident `inc` is handled (true), missed (false) or still going (None).
    fn settle(&mut self, inc: &mut Incident, dt: f32) -> Option<bool> {
        let down = |p: &Person| p.is_dead();
        inc.arrests = inc.culprits.iter().filter(|&&i| self.people[i].arrested).count() as u32;
        let got = !inc.culprits.is_empty() && inc.culprits.iter().all(|&i| down(&self.people[i]));
        let me = self.player.filter(|&p| !self.people[p].aboard()).map(|p| self.people[p].fig.pos);
        let beside = |at: Vec3| me.is_some_and(|m| crate::flat(m, at).length() < BESIDE && (m.y - at.y).abs() < 128.0);
        if let Some(j) = inc.jumper {
            let p = &self.people[j];
            if beside(p.fig.pos) && !down(p) {
                inc.with += dt;
                if inc.with >= TALK_DOWN {
                    // Down off the ledge, and off about their business.
                    let p = &mut self.people[j];
                    (p.mv, p.bent) = (person::MOVE_WANDER, 0);
                    return Some(true);
                }
            } else {
                inc.with = 0.0;
            }
            return (inc.left <= 0.0 || down(p)).then_some(false);
        }
        if let Some(j) = inc.patient {
            let p = &self.people[j];
            if p.is_dead() {
                return Some(false);
            }
            if beside(p.fig.pos) {
                if inc.with == 0.0 {
                    self.messages.push(Message { text: "First aid...".into(), left: FIRST_AID, face: None, filmed: false });
                }
                inc.with += dt;
                if inc.with >= FIRST_AID {
                    return Some(true);
                }
            } else {
                inc.with = 0.0;
            }
            return (inc.left <= 0.0).then_some(false);
        }
        if let Some(k) = inc.bomb {
            if self.mines[k].gone {
                return Some(false);
            }
            if beside(self.mines[k].pos) {
                if inc.with == 0.0 {
                    self.messages.push(Message { text: "Defusing...".into(), left: DEFUSE, face: None, filmed: false });
                }
                inc.with += dt;
                if inc.with >= DEFUSE {
                    self.mines[k].gone = true;
                    return Some(true);
                }
            } else {
                inc.with = 0.0;
            }
            // Its fuse is the incident's time: it goes up on its own.
            return None;
        }
        if let Some(way) = inc.item {
            // The gun's found once it's out of its bin and picked up.
            let lying = self.items.iter().any(|it| it.way == way) || self.hidden.iter().any(|(_, it)| it.way == way);
            if !lying {
                return Some(true);
            }
        } else if got {
            return Some(true);
        }
        let victim_down = inc.victims.iter().any(|&i| down(&self.people[i]));
        if inc.left <= 0.0 || victim_down {
            return Some(false);
        }
        // A wanted man, a bag snatcher, a dealer and his buyer run from a cop they see
        // within eight squares.
        if matches!(inc.kind, Kind::WantedMan | Kind::BagSnatch | Kind::DrugDeal) {
            if let Some(me) = self.player().map(|p| p.fig.pos) {
                for &i in &inc.culprits {
                    let p = &mut self.people[i];
                    if !down(p) && p.scared.is_none() && (p.fig.pos - me).length() < RUNS_FROM {
                        p.mv = person::MOVE_WANDER;
                        p.scare(self.player, me);
                    }
                }
            }
        }
        // A drunk driver pulls over for a siren close by, or stops for someone in the road,
        // and once stopped and out of his car with her beside him, that's him off the road.
        if inc.kind == Kind::DrunkDriver {
            let siren = self.vehicles.iter().filter(|v| v.siren).map(|v| v.pos).collect::<Vec<_>>();
            for &i in &inc.culprits {
                match self.people[i].state {
                    State::InCar { vehicle } => {
                        let v = &mut self.vehicles[vehicle];
                        if siren.iter().any(|&s| (s - v.pos).length() < PULL_OVER) {
                            v.stalled = true;
                        }
                        let stopped = v.speed.abs() < 60.0;
                        let near = me.is_some_and(|m| crate::flat(m, v.pos).length() < 3.0 * 256.0);
                        // Out of the car: pulled over for her, or stopped with her at the door.
                        if stopped && (v.stalled || near) {
                            v.stalled = true;
                            v.driver = None;
                            let door = v.door();
                            let p = &mut self.people[i];
                            p.state = State::Move;
                            p.fig.pos = Vec3::new(door.x, v.pos.y, door.y);
                            p.cues.push(Sfx::CarDoor);
                            if let (Some(pi), Some(m)) = (self.player, me) {
                                self.people[i].hands_up(pi, m);
                            }
                        }
                    }
                    _ if !down(&self.people[i]) && beside(self.people[i].fig.pos) => return Some(true),
                    _ => {}
                }
            }
        }
        // A getaway driver bails out to fight once his car's boxed in or smashed up.
        if inc.kind == Kind::Getaway {
            for &i in &inc.culprits {
                if let State::InCar { vehicle } = self.people[i].state {
                    let v = &self.vehicles[vehicle];
                    // (Not while he's still pulling away.)
                    let going = inc.left < inc.kind.time() - 5.0;
                    inc.with = if going && v.speed.abs() < 60.0 { inc.with + dt } else { 0.0 };
                    if (inc.with > STUCK || v.health < BAIL_HEALTH) && self.people[i].scared.is_none() {
                        let at = v.pos;
                        self.people[i].scare(self.player, at);
                    }
                }
            }
        }
        // A getaway driver out of his car stands and fights.
        if inc.kind == Kind::Getaway {
            for &i in &inc.culprits {
                let p = &mut self.people[i];
                if !down(p) && !p.aboard() && p.ai == person::AI_DRIVER {
                    (p.ai, p.mv, p.scared) = (person::AI_GANG, person::MOVE_WANDER, None);
                    p.go_for();
                }
            }
        }
        // An armed suspect turns on her when she's near; a gunman on a roof once she's up
        // there with him (a fighter gives up on someone more than a storey above or below).
        if matches!(inc.kind, Kind::ArmedSuspect | Kind::Gunman) {
            let range = if inc.kind == Kind::Gunman { OPENS_UP } else { 6.0 * 256.0 };
            // Not while she's down: they leave her be then (the AI drops `aggro` every turn
            // she's lying there), and going for her again every turn they'd shout every turn
            // (#529).
            if let Some(me) = self.player().filter(|p| !p.is_down()).map(|p| p.fig.pos) {
                for &i in &inc.culprits {
                    let p = &mut self.people[i];
                    if !down(p) && !p.aggro() && (p.fig.pos - me).length() < range && (p.fig.pos.y - me.y).abs() < 256.0 {
                        p.ai = person::AI_GANG;
                        p.go_for();
                    }
                }
            }
        }
        None
    }

    /// After an incident: those still standing go about their business (the culprits of a
    /// missed one get away); the item of a missed one goes.
    fn clear_up(&mut self, inc: &Incident) {
        for &i in inc.culprits.iter().chain(&inc.victims) {
            let p = &mut self.people[i];
            if !(p.arrested || p.knocked_out || p.is_dead()) && inc.culprits.contains(&i) {
                p.removed = true;
            }
        }
        if let Some(way) = inc.item {
            self.items.retain(|it| it.way != way);
            self.hidden.retain(|(_, it)| it.way != way);
        }
        // A jumper not talked down has gone over the edge.
        if let Some(j) = inc.jumper.filter(|_| inc.with < TALK_DOWN) {
            self.people[j].removed = true;
        }
        // A man down is taken off in the ambulance, seen to or not.
        if let Some(j) = inc.patient {
            self.people[j].removed = true;
        }
    }

    /// Somewhere on the beat for the next incident: away from the player, but reachable,
    /// and away from the incidents on and the last few while there's anywhere else.
    fn incident_spot(&self, b: &mut Beat) -> Option<Vec3> {
        let me = self.player().map(|p| p.fig.pos)?;
        for k in 0..120 {
            if b.spots.is_empty() {
                return None;
            }
            let r = b.rand() as usize;
            let at = b.spots[r % b.spots.len()];
            if crate::flat(at, me).length() >= NOT_NEARER && (k >= 80 || b.fresh(at)) {
                return Some(at);
            }
        }
        None
    }

    /// A bin or skip on the beat for a dumped gun, as [`Self::incident_spot`] picks a spot:
    /// its map object number and where it is.
    fn bin_spot(&self, b: &mut Beat, level: &Level) -> Option<(usize, Vec3)> {
        let me = self.player().map(|p| p.fig.pos)?;
        for k in 0..60 {
            let r = b.rand() as usize;
            let ob = *b.bins.get(r % b.bins.len().max(1))?;
            let o = &level.map.objects[ob];
            let at = Vec3::new(o.x as f32, o.y as f32, o.z as f32);
            if crate::flat(at, me).length() >= NOT_NEARER && (k >= 40 || b.fresh(at)) {
                return Some((ob, at));
            }
        }
        None
    }

    /// A spot in a lane of a road on the beat, away from the player, for a car; the way the
    /// lane runs goes in `b.yaw`.
    fn road_spot(&self, b: &mut Beat, level: &Level) -> Option<Vec3> {
        let me = self.player().map(|p| p.fig.pos)?;
        for k in 0..60 {
            let r = b.rand() as usize;
            let near = *b.spots.get(r % b.spots.len().max(1))?;
            let heading = b.rand() & 1 == 1;
            let Some((at, yaw)) = self.lane_spot(level, Vec2::new(near.x, near.z), heading) else { continue };
            if (at - Vec2::new(me.x, me.z)).length() < NOT_NEARER || (at - Vec2::new(near.x, near.z)).length() > 6.0 * 256.0 {
                continue;
            }
            if k < 40 && !b.fresh(Vec3::new(at.x, near.y, at.y)) {
                continue;
            }
            b.yaw = yaw;
            let y = level.ground.surface(&level.map, at.x, at.y, 1.0e4);
            return Some(Vec3::new(at.x, y, at.y));
        }
        None
    }

    /// The top of a ladder up onto a roof on the beat, away from the player, for a jumper: a
    /// step in from the edge, facing out over it (in `b.yaw`).
    fn ledge(&self, b: &mut Beat, level: &Level) -> Option<Vec3> {
        let me = self.player().map(|p| p.fig.pos)?;
        let roofs: Vec<(Vec3, f32)> = level.ground.ladders.iter()
            .filter(|l| l.top - l.base > 2.0 * 256.0)
            .filter_map(|l| {
                let on = l.foot - l.out * 160.0;
                let y = level.ground.surface(&level.map, on.x, on.y, l.top + 64.0);
                let at = Vec3::new(on.x, y, on.y);
                let flat = crate::flat(at, me).length();
                ((y - l.top).abs() < 96.0 && (NOT_NEARER..b.radius).contains(&flat)).then(|| (at, l.out.x.atan2(l.out.y)))
            })
            .collect();
        // Not a roof one's on or one of the last few was on, while there's another.
        let fresh: Vec<(Vec3, f32)> = roofs.iter().copied().filter(|&(at, _)| b.fresh(at)).collect();
        let roofs = if fresh.is_empty() { roofs } else { fresh };
        let k = (b.rand() as usize) % roofs.len().max(1);
        let (at, yaw) = *roofs.get(k)?;
        b.yaw = yaw;
        Some(at)
    }

    /// A person for an incident, at `at` (a little round it by `n`), with a point number of
    /// their own.
    fn incident_person(&mut self, b: &mut Beat, level: &Level, kind: u8, at: Vec3, n: usize) -> Option<usize> {
        let a = n as f32 * 2.1 + (b.rand() % 100) as f32 / 50.0;
        let pos = at + Vec3::new(a.cos(), 0.0, a.sin()) * (60.0 + 50.0 * n as f32);
        let y = level.ground.surface(&level.map, pos.x, pos.z, at.y + 64.0);
        let way = b.next_way;
        b.next_way += 1;
        let i = self.spawn(level, kind, false, way, Vec3::new(pos.x, y, pos.z), a)?;
        self.tune(i, self.difficulty().skill_change());
        Some(i)
    }

    /// Makes incident `kind`, somewhere on the beat.
    fn call_in(&mut self, b: &mut Beat, kind: Kind, level: &Level) -> Option<Incident> {
        // A dumped gun goes in a bin or skip on the beat; with none, in the prop nearest
        // where it was dumped, or lying there.
        let mut bin = None;
        let at = match kind {
            Kind::DrunkDriver | Kind::Getaway => self.road_spot(b, level)?,
            Kind::Jumper | Kind::Gunman => self.ledge(b, level)?,
            Kind::DumpedWeapon => match self.bin_spot(b, level) {
                Some((ob, at)) => {
                    bin = Some(ob);
                    let y = level.ground.surface(&level.map, at.x, at.z, at.y + 128.0);
                    Vec3::new(at.x, y, at.z)
                }
                None => self.incident_spot(b)?,
            },
            _ => self.incident_spot(b)?,
        };
        let thug = |b: &mut Beat| [ptype::THUG_GREY, ptype::THUG_RED, ptype::THUG_RASTA][(b.rand() % 3) as usize];
        let mut inc = Incident {
            kind, at, left: kind.time(), culprits: Vec::new(), victims: Vec::new(), item: None, jumper: None, patient: None, bomb: None, with: 0.0, arrests: 0,
        };
        match kind {
            Kind::Mugging => {
                let victim = self.incident_person(b, level, ptype::CIV, at, 0)?;
                let k = thug(b);
                let mugger = self.incident_person(b, level, k, at, 1)?;
                // An assassin of the victim (`PCOM_AI_ASSASIN`, its mark by point number).
                let mark = self.people[victim].way;
                let p = &mut self.people[mugger];
                // An assassin who goes after his mark, not one who stands and snipes
                // (`PCOM_MOVE_FOLLOW`, as the missions' assassins are).
                (p.ai, p.other, p.guilty, p.mv) = (3, mark, true, person::MOVE_FOLLOW);
                self.people[mugger].go_for();
                self.people[mugger].ai = 3;
                (inc.culprits, inc.victims) = (vec![mugger], vec![victim]);
            }
            Kind::ArmedSuspect => {
                let k = thug(b);
                let s = self.incident_person(b, level, k, at, 0)?;
                let p = &mut self.people[s];
                (p.ai, p.mv, p.guilty) = (person::AI_CIV, person::MOVE_WANDER, true);
                p.weapon = Weapon::Pistol;
                p.give(Weapon::Pistol);
                inc.culprits = vec![s];
            }
            Kind::DumpedWeapon => {
                let way = b.next_way;
                b.next_way += 1;
                let item = Item { way, kind: 2, pos: at, rounds: None, born: self.time };
                match bin.or_else(|| find_ob(level, at, 3.0 * 256.0, None)) {
                    Some(ob) => {
                        self.searchable.insert(ob);
                        self.hidden.push((ob, item));
                    }
                    None => self.items.push(item),
                }
                inc.item = Some(way);
            }
            Kind::GangFight => {
                let (ka, kb) = (ptype::THUG_GREY, ptype::THUG_RED);
                let mut side_a = Vec::new();
                let mut side_b = Vec::new();
                for n in 0..2 {
                    side_a.push(self.incident_person(b, level, ka, at, n * 2)?);
                    side_b.push(self.incident_person(b, level, kb, at, n * 2 + 1)?);
                }
                // Each goes for one of the other lot.
                for (&a, &o) in side_a.iter().zip(&side_b) {
                    let (wa, wo) = (self.people[a].way, self.people[o].way);
                    for (i, mark) in [(a, wo), (o, wa)] {
                        let p = &mut self.people[i];
                        (p.ai, p.other, p.guilty, p.mv) = (3, mark, true, person::MOVE_FOLLOW);
                        p.go_for();
                        p.ai = 3;
                    }
                }
                inc.culprits = side_a.into_iter().chain(side_b).collect();
            }
            Kind::WantedMan => {
                let s = self.incident_person(b, level, ptype::CIV, at, 0)?;
                let p = &mut self.people[s];
                (p.ai, p.mv, p.guilty) = (person::AI_CIV, person::MOVE_WANDER, true);
                inc.culprits = vec![s];
            }
            Kind::DrunkDriver | Kind::Getaway => {
                let drunk = kind == Kind::DrunkDriver;
                let who = if drunk { ptype::CIV } else { thug(b) };
                let way = b.next_way;
                b.next_way += 1;
                let i = self.spawn(level, who, false, way, at, 0.0)?;
                self.tune(i, self.difficulty().skill_change());
                // Anything off the street for a drunk; a sedan for a getaway.
                let car = if drunk { city::VEHICLE_RANDOM[(b.rand() % 16) as usize] } else { 7 };
                self.drive_off(i, car, at, b.yaw, Driving { drunk, reckless: !drunk, ..Default::default() });
                let p = &mut self.people[i];
                p.guilty = true;
                if !drunk {
                    p.give(Weapon::Pistol);
                }
                inc.culprits = vec![i];
            }
            Kind::Jumper => {
                let way = b.next_way;
                b.next_way += 1;
                let i = self.spawn(level, ptype::CIV, false, way, at, b.yaw)?;
                // Stock still on the ledge, and deaf to everything (`PCOM_BENT_ROBOT`: no
                // running off from gunfire).
                let p = &mut self.people[i];
                (p.ai, p.mv, p.bent) = (person::AI_CIV, 1, person::BENT_ROBOT);
                inc.jumper = Some(i);
            }
            Kind::Bomb => {
                let way = b.next_way;
                b.next_way += 1;
                self.mines.push(game::mines::Mine { way, pos: at, fuse: Some(kind.time()), gone: false });
                inc.bomb = Some(self.mines.len() - 1);
            }
            Kind::BagSnatch => {
                // The one robbed stands where it happened; the thief's off already.
                let victim = self.incident_person(b, level, ptype::CIV, at, 0)?;
                self.people[victim].mv = 1;
                let k = thug(b);
                let thief = self.incident_person(b, level, k, at, 1)?;
                let p = &mut self.people[thief];
                (p.ai, p.mv, p.guilty) = (person::AI_CIV, person::MOVE_WANDER, true);
                p.scare(None, at);
                inc.culprits = vec![thief];
            }
            Kind::DrugDeal => {
                let k = thug(b);
                let dealer = self.incident_person(b, level, k, at, 0)?;
                let buyer = self.incident_person(b, level, ptype::CIV, at, 1)?;
                // Standing together, face to face.
                for (i, o) in [(dealer, buyer), (buyer, dealer)] {
                    let to = self.people[o].fig.pos;
                    let p = &mut self.people[i];
                    (p.ai, p.mv, p.guilty) = (person::AI_CIV, 1, true);
                    p.face_towards(to);
                }
                inc.culprits = vec![dealer, buyer];
            }
            Kind::Troublemaker => {
                let k = thug(b);
                let s = self.incident_person(b, level, k, at, 0)?;
                let p = &mut self.people[s];
                (p.ai, p.mv, p.guilty) = (person::AI_BULLY, person::MOVE_WANDER, true);
                inc.culprits = vec![s];
            }
            Kind::ManDown => {
                // Lies hurt (`PCOM_AI_HYPOCHONDRIA`), and nobody's to blame.
                let s = self.incident_person(b, level, ptype::CIV, at, 0)?;
                self.people[s].ai = person::AI_INJURED;
                inc.patient = Some(s);
            }
            Kind::Gunman => {
                let way = b.next_way;
                b.next_way += 1;
                let k = thug(b);
                let i = self.spawn(level, k, false, way, at, b.yaw)?;
                self.tune(i, self.difficulty().skill_change());
                // Up on the ledge with a gun, till she comes near.
                let p = &mut self.people[i];
                (p.ai, p.mv, p.guilty) = (person::AI_CIV, 1, true);
                p.weapon = Weapon::Pistol;
                p.give(Weapon::Pistol);
                inc.culprits = vec![i];
            }
        }
        Some(inc)
    }

    /// The shift's beacons: one on each incident (on its culprit, while there's one standing).
    pub(crate) fn beat_beacons(&self) -> Vec<(String, Vec3)> {
        let Some(b) = &self.beat else { return Vec::new() };
        b.incidents.iter().map(|inc| {
            let on = inc.culprits.iter().chain(&inc.jumper).map(|&i| &self.people[i]).find(|p| !p.removed && !p.is_down()).map(|p| p.fig.pos)
                .or_else(|| inc.patient.map(|i| self.people[i].fig.pos));
            let doing = match inc.kind {
                Kind::Jumper if inc.with > 0.0 => " talking",
                Kind::ManDown if inc.with > 0.0 => " first aid",
                Kind::Bomb if inc.with > 0.0 => " defusing",
                _ => "",
            };
            (format!("{} ({:.0}s){doing}", inc.kind.label(), inc.left.max(0.0)), on.unwrap_or(inc.at))
        }).collect()
    }
}
