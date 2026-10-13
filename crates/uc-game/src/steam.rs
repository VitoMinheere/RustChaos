//! Steam vents: what a `create_trap` point makes (`EWAY_DO_EMIT_STEAM`).
//!
//! While the point is active and not dead, `EWAY_process_emit_steam` (fallen/Source/eway.cpp)
//! looks up where it is in its routine: the mission clock in hundredths, times `speed`, over
//! 128, wrapped round `steps`. If the routine's bit for that step is set, out comes a puff
//! that tick, forwards along the point's facing, straight up or straight down, `range` units
//! a tick, nudged at random.
//!
//! A puff is a `PFLAG_HURTPEOPLE` particle (fallen/Source/psystem.cpp): every other game turn
//! it scalds anyone standing within 64 units of it across, between their feet and the top of
//! their head, for `(64 - dist) / 16 + 1`. It lives 100 of the particle system's ticks, at 15
//! a second, growing a unit and thinning a little each one, and goes when it reaches the
//! ground (or a roof).

use crate::rand;
use glam::{vec3, Vec2, Vec3};

/// The game ticks at 20 a second; the particle system counts life at 15.
const TICKS: f32 = 20.0;
const PARTICLE_TICKS: f32 = 15.0;
/// `PARTICLE_Add(..., life 100, size 40, ...)`.
const LIFE: f32 = 100.0 / PARTICLE_TICKS;
const SIZE: f32 = 40.0;
/// `PSYSTEM_MAX_PARTICLES` on PC: once the system is full, new puffs are simply not made.
/// Target UC's seventeen vents can fill it on their own.
const MAX_PUFFS: usize = 2048;
/// How far a puff reaches across to scald (`abs(dist) < 0x40`).
pub const REACH: f32 = 64.0;

/// `EWAY_SUBTYPE_STEAM_*`: which way the vent blows.
pub mod dir {
    pub const FORWARD: u8 = 0;
    pub const UP: u8 = 1;
    pub const DOWN: u8 = 2;
}

/// A vent as its point describes it (`ELEV_load_level` packs these into `arg1`, `arg2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vent {
    pub pos: Vec3,
    /// The point's facing (engine angle, facing (-sin, -cos)).
    pub angle: f32,
    /// `Data[4]`: a [`dir`].
    pub dir: u8,
    /// `Data[3]`: the routine, a bit per step.
    pub routine: u32,
    /// `Data[1]`, `Data[2]` and `Data[5]`, kept to their 6, 4 and 6 bits.
    pub speed: u32,
    pub steps: u32,
    pub range: u32,
}

impl Vent {
    /// Whether it blows on the step the mission clock (seconds) is at.
    pub fn blowing(&self, secs: f32) -> bool {
        if self.steps == 0 {
            return false;
        }
        let hundredths = (secs * 100.0) as u32;
        let step = ((hundredths * self.speed) >> 7) % self.steps;
        self.routine & (1 << step) != 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Puff {
    pub pos: Vec3,
    vel: Vec3,
    age: f32,
}

impl Puff {
    /// Particle ticks lived.
    fn ticks(&self) -> f32 {
        self.age * PARTICLE_TICKS
    }

    /// Half its width: it grows a unit a tick from 40.
    pub fn radius(&self) -> f32 {
        SIZE + self.ticks()
    }

    /// Its colour, `0x7fe8ffd0` to begin with, losing alpha every other tick (`PFLAG_FADE2`).
    pub fn colour(&self) -> [u8; 4] {
        [0xe8, 0xff, 0xd0, (0x7f as f32 - self.ticks() / 2.0).max(0.0) as u8]
    }
}

#[derive(Debug, Clone)]
pub struct Steam {
    pub puffs: Vec<Puff>,
    /// Seconds to the next game tick, and to the next turn puffs scald on.
    tick: f32,
    scald: f32,
    rng: u32,
}

impl Default for Steam {
    fn default() -> Steam {
        Steam { puffs: Vec::new(), tick: 0.0, scald: 0.0, rng: 0x57ea }
    }
}

impl Steam {
    /// A number in -1..1.
    fn nudge(&mut self) -> f32 {
        rand(&mut self.rng) as f32 / 32768.0 - 1.0
    }

    /// Moves the puffs on, letting go of those that are spent or below `ground`, and blows
    /// from each of `vents` that is on at mission time `secs`. Returns whether this frame is
    /// one the puffs scald on (every other game turn).
    pub fn update(&mut self, dt: f32, secs: f32, vents: &[Vent], ground: impl Fn(Vec3) -> f32) -> bool {
        for p in &mut self.puffs {
            p.age += dt;
            p.pos += p.vel * dt;
        }
        self.puffs.retain(|p| p.age < LIFE && p.pos.y >= ground(p.pos));
        self.tick -= dt;
        while self.tick <= 0.0 {
            self.tick += 1.0 / TICKS;
            for v in vents.iter().filter(|v| v.blowing(secs)) {
                if self.puffs.len() >= MAX_PUFFS {
                    break;
                }
                let r = v.range as f32;
                // `range` world units a tick, as `SIN(yaw) * range >> 8` comes to.
                let base = match v.dir {
                    dir::UP => vec3(0.0, r, 0.0),
                    dir::DOWN => vec3(0.0, -r, 0.0),
                    _ => {
                        let f = uc_world::character::facing(v.angle);
                        vec3(f.x * r, 0.0, f.y * r)
                    }
                };
                // Plus up to half a unit a tick either way, from up to 3 units off the point.
                let jitter = vec3(self.nudge(), self.nudge(), self.nudge()) * 0.5;
                let off = vec3(self.nudge(), self.nudge(), self.nudge()) * 3.0;
                self.puffs.push(Puff { pos: v.pos + off, vel: (base + jitter) * TICKS, age: 0.0 });
            }
        }
        self.scald -= dt;
        if self.scald <= 0.0 {
            self.scald += 2.0 / TICKS;
            return true;
        }
        false
    }

    /// How much the puffs scald someone standing at `feet` whose head is `height` above: each
    /// puff near enough does `(64 - dist) / 16 + 1`, measuring across as `QDIST2` does.
    pub fn scalding(&self, feet: Vec3, height: f32) -> i32 {
        self.puffs.iter().filter_map(|p| {
            let d = (Vec2::new(p.pos.x, p.pos.z) - Vec2::new(feet.x, feet.z)).abs();
            let dist = d.max_element() + d.min_element() / 2.0;
            let within = p.pos.y >= feet.y - 32.0 && p.pos.y <= feet.y + height + 32.0;
            (dist < REACH && within).then(|| (REACH - dist) as i32 / 16 + 1)
        }).sum()
    }
}
