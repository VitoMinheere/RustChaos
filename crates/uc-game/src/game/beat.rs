//! Beat Cop (#130): a shift on a district with no mission script. Dispatch calls in
//! incidents one or two at a time (a mugging, an armed suspect, a dumped weapon, a gang
//! fight, a wanted man running), each with a beacon and a time limit; handling them scores
//! and brings the crime rate down, missing them puts it up. The shift ends when its time's
//! up (a win; one of [`SHIFTS`], or open-ended), or when the crime rate reaches 100 or the
//! player goes down. Everything comes from the shift's seed.

use crate::*;
use uc_world::character::person as ptype;

/// The shift lengths on offer, in minutes; `None` is open-ended, until the player drops.
pub const SHIFTS: [Option<u32>; 5] = [Some(5), Some(10), Some(15), Some(20), None];
/// The shift length a new player gets.
pub const DEFAULT_SHIFT: Option<u32> = Some(10);

/// A shift length as the menu and HUD say it: "10 MIN", or "OPEN-ENDED".
pub fn shift_name(minutes: Option<u32>) -> String {
    minutes.map_or("OPEN-ENDED".to_string(), |m| format!("{m} MIN"))
}
/// How far round the start incidents happen, in squares, and no nearer the player than
/// this when they're called in.
const BEAT: f32 = 20.0;
const NOT_NEARER: f32 = 8.0 * 256.0;
/// No more than this many incidents at once.
const AT_ONCE: usize = 2;
/// The event point numbers given to the people incidents make, so the AI's "go for the
/// person point N made" can point at them; well above any mission's.
const FIRST_WAY: usize = 50_000;

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
}

impl Kind {
    const ALL: [Kind; 5] = [Kind::Mugging, Kind::ArmedSuspect, Kind::DumpedWeapon, Kind::GangFight, Kind::WantedMan];

    /// The beacon's label.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Mugging => "Mugging",
            Kind::ArmedSuspect => "Armed suspect",
            Kind::DumpedWeapon => "Dumped weapon",
            Kind::GangFight => "Gang fight",
            Kind::WantedMan => "Wanted man",
        }
    }

    /// What dispatch says.
    fn call(self) -> &'static str {
        match self {
            Kind::Mugging => "Report of a mugging in progress. Get over there before somebody gets hurt!",
            Kind::ArmedSuspect => "We got an armed man walking the streets. Disarm him and bring him in.",
            Kind::DumpedWeapon => "A citizen saw somebody dump a gun in one of the bins round there. We need it as evidence: search them.",
            Kind::GangFight => "Wildcats fighting it out in the street. Break it up!",
            Kind::WantedMan => "A wanted man's been spotted. He'll run if he sees you coming.",
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
    /// Arrests among the culprits, counted as they're made.
    arrests: u32,
}

/// How a shift is going.
#[derive(Debug, Clone)]
pub struct Beat {
    pub seed: u32,
    /// Seconds left of the shift; `None` when it's open-ended.
    pub shift: Option<f32>,
    /// Seconds on duty so far.
    pub on: f32,
    pub score: i64,
    pub handled: u32,
    pub missed: u32,
    pub incidents: Vec<Incident>,
    rng: u32,
    /// Seconds to the next call.
    next: f32,
    next_way: usize,
    /// Where incidents can happen: the beat's squares, walkable from the start.
    spots: Vec<Vec3>,
    red_marks: u8,
}

impl Beat {
    fn rand(&mut self) -> u32 {
        rand(&mut self.rng)
    }
}

/// The end of a shift: what the summary says.
pub fn summary(b: &Beat) -> String {
    format!("Shift over. {} incidents handled, {} missed. Score {}", b.handled, b.missed, b.score)
}

impl Game {
    /// Starts a shift of `minutes` (open-ended if `None`) on `level` with `seed`, round
    /// `centre` (world xz; the start if none): no mission script, the city's crowd and
    /// traffic as usual, the crime rate on the HUD, the first call a few seconds in.
    pub fn new_beat(level: &Level, seed: u32, centre: Option<Vec2>, minutes: Option<u32>) -> Game {
        let mut g = Game::new(level, false);
        if let Some(c) = centre {
            let y = level.ground.surface(&level.map, c.x, c.y, level.ground.surface(&level.map, c.x, c.y, 0.0) + 128.0);
            g.teleport(Vec3::new(c.x, y, c.y), level);
        }
        let start = g.player().map_or(Vec3::ZERO, |p| p.fig.pos);
        let spots = game::endless::arena(level, start, BEAT * 256.0).values().map(|(at, _)| *at).collect();
        g.show_crime_rate = true;
        g.crime_rate = 40;
        g.beat = Some(Beat {
            seed, shift: minutes.map(|m| m as f32 * 60.0), on: 0.0, score: 0, handled: 0, missed: 0, incidents: Vec::new(), rng: seed | 1,
            next: 4.0, next_way: FIRST_WAY, spots, red_marks: 0,
        });
        g
    }

    /// The shift, if this is one.
    pub fn beat(&self) -> Option<&Beat> {
        self.beat.as_ref()
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
            self.messages.push(Message { text: summary(b), left: 10.0, face: Some(talk::face::RADIO), filmed: false });
            self.finish(Outcome::Won);
            return;
        }
        // The incidents on: handled, missed or still going.
        let mut k = 0;
        while k < b.incidents.len() {
            let inc = &mut b.incidents[k];
            inc.left -= dt;
            match self.settle(inc) {
                Some(true) => {
                    let inc = b.incidents.remove(k);
                    let bonus = 50 * inc.arrests as i64;
                    b.score += inc.kind.points() + bonus;
                    b.handled += 1;
                    self.crime_rate = self.crime_rate.saturating_sub(5);
                    self.say(&format!("{} dealt with. Good work.", inc.kind.label()));
                    self.sounds.push((Sfx::Found, inc.at));
                    self.clear_up(&inc);
                }
                Some(false) => {
                    let inc = b.incidents.remove(k);
                    b.score -= 50;
                    b.missed += 1;
                    self.crime_rate = (self.crime_rate + 8).min(100);
                    self.say(&format!("Too late for the {}.", inc.kind.label().to_ascii_lowercase()));
                    self.clear_up(&inc);
                }
                None => k += 1,
            }
        }
        if self.crime_rate >= 100 {
            self.finish(Outcome::Lost("The city's out of control".into()));
            return;
        }
        // The next call, when there's room for one.
        b.next -= dt;
        if b.next <= 0.0 && b.incidents.len() < AT_ONCE {
            let kind = Kind::ALL[(b.rand() as usize) % Kind::ALL.len()];
            if let Some(inc) = self.call_in(b, kind, level) {
                self.say(kind.call());
                b.incidents.push(inc);
            }
            b.next = 25.0 + (b.rand() % 25) as f32;
        }
    }

    /// A message from dispatch.
    fn say(&mut self, text: &str) {
        self.messages.push(Message { text: format!("Despatch: {text}"), left: 6.0, face: Some(talk::face::RADIO), filmed: false });
    }

    /// Whether incident `inc` is handled (true), missed (false) or still going (None).
    fn settle(&mut self, inc: &mut Incident) -> Option<bool> {
        let down = |p: &Person| p.arrested || p.knocked_out || p.is_dead() || p.removed;
        inc.arrests = inc.culprits.iter().filter(|&&i| self.people[i].arrested).count() as u32;
        let got = !inc.culprits.is_empty() && inc.culprits.iter().all(|&i| down(&self.people[i]));
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
        // A wanted man runs from a cop he sees within eight squares.
        if inc.kind == Kind::WantedMan {
            if let Some(me) = self.player().map(|p| p.fig.pos) {
                for &i in &inc.culprits {
                    let p = &mut self.people[i];
                    if !down(p) && p.scared.is_none() && (p.fig.pos - me).length() < 8.0 * 256.0 {
                        p.scare(self.player, me);
                    }
                }
            }
        }
        // An armed suspect turns on her when she's near.
        if inc.kind == Kind::ArmedSuspect {
            if let Some(me) = self.player().map(|p| p.fig.pos) {
                for &i in &inc.culprits {
                    let p = &mut self.people[i];
                    if !down(p) && !p.aggro() && (p.fig.pos - me).length() < 6.0 * 256.0 {
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
    }

    /// Somewhere on the beat for the next incident: away from the player, but reachable.
    fn incident_spot(&self, b: &mut Beat) -> Option<Vec3> {
        let me = self.player().map(|p| p.fig.pos)?;
        for _ in 0..60 {
            if b.spots.is_empty() {
                return None;
            }
            let k = (b.rand() as usize) % b.spots.len();
            let at = b.spots[k];
            if Vec2::new(at.x - me.x, at.z - me.z).length() >= NOT_NEARER {
                return Some(at);
            }
        }
        None
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
        let at = self.incident_spot(b)?;
        let thug = |b: &mut Beat| [ptype::THUG_GREY, ptype::THUG_RED, ptype::THUG_RASTA][(b.rand() % 3) as usize];
        let mut inc = Incident { kind, at, left: kind.time(), culprits: Vec::new(), victims: Vec::new(), item: None, arrests: 0 };
        match kind {
            Kind::Mugging => {
                let victim = self.incident_person(b, level, ptype::CIV, at, 0)?;
                let k = thug(b);
                let mugger = self.incident_person(b, level, k, at, 1)?;
                // An assassin of the victim (`PCOM_AI_ASSASIN`, its mark by point number).
                let mark = self.people[victim].way;
                let p = &mut self.people[mugger];
                (p.ai, p.other, p.guilty) = (3, mark, true);
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
                let item = Item { way, kind: 2, pos: at };
                // In the nearest bin or skip within a few squares, if there is one; else
                // lying where it was dumped.
                match find_ob(level, at, 3.0 * 256.0, None) {
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
                        (p.ai, p.other, p.guilty) = (3, mark, true);
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
        }
        Some(inc)
    }

    /// The shift's beacons: one on each incident (on its culprit, while there's one standing).
    pub(crate) fn beat_beacons(&self) -> Vec<(String, Vec3)> {
        let Some(b) = &self.beat else { return Vec::new() };
        b.incidents.iter().map(|inc| {
            let on = inc.culprits.iter().map(|&i| &self.people[i]).find(|p| !p.removed && !p.is_down()).map(|p| p.fig.pos);
            (format!("{} ({:.0}s)", inc.kind.label(), inc.left.max(0.0)), on.unwrap_or(inc.at))
        }).collect()
    }
}
