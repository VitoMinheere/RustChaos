//! The dirt (fallen/Source/dirt.cpp): leaves, cans and snow lying about the streets, and
//! droplets, the stream from someone a mission sets peeing (`make_person_pee`).
//!
//! `DIRT_new_water` (fallen/Source/dirt.cpp) throws one drop a tick from just under the
//! pelvis, forwards along the person's facing and a little down, each nudged at random; then
//! gravity takes it until it reaches the ground. `DIRT_TYPE_URINE` is the same as water but
//! for its colour. The original's speeds are in sixteenths of a unit a tick at 20 ticks a
//! second; here they are in units a second.

use uc_formats::units::radians;
use crate::rand;
use glam::{vec3, Vec2, Vec3};

/// Ticks a second the original's dirt moves at.
const TICKS: f32 = 20.0;
/// `dx = -SIN(angle) >> 13 << 4`: 128 sixteenths a tick along the facing.
const FORWARD: f32 = 128.0 / 16.0 * TICKS;
/// `dy = -2 << 4`: 32 sixteenths a tick down.
const DOWN: f32 = 32.0 / 16.0 * TICKS;
/// The random nudge, up to 15 sixteenths a tick either way.
const NUDGE: f32 = 15.0 / 16.0 * TICKS;
/// Gravity: 15 sixteenths a tick, every tick.
const GRAVITY: f32 = 15.0 / 16.0 * TICKS * TICKS;
/// Falling no faster than 511 sixteenths a tick.
const TERMINAL: f32 = 511.0 / 16.0 * TICKS;
/// Gone after this long even if it never lands (off an edge, down a hole).
const LIFE: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
pub struct Drop {
    pub pos: Vec3,
    pub vel: Vec3,
    age: f32,
    /// Water (`DIRT_TYPE_WATER`) rather than urine.
    pub water: bool,
}

/// Every drop in the air.
#[derive(Debug, Clone)]
pub struct Drops {
    pub drops: Vec<Drop>,
    /// Seconds to the next tick's drops.
    clock: f32,
    /// Its own chance, so the look of it can't change how the mission plays.
    rng: u32,
}

impl Default for Drops {
    fn default() -> Drops {
        Drops { drops: Vec::new(), clock: 0.0, rng: 0xd1b7 }
    }
}

impl Drops {
    /// A number in -1..1.
    fn nudge(&mut self) -> f32 {
        rand(&mut self.rng) as f32 / 32768.0 - 1.0
    }

    /// Throws one drop from `at` at `vel` (units a second).
    pub fn throw(&mut self, at: Vec3, vel: Vec3) {
        self.drops.push(Drop { pos: at, vel, age: 0.0, water: true });
    }

    /// Moves the drops on, letting go of those that reach the ground (`ground` says how high
    /// it is under a point), and throws a new one from each of `streams`: (where, which way
    /// the person faces), one per tick.
    pub fn update(&mut self, dt: f32, streams: &[(Vec3, Vec2)], ground: impl Fn(Vec3) -> f32) {
        for d in &mut self.drops {
            d.age += dt;
            d.vel.y = (d.vel.y - GRAVITY * dt).max(-TERMINAL);
            d.pos += d.vel * dt;
        }
        self.drops.retain(|d| d.age < LIFE && d.pos.y > ground(d.pos));
        self.clock -= dt;
        while self.clock <= 0.0 {
            self.clock += 1.0 / TICKS;
            for &(at, facing) in streams {
                let jitter = vec3(self.nudge(), self.nudge(), self.nudge()) * NUDGE;
                let vel = vec3(facing.x * FORWARD, -DOWN, facing.y * FORWARD) + jitter;
                self.drops.push(Drop { pos: at, vel, age: 0.0, water: false });
            }
        }
    }
}

// --- Leaves, cans and snow lying about ---------------------------------------------------
//
// `DIRT_init(100, 1, 0, ...)` as each level loads (elev.cpp:2356), `DIRT_set_focus` round
// Darci each turn (Controls.cpp:2273), `DIRT_process` and `DIRT_gust`. Everything here is in
// the original's own units and runs a turn at a time (20 a second), as the original does at
// its normal frame rate: positions in world units, a leaf's dx and dz in units a turn and its
// dy shifted up by `TICK_SHIFT`, a can's dx and dz shifted up by 8. Pigeons are off ("no bug
// ridden pigeons for us!").

/// `DIRT_MAX_DIRT` (PC).
pub const MAX_DIRT: usize = 1024;
/// `DIRT_MAX_TREES`.
const MAX_TREES: usize = 64;
/// `DIRT_LEAVES_PER_TREE` (PC).
const LEAVES_PER_TREE: usize = 200;
/// `TICK_SHIFT`.
const TICK_SHIFT: i32 = 8;
/// The focus radius on the PC: twelve squares.
pub const FOCUS_RADIUS: i32 = 0xc00;
/// `S_KICK_CAN`.
pub const KICK_CAN: usize = 76;

/// `DIRT_TYPE_*`, those this keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Leaf,
    Can,
    Snow,
    /// A can in someone's hand (`DIRT_TYPE_HELDCAN`): the game keeps it there.
    HeldCan,
    /// A can thrown (`DIRT_TYPE_THROWCAN`), flying till it settles and is a can again.
    ThrownCan,
}

/// A can clattering at `at`: guards hear it (`PCOM_SOUND_UNUSUAL`) when `alerts`, and `by`
/// is who kicked or threw it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clatter {
    pub at: (i32, i32, i32),
    pub alerts: bool,
    pub by: Option<usize>,
}

/// `world_type`: the forest has only leaves and the snow only snow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    City,
    Forest,
    Snow,
}

impl Ground {
    /// From the map's texture set (io.cpp:784): 5 is snow, 1 the forest.
    pub fn of(texture_set: Option<i32>) -> Ground {
        match texture_set {
            Some(5) => Ground::Snow,
            Some(1) => Ground::Forest,
            _ => Ground::City,
        }
    }
}

/// `DIRT_Dirt`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub kind: Kind,
    /// The tree whose leaf this is (`owner`, 255 for none), or the person holding a can or
    /// who threw it (`droll`).
    owner: Option<usize>,
    /// `DIRT_FLAG_STILL`: lying on the ground.
    pub still: bool,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    dx: i32,
    dy: i32,
    dz: i32,
    /// 2048 a turn.
    pub yaw: i32,
    pub pitch: i32,
    pub roll: i32,
    dyaw: i32,
    dpitch: i32,
    droll: i32,
    /// Snow fades as it lies (`UU.Leaf.fade`, 255 down).
    pub fade: i32,
}

impl Piece {
    fn new(kind: Kind, x: i32, y: i32, z: i32) -> Piece {
        Piece { kind, owner: None, still: true, x, y, z, dx: 0, dy: 0, dz: 0, yaw: 0, pitch: 0, roll: 0, dyaw: 0, dpitch: 0, droll: 0, fade: 255 }
    }
}

struct Tree {
    x: i32,
    z: i32,
    in_range: bool,
}

/// `QDIST2`.
fn qdist2(x: i32, z: i32) -> i32 {
    if x > z { x + (z >> 1) } else { z + (x >> 1) }
}

/// `SIN` and `COS`: 2048 to a turn, 65536 at full.
fn sin(a: i32) -> i32 {
    (radians((a & 2047) as f32).sin() * 65536.0) as i32
}

fn cos(a: i32) -> i32 {
    sin(a + 512)
}

/// What the dirt needs to know about the map.
pub trait Floor {
    /// `PAP_calc_height_at`: the ground's height (not the roofs').
    fn height(&self, x: i32, z: i32) -> i32;
    /// The square is on the map, not inside a building (`PAP_FLAG_HIDDEN`) and not water or
    /// next to it. `snow` lets it be inside a building.
    fn open(&self, mx: i32, mz: i32, snow: bool) -> bool;
    /// The square is under a building (`PAP_FLAG_HIDDEN`): a wall to the dirt.
    fn hidden(&self, mx: i32, mz: i32) -> bool;
}

impl Floor for uc_formats::iam::Map {
    fn height(&self, x: i32, z: i32) -> i32 {
        self.ground_height(x as f32, z as f32) as i32
    }

    fn open(&self, mx: i32, mz: i32, snow: bool) -> bool {
        use uc_formats::iam::{flags, MAP_SIZE};
        let on = |x: i32, z: i32| (0..MAP_SIZE as i32).contains(&x) && (0..MAP_SIZE as i32).contains(&z);
        let water = |x: i32, z: i32| on(x, z) && self.square(x as usize, z as usize).has(flags::WATER);
        on(mx, mz)
            && (snow || !self.hidden(mx, mz))
            && ![(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| water(mx + dx, mz + dz))
    }

    fn hidden(&self, mx: i32, mz: i32) -> bool {
        use uc_formats::iam::{flags, MAP_SIZE};
        (0..MAP_SIZE as i32).contains(&mx) && (0..MAP_SIZE as i32).contains(&mz) && self.square(mx as usize, mz as usize).has(flags::HIDDEN)
    }
}

/// All the dirt (`DIRT_dirt`), kept round a focus.
pub struct Dirt {
    /// `MAX_DIRT` slots, `None` for `DIRT_TYPE_UNUSED`.
    pub pieces: Vec<Option<Piece>>,
    trees: Vec<Tree>,
    ground: Ground,
    prob_leaf: i32,
    prob_can: i32,
    focus: (i32, i32, i32),
    /// `DIRT_focus_first`: the first few focusings fill the whole circle.
    first: i32,
    /// `DIRT_check`: an eighth of the dirt is checked a turn.
    check: usize,
    rng: u32,
    turn: u32,
}

impl Dirt {
    /// `DIRT_init(prob_leaf, prob_can, 0, ...)`, with the trees (`PRIM_FLAG_TREE` props) at
    /// `trees`.
    pub fn new(ground: Ground, prob_leaf: i32, prob_can: i32, trees: impl IntoIterator<Item = (i32, i32)>) -> Dirt {
        let sum = prob_leaf + prob_can;
        let (prob_leaf, prob_can) = if sum == 0 { (0, 0) } else { (prob_leaf * 256 / sum, prob_can * 256 / sum) };
        Dirt {
            pieces: vec![None; MAX_DIRT],
            trees: trees.into_iter().take(MAX_TREES).map(|(x, z)| Tree { x, z, in_range: false }).collect(),
            ground,
            prob_leaf,
            prob_can,
            focus: (0, 0, 0x100),
            first: 4,
            check: 0,
            rng: 0xd127,
            turn: 0,
        }
    }

    fn random(&mut self) -> i32 {
        crate::rand(&mut self.rng) as i32
    }

    /// `DIRT_get_new_type`.
    fn new_kind(&mut self) -> Option<Kind> {
        match self.ground {
            Ground::Forest => Some(Kind::Leaf),
            Ground::Snow => Some(Kind::Snow),
            Ground::City => {
                let mut choice = self.random() & 0xff;
                choice -= self.prob_leaf;
                if choice < 0 {
                    return Some(Kind::Leaf);
                }
                choice -= self.prob_can;
                (choice < 0).then_some(Kind::Can)
            }
        }
    }

    /// `DIRT_set_focus`: leaves under the trees in range, and anything too far off (or
    /// unused) put back somewhere round (x, z). `player_y` is where snow starts falling from.
    pub fn set_focus(&mut self, x: i32, z: i32, radius: i32, player_y: i32, floor: &impl Floor) {
        self.focus = (x, z, radius);
        for t in 0..self.trees.len() {
            let dist = qdist2((self.trees[t].x - x).abs(), (self.trees[t].z - z).abs());
            if dist > radius {
                if self.trees[t].in_range {
                    for p in &mut self.pieces {
                        if p.is_some_and(|p| p.kind == Kind::Leaf && p.owner == Some(t)) {
                            *p = None;
                        }
                    }
                    self.trees[t].in_range = false;
                }
            } else if !self.trees[t].in_range {
                let mut done = 0;
                let mut i = self.random() as usize % MAX_DIRT;
                for _ in 0..MAX_DIRT {
                    i = (i + 1) % MAX_DIRT;
                    if self.pieces[i].is_some() {
                        continue;
                    }
                    done += 1;
                    let mut dist = (self.random() & 0x7f) + 150;
                    dist = (dist * dist) >> 8;
                    let angle = self.random() & 2047;
                    let lx = self.trees[t].x + ((sin(angle) * dist) >> 16);
                    let lz = self.trees[t].z + ((cos(angle) * dist) >> 16);
                    let (mx, mz) = (lx >> 8, lz >> 8);
                    if (0..128).contains(&mx) && (0..128).contains(&mz) && !floor.hidden(mx, mz) {
                        let mut leaf = Piece::new(Kind::Leaf, lx, floor.height(lx, lz), lz);
                        leaf.owner = Some(t);
                        self.pieces[i] = Some(leaf);
                    }
                    if done >= LEAVES_PER_TREE {
                        break;
                    }
                }
                self.trees[t].in_range = true;
            }
        }
        let count = if self.first > 0 { MAX_DIRT } else { MAX_DIRT / 8 };
        for _ in 0..count {
            self.check = (self.check + 1) % MAX_DIRT;
            let i = self.check;
            // Leaves under trees come and go with their tree, and a can in a hand or in the
            // air stays with whoever has it.
            if self.pieces[i].is_some_and(|p| (p.kind == Kind::Leaf && p.owner.is_some()) || matches!(p.kind, Kind::HeldCan | Kind::ThrownCan)) {
                continue;
            }
            let far = self.pieces[i].is_none_or(|p| qdist2((p.x - x).abs(), (p.z - z).abs()) > radius);
            if !far {
                continue;
            }
            let angle = self.random() & 2047;
            let dist = if self.first > 0 {
                let d = (self.random() & 0xff) + (self.random() & 0xff) + (self.random() & 0xff) + (self.random() & 0xff);
                ((0x3af - d) * radius) >> 10
            } else {
                let mut d = radius - (self.random() & 0xff) - 0x80;
                if self.ground == Ground::Snow {
                    d -= 0xff + (self.random() & 0x1ff);
                }
                d.max(10)
            };
            // `MUL64`.
            let cx = x + ((sin(angle) as i64 * dist as i64) >> 16) as i32;
            let cz = z + ((cos(angle) as i64 * dist as i64) >> 16) as i32;
            if !floor.open(cx >> 8, cz >> 8, self.ground == Ground::Snow) {
                self.pieces[i] = None;
                continue;
            }
            self.pieces[i] = match self.new_kind() {
                None => None,
                Some(Kind::Can) => {
                    let mut can = Piece::new(Kind::Can, cx, floor.height(cx, cz) + 4, cz);
                    can.yaw = self.random() & 2047;
                    Some(can)
                }
                Some(kind) => {
                    let mut p = Piece::new(kind, cx, floor.height(cx, cz), cz);
                    if kind == Kind::Snow {
                        // Some of it falls from up above Darci, once the level's going.
                        if self.turn > 100 && self.random() & 1 != 0 {
                            p.y = player_y + 700 + (self.random() & 0x1ff);
                            p.still = false;
                        } else {
                            p.fade = self.random() & 0xff;
                        }
                    }
                    Some(p)
                }
            };
        }
        self.first = (self.first - 1).max(0);
    }

    /// The can lying about nearest (x, y, z), and how far (`QDIST3`), as
    /// `DIRT_get_nearest_can_or_head_dist` finds it.
    /// Newspaper and crisp packets out of the top of a bin knocked over (`DIRT_create_papers`,
    /// dirt.cpp): four bits of paper about `(x, y, z)`, in free places (looked for every 16th),
    /// to flutter down as leaves do.
    pub fn papers(&mut self, x: i32, y: i32, z: i32) {
        let mut made = 0;
        for i in (0..self.pieces.len()).step_by(16) {
            if self.pieces[i].is_some() {
                continue;
            }
            let (dx, dz, dy) = ((self.random() & 0x3f) - 0x1f, (self.random() & 0x3f) - 0x1f, self.random() & 0x1f);
            let mut p = Piece::new(Kind::Leaf, x + dx, y + dy, z + dz);
            p.still = false;
            p.droll = (self.random() & 0x7f) - 0x3f;
            p.dpitch = (self.random() & 0x7f) - 0x3f;
            self.pieces[i] = Some(p);
            made += 1;
            if made >= 4 {
                return;
            }
        }
    }

    /// A shot with nothing to aim at, from `(x, z)` facing `facing` (`DIRT_shoot`, dirt.cpp,
    /// from Person.cpp:6547): the can in the air nearest straight ahead, within 200 of 2048
    /// either side and five squares (`QDIST` x plus z), is knocked spinning up and about.
    /// Where it was, if there was one.
    pub fn shoot(&mut self, x: i32, z: i32, facing: (f32, f32)) -> Option<(i32, i32, i32)> {
        let wide = 200.0 / 2048.0 * std::f32::consts::TAU;
        let best = self.pieces.iter().enumerate()
            .filter_map(|(i, p)| p.filter(|p| p.kind == Kind::ThrownCan).map(|p| (i, p)))
            .filter_map(|(i, p)| {
                let (dx, dz) = (p.x - x, p.z - z);
                let (fx, fz) = facing;
                let off = (fx * dz as f32 - fz * dx as f32).atan2(fx * dx as f32 + fz * dz as f32).abs();
                let dist = dx.abs() + dz.abs();
                (off <= wide && dist < 256 * 5).then_some((i, ((off / std::f32::consts::TAU * 2048.0) as i64) << 8 | (dist as i64) << 4))
            })
            .min_by_key(|&(_, score)| score)
            .map(|(i, _)| i)?;
        let r: Vec<i32> = (0..6).map(|_| self.random()).collect();
        let d = self.pieces[best].as_mut()?;
        d.dx = (r[0] & 0x1f) - 0xf;
        d.dz = (r[1] & 0x1f) - 0xf;
        d.dy += ((r[2] & 0xf) + 0xf) << TICK_SHIFT;
        d.dyaw = (r[3] & 0x3f) + 0x7f;
        if r[4] & 2 != 0 {
            d.yaw = -d.yaw;
        }
        Some((d.x, d.y, d.z))
    }

    pub fn nearest_can(&self, x: i32, y: i32, z: i32) -> Option<(usize, i32)> {
        self.pieces.iter().enumerate()
            .filter_map(|(i, p)| p.filter(|p| p.kind == Kind::Can).map(|p| (i, p)))
            .map(|(i, p)| {
                let (dx, dy, dz) = ((p.x - x).abs(), (p.y - y).abs(), (p.z - z).abs());
                let big = dx.max(dy).max(dz);
                (i, big + (dx + dy + dz - big) / 2)
            })
            .min_by_key(|&(_, d)| d)
    }

    /// `DIRT_pick_up_can_or_head`: the can nearest (x, y, z), if it's within 0x80, goes into
    /// `person`'s hand. Returns its slot.
    pub fn pick_up(&mut self, person: usize, x: i32, y: i32, z: i32) -> Option<usize> {
        let (i, dist) = self.nearest_can(x, y, z)?;
        let d = self.pieces[i].as_mut().filter(|_| dist < 0x80)?;
        (d.kind, d.owner, d.still) = (Kind::HeldCan, Some(person), false);
        Some(i)
    }

    /// The can held in slot `i` is where the hand holding it is: at (x, y, z), turned `yaw`.
    pub fn hold(&mut self, i: usize, x: i32, y: i32, z: i32, yaw: i32) {
        if let Some(d) = self.pieces[i].as_mut().filter(|d| d.kind == Kind::HeldCan) {
            (d.x, d.y, d.z, d.yaw, d.pitch, d.roll) = (x, y, z, yaw, 0, 0);
        }
    }

    /// `DIRT_release_can_or_head`: the can held in slot `i` is thrown the way someone at
    /// engine angle `angle` (2048 a turn) faces, at `power` (0 to 256; 128 for a throw).
    pub fn release(&mut self, i: usize, angle: i32, power: i32) {
        let spin = (self.random() & 0x3f) - 0x1f;
        let Some(d) = self.pieces[i].as_mut().filter(|d| d.kind == Kind::HeldCan) else { return };
        // `FMATRIX_vector(angle, 0)`; people face (-sin, -cos).
        d.dx = -((sin(angle) as i64 * power as i64) >> 18) as i32;
        d.dz = -((cos(angle) as i64 * power as i64) >> 18) as i32;
        d.dy = (power >> 2) << TICK_SHIFT;
        (d.dyaw, d.dpitch) = (spin, 50);
        d.kind = Kind::ThrownCan;
    }

    /// `DIRT_process`, one turn. Returns where cans landed hard enough to clatter.
    pub fn process(&mut self, floor: &impl Floor) -> Vec<Clatter> {
        self.turn += 1;
        let mut clatter = Vec::new();
        for k in 0..MAX_DIRT {
            let jiggle = if self.pieces[k].is_some_and(|d| d.kind == Kind::ThrownCan) {
                [self.random(), self.random(), self.random(), self.random()]
            } else {
                [0; 4]
            };
            let slot = &mut self.pieces[k];
            let Some(d) = slot else { continue };
            if d.still {
                if d.kind == Kind::Snow {
                    if d.fade > 4 {
                        d.fade -= 4;
                    } else {
                        *slot = None;
                    }
                }
                continue;
            }
            match d.kind {
                Kind::Leaf | Kind::Snow => {
                    let (ox, oz) = (d.x, d.z);
                    d.x += d.dx;
                    d.y += d.dy >> TICK_SHIFT;
                    d.z += d.dz;
                    d.yaw += d.dyaw;
                    d.pitch += d.dpitch;
                    d.roll += d.droll;
                    if (ox >> 8 != d.x >> 8 || oz >> 8 != d.z >> 8) && floor.hidden(d.x >> 8, d.z >> 8) {
                        // Into a wall: back, and away.
                        d.x = ox;
                        d.dx = -d.dx;
                        d.z = oz;
                        d.dz = -d.dz;
                    }
                    let ground = floor.height(d.x, d.z);
                    if d.y <= ground {
                        d.y = ground;
                        d.dy = 0;
                        (d.yaw, d.pitch, d.roll, d.dyaw, d.dpitch, d.droll) = (0, 0, 0, 0, 0, 0);
                        if d.dx.abs() <= 3 {
                            d.dx = 0;
                        }
                        if d.dz.abs() <= 3 {
                            d.dz = 0;
                        }
                        if d.dx == 0 && d.dz == 0 {
                            d.still = true;
                        }
                    }
                    // Lots of resistance to moving and turning.
                    d.dx -= d.dx / 4;
                    d.dy -= d.dy / 2;
                    d.dz -= d.dz / 4;
                    d.dpitch -= d.dpitch / 32;
                    d.droll -= d.droll / 32;
                    // Floating down in a leaf-like fashion.
                    let waftz = (d.pitch / 32).clamp(-5, 5);
                    let waftx = (d.roll / 32).clamp(-5, 5);
                    d.dz -= waftz;
                    d.dx += waftx;
                    d.dpitch -= 0xa * d.pitch.signum();
                    d.droll -= 0xa * d.roll.signum();
                    d.dy -= 4 << TICK_SHIFT;
                    d.dy += d.pitch.abs().min(180) << (TICK_SHIFT - 5);
                    d.dy += d.roll.abs().min(180) << (TICK_SHIFT - 5);
                }
                Kind::Can => {
                    d.yaw = (d.yaw + d.dyaw) & 2047;
                    d.pitch = (d.pitch + d.dpitch) & 2047;
                    d.x += d.dx >> 8;
                    d.z += d.dz >> 8;
                    let (mx, mz) = (d.x >> 8, d.z >> 8);
                    let newy = floor.height(d.x, d.z) + 6;
                    if newy - d.y > 8 && floor.hidden(mx, mz) {
                        // Off a wall, spinning.
                        if (d.x - (d.dx >> 8)) >> 8 != mx {
                            d.dx = -d.dx;
                        }
                        if (d.z - (d.dz >> 8)) >> 8 != mz {
                            d.dz = -d.dz;
                        }
                        d.x += d.dx >> 7;
                        d.z += d.dz >> 7;
                        d.dyaw += 200;
                        d.dpitch = 0;
                        continue;
                    }
                    if d.y > newy {
                        d.dy -= 4 << TICK_SHIFT;
                        d.y += d.dy >> TICK_SHIFT;
                    }
                    if d.y < newy {
                        d.y = newy;
                        if d.dy < -8 << TICK_SHIFT {
                            clatter.push(Clatter { at: (d.x, d.y, d.z), alerts: false, by: None });
                        }
                        d.dy = 0;
                    }
                    d.dx -= d.dx / 16;
                    d.dz -= d.dz / 16;
                    d.dx -= d.dx.signum();
                    d.dz -= d.dz.signum();
                    d.dyaw -= d.dyaw / 32;
                    d.dpitch -= d.dpitch / 32;
                    d.dyaw -= d.dyaw.signum();
                    d.dpitch -= d.dpitch.signum();
                    if d.dx == 0 && d.dz == 0 && d.dyaw == 0 && d.dpitch == 0 {
                        d.still = true;
                    }
                }
                Kind::HeldCan => {}
                Kind::ThrownCan => {
                    // dirt.cpp:1857: gravity a unit a tick, falling 0x20 a tick at most.
                    d.dy = (d.dy - (1 << TICK_SHIFT)).max(-0x20 << TICK_SHIFT);
                    let (ox, oy, oz) = (d.x, d.y, d.z);
                    d.x += d.dx;
                    d.y += d.dy >> TICK_SHIFT;
                    d.z += d.dz;
                    d.yaw = (d.yaw + d.dyaw) & 2047;
                    d.pitch = (d.pitch + d.dpitch) & 2047;
                    let floor_y = floor.height(d.x, d.z) + 6;
                    let into_building = (ox >> 8 != d.x >> 8 || oz >> 8 != d.z >> 8) && floor.hidden(d.x >> 8, d.z >> 8);
                    if d.y < floor_y || into_building {
                        if d.dy.abs() > 8 << TICK_SHIFT {
                            // A clatter the guards hear.
                            clatter.push(Clatter { at: (ox, oy, oz), alerts: true, by: d.owner });
                        }
                        if floor_y - d.y > 0x40 || d.dy > 0 || into_building {
                            // Into a building: back off it the way it came in, at half speed.
                            if ox >> 8 != d.x >> 8 {
                                d.x = ox;
                                d.dx = -d.dx >> 1;
                            }
                            if oz >> 8 != d.z >> 8 {
                                d.z = oz;
                                d.dz = -d.dz >> 1;
                            }
                        } else {
                            // Off the floor, half as high, until it hardly bounces: then it's
                            // a can lying about again, rolling on a little.
                            d.dy = d.dy.abs() >> 1;
                            d.y = 2 * floor_y - d.y;
                            if d.dy.abs() < 10 << TICK_SHIFT {
                                d.dy = 0;
                                d.kind = Kind::Can;
                                d.dx = (d.dx + (jiggle[0] & 0xf) - 7) << 8;
                                d.dz = (d.dz + (jiggle[1] & 0xf) - 7) << 8;
                                d.dyaw = (jiggle[2] & 0x7f) + 0x7f;
                                d.dpitch = (jiggle[3] & 0x3f) + 0x3f;
                            } else {
                                d.dx += (jiggle[0] & 0xf) - 7;
                                d.dz += (jiggle[1] & 0xf) - 7;
                                d.dyaw += (jiggle[2] & 0x7f) - 0x3f;
                                d.dpitch += jiggle[3] & 0x3f;
                            }
                        }
                    }
                }
            }
        }
        clatter
    }

    /// `DIRT_gust`: something moved from (x1, z1) to (x2, z2) this turn. Leaves near it are
    /// blown away from it and along with it; a can it's right on top of is kicked. Returns
    /// where cans were kicked.
    pub fn gust(&mut self, x1: i32, z1: i32, x2: i32, z2: i32) -> Vec<(i32, i32, i32)> {
        let mut kicked = Vec::new();
        let (fx, fz, radius) = self.focus;
        if (fx - x1).abs() + (fz - z1).abs() > radius {
            return kicked;
        }
        let (mut dgx, mut dgz) = (x2 - x1, z2 - z1);
        dgx -= dgx / 4;
        dgz -= dgz / 4;
        let strength = (qdist2(dgx.abs(), dgz.abs()) + 1) * 8;
        for i in 0..MAX_DIRT {
            let Some(mut d) = self.pieces[i] else { continue };
            let (dx, dz) = (d.x - x1, d.z - z1);
            let dist = qdist2(dx.abs(), dz.abs()) + 1;
            if strength <= dist {
                continue;
            }
            match d.kind {
                Kind::Leaf | Kind::Snow => {
                    let push = 256 - dist * 256 / strength;
                    if push != 0 {
                        let pushx = ((dx * 32 / dist + dgx) * push) >> 12;
                        let pushz = ((dz * 32 / dist + dgz) * push) >> 12;
                        let pushy = (pushx.abs() + pushz.abs()) << 1;
                        d.dpitch += (self.random() & 0x3f) - 0x1f;
                        d.droll += (self.random() & 0x3f) - 0x1f;
                        d.dx += pushx;
                        d.dy += pushy << TICK_SHIFT;
                        d.dz += pushz;
                        d.still = false;
                    }
                }
                Kind::HeldCan | Kind::ThrownCan => {}
                Kind::Can => {
                    if dist < 32 {
                        d.dx = (dx << 13) / dist;
                        d.dz = (dz << 13) / dist;
                        d.dyaw = (self.random() & 0xff) + 0x7f;
                        d.dpitch = -400;
                        d.still = false;
                        kicked.push((d.x, d.y, d.z));
                    }
                }
            }
            self.pieces[i] = Some(d);
        }
        kicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat open ground with a building on the squares x >= 100.
    struct Flat;

    impl Floor for Flat {
        fn height(&self, _: i32, _: i32) -> i32 {
            0
        }
        fn open(&self, mx: i32, mz: i32, _: bool) -> bool {
            (0..100).contains(&mx) && (0..128).contains(&mz)
        }
        fn hidden(&self, mx: i32, _: i32) -> bool {
            mx >= 100
        }
    }

    fn city() -> Dirt {
        let mut d = Dirt::new(Ground::City, 100, 1, []);
        for _ in 0..8 {
            d.set_focus(64 * 256, 64 * 256, FOCUS_RADIUS, 0, &Flat);
        }
        d
    }

    #[test]
    fn leaves_and_the_odd_can_round_the_focus() {
        let d = city();
        let pieces: Vec<_> = d.pieces.iter().flatten().collect();
        assert!(pieces.len() > 900, "only {}", pieces.len());
        let cans = pieces.iter().filter(|p| p.kind == Kind::Can).count();
        assert!((1..=40).contains(&cans), "{cans} cans");
        for p in &pieces {
            assert!(qdist2((p.x - 64 * 256).abs(), (p.z - 64 * 256).abs()) <= FOCUS_RADIUS);
            assert!(p.still);
        }
    }

    /// `DIRT_shoot`: a can in the air straight ahead and near is knocked up and spinning; one
    /// behind, or too far off, isn't.
    #[test]
    fn a_shot_knocks_a_can_flying() {
        let mut d = Dirt::new(Ground::City, 0, 0, []);
        let (x, z) = (64 * 256, 64 * 256);
        let mut can = Piece::new(Kind::ThrownCan, x + 400, 120, z + 20);
        can.still = false;
        d.pieces[0] = Some(can);
        assert_eq!(d.shoot(x, z, (-1.0, 0.0)), None, "behind her");
        assert_eq!(d.shoot(x - 1200, z, (1.0, 0.0)), None, "too far");
        assert_eq!(d.shoot(x, z, (1.0, 0.0)), Some((x + 400, 120, z + 20)));
        let p = d.pieces[0].unwrap();
        assert!(p.dy >= 0xf << TICK_SHIFT && p.dyaw >= 0x7f, "dy {} dyaw {}", p.dy, p.dyaw);
    }

    /// `DIRT_create_papers`: four bits of paper about the top of the bin, flying, and down
    /// they flutter to the ground.
    #[test]
    fn a_bin_throws_out_four_papers() {
        let mut d = Dirt::new(Ground::City, 0, 0, []);
        d.papers(64 * 256, 160, 64 * 256);
        let flying = |d: &Dirt| d.pieces.iter().flatten().filter(|p| p.kind == Kind::Leaf && !p.still).count();
        assert_eq!(flying(&d), 4);
        for p in d.pieces.iter().flatten() {
            assert!((p.x - 64 * 256).abs() <= 0x20 && (160..160 + 0x20).contains(&p.y));
        }
        // (A paper can ride the air a while, as a leaf does.)
        for _ in 0..4000 {
            d.process(&Flat);
        }
        assert!(d.pieces.iter().flatten().all(|p| p.y == 0), "still up: {:?}", d.pieces.iter().flatten().map(|p| (p.y, p.still)).collect::<Vec<_>>());
    }

    #[test]
    fn moving_on_brings_the_dirt_along() {
        let mut d = city();
        for _ in 0..40 {
            d.set_focus(84 * 256, 64 * 256, FOCUS_RADIUS, 0, &Flat);
        }
        let near = d.pieces.iter().flatten().filter(|p| qdist2((p.x - 84 * 256).abs(), (p.z - 64 * 256).abs()) <= FOCUS_RADIUS).count();
        let all = d.pieces.iter().flatten().count();
        assert!(near * 10 > all * 9, "{near} of {all}");
        assert!(d.pieces.iter().flatten().all(|p| p.x >> 8 < 100), "dirt inside a building");
    }

    /// A runner blows the leaves near them up and along, and they settle again.
    #[test]
    fn a_gust_lifts_leaves_and_they_settle() {
        let mut d = city();
        let at = d.pieces.iter().flatten().find(|p| p.kind == Kind::Leaf).map(|p| (p.x, p.z)).unwrap();
        d.gust(at.0 - 20, at.1, at.0 + 20, at.1);
        let moving = d.pieces.iter().flatten().filter(|p| !p.still).count();
        assert!(moving > 0);
        let mut highest = 0;
        for _ in 0..200 {
            d.process(&Flat);
            highest = highest.max(d.pieces.iter().flatten().map(|p| p.y).max().unwrap());
        }
        assert!(highest > 0, "nothing left the ground");
        let up: Vec<_> = d.pieces.iter().flatten().filter(|p| p.kind == Kind::Leaf && !(p.still && p.y == 0)).collect();
        assert!(up.is_empty(), "still blowing about: {:?}", &up[..up.len().min(3)]);
    }

    #[test]
    fn a_can_underfoot_is_kicked() {
        let mut d = Dirt::new(Ground::City, 0, 1, []);
        d.set_focus(64 * 256, 64 * 256, FOCUS_RADIUS, 0, &Flat);
        let can = d.pieces.iter().flatten().next().copied().unwrap();
        assert_eq!(can.kind, Kind::Can);
        let kicked = d.gust(can.x - 10, can.z, can.x + 10, can.z);
        assert_eq!(kicked.len(), 1);
        for _ in 0..400 {
            d.process(&Flat);
        }
        let now = d.pieces.iter().flatten().find(|p| p.kind == Kind::Can).unwrap();
        assert!(now.x > can.x + 32, "the can went {}", now.x - can.x);
        assert!(now.still);
    }

    /// Picked up, thrown, it flies the way she faces, clatters where the guards hear it, and
    /// lies about as a can again.
    #[test]
    fn a_can_picked_up_and_thrown_lands_ahead() {
        let mut d = Dirt::new(Ground::City, 0, 1, []);
        d.set_focus(64 * 256, 64 * 256, FOCUS_RADIUS, 0, &Flat);
        let can = d.pieces.iter().flatten().next().copied().unwrap();
        assert_eq!(d.pick_up(7, can.x + 200, can.y, can.z), None, "out of reach");
        let i = d.pick_up(7, can.x + 50, can.y, can.z).expect("in reach");
        assert_eq!(d.pieces[i].unwrap().kind, Kind::HeldCan);
        d.hold(i, can.x, 100, can.z, 0);
        // Facing (-sin, -cos) of angle 512: -x.
        d.release(i, 512, 128);
        let mut clatters = Vec::new();
        for _ in 0..400 {
            clatters.extend(d.process(&Flat));
        }
        let now = d.pieces[i].unwrap();
        assert_eq!(now.kind, Kind::Can);
        assert!(now.x < can.x - 500, "it went {}", can.x - now.x);
        assert!((now.z - can.z).abs() < 200);
        assert!(clatters.iter().any(|c| c.alerts && c.by == Some(7)), "{clatters:?}");
    }

    #[test]
    fn leaves_under_trees_in_range() {
        let mut d = Dirt::new(Ground::City, 0, 0, [(60 * 256, 64 * 256)]);
        d.set_focus(64 * 256, 64 * 256, FOCUS_RADIUS, 0, &Flat);
        assert_eq!(d.pieces.iter().flatten().filter(|p| p.owner == Some(0)).count(), LEAVES_PER_TREE);
        d.set_focus(20 * 256, 64 * 256, FOCUS_RADIUS, 0, &Flat);
        assert_eq!(d.pieces.iter().flatten().filter(|p| p.owner == Some(0)).count(), 0);
    }
}
