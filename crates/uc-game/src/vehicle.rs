//! Cars: arcade driving physics, collision with walls, people, cones and barrels.
//!
//! Not the original's suspension model (vehicle.cpp): a car is two circles (nose and tail)
//! that slide along walls, steered by a bicycle model, sitting on the ground under its four
//! wheels. The meshes and wheel layout come from the original's `veh_info` table.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_formats::iam::Map;
use uc_world::ground::Ground;

/// `VEH_TYPE_*` (fallen/Headers/vehicle.h) and their meshes from `veh_info`
/// (fallen/Source/vehicle.cpp): (body prim, wheel prim, wheel x, front wheel z, rear wheel z,
/// body height above the ground).
pub const TYPES: [(u8, u8, f32, f32, f32, f32); 9] = [
    (88, 87, 120.0, 150.0, -165.0, 30.0),  // van
    (108, 220, 85.0, 160.0, -120.0, 30.0), // car
    (155, 220, 85.0, 160.0, -120.0, 30.0), // taxi
    (150, 220, 85.0, 160.0, -120.0, 30.0), // police
    (159, 87, 120.0, 150.0, -165.0, 30.0), // ambulance
    (90, 87, 120.0, 150.0, -165.0, 30.0),  // jeep
    (91, 87, 120.0, 150.0, -165.0, 30.0),  // meat wagon
    (131, 220, 85.0, 160.0, -120.0, 30.0), // sedan
    (27, 87, 120.0, 150.0, -165.0, 30.0),  // Wildcat van
];

/// `VT_*` (an event point's vehicle type, fallen/Headers/Mission.h) to `VEH_TYPE_*`.
pub fn from_vt(vt: i32) -> u8 {
    match vt {
        2 => 0, // van
        3 => 2, // taxi
        6 => 3, // police
        7 => 4, // ambulance
        8 => 6, // meat wagon
        9 => 5, // jeep
        10 => 7, // sedan
        11 => 8, // Wildcat van
        _ => 1, // car
    }
}

const TOP_SPEED: f32 = 3000.0;
const REVERSE_SPEED: f32 = 900.0;
const ACCEL: f32 = 1300.0;
const BRAKE: f32 = 3200.0;
const DRAG: f32 = 0.35;
/// Distance from the middle to the nose and tail circles, and their radius.
const HALF_LENGTH: f32 = 140.0;
const RADIUS: f32 = 110.0;
/// A crash at more than this speed (world units per second) damages the car.
const CRASH_SPEED: f32 = 900.0;

/// The driver's controls: `throttle` -1 (reverse/brake) .. 1, `steer` -1 (left) .. 1.
#[derive(Debug, Clone, Copy, Default)]
pub struct Drive {
    pub throttle: f32,
    pub steer: f32,
    pub handbrake: bool,
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    /// The event point that created it.
    pub way: usize,
    /// `VEH_TYPE_*`.
    pub kind: u8,
    /// Ground position under the middle, world units.
    pub pos: Vec3,
    /// Heading: forward is (sin, cos), as props are placed.
    pub yaw: f32,
    /// Signed forward speed, world units per second.
    pub speed: f32,
    /// Sideways speed (towards car-space -x, the right), world units per second: the tyres
    /// soon kill it, unless the handbrake has the back end sliding.
    pub slip: f32,
    /// While the handbrake is on: which way the car was going when it went on (+1 forwards,
    /// -1 backwards; 0 = off). A spin keeps turning the same way even once the car is going
    /// backwards, so a handbrake turn can swing it right round.
    slide: f32,
    pub steer: f32,
    pub health: i32,
    /// The person (index) driving.
    pub driver: Option<usize>,
    /// Stopped by the mission (`stall_car`).
    pub stalled: bool,
    /// Body tilt from the ground under the wheels (radians).
    pub pitch: f32,
    pub roll: f32,
    /// Wheel rotation (radians).
    pub wheel_spin: f32,
}

impl Vehicle {
    pub fn new(way: usize, kind: u8, pos: Vec3, yaw: f32) -> Vehicle {
        Vehicle {
            way, kind: kind.min(8), pos, yaw, speed: 0.0, slip: 0.0, slide: 0.0, steer: 0.0, health: 300, driver: None, stalled: false,
            pitch: 0.0, roll: 0.0, wheel_spin: 0.0,
        }
    }

    pub fn wrecked(&self) -> bool {
        self.health <= 0
    }

    pub fn forward(&self) -> Vec2 {
        vec2(self.yaw.sin(), self.yaw.cos())
    }

    /// The heading as the mission scripts measure it (an event point's direction), for
    /// direction triggers.
    pub fn script_angle(&self) -> f32 {
        self.yaw
    }

    /// The engine angle a person sitting in the car faces (people face (-sin, -cos)).
    pub fn driver_angle(&self) -> f32 {
        self.yaw + std::f32::consts::PI
    }

    /// Car space (x right, z forward) to world xz.
    pub fn to_world(&self, x: f32, z: f32) -> Vec2 {
        let (s, c) = self.yaw.sin_cos();
        vec2(self.pos.x + c * x + s * z, self.pos.z - s * x + c * z)
    }

    /// Where the driver gets in and out: beside the driver's (left) door. Car space +x is the
    /// car's left (looking along +z with y up).
    pub fn door(&self) -> Vec2 {
        self.to_world(200.0, 20.0)
    }

    /// One frame of driving. Returns the impact speed if it hit something solid.
    pub fn update(&mut self, d: Drive, dt: f32, ground: &Ground, map: &Map) -> Option<f32> {
        let dead = self.stalled || self.wrecked();
        let d = if dead { Drive { handbrake: true, ..Default::default() } } else { d };
        // Engine and brakes. The handbrake only locks the back wheels: it slows the car
        // gently, and it's for sliding round corners.
        let target = if d.throttle > 0.0 { TOP_SPEED * d.throttle } else { -REVERSE_SPEED * -d.throttle };
        if d.handbrake {
            let brake = if dead { BRAKE } else { BRAKE * 0.12 };
            self.speed -= self.speed.signum() * (brake * dt).min(self.speed.abs());
        } else if d.throttle.abs() < 0.05 {
            self.speed -= self.speed * (DRAG * 2.0 * dt).min(1.0);
        } else if (d.throttle > 0.0) == (self.speed >= 0.0) || self.speed.abs() < 30.0 {
            let rate = if (target - self.speed).abs() > 0.0 { ACCEL } else { 0.0 };
            self.speed += (target - self.speed).signum() * (rate * dt).min((target - self.speed).abs());
        } else {
            // Pressing against the direction of travel brakes.
            self.speed -= self.speed.signum() * (BRAKE * dt).min(self.speed.abs());
        }
        self.speed -= self.speed * DRAG * 0.2 * dt;
        // Steering: tighter at low speed, turning in the direction of travel. With the
        // handbrake on the back end swings out: the car turns faster than it travels.
        let side = |f: Vec2| vec2(f.y, -f.x);
        let f0 = self.forward();
        let vel = f0 * self.speed + side(f0) * self.slip;
        self.steer += (d.steer - self.steer) * (8.0 * dt).min(1.0);
        let grip = 1.0 / (1.0 + self.speed.abs() / 2200.0);
        // (Sliding, it's the car's whole speed that swings it round, not just the forward part.)
        let sliding = d.handbrake && !dead;
        if !sliding {
            self.slide = 0.0;
        } else if self.slide == 0.0 {
            self.slide = if self.speed < 0.0 { -1.0 } else { 1.0 };
        }
        // Sliding, the back end swings round with the car's whole speed (up to 5 radians a
        // second), whichever way it's now travelling.
        let turn = if sliding {
            (self.steer * vel.length() * self.slide / 240.0).clamp(-5.0, 5.0)
        } else {
            self.steer * self.speed / 380.0 * grip
        };
        self.yaw -= turn * dt;
        // The car keeps going the way it was going; the tyres pull it round to where it
        // points, slowly while sliding.
        let f = self.forward();
        self.speed = vel.dot(f);
        self.slip = vel.dot(side(f));
        let hold = if d.handbrake && !dead { 0.9 } else { 7.0 };
        self.slip -= self.slip * (hold * dt).min(1.0);
        // Move nose and tail, each sliding along walls; the car lines up between them.
        let mid = vec2(self.pos.x, self.pos.z);
        let step = (f * self.speed + side(f) * self.slip) * dt;
        let y = self.pos.y + 40.0;
        let nose = ground.slide(mid + f * HALF_LENGTH, y, step, RADIUS);
        let tail = ground.slide(mid - f * HALF_LENGTH, y, step, RADIUS);
        let new_mid = (nose + tail) * 0.5;
        let moved = (new_mid - mid).dot(step.normalize_or_zero());
        let mut impact = None;
        let wanted = step.length();
        if wanted.abs() > 1.0 && (moved / wanted) < 0.6 {
            // Hit something: bounce back a little from a real impact, else just stop (so the
            // driver can back away).
            impact = Some(self.speed.abs());
            if self.speed.abs() > CRASH_SPEED {
                self.health -= ((self.speed.abs() - CRASH_SPEED) / 20.0) as i32;
            }
            self.speed = if self.speed.abs() > 200.0 { self.speed * -0.25 } else { 0.0 };
            self.slip = 0.0;
        }
        let axis = nose - tail;
        if axis.length_squared() > 1.0 {
            let dir = axis.normalize();
            self.yaw = dir.x.atan2(dir.y);
        }
        // Sit on the ground under the wheels.
        let (wx, fz, rz) = (TYPES[self.kind as usize].2, TYPES[self.kind as usize].3, TYPES[self.kind as usize].4);
        let h = |x: f32, z: f32| {
            let p = self.to_world_at(new_mid, x, z);
            ground.surface(map, p.x, p.y, self.pos.y + 60.0)
        };
        let (fl, fr, rl, rr) = (h(-wx, fz), h(wx, fz), h(-wx, rz), h(wx, rz));
        let new_y = (fl + fr + rl + rr) * 0.25;
        self.pos = vec3(new_mid.x, new_y, new_mid.y);
        self.pitch = ((fl + fr) * 0.5 - (rl + rr) * 0.5).atan2(fz - rz);
        self.roll = ((fr + rr) * 0.5 - (fl + rl) * 0.5).atan2(2.0 * wx);
        self.wheel_spin += self.speed * dt / 38.0;
        impact
    }

    fn to_world_at(&self, mid: Vec2, x: f32, z: f32) -> Vec2 {
        let (s, c) = self.yaw.sin_cos();
        vec2(mid.x + c * x + s * z, mid.y - s * x + c * z)
    }

    /// Car space (x right, y up, z forward) to world, with the body's tilt.
    pub fn place(&self, v: Vec3) -> Vec3 {
        let (sp, cp) = self.pitch.sin_cos();
        let (sr, cr) = self.roll.sin_cos();
        // Roll about z, then pitch about x, then yaw.
        let v = vec3(cr * v.x - sr * v.y, sr * v.x + cr * v.y, v.z);
        let v = vec3(v.x, cp * v.y + sp * v.z, -sp * v.y + cp * v.z);
        let (s, c) = self.yaw.sin_cos();
        self.pos + vec3(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
    }

    /// Whether world point `p` (xz) is within the car's footprint, grown by `margin`.
    pub fn covers(&self, p: Vec2, margin: f32) -> bool {
        let d = p - vec2(self.pos.x, self.pos.z);
        let f = self.forward();
        let along = d.dot(f);
        let side = d.dot(vec2(f.y, -f.x));
        along.abs() < HALF_LENGTH + 120.0 + margin && side.abs() < 130.0 + margin
    }
}
