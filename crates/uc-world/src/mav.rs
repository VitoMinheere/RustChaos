//! The navigation map (fallen/Source/mav.cpp): for each map square, how high it is and how
//! someone can get from it to each of its four neighbours (walking, jumping, pulling up,
//! climbing a fence or a ladder, falling off); and the route search over it, `MAV_do`, that
//! says what to do next to get somewhere.
//!
//! Heights are in 64ths of a world unit's 64 (`MAV_height`: a quarter of a block a step),
//! squares are map squares, and directions are `MAV_DIR_*`: -x, +x, -z, +z.

use uc_formats::units::radians;
use crate::ground::Ground;
use crate::level::Level;
use glam::{vec2, Vec2};
use uc_formats::iam::{flags, Map, BUILDING_TYPE_WAREHOUSE, MAP_SIZE};

/// `MAV_ACTION_*`: what to do to get to the next square.
pub const GOTO: u8 = 0;
/// Jump one square over.
pub const JUMP: u8 = 1;
/// Jump one square over, pulling up at the far side.
pub const JUMPPULL: u8 = 2;
/// Jump two squares over, pulling up at the far side.
pub const JUMPPULL2: u8 = 3;
pub const PULLUP: u8 = 4;
pub const CLIMB_OVER: u8 = 5;
pub const FALL_OFF: u8 = 6;
pub const LADDER_UP: u8 = 7;

/// `MAV_CAPS_*`: one bit per action.
pub const fn caps(action: u8) -> u8 {
    1 << action
}
/// `MAV_CAPS_DARCI`: she can do everything.
pub const CAPS_DARCI: u8 = 0xff;

/// `order[]`: the step for each `MAV_DIR_*`.
pub const DIRS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// `MAV_LOOKAHEAD`: how many squares ahead the route search looks.
const LOOKAHEAD: u8 = 32;
/// `MAV_MAX_OVERFLOWS`: how many squares at the edge of the look-ahead before it settles for
/// the best of them.
const MAX_OVERFLOWS: u32 = 8;

/// `MAV_Action`: what to do first, and where to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub action: u8,
    /// For anything but `GOTO`: the direction to do it in.
    pub dir: u8,
    /// For `GOTO`: the furthest square along the route she can walk straight to.
    pub dest: (u8, u8),
    /// The first square along the route (one step on), for someone who can't make for
    /// `dest` directly after all.
    pub first: (u8, u8),
}

/// The navigation map.
pub struct Mav {
    /// `MAV_height`, x-major.
    height: Vec<i8>,
    /// `MAV_opt[MAV_NAV(x, z)]`: per square, the caps for leaving it each way.
    opts: Vec<[u8; 4]>,
}

fn idx(x: i32, z: i32) -> usize {
    x as usize * MAP_SIZE + z as usize
}

fn on_map(x: i32, z: i32) -> bool {
    (0..MAP_SIZE as i32).contains(&x) && (0..MAP_SIZE as i32).contains(&z)
}

/// `QDIST2`.
fn qdist2(x: i32, z: i32) -> i32 {
    if x > z { x + (z >> 1) } else { z + (x >> 1) }
}

/// Whether two segments cross.
fn crosses(p: Vec2, p2: Vec2, q: Vec2, q2: Vec2) -> bool {
    let (r, s) = (p2 - p, q2 - q);
    let denom = r.perp_dot(s);
    if denom.abs() < 1e-6 {
        return false;
    }
    let t = (q - p).perp_dot(s) / denom;
    let u = (q - p).perp_dot(r) / denom;
    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

/// How close anyone comes to a no-go square: `NOGO_COLLIDE_WIDTH`, 0x58 of a square.
const NOGO_WIDTH: f32 = 88.0;

/// Keeps someone at `p` out of the no-go squares (`PAP_FLAG_NOGO`) round the one they're
/// in, as `slide_along` does (collide.cpp:4111): no nearer than [`NOGO_WIDTH`] to a side
/// shared with one, or to a corner touching one diagonally. It doesn't look at height, so
/// a no-go square is a wall from the ground to the sky.
pub fn keep_off_nogo(map: &Map, p: Vec2) -> Vec2 {
    let (mx, mz) = ((p.x / 256.0).floor() as i32, (p.y / 256.0).floor() as i32);
    if !(1..MAP_SIZE as i32 - 1).contains(&mx) || !(1..MAP_SIZE as i32 - 1).contains(&mz) {
        return p;
    }
    let nogo = |dx: i32, dz: i32| map.square((mx + dx) as usize, (mz + dz) as usize).has(flags::NOGO);
    let base = vec2(mx as f32, mz as f32) * 256.0;
    let mut f = p - base;
    if nogo(-1, 0) { f.x = f.x.max(NOGO_WIDTH); }
    if nogo(1, 0) { f.x = f.x.min(256.0 - NOGO_WIDTH); }
    if nogo(0, -1) { f.y = f.y.max(NOGO_WIDTH); }
    if nogo(0, 1) { f.y = f.y.min(256.0 - NOGO_WIDTH); }
    // The corners, each on its own (the original lumps them together, which only matters
    // where two diagonal no-go squares meet the same square).
    for (dx, dz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        if !nogo(dx, dz) {
            continue;
        }
        let corner = vec2(if dx > 0 { 256.0 } else { 0.0 }, if dz > 0 { 256.0 } else { 0.0 });
        let d = (f - corner).abs();
        let dist = qdist2(d.x as i32, d.y as i32) as f32 + 1.0;
        if dist < NOGO_WIDTH {
            let away = d * (NOGO_WIDTH / dist);
            f = corner + vec2(if dx > 0 { -away.x } else { away.x }, if dz > 0 { -away.y } else { away.y });
        }
    }
    base + f
}

impl Mav {
    /// `MAV_calc_height_array` (warehouses included) and `MAV_precalculate` for a level.
    pub fn new(level: &Level) -> Mav {
        let height = heights(&level.map, level, false);
        let mut mav = Mav { height, opts: vec![[0; 4]; MAP_SIZE * MAP_SIZE] };
        mav.precalculate(&level.map, &level.ground, false);
        mav
    }

    /// The navigation map for getting about inside the warehouses (`BUILDING_TYPE_WAREHOUSE`),
    /// or None for a map without any: as [`Mav::new`], but with their roofs left off, so
    /// that what's under them is their floors (`MAV_calc_height_array(TRUE)`, which
    /// `WARE_init` gives `MAV_precalculate_warehouse_nav`). The original keeps a map like it
    /// for each warehouse and goes in and out by their doors; this one has the streets round
    /// them too, which come to the same thing.
    pub fn inside(level: &Level) -> Option<Mav> {
        level.map.buildings.iter().any(|b| b.kind == BUILDING_TYPE_WAREHOUSE).then(|| {
            let height = heights(&level.map, level, true);
            let mut mav = Mav { height, opts: vec![[0; 4]; MAP_SIZE * MAP_SIZE] };
            mav.precalculate(&level.map, &level.ground, true);
            mav
        })
    }

    /// `MAVHEIGHT`.
    pub fn height(&self, x: i32, z: i32) -> i8 {
        self.height[idx(x, z)]
    }

    /// The caps for leaving square (x, z) in direction `dir`.
    pub fn opt(&self, x: i32, z: i32, dir: usize) -> u8 {
        self.opts[idx(x, z)][dir]
    }

    fn precalculate(&mut self, map: &Map, ground: &Ground, inside: bool) {
        let hidden = |x: i32, z: i32| map.square(x as usize, z as usize).has(flags::HIDDEN);
        // `PAP_calc_map_height_at` at a square's middle, plus 0x50: the height a line of
        // sight is tested at. Inside, the floor under the warehouse's roof.
        let heights = self.height.clone();
        let eye = |x: i32, z: i32| {
            let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
            let top = if inside { ((heights[idx(x, z)] as i32) << 6) as f32 } else { f32::MAX / 2.0 };
            ground.surface(map, cx, cz, top) + 0x50 as f32
        };
        let middle = |x: i32, z: i32| vec2(x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
        // The walls (and closed doors) and climbable fences by the map squares their bounding
        // boxes cover, so a look from one square to the next only tries those near it.
        let walls: Vec<_> = ground.blocking().copied().collect();
        let index = |segs: &[uc_formats::facet::WallSegment]| {
            let mut grid: Vec<Vec<usize>> = vec![Vec::new(); MAP_SIZE * MAP_SIZE];
            for (k, w) in segs.iter().enumerate() {
                let (x0, x1) = (w.a[0].min(w.b[0]) as i32 >> 8, w.a[0].max(w.b[0]) as i32 >> 8);
                let (z0, z1) = (w.a[1].min(w.b[1]) as i32 >> 8, w.a[1].max(w.b[1]) as i32 >> 8);
                for x in x0.max(0)..=x1.min(MAP_SIZE as i32 - 1) {
                    for z in z0.max(0)..=z1.min(MAP_SIZE as i32 - 1) {
                        grid[idx(x, z)].push(k);
                    }
                }
            }
            grid
        };
        let (wall_grid, fence_grid) = (index(&walls), index(&ground.fences));
        let near = |grid: &[Vec<usize>], a: (i32, i32), b: (i32, i32)| {
            let mut out: Vec<usize> = Vec::new();
            for x in a.0.min(b.0)..=a.0.max(b.0) {
                for z in a.1.min(b.1)..=a.1.max(b.1) {
                    if on_map(x, z) {
                        out.extend(&grid[idx(x, z)]);
                    }
                }
            }
            out
        };
        // `there_is_a_los` from square to square at heights `ya`, `yb`: clear, or blocked only
        // by a climbable fence (for `CLIMB_OVER`).
        let los = |a: (i32, i32), b: (i32, i32), ya: f32, yb: f32| -> (bool, bool) {
            let (pa, pb) = (middle(a.0, a.1), middle(b.0, b.1));
            let r = pb - pa;
            let hit = near(&wall_grid, a, b).into_iter().any(|k| {
                let w = &walls[k];
                let (q, sv) = (Vec2::from(w.a), Vec2::from(w.b) - Vec2::from(w.a));
                let denom = r.perp_dot(sv);
                if denom.abs() < 1e-6 {
                    return false;
                }
                let t = (q - pa).perp_dot(sv) / denom;
                let u = (q - pa).perp_dot(r) / denom;
                let y = ya + (yb - ya) * t;
                (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) && y >= w.y0 && y <= w.y1
            });
            if !hit {
                return (true, false);
            }
            // Only a fence that can be scaled (mav.cpp:797): an unclimbable one is a wall.
            let fence = near(&fence_grid, a, b).into_iter().any(|k| {
                let f = &ground.fences[k];
                ground.fence_climb[k].scalable() && crosses(pa, pb, f.a.into(), f.b.into())
            });
            (false, fence)
        };
        for x in 0..MAP_SIZE as i32 {
            for z in 0..MAP_SIZE as i32 {
                // `find_nearby_ladder_colvect_radius`: a ladder within a square of the middle.
                let me = middle(x, z);
                let ladder = ground.ladders.iter().find(|l| (l.foot - me).length() < 256.0);
                let mut opt = [0u8; 4];
                for (i, &(dx, dz)) in DIRS.iter().enumerate() {
                    let (tx, tz) = (x + dx, z + dz);
                    if !on_map(tx, tz) {
                        continue;
                    }
                    let both_ground = !hidden(x, z) && !hidden(tx, tz);
                    let dh = self.height(tx, tz) as i32 - self.height(x, z) as i32;
                    if (!both_ground && dh.abs() > 1) || dh.abs() > 2 {
                        if dh < 0 {
                            let y = eye(x, z).max(eye(tx, tz));
                            let (clear, fence) = los((x, z), (tx, tz), y, y);
                            if clear {
                                opt[i] |= caps(FALL_OFF);
                            } else if fence {
                                opt[i] |= caps(CLIMB_OVER);
                            }
                        } else {
                            if (2..=4).contains(&dh) {
                                // Not with a fence along the edge (`does_fence_lie_along_line`).
                                let e1 = me + vec2((dx * 128 - dz * 128) as f32, (dz * 128 + dx * 128) as f32);
                                let e2 = me + vec2((dx * 128 + dz * 128) as f32, (dz * 128 - dx * 128) as f32);
                                let edge_mid = (e1 + e2) / 2.0;
                                let fenced = ground.fences.iter().any(|f| {
                                    let (a, b): (Vec2, Vec2) = (f.a.into(), f.b.into());
                                    let along = (b - a).normalize_or_zero();
                                    let off = edge_mid - a;
                                    off.perp_dot(along).abs() < 16.0 && (0.0..=(b - a).length()).contains(&off.dot(along))
                                });
                                if !fenced {
                                    opt[i] |= caps(PULLUP);
                                }
                            }
                            if let Some(l) = ladder {
                                // The ladder crosses the step, and reaches the bottom.
                                let side = vec2(-l.out.y, l.out.x) * 64.0;
                                if crosses(me, middle(tx, tz), l.foot - side, l.foot + side)
                                    && (l.base - (eye(x, z) - 0x50 as f32)).abs() <= 0x50 as f32 {
                                    opt[i] |= caps(LADDER_UP);
                                }
                            }
                        }
                    } else {
                        // The original looks 0x50 up, at what stops its own walker; here it's
                        // at this engine's walker's waist (`Ground::slide`), which is what
                        // stops it.
                        let waist = |x: i32, z: i32| eye(x, z) - 0x50 as f32 + crate::ground::STEP_HEIGHT - 4.0;
                        let w = waist(x, z).max(waist(tx, tz));
                        let (clear, fence) = los((x, z), (tx, tz), w, w);
                        if clear {
                            opt[i] |= caps(GOTO);
                        } else if fence {
                            opt[i] |= caps(CLIMB_OVER);
                        }
                    }
                    if opt[i] & (caps(GOTO) | caps(CLIMB_OVER)) == 0 {
                        // Jumping one square over?
                        let (tx, tz) = (tx + dx, tz + dz);
                        if on_map(tx, tz) {
                            let dh = self.height(tx, tz) as i32 - self.height(x, z) as i32;
                            let y1 = ((self.height(x, z) as i32) << 6) as f32 + 0xa0 as f32;
                            let y2 = ((self.height(tx, tz) as i32) << 6) as f32 + 0xa0 as f32;
                            if los((x, z), (tx, tz), y1, y2).0 {
                                if dh < 2 {
                                    opt[i] |= caps(JUMP);
                                } else if (2..=5).contains(&dh) {
                                    opt[i] |= caps(JUMPPULL);
                                }
                            }
                            // Two squares over?
                            let (tx, tz) = (tx + dx, tz + dz);
                            if on_map(tx, tz) {
                                let dh = self.height(tx, tz) as i32 - self.height(x, z) as i32;
                                if dh <= 4 && dh > -6 {
                                    let y2 = ((self.height(tx, tz) as i32) << 6) as f32 + 0xa0 as f32;
                                    if los((x, z), (tx, tz), y1, y2).0 {
                                        opt[i] |= caps(JUMPPULL2);
                                    }
                                }
                            }
                        }
                    }
                }
                self.opts[idx(x, z)] = opt;
            }
        }
        // No way into a square that's off limits (`PAP_FLAG_NOGO`: inside a building with
        // no roof), as `MAV_turn_off_whole_square` does.
        for x in 0..MAP_SIZE as i32 {
            for z in 0..MAP_SIZE as i32 {
                if map.square(x as usize, z as usize).has(flags::NOGO) || self.height(x, z) == 127 {
                    self.turn_off_whole_square(x, z);
                }
            }
        }
        // Nor into one too steep to stand on (`MAV_precalculate`, mav.cpp:1270): where
        // `PAP_on_slope` is over 100 a quarter of the way in from either corner, 50 as
        // `slope_at` halves it. A walker heading straight into one would slide back down it
        // for ever: Mr X's Getaway's Mr X, at the foot of the ramp up to his van.
        for x in 0..MAP_SIZE as i32 - 1 {
            for z in 0..MAP_SIZE as i32 - 1 {
                let steep = |dx: f32| crate::ground::slope_at(map, x as f32 * 256.0 + dx, z as f32 * 256.0 + dx).is_some_and(|(s, _)| s > 50.0);
                if steep(64.0) || steep(192.0) {
                    self.turn_off_whole_square(x, z);
                }
            }
        }
    }

    /// `MAV_turn_off_whole_square`: nothing gets into (x, z) from any side.
    fn turn_off_whole_square(&mut self, x: i32, z: i32) {
        for (i, &(dx, dz)) in DIRS.iter().enumerate() {
            let (mx, mz) = (x - dx, z - dz);
            if on_map(mx, mz) {
                self.opts[idx(mx, mz)][i] = 0;
            }
        }
    }

    /// `MAV_turn_movement_on`: walking out of (x, z) towards `dir` is all there is that way.
    fn turn_movement_on(&mut self, x: i32, z: i32, dir: usize) {
        if on_map(x, z) {
            self.opts[idx(x, z)][dir] = caps(GOTO);
        }
    }

    /// Opens the navigation map through a door that's been opened, from map point `a` to
    /// `b` (its facet's ends): each square along it, on either side, can be walked out of
    /// across it (`DOOR_open`, door.cpp:162).
    pub fn open_door(&mut self, a: (i32, i32), b: (i32, i32)) {
        // `MAV_DIR_*`.
        const XS: usize = 0;
        const XL: usize = 1;
        const ZS: usize = 2;
        const ZL: usize = 3;
        let (dx, dz) = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
        if (dx, dz) == (0, 0) {
            return;
        }
        let (left, right) = match (dx, dz) {
            (1, _) => (ZS, ZL),
            (-1, _) => (ZL, ZS),
            (_, 1) => (XL, XS),
            _ => (XS, XL),
        };
        let (mut x, mut z) = a;
        // A slanting one that never lands on its far end would go on for ever: no further
        // than across the map.
        for _ in 0..MAP_SIZE {
            // The squares either side of this stretch of it: half a square along, and half
            // a square off to one side, then the other.
            let (mx, mz) = ((x << 8) + (dx << 7), (z << 8) + (dz << 7));
            self.turn_movement_on((mx + (-dz << 7)) >> 8, (mz + (dx << 7)) >> 8, left);
            self.turn_movement_on((mx - (-dz << 7)) >> 8, (mz - (dx << 7)) >> 8, right);
            x += dx;
            z += dz;
            if (x, z) == b {
                break;
            }
        }
    }

    /// `MAV_can_i_walk`: a straight walk from square a to square b, crossing only squares
    /// that can be walked between (corners included on the diagonals).
    pub fn can_walk(&self, a: (u8, u8), b: (u8, u8)) -> bool {
        let (ax, az, bx, bz) = (a.0 as i32, a.1 as i32, b.0 as i32, b.1 as i32);
        let (mut dx, mut dz) = ((bx - ax) << 4, (bz - az) << 4);
        let dist = qdist2(dx.abs(), dz.abs());
        if dist == 0 {
            return true;
        }
        let over = 0x4000 / dist;
        dx = (dx * over) >> 8;
        dz = (dz * over) >> 8;
        let (mut lx, mut lz) = (ax, az);
        let (mut x, mut z) = ((ax << 8) + 0x80, (az << 8) + 0x80);
        let goto = |x: i32, z: i32, dir: usize| on_map(x, z) && self.opt(x, z, dir) & caps(GOTO) != 0;
        let dir_of = |d: i32, neg: usize| if d < 0 { neg } else { neg + 1 };
        for _ in 0..4096 {
            x += dx;
            z += dz;
            let (mx, mz) = (x >> 8, z >> 8);
            let (dmx, dmz) = (mx - lx, mz - lz);
            if dmx != 0 || dmz != 0 {
                if dmx != 0 && !goto(lx, lz, dir_of(dmx, 0)) {
                    return false;
                }
                if dmz != 0 && !goto(lx, lz, dir_of(dmz, 2)) {
                    return false;
                }
                if dmx != 0 && dmz != 0 && (!goto(mx, lz, dir_of(dmz, 2)) || !goto(lx, mz, dir_of(dmx, 0))) {
                    return false;
                }
                if mx == bx && mz == bz {
                    return true;
                }
                lx = mx;
                lz = mz;
            }
        }
        false
    }

    /// `MAV_do`: the first thing to do to get from square `from` towards `to`, with the
    /// moves `caps` allows; and whether a route all the way there was found
    /// (`MAV_do_found_dest`). The search looks up to 32 squares ahead, best-first by
    /// distance to go, and odd moves cost a little extra (`rand` adds the original's random
    /// 0 to 3 to them).
    pub fn route(&self, from: (u8, u8), to: (u8, u8), caps_allowed: u8, rand: &mut dyn FnMut() -> u32) -> (Action, bool) {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let score = |x: i32, z: i32| -> u8 {
            let (dx, dz) = (((to.0 as i32 - x) << 1).abs(), ((to.1 as i32 - z) << 1).abs());
            qdist2(dx, dz).min(255) as u8
        };
        // How each square was reached: (action, dir).
        let mut came: std::collections::HashMap<(i32, i32), (u8, u8)> = std::collections::HashMap::new();
        let mut heap = BinaryHeap::new();
        let start = (from.0 as i32, from.1 as i32);
        let mut order = 0u32;
        heap.push(Reverse((score(start.0, start.1), order, 0u8, start)));
        came.insert(start, (GOTO, 0));
        let mut answer = Action { action: GOTO, dir: 0, dest: from, first: from };
        let mut best_score = u8::MAX as u32 + 1;
        let mut overflows = 0;
        while let Some(Reverse((sc, _, length, (x, z)))) = heap.pop() {
            if length >= LOOKAHEAD {
                if (sc as u32) < best_score {
                    best_score = sc as u32;
                    answer = self.first_action(&came, from, (x, z));
                }
                overflows += 1;
                if overflows >= MAX_OVERFLOWS {
                    return (answer, false);
                }
                continue;
            }
            if (x, z) == (to.0 as i32, to.1 as i32) {
                return (self.first_action(&came, from, (x, z)), true);
            }
            for (i, &(dx, dz)) in DIRS.iter().enumerate() {
                let opt = self.opt(x, z, i) & caps_allowed;
                let one = opt & (caps(GOTO) | caps(PULLUP) | caps(CLIMB_OVER) | caps(FALL_OFF) | caps(LADDER_UP));
                let two = opt & (caps(JUMP) | caps(JUMPPULL));
                let three = opt & caps(JUMPPULL2);
                for (moves, steps) in [(one, 1), (two, 2), (three, 3)] {
                    if moves == 0 {
                        continue;
                    }
                    let (mx, mz) = (x + dx * steps, z + dz * steps);
                    if !on_map(mx, mz) || came.contains_key(&(mx, mz)) {
                        continue;
                    }
                    let action = [GOTO, PULLUP, CLIMB_OVER, FALL_OFF, LADDER_UP, JUMP, JUMPPULL, JUMPPULL2]
                        .into_iter().find(|&a| moves & caps(a) != 0).unwrap();
                    came.insert((mx, mz), (action, i as u8));
                    let mut s = score(mx, mz) as u32;
                    if steps == 1 && action != GOTO {
                        // A bias against strange ways of getting about, if less for falling.
                        s += rand() & 3;
                        if action != FALL_OFF {
                            s += 3;
                        }
                    }
                    order += 1;
                    heap.push(Reverse((s.min(255) as u8, order, length + 1, (mx, mz))));
                }
            }
        }
        (answer, false)
    }

    /// `MAV_create_nodelist_from_pos` and `MAV_get_first_action_from_nodelist`: back from
    /// `end` to `from`, then forwards: the first move that isn't a walk, or the furthest
    /// square of the route that can be walked to in a straight line.
    fn first_action(&self, came: &std::collections::HashMap<(i32, i32), (u8, u8)>, from: (u8, u8), end: (i32, i32)) -> Action {
        let mut nodes = Vec::new();
        let (mut x, mut z) = end;
        while (x, z) != (from.0 as i32, from.1 as i32) {
            let (action, dir) = came[&(x, z)];
            nodes.push((x, z, action, dir));
            let steps = match action {
                JUMP | JUMPPULL => 2,
                JUMPPULL2 => 3,
                _ => 1,
            };
            let (dx, dz) = DIRS[dir as usize];
            x -= dx * steps;
            z -= dz * steps;
        }
        let first = nodes.last().map_or(from, |&(x, z, ..)| (x as u8, z as u8));
        let mut ans = Action { action: GOTO, dir: 0, dest: from, first };
        for &(x, z, action, dir) in nodes.iter().rev() {
            if action != GOTO {
                return Action { action, dir, dest: ans.dest, first };
            }
            if self.can_walk(from, (x as u8, z as u8)) {
                ans.dest = (x as u8, z as u8);
            } else {
                return ans;
            }
        }
        ans
    }
}

/// `MAV_calc_height_array`: the ground at each square's middle, a rooftop's own height, the
/// highest walkable roof face over a building, and squares inside a building with no roof
/// off limits at the top of the scale; on an older map, its walkable faces' heights over
/// its buildings. With `ignore_warehouses`, the warehouses' roof faces are left out and the
/// ground is under them.
fn heights(map: &Map, level: &Level, ignore_warehouses: bool) -> Vec<i8> {
    let mut h = vec![0i8; MAP_SIZE * MAP_SIZE];
    for x in 0..MAP_SIZE {
        for z in 0..MAP_SIZE {
            let sq = map.square(x, z);
            h[x * MAP_SIZE + z] = if sq.has(flags::ROOF_EXISTS) {
                sq.height
            } else if sq.has(flags::HIDDEN) && !ignore_warehouses {
                -127
            } else {
                let y = map.ground_height(x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0) as i32;
                (y / 0x40).clamp(-127, 127) as i8
            };
        }
    }
    let warehouse = |b: u16| map.buildings.get(b as usize).is_some_and(|b| b.kind == BUILDING_TYPE_WAREHOUSE);
    for rf in map.roof_faces.iter().filter(|rf| !(ignore_warehouses && warehouse(rf.building))) {
        let (x, z) = ((rf.rx & 127) as usize, (rf.rz & 127) as usize);
        let y = (rf.y as i32 / 0x40).clamp(-127, 127) as i8;
        if h[x * MAP_SIZE + z] < y {
            h[x * MAP_SIZE + z] = y;
        }
    }
    // An older map's roofs are walkable prim faces rather than roof faces
    // (`uc_formats::iam::WalkablesLayout::Old`): a hidden square gets the height of the face
    // over its middle, as the original did with these before its roof faces (mav.cpp:254,
    // since commented out; the editor makes roof faces of them the same way,
    // supermap.cpp:1898). Without, each building is off limits, roof and all.
    for (w, f) in map.walkables.iter().flat_map(|w| w.faces.iter().map(move |f| (w, f))) {
        let p: Vec<[f32; 3]> = f.points.iter().map(|&i| w.points[i as usize].map(|v| v as f32)).collect();
        let tris: &[[usize; 3]] = if p.len() == 4 { &[[0, 1, 2], [1, 3, 2]] } else { &[[0, 1, 2]] };
        for &[a, b, c] in tris {
            let (a, b, c) = (p[a], p[b], p[c]);
            let (lo_x, hi_x) = (a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]));
            let (lo_z, hi_z) = (a[2].min(b[2]).min(c[2]), a[2].max(b[2]).max(c[2]));
            let det = (b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2]);
            if det.abs() < 1.0 {
                continue;
            }
            for x in (lo_x / 256.0).floor() as i32..=(hi_x / 256.0).floor() as i32 {
                for z in (lo_z / 256.0).floor() as i32..=(hi_z / 256.0).floor() as i32 {
                    if !on_map(x, z) || !map.square(x as usize, z as usize).has(flags::HIDDEN) {
                        continue;
                    }
                    let (mx, mz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
                    let u = ((mx - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (mz - a[2])) / det;
                    let v = ((b[0] - a[0]) * (mz - a[2]) - (mx - a[0]) * (b[2] - a[2])) / det;
                    if u < 0.0 || v < 0.0 || u + v > 1.0 {
                        continue;
                    }
                    let y = a[1] + u * (b[1] - a[1]) + v * (c[1] - a[1]);
                    let y = ((y as i32) / 0x40).clamp(-127, 127) as i8;
                    if h[idx(x, z)] < y {
                        h[idx(x, z)] = y;
                    }
                }
            }
        }
    }
    // The staircase prim (41) sets the height of the square under its middle to its own.
    if let Some(prim) = crate::level::load_prim(&level.game, 41u8) {
        let (lo, hi) = crate::level::prim_bounds(&prim);
        let mid = vec2(lo.x + hi.x, lo.z + hi.z) / 2.0;
        for o in map.objects.iter().filter(|o| o.prim == 41) {
            let a = radians(o.yaw as f32);
            let (s, c) = a.sin_cos();
            let (rx, rz) = (o.x as f32 + c * mid.x + s * mid.y, o.z as f32 - s * mid.x + c * mid.y);
            let (mx, mz) = ((rx / 256.0).floor() as i32, (rz / 256.0).floor() as i32);
            if on_map(mx, mz) {
                h[idx(mx, mz)] = ((o.y + 0x40) >> 6).clamp(-127, 127) as i8;
            }
        }
    }
    for x in 0..MAP_SIZE {
        for z in 0..MAP_SIZE {
            if h[x * MAP_SIZE + z] <= -127 && map.square(x, z).has(flags::HIDDEN) {
                h[x * MAP_SIZE + z] = 127;
            }
        }
    }
    h
}
