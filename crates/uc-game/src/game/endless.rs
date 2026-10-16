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
        let centre = g.player().map_or(Vec3::ZERO, |p| p.fig.pos);
        let grid = arena(level, centre, radius * 256.0);
        let spots = grid.values().map(|(at, _)| *at).collect();
        g.show_crime_rate = true;
        g.crime_rate = 20;
        g.endless = Some(Endless {
            seed, wave: 0, phase: Phase::Breather(5.0), score: 0, arrests: 0, downed: 0,
            rng: seed | 1, centre, spots, grid, to_come: 0, foes: Vec::new(), red_marks: 0, crime_clock: 0.0,
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
        let health_chance = (30 - n as i32 + self.difficulty().finds()).max(0) as u32;
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
        let change = self.difficulty().skill_change();
        self.tune(i, change);
        e.foes.push(Foe { person: i, counted: false });
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
        for _ in 0..60 {
            if e.spots.is_empty() {
                return None;
            }
            let k = (e.rand() as usize) % e.spots.len();
            let at = e.spots[k];
            if flat_dist(at, me) < not_nearer {
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
/// middles at waist height, as the walker finds them. So everyone made there can get to the
/// player, and nobody is made outside the playable area or on a roof.
pub fn arena(level: &Level, start: Vec3, radius: f32) -> Grid {
    let n = uc_formats::iam::MAP_SIZE as i32;
    let sq = |v: Vec3| ((v.x / 256.0) as i32, (v.z / 256.0) as i32);
    let middle = |x: i32, z: i32, from: f32| {
        let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
        Vec3::new(cx, level.ground.surface(&level.map, cx, cz, from + uc_world::ground::STEP_HEIGHT), cz)
    };
    let waist = Vec3::new(0.0, uc_world::ground::STEP_HEIGHT - 4.0, 0.0);
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

/// An arena: each square's middle, and the squares it leads to.
pub type Grid = std::collections::BTreeMap<(i32, i32), (Vec3, Vec<(i32, i32)>)>;

fn square(v: Vec3) -> (i32, i32) {
    ((v.x / 256.0) as i32, (v.z / 256.0) as i32)
}

/// For every square of the arena, the next one on the way to `to`'s square (a search out
/// from it), or None if `to` isn't in the arena.
fn towards(grid: &Grid, to: Vec3) -> Option<std::collections::BTreeMap<(i32, i32), (i32, i32)>> {
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
fn next_step(grid: &Grid, ways: &std::collections::BTreeMap<(i32, i32), (i32, i32)>, from: Vec3, me: Vec3, level: &Level) -> Option<Vec3> {
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
