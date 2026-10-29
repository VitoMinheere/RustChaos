//! Cars: arcade driving physics, collision with walls, people, cones and barrels.
//!
//! Not the original's suspension model (vehicle.cpp): a car is two circles (nose and tail)
//! that slide along walls, steered by a bicycle model, sitting on the ground under its four
//! wheels. The meshes and wheel layout come from the original's `veh_info` table.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_formats::iam::Map;
use uc_formats::prm::Prim;
use uc_world::ground::{Ground, Wmove};

/// `VEH_TYPE_*` (fallen/Headers/vehicle.h) and their meshes from `veh_info`
/// (fallen/Source/vehicle.cpp): (body prim, wheel prim, wheel x, front wheel z, rear wheel z).
///
/// The wheel positions are the table's `DX` and `DZ`. Heights come from how the original
/// draws a car (`VEH_draw` and `process_car`, fallen/Source/vehicle.cpp): the body 64
/// (`BodyOffset` 0x4000) below the car's position, and each wheel's middle at
/// `51 - spring length` from it. At rest the springs settle where they hold the car up
/// against gravity (compression about 5200 of 0x8000, gravity 51 a tick), so a wheel's middle
/// is [`WHEEL_ABOVE_BODY`] above the body's origin, for every type. A wheel is modelled
/// centred on its axle, and here it rides on the road on its own radius (38 for a car's, 51
/// for the larger one the vans and the jeep run on), so the body goes that radius less the
/// 8 above it. In the original a car's wheels float about 13 above the road, since that
/// `51` is a van wheel's radius; a van's sit on it.
pub const TYPES: [(u8, u8, f32, f32, f32); 9] = [
    (88, 87, 120.0, 150.0, -165.0),  // van
    (108, 220, 85.0, 160.0, -120.0), // car
    (155, 220, 85.0, 160.0, -120.0), // taxi
    (150, 220, 85.0, 160.0, -120.0), // police
    (159, 87, 120.0, 150.0, -165.0), // ambulance
    (90, 87, 120.0, 150.0, -165.0),  // jeep
    (91, 87, 120.0, 150.0, -165.0),  // meat wagon
    (131, 220, 85.0, 160.0, -120.0), // sedan
    (27, 87, 120.0, 150.0, -165.0),  // Wildcat van
];

/// The flashing lights of the types with a light bar (`veh_info`'s `FLX`, `FLY`, `FLZ` and
/// `FLRED`, fallen/Source/vehicle.cpp:181): the police car, the ambulance and the meat wagon,
/// which are also the types with a siren (`siren`, vehicle.cpp:3548). Each is a pair of
/// lights `FLZ` either side of the middle and `FLX` back from it, in the original's car
/// space, and `FLY` up from the car's position. That's `BodyOffset` above the body's origin:
/// 64 for the police car (0x4000) and 104 for the vans (0x6800), which the heights here
/// include. Both lights are red when `FLRED` is set; otherwise the first is blue.
pub const LIGHT_BARS: [Option<(f32, f32, f32, bool)>; 9] = [
    None,
    None,
    None,
    Some((0.0, 70.0 + 64.0, 40.0, false)),    // police
    Some((-40.0, 210.0 + 104.0, 50.0, true)), // ambulance
    None,
    Some((50.0, 140.0 + 104.0, 40.0, false)), // meat wagon
    None,
    None,
];

/// How far a wheel's middle is above a car body's origin (see [`TYPES`]): the spring at rest,
/// 51 less its length of 107, from a body drawn 64 down.
pub const WHEEL_ABOVE_BODY: f32 = 8.0;

/// The points of each type's walkable faces in its body prim, three to a face: the corner
/// and its two neighbours (`tri_vehicle`, `WMOVE_get_pos`, fallen/Source/wmove.cpp:216). A
/// van has its roof; a car its bonnet, windscreen, roof, back window and boot. The taxi and
/// the sedan use the car's points, the meat wagon the jeep's and the Wildcat van the van's,
/// as the original does.
///
/// Two of the original's lists don't fit the shipped meshes. Its police car points
/// (`tri_police`: 28, 29, 50 and on) land on the wheel arches and the lights, low down the
/// sides, but the police body has the car's points in the car's order, so it uses the car's.
/// Its ambulance points (119 to 121) are past the end of the body's 96, so the original read
/// whatever came after; here it has the roof of its box (61, 62, 67).
const TOPS: [&[[usize; 3]]; 9] = [VAN, CAR, CAR, CAR, AMBULANCE, JEEP, JEEP, CAR, VAN];
const VAN: &[[usize; 3]] = &[[1, 2, 21]];
const CAR: &[[usize; 3]] = &[[33, 11, 34], [34, 12, 35], [35, 13, 37], [37, 15, 38], [38, 16, 39]];
const AMBULANCE: &[[usize; 3]] = &[[61, 62, 67]];
const JEEP: &[[usize; 3]] = &[[14, 33, 2], [2, 4, 11], [11, 30, 9], [9, 28, 19]];

/// The radius of a wheel prim, which it rides on (38 if it's missing).
pub fn wheel_radius(wheel: Option<&Prim>) -> f32 {
    wheel.map_or(38.0, |prim| {
        let (lo, hi) = uc_world::level::prim_bounds(prim);
        (hi.y - lo.y) / 2.0
    })
}

/// What it takes to get into a car: the special the original keeps as the vehicle's `key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Anyone can (`SPECIAL_NONE`).
    None,
    /// Whoever carries a key (`SPECIAL_KEY`), the item a mission puts down with `create_item`
    /// `IT_KEY`.
    Key,
    /// Nobody, ever (`SPECIAL_NUM_TYPES`, "a special that doesn't exist").
    Never,
}

impl Key {
    /// A `create_vehicle` point's key (`VK_*`, `Data[3]`), as `ELEV_load_level` reads it:
    /// red, blue, green, black and white all ask for the same `SPECIAL_KEY`, and `VK_LOCKED`
    /// for a special that doesn't exist.
    pub fn from_vk(vk: i32) -> Key {
        match vk {
            0 => Key::None,
            1..=5 => Key::Key,
            _ => Key::Never,
        }
    }
}

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

/// Each type's engine (`ENGINE_*`, fallen/Source/vehicle.cpp:175): forwards, backwards,
/// acceleration and brakes, as fractions of the police car's (`ENGINE_PIG`), which is what
/// [`TOP_SPEED`], [`REVERSE_SPEED`], [`ACCEL`] and [`BRAKE`] are. The vans are light goods
/// vehicles (`ENGINE_LGV`), the car, taxi and jeep cars (`ENGINE_CAR`), the ambulance and the
/// meat wagon `ENGINE_AMB`, and the sedan has a cop car's.
const ENGINES: [(f32, f32, f32, f32); 9] = {
    const LGV: (f32, f32, f32, f32) = (17.0 / 25.0, 10.0 / 15.0, 4.0 / 5.0, 8.0 / 10.0);
    const CAR: (f32, f32, f32, f32) = (21.0 / 25.0, 10.0 / 15.0, 4.0 / 5.0, 8.0 / 10.0);
    const PIG: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 1.0);
    const AMB: (f32, f32, f32, f32) = (1.0, 10.0 / 15.0, 1.0, 8.0 / 10.0);
    [LGV, CAR, CAR, PIG, AMB, CAR, AMB, PIG, LGV]
};
const BRAKE: f32 = 3200.0;
const DRAG: f32 = 0.35;
/// Distance from the middle to the nose and tail circles, and their radius.
const HALF_LENGTH: f32 = 140.0;
const RADIUS: f32 = 110.0;
/// How far from the leading end a prop's outline can be and still be touching: the radius of
/// the car's round end, and a little.
pub const REACH: f32 = RADIUS + 16.0;
/// A crash at more than this speed (world units per second) damages the car.
const CRASH_SPEED: f32 = 900.0;

/// Under this forwards, pulling away with [`Drive::faster`] spins the wheels (`velocity <
/// 400`, vehicle.cpp:3622, in the original's 1.25 units a second).
const WHEELSPIN_SPEED: f32 = 400.0 * 1.25;
/// Rolling backwards faster than this, putting the foot down spins them anyway (`-200`).
const ROLLBACK_SPEED: f32 = 200.0 * 1.25;
/// Stopped: under the original's 10 (`newmag < 10`, vehicle.cpp:3799).
const STOPPED: f32 = 10.0 * 1.25;

/// The driver's controls: `throttle` -1 (reverse/brake) .. 1, `steer` -1 (left) .. 1.
#[derive(Debug, Clone, Copy, Default)]
pub struct Drive {
    pub throttle: f32,
    pub steer: f32,
    pub handbrake: bool,
    /// `VEH_FASTER`: the foot down hard. The player's pedals always are (`INPUT_CAR_GOFASTER`
    /// is either pedal, interfac.cpp:5865); the people driving round never are (pcom.cpp:14681).
    pub faster: bool,
    /// `VEH_SIREN`: pressed this turn, switching the siren on or off (the manual's pursuit
    /// mode). Only the player has it (`INPUT_CAR_SIREN`, interfac.cpp:5866).
    pub siren: bool,
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
    /// Whoever last did it damage, a shot or a blast, for the blame when it goes up.
    pub hurt_by: Option<usize>,
    /// The person (index) driving.
    pub driver: Option<usize>,
    /// Stopped by the mission (`stall_car`).
    pub stalled: bool,
    /// What it takes to get in: set by the event point that made it, and by `lock_vehicle`
    /// points since, which lock it by asking for a special that doesn't exist and unlock it by
    /// asking for nothing (`EWAY_DO_LOCK_VEHICLE`).
    pub key: Key,
    /// Body tilt from the ground under the wheels (radians).
    pub pitch: f32,
    pub roll: f32,
    /// Wheel rotation (radians).
    pub wheel_spin: f32,
    /// `Smokin`: the wheels spinning as it pulls away hard (vehicle.cpp:3625), smoking and
    /// laying skid marks. Set afresh every turn.
    pub smokin: bool,
    /// `Dir < 0`: put in reverse, till it stops; pressing on then brakes rather than spins.
    reverse: bool,
    /// `Siren`: the siren going and the light bar flashing.
    pub siren: bool,
}

impl Vehicle {
    pub fn new(way: usize, kind: u8, pos: Vec3, yaw: f32, key: Key) -> Vehicle {
        Vehicle {
            way, kind: kind.min(8), pos, yaw, speed: 0.0, slip: 0.0, slide: 0.0, steer: 0.0, health: 300, hurt_by: None, driver: None, stalled: false,
            key, pitch: 0.0, roll: 0.0, wheel_spin: 0.0, smokin: false, reverse: false, siren: false,
        }
    }

    /// Locked to anyone without a key.
    pub fn locked(&self) -> bool {
        self.key != Key::None
    }

    /// Whether someone can get in, carrying a key or not (`set_person_enter_vehicle`,
    /// fallen/Source/Person.cpp: `person_has_special(p_person, key)`).
    pub fn opens_for(&self, has_key: bool) -> bool {
        match self.key {
            Key::None => true,
            Key::Key => has_key,
            Key::Never => false,
        }
    }

    /// Puts the car down at `pos` facing `yaw`, standing still (`reinit_vehicle`).
    pub fn reset(&mut self, pos: Vec3, yaw: f32) {
        (self.pos, self.yaw) = (pos, yaw);
        (self.speed, self.slip, self.slide, self.steer, self.pitch, self.roll) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        (self.smokin, self.reverse) = (false, false);
    }

    /// Whether it has a siren and a light bar.
    pub fn has_siren(&self) -> bool {
        LIGHT_BARS[self.kind as usize].is_some()
    }

    /// The siren for the turn (`siren`, vehicle.cpp:3548): switched by the driver's `VEH_SIREN`,
    /// and off with nobody at the wheel (`pedals`, vehicle.cpp:3587; `do_car_input`, :3716).
    pub fn work_siren(&mut self, d: Drive) {
        if self.driver.is_none() {
            self.siren = false;
        } else if d.siren && self.has_siren() {
            self.siren = !self.siren;
        }
    }

    /// The light bar's two lights in the world, and whether the first is red (see
    /// [`LIGHT_BARS`]), for a body drawn `lift` up. The original's car space has x back and z
    /// to the side, which in the body prim are z and x.
    pub fn light_bar(&self, lift: f32) -> Option<([Vec3; 2], bool)> {
        let (x, y, z, red) = LIGHT_BARS[self.kind as usize]?;
        let at = |side: f32| self.body_point(vec3(side, y, x), lift);
        Some(([at(z), at(-z)], red))
    }

    /// Smoking and laying skid marks: spinning its wheels, or sliding on the handbrake (as the
    /// original's skid sets `Smokin`, vehicle.cpp:3934).
    pub fn skidding(&self) -> bool {
        self.smokin || self.slip.abs() > 300.0
    }

    /// The throttle that settles the car at `speed` (negative is reversing).
    pub fn throttle_for(speed: f32) -> f32 {
        if speed >= 0.0 { (speed / TOP_SPEED).min(1.0) } else { (speed / REVERSE_SPEED).max(-1.0) }
    }

    /// The end of the car leading at `speed` (the nose going forwards, the tail in reverse).
    pub fn front(&self, speed: f32) -> Vec2 {
        let ahead = if self.speed >= 0.0 || speed < 0.0 { 1.0 } else { -1.0 };
        vec2(self.pos.x, self.pos.z) + self.forward() * HALF_LENGTH * ahead
    }

    pub fn wrecked(&self) -> bool {
        self.health <= 0
    }

    /// `VEH_reduce_health` (fallen/Source/vehicle.cpp:4780): half the damage comes off.
    pub fn reduce_health(&mut self, damage: i32) {
        self.health -= damage >> 1;
    }

    /// What a crash at `speed` takes off a car's health: nothing under [`CRASH_SPEED`], as
    /// running into a wall does in [`Vehicle::update`].
    pub fn crash_damage(speed: f32) -> i32 {
        if speed > CRASH_SPEED { ((speed - CRASH_SPEED) / 20.0) as i32 } else { 0 }
    }

    /// Its outline on the ground, the body round its wheels (`GetCarPoints`, vehicle.cpp:
    /// 2287): front right, front left, back left, back right.
    pub fn corners(&self) -> [Vec2; 4] {
        let (wx, fz, rz) = (TYPES[self.kind as usize].2 + 25.0, TYPES[self.kind as usize].3 + 45.0, TYPES[self.kind as usize].4 - 45.0);
        [self.to_world(wx, fz), self.to_world(-wx, fz), self.to_world(-wx, rz), self.to_world(wx, rz)]
    }

    /// Its outline and `other`'s overlap (no edge of either separates them), and they're at
    /// about the same height.
    pub fn overlaps(&self, other: &Vehicle) -> bool {
        if (self.pos.y - other.pos.y).abs() > 150.0 {
            return false;
        }
        let (a, b) = (self.corners(), other.corners());
        for poly in [&a, &b] {
            for i in 0..4 {
                let e = poly[(i + 1) % 4] - poly[i];
                let axis = vec2(-e.y, e.x);
                let span = |q: &[Vec2; 4]| q.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.dot(axis)), hi.max(p.dot(axis))));
                let ((alo, ahi), (blo, bhi)) = (span(&a), span(&b));
                if ahi < blo || bhi < alo {
                    return false;
                }
            }
        }
        true
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
        let (top, back, accel, brakes) = ENGINES[self.kind as usize];
        let target = if d.throttle > 0.0 { TOP_SPEED * top * d.throttle } else { -REVERSE_SPEED * back * -d.throttle };
        // Wheelspin (`pedals`, vehicle.cpp:3622): the foot down, not braking out of reverse,
        // and slow or still rolling back.
        if self.speed.abs() < STOPPED {
            self.reverse = false;
        }
        self.smokin = !d.handbrake && d.throttle >= 0.05 && !self.reverse
            && (self.speed < -ROLLBACK_SPEED || d.faster && self.speed < WHEELSPIN_SPEED);
        if d.handbrake {
            let brake = if dead { BRAKE } else { BRAKE * 0.12 };
            self.speed -= self.speed.signum() * (brake * dt).min(self.speed.abs());
        } else if d.throttle.abs() < 0.05 {
            self.speed -= self.speed * (DRAG * 2.0 * dt).min(1.0);
        } else if (d.throttle > 0.0) == (self.speed >= 0.0) || self.speed.abs() < 30.0 {
            self.reverse = d.throttle < 0.0;
            let rate = if (target - self.speed).abs() > 0.0 { ACCEL * accel } else { 0.0 };
            self.speed += (target - self.speed).signum() * (rate * dt).min((target - self.speed).abs());
        } else {
            // Pressing against the direction of travel brakes.
            self.speed -= self.speed.signum() * (BRAKE * brakes * dt).min(self.speed.abs());
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
            ground.road(map, p.x, p.y, self.pos.y + 60.0)
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

    /// Car body prim space to world, as the body is drawn: the meshes are modelled nose
    /// towards -z, and the body sits `lift` up so the wheels are where the original has them.
    pub fn body_point(&self, p: Vec3, lift: f32) -> Vec3 {
        self.place(vec3(-p.x, p.y + lift, -p.z))
    }

    /// The walkable faces on top of the car (`WMOVE_get_pos`, `CLASS_VEHICLE`), from its body
    /// prim and wheel radius. Every type has them on the PC: `WMOVE_create` keeps them to the
    /// vans only for the PlayStation. Only the van's, the Wildcat van's and the ambulance's
    /// have edges to hang from (wmove.cpp:510).
    pub fn tops(&self, body: &Prim, wheel: f32) -> impl Iterator<Item = Wmove> + '_ {
        let lift = wheel - WHEEL_ABOVE_BODY;
        let edges = matches!(self.kind, 0 | 4 | 8);
        let points: Vec<Vec3> = body.points.iter().map(|p| self.body_point(vec3(p[0] as f32, p[1] as f32, p[2] as f32), lift)).collect();
        TOPS[self.kind as usize].iter().filter_map(move |t| {
            let [o, a, b] = t.map(|i| points.get(i).copied());
            let o = o?;
            Some(Wmove { o, a: a? - o, b: b? - o, vehicle: true, edges })
        })
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
