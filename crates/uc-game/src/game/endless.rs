//! Endless mode, "Riot" (#129): no mission script, just waves of thugs on the map, each
//! bigger and better armed than the last, until the player goes down or the city falls (the
//! crime rate reaches 100, which it only climbs towards while the player keeps out of the
//! fight). Each wave comes from one side of the arena, a different one from the last, marked
//! with a beacon; every tenth brings a boss. Every wave comes from the run's own seed, so the
//! same seed gives the same waves.

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
        g.endless = Some(Endless {
            seed, wave: 0, phase: Phase::Breather(5.0), score: 0, arrests: 0, downed: 0,
            rng: seed | 1, centre, spots, grid, to_come: 0, foes: Vec::new(), red_marks: 0, crime_clock: 0.0,
            idle: 0.0, wave_health: 0, side: SIDES, side_beacon: 0.0, boss_due: false,
        });
        g
    }

    /// The endless run, if this is one.
    pub fn endless(&self) -> Option<&Endless> {
        self.endless.as_ref()
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

    fn endless_turn(&mut self, e: &mut Endless, dt: f32, level: &Level) {
        // Innocents hurt: a red mark costs 200.
        if self.red_marks > e.red_marks {
            e.score -= 200 * (self.red_marks - e.red_marks) as i64;
            e.red_marks = self.red_marks;
        }
        match e.phase {
            Phase::Breather(t) => {
                // Three seconds before the next: the radio, and dispatch says what's coming.
                let soon = 3.0;
                if t > soon && t - dt <= soon {
                    let next = e.wave + 1;
                    let text = if next.is_multiple_of(BOSS_EVERY) {
                        "Despatch: a big one's leading the next lot. Watch yourself".to_string()
                    } else {
                        format!("Despatch: wave {next} on its way")
                    };
                    self.messages.push(Message { text, left: 3.0, face: Some(talk::face::RADIO), filmed: false });
                    self.sounds.push((Sfx::Radio, e.centre));
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
            if !p.aggro() && !p.is_down() {
                p.go_for();
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
                self.messages.push(Message { text: "The streets need you: get stuck in".into(), left: 3.0, face: None, filmed: false });
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
            self.finish(Outcome::Lost("The Wildcats have taken the streets".into()));
            return;
        }
        // All down: a bonus, and a breather.
        if e.to_come == 0 && e.foes.iter().all(|f| f.counted) {
            e.score += 250 * mult;
            e.phase = Phase::Breather(BREATHER);
            let at = self.player().map_or(e.centre, |p| p.fig.pos);
            self.messages.push(Message { text: format!("Wave {} cleared", e.wave), left: 3.0, face: Some(talk::face::RADIO), filmed: false });
            self.sounds.push((Sfx::Radio, at));
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
        } else if n >= 15 && e.rand().is_multiple_of(4) {
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
        let mut weapon = arms[(e.rand() as usize) % arms.len()];
        // No more guns about at once than one, and another every four waves.
        let guns = e.foes.iter().filter(|f| !f.counted && self.people[f.person].weapon.is_gun()).count() as u32;
        if weapon.is_gun() && guns > n / 4 {
            weapon = if n >= 11 { Weapon::Bat } else { Weapon::Knife };
        }
        if boss {
            weapon = if n >= 20 { Weapon::Ak47 } else { Weapon::Bat };
        }
        // Something to find on them: health or ammo, less often as the waves go on, but
        // always some chance of health.
        let find = e.rand() % 100;
        let health_chance = (30 - n as i32 + self.difficulty().finds()).max(15) as u32;
        let drop = if find < health_chance {
            Some(3)
        } else if find < health_chance + (25 + self.difficulty().finds()).max(0) as u32 {
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
        if boss {
            p.max_health *= 4;
            p.health = p.max_health;
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
            if !level.ground.line_of_sight(here + waist, next + waist) {
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
