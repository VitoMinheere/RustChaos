//! Cloth and flags (fallen/Source/cloth.cpp): a sheet of six by five points, each tied to
//! its eight neighbours by links that pull when stretched, some points pinned in place,
//! the rest falling and, on a flag, blown about by a gusting wind. Nothing in the shipped
//! game makes any: the one caller, a debug key that hung a pink flag in front of Darci
//! (Controls.cpp:4070), is commented out, and so are `CLOTH_process`'s body (`#if 0`), its
//! call (Controls.cpp:2908) and the drawing (aeng.cpp:11290). No map places a flag. Here
//! it's an extra, `flag` in the console, hung the way the debug key hung it with Ctrl
//! held: by its two left corners, as on a pole.
//!
//! A step is three goes round the links, once a turn of twenty to the second, in world
//! units. The original keeps points from going below a height of 2, wherever they are on
//! the map; here it's 2 above the ground under them. Its wind gusts by `rand()`, which here
//! is the cloth's own generator.

use glam::{vec3, Vec3};

/// `CLOTH_WIDTH`, `CLOTH_HEIGHT`.
pub const WIDTH: usize = 6;
pub const HEIGHT: usize = 5;
const POINTS: usize = WIDTH * HEIGHT;

/// Steps a second: `CLOTH_process` once a turn.
pub const STEPS_PER_SECOND: f32 = 20.0;

/// `CLOTH_elasticity`, `_damping`, `_gravity`.
const ELASTICITY: f32 = 0.0003;
const DAMPING: f32 = 0.95;
const GRAVITY: f32 = -0.15;
/// The wind on a flag, before its gusts.
const WIND: Vec3 = Vec3::new(-0.2, 0.0, 0.25);

/// `CLOTH_INDEX`.
pub fn index(w: usize, h: usize) -> usize {
    w + h * WIDTH
}

/// What cloth needs to know about the world.
pub trait Ground {
    /// The floor's height under (x, z) for a point at height `y`.
    fn height(&self, x: f32, z: f32, y: f32) -> f32;
}

/// `CLOTH_TYPE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Flag,
    Cloak,
}

/// `CLOTH_Link`: two points, and whether it's a diagonal (`CLOTH_LINK_TYPE_HYP`).
#[derive(Debug, Clone, Copy)]
struct Link {
    a: usize,
    b: usize,
    diagonal: bool,
}

/// The links between neighbours, every one both ways round as the original's `CLOTH_init`
/// makes them.
fn links() -> Vec<Link> {
    let mut out = Vec::new();
    for x in 0..WIDTH as i32 {
        for y in 0..HEIGHT as i32 {
            for (dx, dy) in (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (dx, dy))).filter(|&d| d != (0, 0)) {
                let (x2, y2) = (x + dx, y + dy);
                if (0..WIDTH as i32).contains(&x2) && (0..HEIGHT as i32).contains(&y2) {
                    out.push(Link { a: index(x as usize, y as usize), b: index(x2 as usize, y2 as usize), diagonal: dx != 0 && dy != 0 });
                }
            }
        }
    }
    out
}

/// `CLOTH_Cloth`.
#[derive(Debug, Clone)]
pub struct Cloth {
    pub kind: Kind,
    /// 0xRRGGBB.
    pub colour: u32,
    /// Pinned points, a bit each.
    lock: u32,
    /// The square of how far apart neighbours like to be, along and across.
    dist: [f32; 2],
    pub pos: [Vec3; POINTS],
    vel: [Vec3; POINTS],
}

/// All the cloth.
#[derive(Debug, Clone, Default)]
pub struct Cloths {
    pub cloths: Vec<Cloth>,
    links: Vec<Link>,
    /// `GAME_TURN`, `CLOTH_process`'s own count of goes round, for the gusts.
    turn: u32,
    seed: u32,
    clock: f32,
}

impl Cloths {
    /// `CLOTH_create` (cloth.cpp:277): a sheet with its first point at `origin`, each point
    /// along a row `along` from the last and each row `up` from the last, neighbours liking
    /// to be `dist` apart. At most fifteen at once, as the original.
    pub fn create(&mut self, kind: Kind, origin: Vec3, along: Vec3, up: Vec3, dist: f32, colour: u32) -> Option<usize> {
        if self.cloths.len() >= 15 {
            return None;
        }
        if self.links.is_empty() {
            self.links = links();
        }
        let pos = std::array::from_fn(|i| origin + along * (i % WIDTH) as f32 + up * (i / WIDTH) as f32);
        self.cloths.push(Cloth { kind, colour, lock: 0, dist: [dist * dist, dist * dist * 2.0], pos, vel: [Vec3::ZERO; POINTS] });
        Some(self.cloths.len() - 1)
    }

    /// `CLOTH_point_lock`: pins point (w, h) of cloth `c` where it is.
    pub fn lock(&mut self, c: usize, w: usize, h: usize) {
        self.cloths[c].lock |= 1 << index(w, h);
    }

    /// `rand()`, as MSVC's: 0 to 32767.
    fn rand(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(214013).wrapping_add(2531011);
        (self.seed >> 16) & 0x7fff
    }

    /// Runs `dt` seconds of steps.
    pub fn update(&mut self, dt: f32, ground: &impl Ground) {
        if self.cloths.is_empty() {
            self.clock = 0.0;
            return;
        }
        self.clock += dt;
        while self.clock >= 1.0 / STEPS_PER_SECOND {
            self.clock -= 1.0 / STEPS_PER_SECOND;
            self.step(ground);
        }
    }

    /// `CLOTH_process` (cloth.cpp:449): one step.
    pub fn step(&mut self, ground: &impl Ground) {
        for c in 0..self.cloths.len() {
            for _ in 0..3 {
                self.turn += 1;
                // Flags flap in a wind that gusts.
                if self.cloths[c].kind == Kind::Flag {
                    let t = self.turn as f32;
                    let upndown = ((t * 0.05).sin().abs() * ((t * 0.023).sin().abs() + 0.5) + 2.5) * 0.2;
                    for j in 0..POINTS {
                        if self.cloths[c].lock & (1 << j) == 0 {
                            let change = (self.rand() as f32 * (0.25 / 32767.0) + 0.75) * upndown;
                            self.cloths[c].vel[j] += WIND * change;
                        }
                    }
                }
                let cc = &mut self.cloths[c];
                // Stretched links pull their ends together.
                for l in &self.links {
                    let want = cc.dist[l.diagonal as usize];
                    let d = cc.pos[l.b] - cc.pos[l.a];
                    let dist = d.length_squared();
                    if dist > want {
                        let f = d * ((dist - want) * ELASTICITY);
                        cc.vel[l.a] += f;
                        cc.vel[l.b] -= f;
                    }
                }
                for j in 0..POINTS {
                    cc.vel[j] *= DAMPING;
                    if cc.lock & (1 << j) != 0 {
                        cc.vel[j] = Vec3::ZERO;
                    }
                }
                for j in 0..POINTS {
                    cc.pos[j] += cc.vel[j];
                    cc.vel[j].y += GRAVITY;
                    let floor = ground.height(cc.pos[j].x, cc.pos[j].z, cc.pos[j].y) + 2.0;
                    if cc.pos[j].y < floor {
                        cc.pos[j].y = floor;
                        cc.vel[j].y = 0.0;
                    }
                }
            }
        }
    }
}

impl Cloth {
    /// `CLOTH_get_info`'s normal at point `i` (cloth.cpp:706): the average of the cross
    /// products round it, near enough unit length (`qdist`).
    pub fn normal(&self, i: usize) -> Vec3 {
        // Right, down, left, up, as `CLOTH_init`'s `order`.
        const ORDER: [(i32, i32); 4] = [(1, 0), (0, -1), (-1, 0), (0, 1)];
        let (x, y) = ((i % WIDTH) as i32, (i / WIDTH) as i32);
        let at = |(dx, dy): (i32, i32)| {
            let (x, y) = (x + dx, y + dy);
            ((0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y)).then(|| self.pos[index(x as usize, y as usize)])
        };
        let (mut n, mut num) = (Vec3::ZERO, 0.0);
        for k in 0..4 {
            if let (Some(a), Some(b)) = (at(ORDER[k]), at(ORDER[(k + 1) & 3])) {
                n += (a - self.pos[i]).cross(b - self.pos[i]);
                num += 1.0;
            }
        }
        let n = n / num;
        let a = n.abs();
        let m = a.max_element();
        let len = m + (a.x + a.y + a.z - m) * 0.2941;
        if len > 0.0 { n / len } else { vec3(0.0, 1.0, 0.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flat;
    impl Ground for Flat {
        fn height(&self, _: f32, _: f32, _: f32) -> f32 {
            0.0
        }
    }

    /// The debug key's flag: 16 apart, pink, 0x80 up.
    fn flag(cs: &mut Cloths) -> usize {
        let c = cs.create(Kind::Flag, vec3(1000.0, 128.0, 1000.0), vec3(16.0, 0.0, 0.0), vec3(0.0, 16.0, 0.0), 16.0, 0xff8888).unwrap();
        cs.lock(c, 0, 0);
        cs.lock(c, 0, 4);
        c
    }

    #[test]
    fn a_pinned_flag_flaps_and_stays_on() {
        let mut cs = Cloths::default();
        let c = flag(&mut cs);
        let pins = (cs.cloths[c].pos[index(0, 0)], cs.cloths[c].pos[index(0, 4)]);
        let mut seen = Vec::new();
        for _ in 0..200 {
            cs.step(&Flat);
            seen.push(cs.cloths[c].pos[index(5, 2)]);
        }
        let cl = &cs.cloths[c];
        assert_eq!((cl.pos[index(0, 0)], cl.pos[index(0, 4)]), pins, "pins moved");
        for p in cl.pos {
            assert!(p.is_finite() && p.y >= 2.0);
            // Held on: nowhere further from the pole than a stretched flag's width.
            assert!((p - pins.0).length() < 200.0, "torn off: {p}");
        }
        // The wind keeps it moving.
        let last = &seen[150..];
        let spread = last.iter().map(|p| (*p - last[0]).length()).fold(0.0, f32::max);
        assert!(spread > 1.0, "still");
        assert!(cl.normal(index(2, 2)).length() > 0.5);
    }

    #[test]
    fn an_unpinned_cloth_falls_to_the_ground() {
        let mut cs = Cloths::default();
        let c = cs.create(Kind::Cloak, vec3(0.0, 300.0, 0.0), vec3(16.0, 0.0, 0.0), vec3(0.0, 0.0, 16.0), 16.0, 0x8888ff).unwrap();
        for _ in 0..200 {
            cs.step(&Flat);
        }
        assert!(cs.cloths[c].pos.iter().all(|p| (p.y - 2.0).abs() < 1.0), "{:?}", cs.cloths[c].pos[0]);
    }
}
