//! Barrels on the move (fallen/Source/barrel.cpp): a barrel that's been hit is two spheres a
//! barrel's length apart along its axis, each falling, damped and knocked about on its own
//! (`BARREL_process_sphere`), bounced off the ground, walls and other barrels, and pulled
//! back to the right distance apart (`BARREL_push_apart`). The barrel is drawn between them,
//! along the line joining them. Once both have hardly moved for 64 turns it's still again
//! (`BARREL_convert_moving_to_stationary`). A barrel that isn't moving is hit as if it were
//! the two spheres it would be (`BARREL_stacked_sphere`), and starts moving if they're hit.
//!
//! It runs a turn (a twentieth of a second) at a time, in the original's units: positions
//! in 256ths of a world unit.

// The index loops follow the original's, over x, y and z and over other barrels.
#![allow(clippy::needless_range_loop)]

use uc_formats::units::radians;
use crate::Barrel;
use glam::{Vec2, Vec3};

pub const TICK: f32 = 1.0 / 20.0;

/// `BARREL_HEIGHT`, `_SPHERE_RADIUS`, `_SPHERE_DIST`, `_GRAVITY`.
pub const HEIGHT: f32 = 93.0;
const SPHERE_RADIUS: i64 = 85;
const SPHERE_DIST: i64 = 50;
const GRAVITY: i64 = 0x80;

/// `BARREL_Sphere`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub p: [i64; 3],
    pub d: [i64; 3],
    /// Turns spent hardly moving.
    still: i32,
    radius: i64,
}

/// What the barrels need from the world.
pub trait World {
    /// The ground or roof under (x, z) at height `y` (`PAP_calc_map_height_at`).
    fn height(&self, x: f32, z: f32, y: f32) -> f32;
    /// A sphere's move from `from` to `to` (xz) kept off walls and fences.
    fn slide(&self, from: Vec2, to: Vec2, y: f32) -> Vec2;
}

/// `QDIST3`: the largest axis plus a quarter of each of the other two.
pub(crate) fn qdist3(x: i64, y: i64, z: i64) -> i64 {
    let (x, y, z) = (x.abs(), y.abs(), z.abs());
    let m = x.max(y).max(z);
    m + (x + y + z - m) / 4
}

fn to_world(p: [i64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 256.0
}

fn to_fixed(v: Vec3) -> [i64; 3] {
    [(v.x * 256.0) as i64, (v.y * 256.0) as i64, (v.z * 256.0) as i64]
}

impl Barrel {
    /// The way up its axis, from its yaw and tilt (as it's drawn).
    fn axis(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (st, ct) = self.tilt.sin_cos();
        Vec3::new(sy * st, ct, cy * st)
    }

    /// Its middle (its position is its base).
    pub fn middle(&self) -> Vec3 {
        self.pos + self.axis() * (HEIGHT / 2.0)
    }

    /// `BARREL_stacked_sphere`: the two spheres it would be if it moved, a little either
    /// side of its middle along its axis; a cone's are smaller, the top one most.
    fn stacked_spheres(&self) -> [Sphere; 2] {
        let mid = self.middle();
        let half = self.axis() * (SPHERE_DIST as f32 / 2.0);
        let r = SPHERE_RADIUS << 8;
        let cone = self.kind == 1;
        [
            Sphere { p: to_fixed(mid - half), d: [0; 3], still: 0, radius: if cone { r - (r >> 2) } else { r } },
            Sphere { p: to_fixed(mid + half), d: [0; 3], still: 0, radius: if cone { r >> 1 } else { r } },
        ]
    }

    /// `BARREL_convert_stationary_to_moving`: off whatever it stood on, as two spheres, with
    /// a little upward jolt (`Random() & 0xff`).
    fn start_moving(&mut self, jolt: [i64; 2]) {
        let mut s = self.stacked_spheres();
        for (k, sp) in s.iter_mut().enumerate() {
            sp.d[1] = jolt[k];
        }
        self.spheres = Some(s);
        self.on = None;
    }

    /// Back to a pose from its spheres: in the middle of them and along the line joining
    /// them, the yaw kept when it's nearly upright.
    fn pose_from_spheres(&mut self) {
        let Some([a, b]) = self.spheres else { return };
        let (pa, pb) = (to_world(a.p), to_world(b.p));
        let axis = (pb - pa).normalize_or(Vec3::Y);
        let flat = Vec2::new(axis.x, axis.z).length();
        if flat * SPHERE_DIST as f32 >= 1.0 {
            self.yaw = axis.x.atan2(axis.z);
        }
        self.tilt = axis.y.clamp(-1.0, 1.0).acos();
        self.pos = (pa + pb) / 2.0 - axis * (HEIGHT / 2.0);
    }
}

/// `BARREL_hit_with_sphere`: everything in the sphere at `at` (world units) of `radius`.
/// A still barrel whose spheres would be in it starts moving; a moving one's spheres are
/// pushed out of it, and away. Returns the barrels it hit.
pub fn hit_with_sphere(barrels: &mut [Barrel], at: Vec3, radius: f32, rand: &mut dyn FnMut() -> u32) -> Vec<usize> {
    let (c, r) = (to_fixed(at), (radius * 256.0) as i64);
    let mut hit = Vec::new();
    for (k, b) in barrels.iter_mut().enumerate() {
        if b.gone {
            continue;
        }
        if b.spheres.is_none() {
            let reached = b.stacked_spheres().iter().any(|s| qdist3(s.p[0] - c[0], s.p[1] - c[1], s.p[2] - c[2]) < r + s.radius);
            if !reached {
                continue;
            }
            b.start_moving([(rand() & 0xff) as i64, (rand() & 0xff) as i64]);
        }
        let Some(spheres) = b.spheres.as_mut() else { continue };
        let mut any = false;
        for s in spheres.iter_mut() {
            let d = [s.p[0] - c[0], s.p[1] - c[1], s.p[2] - c[2]];
            let dist = qdist3(d[0], d[1], d[2]) + 1;
            let ddist = r + s.radius - dist;
            if ddist > 0 {
                for i in 0..3 {
                    let push = d[i] * ddist / dist;
                    s.p[i] += push;
                    s.d[i] += push / 8;
                }
                any = true;
            }
        }
        if any {
            hit.push(k);
        }
    }
    hit
}

/// `BARREL_process_normal` for every barrel, one turn: those stacked on one that's moved or
/// gone start moving, and the moving ones move. Returns those that knocked into something
/// hard enough to be heard (`BARREL_hit_noise`).
pub fn process(barrels: &mut [Barrel], world: &impl World, rand: &mut dyn FnMut() -> u32) -> Vec<usize> {
    let mut noisy = Vec::new();
    // On a barrel that's moved or gone: off it comes.
    for k in 0..barrels.len() {
        if let Some(j) = barrels[k].on {
            if barrels[j].gone || barrels[j].spheres.is_some() {
                barrels[k].start_moving([0, 0]);
            }
        }
    }
    for k in 0..barrels.len() {
        if barrels[k].gone || barrels[k].spheres.is_none() {
            continue;
        }
        let mut spheres = barrels[k].spheres.unwrap();
        let mut grounded = false;
        let mut noise = false;
        for s in spheres.iter_mut() {
            // Gravity and damping.
            for i in 0..3 {
                s.d[i] -= s.d[i] / 32;
            }
            s.d[1] -= GRAVITY;
            let old = (s.p[0], s.p[2]);
            for i in 0..3 {
                s.p[i] += s.d[i];
            }
            // The other barrels: a still one this sphere is in starts moving; a moving one's
            // spheres and this one push each other apart.
            for j in 0..barrels.len() {
                if j == k || barrels[j].gone {
                    continue;
                }
                let near = to_world(s.p).distance(barrels[j].middle()) < HEIGHT * 2.0;
                if !near {
                    continue;
                }
                if barrels[j].spheres.is_none() {
                    let inside = barrels[j].stacked_spheres().iter()
                        .any(|o| qdist3(o.p[0] - s.p[0], o.p[1] - s.p[1], o.p[2] - s.p[2]) + 1 < o.radius);
                    if !inside {
                        continue;
                    }
                    barrels[j].start_moving([(rand() & 0xff) as i64, (rand() & 0xff) as i64]);
                }
                let Some(others) = barrels[j].spheres.as_mut() else { continue };
                for o in others.iter_mut() {
                    let d = [o.p[0] - s.p[0], o.p[1] - s.p[1], o.p[2] - s.p[2]];
                    let dist = qdist3(d[0], d[1], d[2]) + 1;
                    if dist < s.radius {
                        let ddist = (s.radius - dist) / 4;
                        for i in 0..3 {
                            let push = d[i] * ddist / dist;
                            s.p[i] -= push;
                            o.p[i] += push;
                        }
                        noise = true;
                    }
                }
            }
            // The ground: a quarter of the sphere's radius under its middle.
            let w = to_world(s.p);
            let ground = ((world.height(w.x, w.z, w.y) * 256.0) as i64) + (s.radius >> 2);
            if s.p[1] < ground {
                if s.d[1] < -1000 {
                    noise = true;
                }
                if s.p[1] < ground - 0x8000 || s.d[1] > 0 {
                    // A wall, not the ground: back, and bounce off it.
                    s.p[0] -= s.d[0];
                    s.p[2] -= s.d[2];
                    s.d[0] = -s.d[0];
                    s.d[2] = -s.d[2];
                } else {
                    s.p[1] = ground;
                    s.d = [0, s.d[1].abs() >> 1, 0];
                    grounded = true;
                }
            }
            // Walls and fences (the original keeps it off the sides of squares the
            // navigation map won't walk off; here the walls themselves).
            let from = Vec2::new(old.0 as f32, old.1 as f32) / 256.0;
            let to = Vec2::new(s.p[0] as f32, s.p[2] as f32) / 256.0;
            let at = world.slide(from, to, to_world(s.p).y);
            if (at - to).length() > 0.5 {
                s.p[0] = (at.x * 256.0) as i64;
                s.p[2] = (at.y * 256.0) as i64;
                s.d[0] = -(s.d[0] >> 1);
                s.d[2] = -(s.d[2] >> 1);
            }
            if s.d[0].abs() + s.d[1].abs() + s.d[2].abs() < 0x200 {
                s.still += 1;
            } else {
                s.still = 0;
            }
        }
        // `BARREL_push_apart`: back to the right distance apart, and pushed apart outright if
        // dangerously close.
        let [mut a, mut b] = spheres;
        let d = [b.p[0] - a.p[0], b.p[1] - a.p[1], b.p[2] - a.p[2]];
        let dist = qdist3(d[0], d[1], d[2]) + 1;
        let ddist = ((SPHERE_DIST << 8) - dist) / 4;
        for i in 0..3 {
            let push = d[i] * ddist / dist;
            a.d[i] -= push;
            b.d[i] += push;
            if ddist > (SPHERE_DIST << 7) / 8 {
                a.p[i] -= push * 4;
                b.p[i] += push * 4;
            }
        }
        // Standing up: grounded and nearly upright, the spheres are drawn together across
        // the ground. Cones and bins only stand up the right way up.
        let bar = &mut barrels[k];
        bar.spheres = Some([a, b]);
        bar.pose_from_spheres();
        if grounded {
            let off_upright = bar.tilt.min(std::f32::consts::PI - bar.tilt);
            let upside_down = bar.tilt > std::f32::consts::FRAC_PI_2;
            if off_upright < radians(224.0) && !(matches!(bar.kind, 1 | 2 | 4) && upside_down) {
                let (dx, dz) = ((b.p[0] - a.p[0]) / 4, (b.p[2] - a.p[2]) / 4);
                a.p[0] += dx;
                a.p[2] += dz;
                b.p[0] -= dx;
                b.p[2] -= dz;
                for s in [&mut a, &mut b] {
                    s.d[0] /= 2;
                    s.d[2] /= 2;
                }
                bar.spheres = Some([a, b]);
                bar.pose_from_spheres();
            }
        }
        // Still long enough: no more moving for now.
        if a.still > 64 && b.still > 64 {
            bar.spheres = None;
        }
        if noise {
            noisy.push(k);
        }
    }
    noisy
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat ground with a wall along x = 0.
    struct Flat;

    impl World for Flat {
        fn height(&self, _: f32, _: f32, _: f32) -> f32 {
            0.0
        }
        fn slide(&self, _: Vec2, to: Vec2, _: f32) -> Vec2 {
            Vec2::new(to.x.max(30.0), to.y)
        }
    }

    fn barrel(x: f32, z: f32) -> Barrel {
        Barrel { kind: 0, prim: 141, explodes: true, way: 0, gone: false, pos: Vec3::new(x, 0.0, z), yaw: 0.0, tilt: 0.0, on: None, spheres: None }
    }

    fn run(barrels: &mut [Barrel], turns: usize) {
        let mut seed = 5u32;
        let mut rand = || { seed = seed.wrapping_mul(69069).wrapping_add(1); seed >> 16 };
        for _ in 0..turns {
            process(barrels, &Flat, &mut rand);
        }
    }

    #[test]
    fn untouched_it_stands_still() {
        let mut b = vec![barrel(1000.0, 1000.0)];
        run(&mut b, 100);
        assert!(b[0].spheres.is_none());
        assert_eq!(b[0].pos, Vec3::new(1000.0, 0.0, 1000.0));
    }

    /// Knocked from the side it's sent away, lies down or stands, and comes to rest on the
    /// ground, still again.
    #[test]
    fn knocked_it_moves_and_settles() {
        let mut b = vec![barrel(1000.0, 1000.0)];
        let mut rand = || 0;
        let hit = hit_with_sphere(&mut b, Vec3::new(1000.0 - 100.0, 40.0, 1000.0), 0x50 as f32, &mut rand);
        assert_eq!(hit, vec![0]);
        assert!(b[0].spheres.is_some());
        run(&mut b, 400);
        assert!(b[0].spheres.is_none(), "still moving after 20 seconds");
        assert!(b[0].pos.x > 1000.0 + 20.0, "pushed only to {}", b[0].pos.x);
        let mid = b[0].middle();
        assert!(mid.y > 10.0 && mid.y < HEIGHT, "resting with its middle at {}", mid.y);
    }

    /// A barrel on top of another falls when the one under it is knocked away.
    #[test]
    fn the_one_on_top_falls() {
        let mut b = vec![barrel(1000.0, 1000.0), barrel(1000.0, 1000.0)];
        b[1].pos.y = HEIGHT;
        b[1].on = Some(0);
        let mut rand = || 0;
        hit_with_sphere(&mut b, Vec3::new(1000.0 - 100.0, 20.0, 1000.0), 0x50 as f32, &mut rand);
        run(&mut b, 400);
        assert!(b[1].on.is_none());
        assert!(b[1].middle().y < HEIGHT, "still up at {}", b[1].middle().y);
    }

    /// Knocked into a wall, it stays on its side of it.
    #[test]
    fn walls_stop_it() {
        let mut b = vec![barrel(100.0, 1000.0)];
        let mut rand = || 0;
        hit_with_sphere(&mut b, Vec3::new(300.0, 40.0, 1000.0), 0xa0 as f32, &mut rand);
        run(&mut b, 400);
        assert!(b[0].pos.x > 0.0, "through the wall to {}", b[0].pos.x);
    }
}
