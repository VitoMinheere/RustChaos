//! What gunfire and blows leave in the air (#237): sparks from the muzzle, the casings a gun
//! throws out, the spark where a bullet lands, and blood.
//!
//! - **Sparks** (`DIRT_new_sparks`, fallen/Source/dirt.cpp:1303): four from the muzzle on
//!   every shot (Person.cpp:6056), thrown out along the gun's way at 5 to 8 units a tick and
//!   up to 3 off it, then falling.
//! - **Casings** (`DIRT_create_brass`, dirt.cpp:3095): one from the gun hand on every shot,
//!   three for an AK-47 or a Man in Black (Person.cpp:6261), thrown out to the shooter's
//!   right, give or take a sixteenth of a turn, and a little up; they bounce and lie where
//!   they come to rest, until the oldest make room for new ones.
//! - **The spark where a bullet lands** (`PYRO_hitspang`, pyro.cpp:545): a quick flash, on a
//!   person, on a barrel or on the ground.
//! - **Blood** (`DIRT_new_water` with `DIRT_TYPE_BLOOD`): two drops from the head of someone
//!   shot (pyro.cpp:553), one from a blow and five from a knife (combat.cpp:2252), each
//!   thrown up to 7 units a tick either way and falling as water does.
//!
//! Speeds are the original's units a tick at 20 ticks a second, given here a second.

use crate::rand;
use glam::{vec3, Vec2, Vec3};

const TICKS: f32 = 20.0;
/// Gravity on water and sparks: 15 sixteenths of a unit a tick, every tick.
const GRAVITY: f32 = 15.0 / 16.0 * TICKS * TICKS;
/// The casings' heavier fall, and how much of their speed a bounce keeps.
const BRASS_GRAVITY: f32 = 2.0 * GRAVITY;
const BOUNCE: f32 = 0.35;
/// Casings left lying at once, before the oldest go.
const MAX_BRASS: usize = 64;
/// How long a spark and a bullet's spark last (seconds).
const SPARK_LIFE: f32 = 0.35;
const SPANG_LIFE: f32 = 0.12;
const BLOOD_LIFE: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
pub struct Bit {
    pub pos: Vec3,
    pub vel: Vec3,
    pub age: f32,
    /// A casing has come to rest.
    pub still: bool,
}

#[derive(Debug, Clone)]
pub struct GunFx {
    pub sparks: Vec<Bit>,
    pub brass: Vec<Bit>,
    pub spangs: Vec<Bit>,
    pub blood: Vec<Bit>,
    /// Its own chance, so the look of it can't change how the mission plays.
    rng: u32,
}

impl Default for GunFx {
    fn default() -> GunFx {
        GunFx { sparks: Vec::new(), brass: Vec::new(), spangs: Vec::new(), blood: Vec::new(), rng: 0x6f17 }
    }
}

impl GunFx {
    /// A number in -1..1.
    fn nudge(&mut self) -> f32 {
        rand(&mut self.rng) as f32 / 32768.0 - 1.0
    }

    /// A gun going off at `muzzle`, pointing along `dir` (xz), from the hand at `hand`:
    /// sparks, and `casings` casings thrown to the right of `dir`.
    pub fn fire(&mut self, muzzle: Vec3, hand: Vec3, dir: Vec2, casings: usize) {
        let dir = dir.normalize_or(Vec2::Y);
        for _ in 0..4 {
            let along = (5.0 + (self.nudge() * 0.5 + 0.5) * 3.0) * TICKS;
            let off = vec3(self.nudge(), self.nudge(), self.nudge()) * 3.0 * TICKS;
            self.sparks.push(Bit { pos: muzzle, vel: vec3(dir.x, 0.0, dir.y) * along + off, age: 0.0, still: false });
        }
        let right = Vec2::new(-dir.y, dir.x);
        for _ in 0..casings {
            // A sixteenth of a turn either way, and 64 to 127 of `SIN >> 11` speed: two to
            // four units a tick.
            let a = self.nudge() * std::f32::consts::TAU / 32.0;
            let way = Vec2::from_angle(a).rotate(right);
            let speed = (2.0 + (self.nudge() * 0.5 + 0.5) * 2.0) * TICKS;
            self.brass.push(Bit { pos: hand, vel: vec3(way.x * speed, 1.5 * TICKS, way.y * speed), age: 0.0, still: false });
        }
        if self.brass.len() > MAX_BRASS {
            let extra = self.brass.len() - MAX_BRASS;
            self.brass.drain(..extra);
        }
    }

    /// A bullet landing at `at`.
    pub fn spang(&mut self, at: Vec3) {
        self.spangs.push(Bit { pos: at, vel: Vec3::ZERO, age: 0.0, still: false });
    }

    /// A miss landing on the ground somewhere within a quarter of a square of `at`
    /// (`floor` the height there).
    pub fn spang_beside(&mut self, at: Vec3, floor: impl Fn(f32, f32) -> f32) {
        let (x, z) = (at.x + self.nudge() * 64.0, at.z + self.nudge() * 64.0);
        self.spang(vec3(x, floor(x, z) + 2.0, z));
    }

    /// `drops` drops of blood thrown from `at`.
    pub fn bleed(&mut self, at: Vec3, drops: usize) {
        for _ in 0..drops {
            let vel = vec3(self.nudge() * 7.0, self.nudge() * 15.0 / 16.0, self.nudge() * 7.0) * TICKS;
            self.blood.push(Bit { pos: at, vel, age: 0.0, still: false });
        }
    }

    /// Moves everything on: `ground` says how high the floor is under a point.
    pub fn update(&mut self, dt: f32, ground: impl Fn(Vec3) -> f32) {
        for s in self.sparks.iter_mut().chain(&mut self.blood) {
            s.age += dt;
            s.vel.y -= GRAVITY * dt;
            s.pos += s.vel * dt;
        }
        self.sparks.retain(|s| s.age < SPARK_LIFE && s.pos.y > ground(s.pos));
        self.blood.retain(|s| s.age < BLOOD_LIFE && s.pos.y > ground(s.pos));
        for s in &mut self.spangs {
            s.age += dt;
        }
        self.spangs.retain(|s| s.age < SPANG_LIFE);
        for b in self.brass.iter_mut().filter(|b| !b.still) {
            b.age += dt;
            b.vel.y -= BRASS_GRAVITY * dt;
            let next = b.pos + b.vel * dt;
            let floor = ground(next);
            if next.y <= floor {
                b.pos = vec3(next.x, floor, next.z);
                b.vel = vec3(b.vel.x, -b.vel.y, b.vel.z) * BOUNCE;
                if b.vel.length() < 0.5 * TICKS || b.age > 3.0 {
                    b.still = true;
                }
            } else {
                b.pos = next;
            }
        }
    }

    /// How far through its life a spark or a bullet's spark is, 0 to 1.
    pub fn spark_t(s: &Bit) -> f32 {
        (s.age / SPARK_LIFE).clamp(0.0, 1.0)
    }

    pub fn spang_t(s: &Bit) -> f32 {
        (s.age / SPANG_LIFE).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casings_come_to_rest_on_the_floor_and_the_rest_goes() {
        let mut fx = GunFx::default();
        fx.fire(vec3(0.0, 120.0, 0.0), vec3(0.0, 110.0, 0.0), Vec2::Y, 3);
        fx.spang(vec3(500.0, 50.0, 0.0));
        fx.bleed(vec3(500.0, 160.0, 0.0), 2);
        assert_eq!((fx.sparks.len(), fx.brass.len(), fx.spangs.len(), fx.blood.len()), (4, 3, 1, 2));
        for _ in 0..200 {
            fx.update(1.0 / 30.0, |_| 0.0);
        }
        assert!(fx.sparks.is_empty() && fx.spangs.is_empty() && fx.blood.is_empty());
        assert_eq!(fx.brass.len(), 3, "casings stay where they lie");
        for b in &fx.brass {
            assert!(b.still && b.pos.y == 0.0, "{b:?}");
            // Thrown to the right of +z, which is -x.
            assert!(b.pos.x < 0.0, "{b:?}");
        }
    }

    #[test]
    fn the_oldest_casings_make_room() {
        let mut fx = GunFx::default();
        for _ in 0..30 {
            fx.fire(Vec3::ZERO, Vec3::ZERO, Vec2::Y, 3);
        }
        assert_eq!(fx.brass.len(), MAX_BRASS);
    }
}
