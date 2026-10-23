//! Explosions and the fires they leave behind.
//!
//! The original builds a bang out of several particle systems at once: `PYRO_construct`
//! (fallen/Source/pyro.cpp) reads a bitfield and makes a flash (`PYRO_TWANGER`), a fireball
//! (`PYRO_EXPLODE2`), a ring of dust (`PYRO_DUSTWAVE`), streamers, a bonfire left burning and
//! so on, each its own system with its own meshes. This is a compact stand-in driven by the
//! same bitfield: soft round puffs that fly out, rise, grow and fade, drawn as camera-facing
//! billboards. Fire and dust are added to what's behind them; smoke is blended over it.

use crate::rand;
use glam::{vec3, Vec3};

/// The bits of `PYRO_construct`'s `types` (its `scale` is [`Pyro::burst`]'s).
pub const FLASH: u8 = 1;
pub const FIREBALL: u8 = 2;
pub const DUST: u8 = 4;
pub const STREAMERS: u8 = 8;
pub const FIRE: u8 = 16;
pub const DOME: u8 = 32;
pub const FIREBOMB: u8 = 64;

/// What a car blowing up makes: the same set Nitro Car's own `visual_effect` points ask for
/// (31 = flash, fireball, dust, streamers and a fire left burning).
pub const CAR_BOMB: u8 = FLASH | FIREBALL | DUST | STREAMERS | FIRE;

/// How long a bonfire burns (seconds). `PYRO_BONFIRE` has no set lifetime; this keeps a
/// mission from filling up with fires.
const FIRE_SECS: f32 = 30.0;

/// One round puff of fire, dust or smoke.
#[derive(Debug, Clone, Copy)]
pub struct Puff {
    pub pos: Vec3,
    vel: Vec3,
    /// Radius in world units, and how fast it grows (units per second).
    radius: f32,
    grow: f32,
    age: f32,
    life: f32,
    /// Drawn blended and dark rather than added and bright.
    pub smoke: bool,
    /// Colour at birth. Fire cools towards red as it goes.
    tint: [u8; 3],
}

impl Puff {
    /// How far through its life it is, 0 at birth to 1 when it's gone.
    fn t(&self) -> f32 {
        (self.age / self.life).clamp(0.0, 1.0)
    }

    pub fn radius(&self) -> f32 {
        self.radius + self.grow * self.age
    }

    /// Its colour and how strongly it shows: fire cools and dims, smoke thins out.
    pub fn colour(&self) -> [u8; 4] {
        let t = self.t();
        if self.smoke {
            // Smoke billows up, pale at first, then thins away.
            let a = (110.0 * (1.0 - t) * (0.3 + 0.7 * (t * 4.0).min(1.0))) as u8;
            return [self.tint[0], self.tint[1], self.tint[2], a];
        }
        // Fire: full brightness for the first moment, then cooling towards red.
        let cool = (t * 1.4).min(1.0);
        let c = |i: usize| (self.tint[i] as f32 * (1.0 - cool * [0.0, 0.8, 0.95][i])) as u8;
        [c(0), c(1), c(2), (255.0 * (1.0 - t) * (1.0 - t)) as u8]
    }
}

/// Every explosion and fire still going.
#[derive(Debug, Clone)]
pub struct Pyro {
    pub puffs: Vec<Puff>,
    /// Fires still burning: (where, how big, seconds left, seconds to the next puff).
    fires: Vec<(Vec3, f32, f32, f32)>,
    /// Its own chance, kept apart from the game's: how an explosion looks mustn't change how
    /// the mission plays out.
    rng: u32,
}

impl Default for Pyro {
    fn default() -> Pyro {
        Pyro { puffs: Vec::new(), fires: Vec::new(), rng: 0x5eed }
    }
}

impl Pyro {
    /// Sets off `kinds` (the bitfield above) at `pos`. `scale` is roughly how wide the whole
    /// thing is in world units, so each puff is a fraction of it.
    pub fn burst(&mut self, pos: Vec3, kinds: u8, scale: f32) {
        let mut made: Vec<Puff> = Vec::new();
        let mut rng = self.rng;
        // A number in -1..1.
        let mut spread = || rand(&mut rng) as f32 / 32768.0 - 1.0;
        let mut puff = |pos: Vec3, vel: Vec3, radius: f32, grow: f32, life: f32, smoke: bool, tint: [u8; 3]| {
            made.push(Puff { pos, vel, radius, grow, age: 0.0, life, smoke, tint });
        };
        // The white flash at the heart of it, gone in a moment.
        if kinds & FLASH != 0 {
            puff(pos, Vec3::ZERO, scale * 0.16, scale * 0.9, 0.18, false, [255, 250, 230]);
        }
        // The fireball, and the smoke it leaves climbing.
        if kinds & (FIREBALL | FIREBOMB | DOME) != 0 {
            for k in 0..9 {
                let dir = vec3(spread(), 0.35 + 0.5 * spread().abs(), spread()).normalize_or_zero();
                let out = scale * (0.05 + 0.2 * (k as f32 / 9.0));
                puff(pos + dir * out, dir * scale * 0.5, scale * 0.14, scale * 0.22, 0.5 + 0.3 * k as f32 / 9.0,
                     false, [255, 190 - 8 * k as u8, 90]);
            }
            for _ in 0..5 {
                let dir = vec3(spread() * 0.5, 1.0, spread() * 0.5);
                puff(pos + dir * scale * 0.15, dir * scale * 0.3, scale * 0.16, scale * 0.16, 2.6, true, [60, 56, 52]);
            }
        }
        // A ring of dust thrown out along the ground.
        if kinds & DUST != 0 {
            for k in 0..10 {
                let a = std::f32::consts::TAU * k as f32 / 10.0;
                let dir = vec3(a.cos(), 0.12, a.sin());
                puff(pos + dir * scale * 0.2, dir * scale * 0.9, scale * 0.1, scale * 0.3, 1.1, true, [120, 112, 96]);
            }
        }
        // Streamers: small bright bits flung out and falling.
        if kinds & STREAMERS != 0 {
            for _ in 0..12 {
                let dir = vec3(spread(), 0.6 + 0.8 * spread().abs(), spread()).normalize_or_zero();
                puff(pos, dir * scale * 1.6, scale * 0.025, 0.0, 0.7 + 0.5 * spread().abs(), false, [255, 210, 130]);
            }
        }
        if kinds & FIRE != 0 {
            self.fires.push((pos, scale * 0.22, FIRE_SECS, 0.0));
        }
        self.puffs.append(&mut made);
        self.rng = rng;
    }

    /// A Man in Black going up (Person.cpp:17575): a big white bloom (`POLY_PAGE_BLOOM1`,
    /// 255 across) and twenty pale clouds (`POLY_PAGE_SMOKECLOUD2`, `0x7f7fffff`) climbing one
    /// to five units a tick and drifting, growing from 70 as they fade.
    pub fn lightning_strike(&mut self, pos: Vec3) {
        self.puffs.push(Puff { pos, vel: Vec3::ZERO, radius: 60.0, grow: 900.0, age: 0.0, life: 0.6, smoke: false, tint: [255, 255, 255] });
        for _ in 0..20 {
            let at = pos + vec3(self.spread(), self.spread(), self.spread()) * 20.0;
            let vel = vec3(self.spread() * 20.0, 60.0 + 40.0 * self.spread(), self.spread() * 20.0);
            self.puffs.push(Puff { pos: at, vel, radius: 70.0, grow: 40.0, age: 0.0, life: 3.0, smoke: true, tint: [127, 255, 255] });
        }
    }

    /// Smoke off a spinning tyre (vehicle.cpp:1491): a pale brown cloud (`0x7fc9b7a3`) of
    /// size 60, growing 10 and fading 16 a particle tick, so gone in eight.
    pub fn tyre_smoke(&mut self, at: Vec3) {
        let life = 8.0 / 15.0;
        self.puffs.push(Puff { pos: at, vel: Vec3::ZERO, radius: 60.0, grow: 150.0, age: 0.0, life, smoke: true, tint: [0xc9, 0xb7, 0xa3] });
    }

    /// A number in -1..1.
    fn spread(&mut self) -> f32 {
        rand(&mut self.rng) as f32 / 32768.0 - 1.0
    }

    /// Moves everything on and lets the fires flicker.
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.puffs {
            p.age += dt;
            p.pos += p.vel * dt;
            // Fire and smoke rise and slow down; streamers fall.
            let lift = if p.smoke { 90.0 } else { 40.0 };
            p.vel.y += (lift - p.vel.y.max(0.0) * 1.6) * dt;
            p.vel *= 1.0 - (2.2 * dt).min(1.0);
        }
        self.puffs.retain(|p| p.age < p.life);
        for i in 0..self.fires.len() {
            let (pos, size, left, next) = self.fires[i];
            let next = next - dt;
            self.fires[i] = (pos, size, left - dt, if next > 0.0 { next } else { 0.1 });
            if next > 0.0 {
                continue;
            }
            // A tongue of flame, and now and then some smoke off the top of it.
            let jitter = vec3(self.spread(), 0.0, self.spread()) * size * 0.5;
            self.puffs.push(Puff {
                pos: pos + jitter, vel: vec3(0.0, size * 1.6, 0.0), radius: size * 0.5, grow: -size * 0.2,
                age: 0.0, life: 0.6, smoke: false, tint: [255, 170, 70],
            });
            if rand(&mut self.rng) & 3 == 0 {
                self.puffs.push(Puff {
                    pos: pos + jitter + vec3(0.0, size, 0.0), vel: vec3(0.0, size * 1.2, 0.0), radius: size * 0.6,
                    grow: size * 0.5, age: 0.0, life: 2.4, smoke: true, tint: [50, 47, 44],
                });
            }
        }
        self.fires.retain(|f| f.2 > 0.0);
    }

    /// A single tongue of flame at `at`, as a fire throws up: for something on fire that moves.
    pub fn lick(&mut self, at: Vec3, size: f32) {
        self.puffs.push(Puff {
            pos: at, vel: vec3(0.0, size * 1.6, 0.0), radius: size * 0.5, grow: -size * 0.2,
            age: 0.0, life: 0.5, smoke: false, tint: [255, 170, 70],
        });
    }

    /// Where the fires are burning, and how big: for lighting the scene, and for tests.
    pub fn fires(&self) -> impl Iterator<Item = (Vec3, f32)> + '_ {
        self.fires.iter().map(|&(pos, size, ..)| (pos, size))
    }
}
