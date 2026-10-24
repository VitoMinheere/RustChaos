//! Puddles and the drips in them (fallen/Source/puddle.cpp, drip.cpp; #239).
//!
//! When it's raining (always, at night), `PUDDLE_precalculate` lays puddles along the city's
//! edges as the level loads: in the corners and along the walls of buildings, and in the
//! corners and along the kerbs of the roads, each on a roll of the dice and only where the
//! ground is flat. A puddle is a quad of `puddle01.tga`, one of four shapes cut from it: two
//! strips, a corner and a whole round one.
//!
//! The rain drips into them: each frame a puddle in view has an even chance of a drip
//! somewhere in it (`AENG_draw_puddles`), a ring of `drip.tga` that spreads 4 a tick and
//! fades 16 a tick. Feet coming down in one drip too (`DRIP_create_if_in_puddle`, from
//! `person_splash`). Here the drips come a tick at a time for the puddles near the player.
//!
//! What they reflect, the moon and the people standing in them (#268), is drawn with the
//! rest of the game (see `Game::batches`), and shown by the renderer only where a puddle is.
//!
//! Not done: the ripples a splash sets going in the puddle's own picture (`PUDDLE_splash`,
//! drawn as a wobble of the screen behind it).

use crate::rand;
use glam::{vec3, Vec3};
use uc_formats::iam::{flags, Map, MAP_SIZE};

pub use uc_world::geometry::{DRIP_PAGE, PUDDLE_PAGE};

/// `PUDDLE_TYPE_*`: which part of the texture a puddle uses, (u1, v1, u2, v2) of 256.
const TEXTURE: [[f32; 4]; 4] = [[0.0, 0.0, 256.0, 62.0], [0.0, 64.0, 256.0, 126.0], [0.0, 128.0, 128.0, 256.0], [128.0, 128.0, 256.0, 256.0]];
const STRIP1: u8 = 0;
const CORNER: u8 = 2;

/// `PUDDLE_MAX_PUDDLES`, less the unused first.
const MAX_PUDDLES: usize = 255;
/// `DRIP_MAX_DRIPS`.
const MAX_DRIPS: usize = 1024;
/// Ticks a second the original's drips grow and fade at.
const TICKS: f32 = 20.0;
/// How near the player a puddle drips, in world units.
const NEAR: f32 = 24.0 * 256.0;

#[derive(Debug, Clone, Copy)]
pub struct Puddle {
    /// Two opposite corners, in the order that flips the texture as the original's random
    /// swaps do.
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub y: f32,
    kind: u8,
    /// The texture runs across the puddle, not along it (`rotate_uvs`).
    pub rotate: bool,
}

impl Puddle {
    /// The texture's corners for `a` and `b`, 0 to 1.
    pub fn uvs(&self) -> ([f32; 2], [f32; 2]) {
        let [u1, v1, u2, v2] = TEXTURE[self.kind as usize];
        ([u1 / 256.0, v1 / 256.0], [u2 / 256.0, v2 / 256.0])
    }

    pub fn contains(&self, x: f32, z: f32) -> bool {
        let (x1, x2) = (self.a.0.min(self.b.0), self.a.0.max(self.b.0));
        let (z1, z2) = (self.a.1.min(self.b.1), self.a.1.max(self.b.1));
        (x1..=x2).contains(&x) && (z1..=z2).contains(&z)
    }
}

/// A ring spreading in a puddle (`DRIP_Drip`).
#[derive(Debug, Clone, Copy)]
pub struct Drip {
    pub pos: Vec3,
    /// Radius, in world units.
    pub size: f32,
    /// 255 when new, gone at 0.
    pub fade: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Puddles {
    pub puddles: Vec<Puddle>,
    pub drips: Vec<Drip>,
    /// `PAP_FLAG_REFLECTIVE`: the squares in and round a puddle.
    reflective: Vec<bool>,
    clock: f32,
    rng: u32,
}

const N: i32 = MAP_SIZE as i32;

impl Puddles {
    /// `PUDDLE_precalculate`, for a level where it's raining. `mav_height(x, z)` is the
    /// navigation map's height there (`MAVHEIGHT`).
    pub fn precalculate(map: &Map, mav_height: impl Fn(i32, i32) -> i32) -> Puddles {
        let mut p = Puddles { reflective: vec![false; (N * N) as usize], rng: 0x9dd1, ..Default::default() };
        let has = |x: i32, z: i32, f: u16| map.square(x as usize, z as usize).has(f);
        for mx in 1..N - 2 {
            for mz in 1..N - 2 {
                // Corners of buildings: a hidden square with the three round one corner open
                // and no higher.
                if has(mx, mz, flags::HIDDEN) {
                    for i in 0..4 {
                        let (dx, dz) = (if i & 1 != 0 { 1 } else { -1 }, if i & 2 != 0 { 1 } else { -1 });
                        let ok = (1..4).all(|j| {
                            let (cx, cz) = (mx + if j & 1 != 0 { dx } else { 0 }, mz + if j & 2 != 0 { dz } else { 0 });
                            !p.blocked(map, cx, cz) && mav_height(cx, cz) <= mav_height(mx, mz)
                        });
                        if ok && (p.roll() & 3) as i32 == (mx + mz) & 3 {
                            p.corner(map, mx, mz, dx, dz, 8.0);
                        }
                    }
                }
                // Corners of roads.
                if !has(mx, mz, flags::SINK_SQUARE) {
                    for i in 0..4 {
                        let (dx, dz) = (if i & 1 != 0 { 1 } else { -1 }, if i & 2 != 0 { 1 } else { -1 });
                        let ok = (1..4).all(|j| {
                            let (cx, cz) = (mx + if j & 1 != 0 { dx } else { 0 }, mz + if j & 2 != 0 { dz } else { 0 });
                            !p.blocked(map, cx, cz) && has(cx, cz, flags::SINK_SQUARE)
                        });
                        if ok && (p.roll() & 3) as i32 == (mx + mz) & 3 {
                            p.corner(map, mx, mz, dx, dz, 2.0);
                        }
                    }
                }
                const ORDER: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                // Along the walls of buildings.
                if !has(mx, mz, flags::HIDDEN) && !p.reflective(mx, mz) {
                    for (i, &(dx, dz)) in ORDER.iter().enumerate() {
                        let (v2x, v2z) = (dz, -dx);
                        let wall = |cx: i32, cz: i32| has(cx, cz, flags::HIDDEN) && !p.wet(map, cx, cz) && mav_height(cx, cz) >= mav_height(mx, mz);
                        if wall(mx + dx, mz + dz) && wall(mx + dx + v2x, mz + dz + v2z) && !p.blocked(map, mx + v2x, mz + v2z)
                            && (p.roll() & 7) as i32 == (mx + mz) & 7
                        {
                            p.strip(map, mx, mz, (dx, dz), (v2x, v2z), 4.0, i < 2);
                        }
                    }
                }
                // Along the kerbs of roads.
                if has(mx, mz, flags::SINK_SQUARE) && !p.wet(map, mx, mz) {
                    for (i, &(dx, dz)) in ORDER.iter().enumerate() {
                        let (v2x, v2z) = (dz, -dx);
                        let pavement = |cx: i32, cz: i32| !has(cx, cz, flags::SINK_SQUARE) && !p.wet(map, cx, cz);
                        let road = |cx: i32, cz: i32| has(cx, cz, flags::SINK_SQUARE) && !p.wet(map, cx, cz);
                        if pavement(mx + dx, mz + dz) && pavement(mx + dx + v2x, mz + dz + v2z) && road(mx + v2x, mz + v2z)
                            && (p.roll() & 3) as i32 == (mx + mz) & 3
                        {
                            p.strip(map, mx, mz, (dx, dz), (v2x, v2z), 2.0, i < 2);
                        }
                    }
                }
            }
        }
        p
    }

    fn roll(&mut self) -> u32 {
        rand(&mut self.rng)
    }

    fn reflective(&self, x: i32, z: i32) -> bool {
        // A dry level has no grid at all.
        (0..N).contains(&x) && (0..N).contains(&z) && self.reflective.get((x * N + z) as usize).copied().unwrap_or(false)
    }

    /// Water or a puddle already: no puddle goes there (`PAP_FLAG_REFLECTIVE | PAP_FLAG_WATER`,
    /// `MAV_SPARE_FLAG_WATER`).
    fn wet(&self, map: &Map, x: i32, z: i32) -> bool {
        self.reflective(x, z) || map.square(x as usize, z as usize).has(flags::WATER)
    }

    /// Wet, or under a building.
    fn blocked(&self, map: &Map, x: i32, z: i32) -> bool {
        self.wet(map, x, z) || map.square(x as usize, z as usize).has(flags::HIDDEN)
    }

    /// A corner puddle at the corner of (mx, mz) towards (dx, dz), `lift` above the ground.
    fn corner(&mut self, map: &Map, mx: i32, mz: i32, dx: i32, dz: i32, lift: f32) {
        let mid = ((mx * 256 + 128 + dx * 128) as f32, (mz * 256 + 128 + dz * 128) as f32);
        let size = (0xa0 + (self.roll() & 0x3f)) as f32;
        let a = (mid.0 - dx as f32 * size, mid.1 - dz as f32 * size);
        let b = (mid.0 + dx as f32 * size, mid.1 + dz as f32 * size);
        let y = map.ground_height(b.0, b.1) + lift;
        self.create(map, a, b, y, CORNER, false);
    }

    /// A strip along the side of (mx, mz) towards `v1`, running along `v2`.
    #[allow(clippy::too_many_arguments)]
    fn strip(&mut self, map: &Map, mx: i32, mz: i32, v1: (i32, i32), v2: (i32, i32), lift: f32, rotate: bool) {
        let size = (0xa0 + (self.roll() & 0x3f)) as f32;
        let a = ((mx * 256 + 128 + v1.0 * 128 - v2.0 * 128) as f32, (mz * 256 + 128 + v1.1 * 128 - v2.1 * 128) as f32);
        let b = (a.0 - v1.0 as f32 * size + v2.0 as f32 * 512.0, a.1 - v1.1 as f32 * size + v2.1 as f32 * 512.0);
        let y = map.ground_height(b.0, b.1) + lift;
        let kind = STRIP1 + (self.roll() & 1) as u8;
        self.create(map, a, b, y, kind, rotate);
    }

    /// `PUDDLE_create_do`: on flat ground only (`PAP_is_flattish`: eight points no more than
    /// 0x10 apart), and marking the squares round it reflective.
    fn create(&mut self, map: &Map, a: (f32, f32), b: (f32, f32), y: f32, kind: u8, rotate: bool) {
        if self.puddles.len() >= MAX_PUDDLES {
            return;
        }
        let (lo, hi) = (0..8).fold((f32::MAX, f32::MIN), |(lo, hi), _| {
            let (u, v) = ((self.roll() & 0xff) as f32 / 256.0, (self.roll() & 0xff) as f32 / 256.0);
            let h = map.ground_height(a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * v);
            (lo.min(h), hi.max(h))
        });
        if hi - lo > 16.0 {
            return;
        }
        self.puddles.push(Puddle { a, b, y, kind, rotate });
        // `PUDDLE_EXTEND_REFLECTION`: 128 round it.
        let (x1, x2) = (a.0.min(b.0) - 128.0, a.0.max(b.0) + 128.0);
        let (z1, z2) = (a.1.min(b.1) - 128.0, a.1.max(b.1) + 128.0);
        for x in ((x1 / 256.0) as i32).max(0)..=((x2 / 256.0) as i32).min(N - 1) {
            for z in ((z1 / 256.0) as i32).max(0)..=((z2 / 256.0) as i32).min(N - 1) {
                self.reflective[(x * N + z) as usize] = true;
            }
        }
    }

    /// `PUDDLE_in`: whether (x, z) is in a puddle.
    pub fn in_puddle(&self, x: f32, z: f32) -> bool {
        self.reflective((x / 256.0) as i32, (z / 256.0) as i32) && self.puddles.iter().any(|p| p.contains(x, z))
    }

    fn drip(&mut self, pos: Vec3) {
        if self.drips.len() >= MAX_DRIPS {
            self.drips.remove(0);
        }
        // `DRIP_SSIZE`, `DRIP_SFADE`.
        let size = (self.roll() & 7) as f32;
        self.drips.push(Drip { pos, size, fade: 255.0 });
    }

    /// A foot coming down at `at` (`person_splash`): a drip, somewhere within 15 of it, if
    /// that's in a puddle.
    pub fn splash(&mut self, at: Vec3) {
        let x = at.x + (self.roll() & 0x1f) as f32 - 15.0;
        let z = at.z + (self.roll() & 0x1f) as f32 - 15.0;
        if self.in_puddle(x, z) {
            self.drip(vec3(x, at.y, z));
        }
    }

    /// The drips spread and fade a tick at a time (`DRIP_process`), and while it rains each
    /// puddle near `near` has an even chance a tick of one landing in it.
    pub fn update(&mut self, dt: f32, raining: bool, near: Option<Vec3>) {
        self.clock += dt * TICKS;
        while self.clock >= 1.0 {
            self.clock -= 1.0;
            for d in &mut self.drips {
                d.fade -= 16.0;
                d.size += 4.0;
            }
            self.drips.retain(|d| d.fade > 0.0);
            let Some(at) = near.filter(|_| raining) else { continue };
            for k in 0..self.puddles.len() {
                let p = self.puddles[k];
                let mid = ((p.a.0 + p.b.0) / 2.0, (p.a.1 + p.b.1) / 2.0);
                if (mid.0 - at.x).abs() > NEAR || (mid.1 - at.z).abs() > NEAR || self.roll() & 0x100 == 0 {
                    continue;
                }
                let (u, v) = ((self.roll() & 0xff) as f32 / 256.0, (self.roll() & 0xff) as f32 / 256.0);
                self.drip(vec3(p.a.0 + (p.b.0 - p.a.0) * u, p.y, p.a.1 + (p.b.1 - p.a.1) * v));
            }
        }
    }
}
