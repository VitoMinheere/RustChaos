//! Where people on foot wander (fallen/Source/wand.cpp): the pavements, every square within
//! two of a road that isn't road itself, plus the zebra crossings, found as the level loads.
//! The city's extra civilians stroll from one of these squares to the next, and appear on
//! one when they're made or brought back near the player.

use uc_formats::units::radians;
use glam::{vec2, Vec2, Vec3};
use std::collections::HashMap;
use std::path::Path;
use uc_formats::iam::{flags, Map, MAP_SIZE};

use crate::road;

const N: i32 = MAP_SIZE as i32;

pub struct Wander {
    squares: Vec<bool>,
}

/// `ROAD_is_zebra`: a crossing's texture.
fn is_zebra(map: &Map, x: i32, z: i32) -> bool {
    (0..N).contains(&x) && (0..N).contains(&z) && matches!(map.square(x as usize, z as usize).page(), 333 | 334)
}

impl Wander {
    /// `WAND_init`.
    pub fn new(map: &Map, game: &Path) -> Wander {
        let mut squares = vec![false; (N * N) as usize];
        for x in 0..N {
            for z in 0..N {
                if map.square(x as usize, z as usize).has(flags::HIDDEN) {
                    continue;
                }
                squares[(x * N + z) as usize] = if !road::is_road(map, x, z) {
                    (-2..=2).any(|dx| (-2..=2).any(|dz| road::is_road(map, x + dx, z + dz)))
                } else {
                    is_zebra(map, x, z)
                };
            }
        }
        // Not where a prop stands on the ground over the middle of the square: boxes and
        // cylinders only, the props people walk into.
        let mut coltypes: HashMap<u8, Option<u8>> = HashMap::new();
        for ob in &map.objects {
            let col = *coltypes.entry(ob.prim).or_insert_with(|| uc_world::level::load_prim(game, ob.prim).map(|p| p.coltype));
            if !matches!(col, Some(0 | 2 | 3)) {
                continue;
            }
            let (mx, mz) = (ob.x >> 8, ob.z >> 8);
            if !(0..N).contains(&mx) || !(0..N).contains(&mz) || map.square(mx as usize, mz as usize).has(flags::HIDDEN) {
                continue;
            }
            if (ob.y as f32) <= map.ground_height(ob.x as f32, ob.z as f32) + 64.0 {
                squares[(mx * N + mz) as usize] = false;
            }
        }
        Wander { squares }
    }

    /// `WAND_square_is_wander`.
    pub fn is_wander(&self, x: i32, z: i32) -> bool {
        (0..N).contains(&x) && (0..N).contains(&z) && self.squares[(x * N + z) as usize]
    }

    /// Whether there's anywhere on the map to wander at all.
    pub fn any(&self) -> bool {
        self.squares.iter().any(|&s| s)
    }

    /// `WAND_get_next_place`: the middle of a wander square three or so ahead, behind or to
    /// either side, preferring to carry on the way they're facing; failing that, the nearest
    /// of a few anywhere within fifteen; failing that, straight on.
    pub fn next_place(&self, pos: Vec3, facing: Vec2, rand: &mut impl FnMut() -> u32) -> Vec2 {
        let (mid_x, mid_z) = ((pos.x as i32) >> 8, (pos.z as i32) >> 8);
        // Two squares ahead, as the original's `SIN(angle) >> 7`.
        let (dx, dz) = ((facing.x * 512.0) as i32, (facing.y * 512.0) as i32);
        // Only the first four of its eight looks are filled in: the rest look about where
        // they stand.
        const LOOKS: [(i32, i32); 8] = [(3, 0), (-3, 0), (0, 3), (0, -3), (0, 0), (0, 0), (0, 0), (0, 0)];
        const OFFSET: [i32; 4] = [-1, 0, 0, 1];
        let mut best = ((((pos.x as i32) + dx) >> 8).clamp(0, N - 1), (((pos.z as i32) + dz) >> 8).clamp(0, N - 1));
        let mut best_score = None;
        for (lx, lz) in LOOKS {
            let off_x = lx + OFFSET[((rand() >> 3) & 3) as usize];
            let off_z = lz + OFFSET[((rand() >> 3) & 3) as usize];
            let (mx, mz) = (mid_x + off_x, mid_z + off_z);
            if !self.is_wander(mx, mz) {
                continue;
            }
            let dprod = dx * off_x + dz * off_z;
            let score = match dprod {
                0 => 0,
                d if d < 0 => -((rand() as i32) % (-d >> 1).max(1)),
                d => (rand() as i32) % d,
            };
            if best_score.is_none_or(|b| score > b) {
                best = (mx, mz);
                best_score = Some(score);
            }
        }
        if best_score.is_none() {
            let mut nearest = i32::MAX;
            for _ in 0..16 {
                let off_x = (rand() & 0x1f) as i32 - 0xf;
                let off_z = (rand() & 0x1f) as i32 - 0xf;
                let (mx, mz) = (mid_x + off_x, mid_z + off_z);
                let score = off_x.abs() + off_z.abs();
                if self.is_wander(mx, mz) && score < nearest {
                    nearest = score;
                    best = (mx, mz);
                }
            }
        }
        vec2((best.0 << 8) as f32 + 128.0, (best.1 << 8) as f32 + 128.0)
    }

    /// The first wander square (by x, then z) in the square `radius` around map square
    /// (`x`, `z`), kept two from the edge of the map: `WAND_find_good_start_point_near`
    /// (radius 2) and `WAND_find_good_start_point` (radius 1).
    pub fn start_near(&self, x: i32, z: i32, radius: i32) -> Option<Vec2> {
        let lo = |v: i32| (v - radius).clamp(2, 125);
        let hi = |v: i32| (v + radius).clamp(2, 125);
        for mx in lo(x)..=hi(x) {
            for mz in lo(z)..=hi(z) {
                if self.is_wander(mx, mz) {
                    return Some(vec2((mx << 8) as f32 + 128.0, (mz << 8) as f32 + 128.0));
                }
            }
        }
        None
    }

    /// `WAND_find_good_start_point`: near the spot `dist` ahead of the player, give or take
    /// an eighth of a turn, so the people brought back appear where they're about to be seen.
    pub fn start_ahead(&self, player: Vec3, facing: Vec2, dist: f32, rand: &mut impl FnMut() -> u32) -> Option<Vec2> {
        let turn = radians((rand() & 511) as f32 - 256.0);
        let dir = Vec2::from_angle(turn).rotate(facing);
        let at = vec2(player.x, player.z) + dir * dist;
        self.start_near((at.x as i32) >> 8, (at.y as i32) >> 8, 1)
    }
}
