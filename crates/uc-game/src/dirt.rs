//! Droplets: the stream from someone a mission sets peeing (`make_person_pee`).
//!
//! `DIRT_new_water` (fallen/Source/dirt.cpp) throws one drop a tick from just under the
//! pelvis, forwards along the person's facing and a little down, each nudged at random; then
//! gravity takes it until it reaches the ground. `DIRT_TYPE_URINE` is the same as water but
//! for its colour. The original's speeds are in sixteenths of a unit a tick at 20 ticks a
//! second; here they are in units a second.

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
                self.drops.push(Drop { pos: at, vel, age: 0.0 });
            }
        }
    }
}
