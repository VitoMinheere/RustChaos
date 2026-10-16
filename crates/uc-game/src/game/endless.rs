//! Endless mode, "Riot" (#129): no mission script, just waves of thugs on the map, each
//! bigger and better armed than the last, until the player goes down or the city falls (the
//! crime rate reaches 100). Every wave comes from the run's own seed, so the same seed gives
//! the same waves.

use crate::*;
use uc_world::character::person as ptype;

/// Seconds between waves.
pub const BREATHER: f32 = 15.0;
/// No more than this many of a wave about at once; the rest come as others go down.
const AT_ONCE: usize = 16;
/// Spawned no nearer the player than this (world units), and out of their sight if it can be.
const NOT_NEARER: f32 = 6.0 * 256.0;
/// Every tenth wave has a boss.
const BOSS_EVERY: u32 = 10;

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
}

/// An endless run.
#[derive(Debug, Clone)]
pub struct Endless {
    pub seed: u32,
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
    radius: f32,
    /// Still to come in this wave.
    to_come: u32,
    foes: Vec<Foe>,
    /// The red marks already charged for.
    red_marks: u8,
    /// Seconds towards the crime rate's next rise.
    crime_clock: f32,
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
}

/// What wave `n` sends: how many, and how tough.
pub fn wave_size(n: u32) -> u32 {
    3 + n * 3 / 2
}

impl Game {
    /// Starts an endless run on `level` with `seed`, the arena `radius` squares round the
    /// player's start: no mission script, the city's crowd about as usual, the crime rate on the HUD.
    pub fn new_endless(level: &Level, seed: u32, radius: f32) -> Game {
        let mut g = Game::new(level, false);
        let centre = g.player().map_or(Vec3::ZERO, |p| p.fig.pos);
        g.show_crime_rate = true;
        g.crime_rate = 20;
        g.endless = Some(Endless {
            seed, wave: 0, phase: Phase::Breather(5.0), score: 0, arrests: 0, downed: 0,
            rng: seed | 1, centre, radius: radius * 256.0, to_come: 0, foes: Vec::new(), red_marks: 0, crime_clock: 0.0,
        });
        g
    }

    /// The endless run, if this is one.
    pub fn endless(&self) -> Option<&Endless> {
        self.endless.as_ref()
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

    fn endless_turn(&mut self, e: &mut Endless, dt: f32, level: &Level) {
        // Innocents hurt: a red mark costs 200.
        if self.red_marks > e.red_marks {
            e.score -= 200 * (self.red_marks - e.red_marks) as i64;
            e.red_marks = self.red_marks;
        }
        match e.phase {
            Phase::Breather(t) => {
                let t = t - dt;
                if t > 0.0 {
                    e.phase = Phase::Breather(t);
                    return;
                }
                e.wave += 1;
                e.to_come = wave_size(e.wave);
                e.foes.clear();
                e.phase = Phase::Wave;
                let at = self.player().map_or(e.centre, |p| p.fig.pos);
                self.messages.push(Message { text: format!("Wave {}", e.wave), left: 3.0, face: None, filmed: false });
                self.sounds.push((Sfx::Alert { kind: ptype::THUG_GREY }, at));
            }
            Phase::Wave => {}
        }
        // Count those who've gone down: an arrest is worth more than a knockout.
        let mult = e.wave as i64;
        for f in e.foes.iter_mut().filter(|f| !f.counted) {
            let p = &self.people[f.person];
            if p.arrested {
                f.counted = true;
                e.arrests += 1;
                e.score += 150 * mult;
            } else if p.knocked_out || p.is_dead() || p.removed {
                f.counted = true;
                e.downed += 1;
                e.score += 100 * mult;
            }
            if f.counted {
                self.crime_rate = self.crime_rate.saturating_sub(2);
            }
        }
        // The rest keep coming for the player.
        for f in e.foes.iter().filter(|f| !f.counted) {
            let p = &mut self.people[f.person];
            if !p.aggro() && !p.is_down() {
                p.go_for();
            }
        }
        // More of the wave, as there's room.
        let about = e.foes.iter().filter(|f| !f.counted).count();
        for _ in about..AT_ONCE {
            if e.to_come == 0 {
                break;
            }
            let boss = e.wave.is_multiple_of(BOSS_EVERY) && e.to_come == wave_size(e.wave);
            if !self.spawn_foe(e, level, boss) {
                break;
            }
            e.to_come -= 1;
        }
        // The crime rate rises a point a second for every four standing; at 100 the city's lost.
        let standing = e.foes.iter().filter(|f| !f.counted).count() as f32;
        e.crime_clock += dt * standing / 4.0;
        while e.crime_clock >= 1.0 {
            e.crime_clock -= 1.0;
            self.crime_rate = (self.crime_rate + 1).min(100);
        }
        if self.crime_rate >= 100 {
            self.finish(Outcome::Lost("The Wildcats have taken the streets".into()));
            return;
        }
        // All down: a bonus, and a breather.
        if e.to_come == 0 && e.foes.iter().all(|f| f.counted) {
            e.score += 250 * mult;
            e.phase = Phase::Breather(BREATHER);
            let at = self.player().map_or(e.centre, |p| p.fig.pos);
            self.messages.push(Message { text: format!("Wave {} cleared", e.wave), left: 3.0, face: None, filmed: false });
            self.sounds.push((Sfx::Found, at));
        }
    }

    /// Makes one of the wave somewhere out of the player's way. Whether there was room.
    fn spawn_foe(&mut self, e: &mut Endless, level: &Level, boss: bool) -> bool {
        let Some(pos) = self.spawn_spot(e, level) else { return false };
        let n = e.wave;
        let kind = if boss || (n >= 15 && e.rand().is_multiple_of(4)) {
            [ptype::MIB1, ptype::MIB2, ptype::MIB3][(e.rand() % 3) as usize]
        } else {
            [ptype::THUG_GREY, ptype::THUG_RED, ptype::THUG_RASTA][(e.rand() % 3) as usize]
        };
        // Fists first, then bats and knives, then pistols, shotguns and AKs.
        let arms: &[Weapon] = match n {
            0..=3 => &[Weapon::None],
            4..=5 => &[Weapon::None, Weapon::Bat, Weapon::Knife],
            6..=8 => &[Weapon::None, Weapon::Bat, Weapon::Knife, Weapon::Pistol],
            9..=11 => &[Weapon::Bat, Weapon::Knife, Weapon::Pistol, Weapon::Pistol, Weapon::Shotgun],
            _ => &[Weapon::Knife, Weapon::Pistol, Weapon::Shotgun, Weapon::Shotgun, Weapon::Ak47],
        };
        let weapon = if boss { Weapon::Ak47 } else { arms[(e.rand() as usize) % arms.len()] };
        // Something to find on them: health or ammo, less often as the waves go on.
        let find = e.rand() % 100;
        let health_chance = 30u32.saturating_sub(n);
        let drop = if find < health_chance {
            Some(3)
        } else if find < health_chance + 25 {
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
        p.skill = if boss { 15 } else { (2 + n / 2).min(15) as u8 };
        p.fig.jacket = p.skill / 4;
        p.weapon = weapon;
        p.give(weapon);
        p.drop = drop;
        if boss {
            p.max_health *= 4;
            p.health = p.max_health;
        }
        p.go_for();
        e.foes.push(Foe { person: i, counted: false });
        true
    }

    /// A spot for the next of the wave: on the ground in the arena, no nearer the player
    /// than [`NOT_NEARER`] and out of their sight if any such spot turns up, preferring the
    /// pavements the crowd walks (where they're sure to be on the street).
    fn spawn_spot(&mut self, e: &mut Endless, level: &Level) -> Option<Vec3> {
        let me = self.player().map(|p| p.fig.pos)?;
        let eye = me + Vec3::new(0.0, 120.0, 0.0);
        let mut seen = None;
        for k in 0..60 {
            let a = (e.rand() % 2048) as f32 * std::f32::consts::TAU / 2048.0;
            let r = e.radius * (0.3 + 0.7 * (e.rand() % 1000) as f32 / 1000.0);
            let (x, z) = (e.centre.x + r * a.cos(), e.centre.z + r * a.sin());
            let size = (uc_formats::iam::MAP_SIZE * 256) as f32;
            if !(256.0..size - 256.0).contains(&x) || !(256.0..size - 256.0).contains(&z) {
                continue;
            }
            let (sx, sz) = ((x / 256.0) as i32, (z / 256.0) as i32);
            if k < 40 && self.streets.wand.any() && !self.streets.wand.is_wander(sx, sz) {
                continue;
            }
            let y = level.ground.surface(&level.map, x, z, me.y + 256.0);
            // On the player's level, give or take a storey, so they're reachable.
            if (y - me.y).abs() > 300.0 {
                continue;
            }
            let at = Vec3::new(x, y, z);
            if Vec2::new(x - me.x, z - me.z).length() < NOT_NEARER {
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
