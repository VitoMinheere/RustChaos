//! Sphere-matter (fallen/Source/sm.cpp), "Hypermatter version 2!": an object as a cube of
//! spheres held together by springs, each sphere falling, bouncing off the ground and
//! dragging on the air. The shipped game never makes one: its one caller, a debug key that
//! drops a cube in front of Darci (Controls.cpp:4233), is commented out, though
//! `SM_process` still runs eight times a turn and the engine still draws whatever there
//! is (aeng.cpp:11219). Here it's an extra, `sm` in the console.
//!
//! It runs in the original's own units: positions in 256ths of a world unit, velocities in
//! 256ths of those a step, at eight steps a turn of twenty to the second. `MUL64` and
//! `DIV64` are its 16.16 multiply and divide.
//!
//! Two slips of the original's are kept: the spheres' z positions step by the cube's x size
//! (`sz = cz1 + k * dx`), and each object's springs index the spheres from the first of all
//! of them rather than its own, which only works for the first object; and every object
//! runs every spring. The debug key clears them all before making one, so it never shows.

use glam::{vec3, Vec3};
use crate::barrel::qdist3;

/// Steps a second: `SM_process` eight times a twentieth of a second (Controls.cpp:2981).
pub const STEPS_PER_SECOND: f32 = 160.0;

/// `SM_GRAVITY`.
const GRAVITY: i64 = -0x200;

/// `MUL64`: 16.16 multiply.
fn mul64(a: i64, b: i64) -> i64 {
    (a * b) >> 16
}

/// `DIV64`: 16.16 divide.
fn div64(a: i64, b: i64) -> i64 {
    if b == 0 { 0 } else { (a << 16) / b }
}

/// What sphere-matter needs to know about the world.
pub trait Ground {
    /// `PAP_calc_height_at`, world units: the floor under (x, z) for something whose bottom
    /// is at `y` (indoors, the floor rather than the roof).
    fn height(&self, x: f32, z: f32, y: f32) -> f32;
}

/// `SM_Sphere`.
#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub pos: [i64; 3],
    vel: [i64; 3],
    pub radius: i64,
    mass: i64,
}

/// `SM_Link`: two spheres and the square of how far apart they like to be.
#[derive(Debug, Clone, Copy)]
struct Link {
    a: usize,
    b: usize,
    dist: i64,
}

/// `SM_Object`: its springs, and how stiff they are.
#[derive(Debug, Clone, Copy)]
struct Object {
    jellyness: i64,
}

/// All the sphere-matter.
#[derive(Debug, Clone, Default)]
pub struct SphereMatter {
    pub spheres: Vec<Sphere>,
    links: Vec<Link>,
    objects: Vec<Object>,
    /// Time not yet run, in seconds.
    clock: f32,
}

impl SphereMatter {
    /// `SM_init`: none of it.
    pub fn clear(&mut self) {
        *self = SphereMatter::default();
    }

    /// `SM_create_cube`: a cube from `lo` to `hi` (world units) of `resolution` spheres a
    /// side, sharing the mass `density` gives the volume, its springs as stiff as
    /// `jellyness` (all 16.16; the original works out ranges for these, then uses what it's
    /// given).
    pub fn create_cube(&mut self, lo: Vec3, hi: Vec3, resolution: i64, density: i64, jellyness: i64) {
        let fix = |v: f32| (v as i64) << 8;
        let (c1, c2) = ([fix(lo.x), fix(lo.y), fix(lo.z)], [fix(hi.x), fix(hi.y), fix(hi.z)]);
        let (dx, dy, dz) = (c2[0] - c1[0], c2[1] - c1[1], c2[2] - c1[2]);
        let volume = mul64(mul64(dx, dy), dz);
        let mass = mul64(volume, density) / (resolution * resolution * resolution);
        let (dx, dy) = (dx / resolution, dy / resolution);
        let dz = dz / resolution;
        let radius = qdist3((dx >> 1).abs(), (dy >> 1).abs(), (dz >> 1).abs());
        self.objects.push(Object { jellyness });
        let at = |i: i64, j: i64, k: i64| [c1[0] + i * dx + (dx >> 1), c1[1] + j * dy + (dy >> 1), c1[2] + k * dx + (dx >> 1)];
        let index = |i: i64, j: i64, k: i64| (k + j * resolution + i * resolution * resolution) as usize;
        for i in 0..resolution {
            for j in 0..resolution {
                for k in 0..resolution {
                    let s = at(i, j, k);
                    self.spheres.push(Sphere { pos: s, vel: [0; 3], radius, mass });
                    // Springs to the spheres next along, and itself (a spring of no length).
                    for (di, dj, dk) in (0..8).map(|n| (n >> 2 & 1, n >> 1 & 1, n & 1)) {
                        let (i2, j2, k2) = (i + di, j + dj, k + dk);
                        if i2 >= resolution || j2 >= resolution || k2 >= resolution {
                            continue;
                        }
                        let s2 = at(i2, j2, k2);
                        let d = [s2[0] - s[0], s2[1] - s[1], s2[2] - s[2]];
                        let dist = mul64(d[0], d[0]) + mul64(d[1], d[1]) + mul64(d[2], d[2]);
                        self.links.push(Link { a: index(i, j, k), b: index(i2, j2, k2), dist });
                    }
                }
            }
        }
    }

    /// Runs `dt` seconds of steps.
    pub fn update(&mut self, dt: f32, ground: &impl Ground) {
        if self.spheres.is_empty() {
            self.clock = 0.0;
            return;
        }
        self.clock += dt;
        while self.clock >= 1.0 / STEPS_PER_SECOND {
            self.clock -= 1.0 / STEPS_PER_SECOND;
            self.step(ground);
        }
    }

    /// `SM_process` (sm.cpp:298): one step.
    pub fn step(&mut self, ground: &impl Ground) {
        for s in &mut self.spheres {
            s.vel[1] += GRAVITY;
            for k in 0..3 {
                s.pos[k] += s.vel[k] / 256;
            }
            // Off the ground: bounce, losing half the speed along it.
            let bottom = ((s.pos[1] - s.radius) >> 8) as f32;
            let height = (ground.height((s.pos[0] >> 8) as f32, (s.pos[2] >> 8) as f32, bottom) as i64) << 8;
            if s.pos[1] - s.radius < height {
                s.pos[1] = s.radius + height;
                s.vel[1] = -s.vel[1].abs();
                s.vel[1] -= s.vel[1] >> 9;
                s.vel[0] /= 2;
                s.vel[2] /= 2;
            }
            // Friction.
            for v in &mut s.vel {
                *v -= *v / 256;
                *v -= *v / 512;
                *v -= v.signum();
            }
        }
        for o in &self.objects {
            for l in &self.links {
                let (a, b) = (self.spheres[l.a], self.spheres[l.b]);
                let d = [b.pos[0] - a.pos[0], b.pos[1] - a.pos[1], b.pos[2] - a.pos[2]];
                let dist = mul64(d[0], d[0]) + mul64(d[1], d[1]) + mul64(d[2], d[2]);
                let ddist = dist - l.dist;
                if ddist == 0 {
                    continue;
                }
                // Equal and opposite on both.
                let force = mul64(ddist, o.jellyness);
                let accel = div64(force, a.mass);
                for (v, d) in self.spheres[l.a].vel.iter_mut().zip(d) {
                    *v += mul64(d, accel);
                }
                let accel = div64(force, b.mass);
                for (v, d) in self.spheres[l.b].vel.iter_mut().zip(d) {
                    *v -= mul64(d, accel);
                }
            }
        }
    }

    /// `SM_get_next`: each sphere's middle and radius (world units), and its colour, the
    /// original's `123456789 * n + 3141592653` for the nth (from 1), a different one each.
    pub fn spheres(&self) -> impl Iterator<Item = (Vec3, f32, [u8; 4])> + '_ {
        self.spheres.iter().enumerate().map(|(i, s)| {
            let c = 123456789u32.wrapping_mul(i as u32 + 1).wrapping_add(3141592653);
            let [_, r, g, b] = c.to_be_bytes();
            (vec3((s.pos[0] >> 8) as f32, (s.pos[1] >> 8) as f32, (s.pos[2] >> 8) as f32), (s.radius >> 8) as f32, [r, g, b, 255])
        })
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

    /// The debug key's cube: 0x80 a side, 0x80 up, three spheres a side.
    fn dropped() -> SphereMatter {
        let mut sm = SphereMatter::default();
        sm.create_cube(vec3(1000.0, 128.0, 1000.0), vec3(1128.0, 256.0, 1128.0), 3, 0x10000, 0x10000);
        sm
    }

    #[test]
    fn a_cube_of_spheres_with_springs() {
        let sm = dropped();
        assert_eq!(sm.spheres.len(), 27);
        // Each sphere to itself and up to seven neighbours further along.
        assert_eq!(sm.links.len(), 27 + 3 * 18 + 3 * 12 + 8);
        assert!(sm.spheres.iter().all(|s| s.radius > 0 && s.mass > 0));
    }

    #[test]
    fn it_falls_and_settles_on_the_ground() {
        let mut sm = dropped();
        for _ in 0..(STEPS_PER_SECOND as usize * 8) {
            sm.step(&Flat);
        }
        for (p, r, _) in sm.spheres() {
            assert!(p.is_finite());
            assert!(p.y >= r - 1.0 && p.y < 200.0, "sphere at {p}");
            assert!((p.x - 1064.0).abs() < 300.0 && (p.z - 1064.0).abs() < 300.0, "wandered to {p}");
        }
        // Come to rest.
        let before: Vec<Vec3> = sm.spheres().map(|s| s.0).collect();
        for _ in 0..STEPS_PER_SECOND as usize {
            sm.step(&Flat);
        }
        let moved = sm.spheres().zip(&before).map(|(s, b)| (s.0 - *b).length()).fold(0.0, f32::max);
        assert!(moved < 4.0, "still moving {moved}");
    }
}
