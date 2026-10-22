//! The dike (fallen/Source/dike.cpp): "The new bikes are called dikes." A second go at the
//! motorbike, two wheels joined by a springy frame, the back one driven and gripping the
//! ground sideways, the front one pushed along where the bars point, both by how much grip
//! the ground has (`DIKE_get_grip`: full on a road, half off it). It never got a model: the
//! original draws it as two lines, the frame and the bars. The shipped game never makes
//! one: the debug code that did, `7` to make one where Darci stands and the keypad's `8`,
//! `/` and `*` to ride it (Controls.cpp:2213), is commented out. Here it's an extra, `dike`
//! in the console, ridden on the same keys.
//!
//! It runs a step a turn of twenty to the second, in the original's own units: positions in
//! 256ths of a world unit, angles in 2048ths of a turn. `DIKE_create` ignores the angle
//! it's given and always faces the dike down z (`FMATRIX_vector(vector, 0, 0)`), and so
//! does this.

use glam::{vec3, Vec3};

/// Steps a second: once a turn.
pub const STEPS_PER_SECOND: f32 = 20.0;

/// `DIKE_WHEEL_APART`, `_RADIUS`, `DIKE_GRAVITY`.
const WHEEL_APART: i64 = 0xc0;
const WHEEL_RADIUS: i64 = 0x50;
const GRAVITY: i64 = -0x400;

/// `DIKE_CONTROL_*`: what the rider is pressing.
pub const ACCEL: u8 = 1 << 0;
pub const LEFT: u8 = 1 << 1;
pub const RIGHT: u8 = 1 << 2;

fn sin(a: i64) -> i64 {
    (((a & 2047) as f64 * std::f64::consts::TAU / 2048.0).sin() * 65536.0) as i64
}

fn cos(a: i64) -> i64 {
    sin(a + 512)
}

/// `MUL64`: 16.16 multiply.
fn mul64(a: i64, b: i64) -> i64 {
    (a * b) >> 16
}

/// `DIV64`: 16.16 divide.
fn div64(a: i64, b: i64) -> i64 {
    if b == 0 { 0 } else { (a << 16) / b }
}

/// `QDIST2`.
fn qdist2(x: i64, z: i64) -> i64 {
    if x > z { x + (z >> 1) } else { z + (x >> 1) }
}

/// `calc_angle`: the angle whose `SIN` goes with `dx` and `COS` with `dz`.
fn calc_angle(dx: i64, dz: i64) -> i64 {
    ((dx as f64).atan2(dz as f64) * 2048.0 / std::f64::consts::TAU).round() as i64 & 2047
}

/// `Root`: the integer square root.
fn root(v: i64) -> i64 {
    (v.max(0) as f64).sqrt() as i64
}

/// `FMATRIX_vector`: the unit vector (16.16) `yaw` round and `pitch` up.
fn vector(yaw: i64, pitch: i64) -> [i64; 3] {
    let (sy, cy, sp, cp) = (sin(yaw), cos(yaw), sin(pitch), cos(pitch));
    [mul64(sy, cp), sp, mul64(cy, cp)]
}

/// What the dike needs to know about the world.
pub trait Road {
    /// `PAP_calc_map_height_at`, world units: the floor under (x, z) for a wheel whose
    /// bottom is at `y`.
    fn height(&self, x: f32, z: f32, y: f32) -> f32;
    /// `ROAD_is_road` for the map square under (x, z).
    fn is_road(&self, x: f32, z: f32) -> bool;
}

/// `DIKE_get_grip` (dike.cpp:48): 256 on a road, 128 anywhere else.
fn grip(road: &impl Road, x: i64, z: i64) -> i64 {
    if road.is_road(x as f32, z as f32) { 256 } else { 128 }
}

/// `DIKE_Dike`.
#[derive(Debug, Clone)]
pub struct Dike {
    front: [i64; 3],
    front_d: [i64; 3],
    back: [i64; 3],
    back_d: [i64; 3],
    /// `DIKE_CONTROL_*`, set before each step.
    pub control: u8,
    pub steer: i64,
    pub power: i64,
    on_ground_front: bool,
    on_ground_back: bool,
    pub yaw: i64,
    pub pitch: i64,
}

impl Dike {
    /// `DIKE_create` (dike.cpp:63): standing at (x, z) on the floor under `y`, facing down z.
    pub fn new(x: f32, y: f32, z: f32, road: &impl Road) -> Dike {
        let v = vector(0, 0);
        let (vx, vz) = ((v[0] * WHEEL_APART) >> 9, (v[2] * WHEEL_APART) >> 9);
        let (x, z) = ((x as i64) << 8, (z as i64) << 8);
        let on_ground = |x: i64, z: i64| ((road.height((x >> 8) as f32, (z >> 8) as f32, y) as i64) + WHEEL_RADIUS) << 8;
        let front = [x + vx, on_ground(x + vx, z + vz), z + vz];
        let back = [x - vx, on_ground(x - vx, z - vz), z - vz];
        Dike {
            front, front_d: [0; 3], back, back_d: [0; 3], control: 0, steer: 0, power: 0,
            on_ground_front: false, on_ground_back: false, yaw: 0, pitch: 0,
        }
    }

    /// `DIKE_process` (dike.cpp:108): one step.
    pub fn step(&mut self, road: &impl Road) {
        if self.control & ACCEL != 0 {
            self.power += 8;
        } else if self.power > 0 {
            self.power -= 1;
        }
        if self.control & LEFT != 0 {
            self.steer -= 7;
        }
        if self.control & RIGHT != 0 {
            self.steer += 7;
        }
        // `SBYTE`s, saturated.
        self.steer = self.steer.clamp(-80, 80);
        self.power = self.power.clamp(-80, 80);
        // Driving the back wheel along the way it points, and stopping it sliding sideways
        // as far as the grip goes ("THIS IS ONLY 2D!!!").
        if self.on_ground_back {
            let grip = grip(road, self.back[0] >> 8, self.back[2] >> 8);
            let v = vector(self.yaw, 0);
            for (b, v) in self.back_d.iter_mut().zip(v) {
                *b += (v * ((self.power * grip) >> 8)) >> 13;
            }
            let (radx, radz) = (-v[2], v[0]);
            let dprod = ((mul64(radx, self.back_d[0]) + mul64(radz, self.back_d[2])) * grip) >> 8;
            self.back_d[0] -= (radx * dprod) >> 16;
            self.back_d[2] -= (radz * dprod) >> 16;
        }
        self.back_d[1] += GRAVITY;
        self.front_d[1] += GRAVITY;
        // The frame pulls (or pushes) the wheels back to the right distance apart.
        const SHIFT: i64 = 2;
        let d = [self.front[0] - self.back[0], self.front[1] - self.back[1], self.front[2] - self.back[2]];
        let s = d.map(|v| v >> SHIFT);
        let dist = root(s[0] * s[0] + s[1] * s[1] + s[2] * s[2]) << SHIFT;
        let ddist = dist - (WHEEL_APART << 8);
        for (k, dk) in d.iter().enumerate() {
            let f = (dk * ddist) >> 19;
            self.front_d[k] -= f;
            self.back_d[k] += f;
        }
        for k in 0..3 {
            self.back[k] += self.back_d[k];
        }
        // On the ground, the front wheel only rolls: along where the bars point with full
        // grip, along the frame with none, and in between in between.
        if self.on_ground_front {
            let steer = vector((self.yaw + (self.steer << 2)) & 2047, self.pitch);
            let v = vector(self.yaw, 0);
            let grip = grip(road, self.front[0] >> 8, self.front[2] >> 8);
            let s = [0, 1, 2].map(|k| ((steer[k] * grip) >> 8) + ((v[k] * (256 - grip)) >> 8));
            let q = s.map(|v| v >> 2);
            let over = div64(0x10000, root(q[0] * q[0] + q[1] * q[1] + q[2] * q[2]) << 2);
            let s = s.map(|v| mul64(v, over));
            let dprod = mul64(self.front_d[0], s[0]) + mul64(self.front_d[1], s[1]) + mul64(self.front_d[2], s[2]);
            self.front_d = s.map(|v| mul64(dprod, v));
        }
        for k in 0..3 {
            self.front[k] += self.front_d[k];
        }
        // The wheels on the ground.
        let ground = |p: [i64; 3]| (road.height((p[0] >> 8) as f32, (p[2] >> 8) as f32, ((p[1] >> 8) - WHEEL_RADIUS) as f32) as i64) << 8;
        let floor = ground(self.back) + (WHEEL_RADIUS << 8);
        self.on_ground_back = self.back[1] < floor;
        if self.on_ground_back {
            self.back[1] = floor;
            self.back_d[1] = 0;
        }
        let floor = ground(self.front) + (WHEEL_RADIUS << 8);
        self.on_ground_front = self.front[1] < floor;
        if self.on_ground_front {
            self.front[1] = floor;
            self.front_d[1] = 0;
        }
        for v in self.front_d.iter_mut().chain(self.back_d.iter_mut()) {
            *v -= *v / 16;
        }
        // Which way it's facing, unless the frame stands too near upright to tell.
        let dxdz = qdist2(d[0].abs(), d[2].abs());
        if dxdz >= 0x10 {
            self.yaw = calc_angle(d[0], d[2]) & 2047;
        }
        self.pitch = calc_angle(d[1], dxdz);
    }

    /// The wheels' middles, front and back (world units).
    pub fn wheels(&self) -> (Vec3, Vec3) {
        let w = |p: [i64; 3]| vec3((p[0] >> 8) as f32, (p[1] >> 8) as f32, (p[2] >> 8) as f32);
        (w(self.front), w(self.back))
    }

    /// The bars (`DIKE_draw`, dike.cpp:423): the two ends of a line through the front
    /// wheel, an eighth of a world unit's worth of the steering vector either side.
    pub fn bars(&self) -> (Vec3, Vec3) {
        let s = vector((self.yaw + (self.steer << 2)) & 2047, self.pitch).map(|v| v >> 3);
        let p = |sign: i64| vec3(((self.front[0] + sign * s[0]) >> 8) as f32, ((self.front[1] + sign * s[1]) >> 8) as f32, ((self.front[2] + sign * s[2]) >> 8) as f32);
        (p(1), p(-1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat ground, road where x < 2000.
    struct Flat;
    impl Road for Flat {
        fn height(&self, _: f32, _: f32, _: f32) -> f32 {
            0.0
        }
        fn is_road(&self, x: f32, _: f32) -> bool {
            x < 2000.0
        }
    }

    #[test]
    fn it_stands_on_its_wheels_facing_down_z() {
        let mut d = Dike::new(1000.0, 0.0, 1000.0, &Flat);
        for _ in 0..40 {
            d.step(&Flat);
        }
        let (f, b) = d.wheels();
        assert!((f.y - 80.0).abs() <= 1.0 && (b.y - 80.0).abs() <= 1.0, "{f} {b}");
        assert!(f.z > b.z && (f.x - b.x).abs() < 1.0, "{f} {b}");
        assert!(((f - b).length() - 192.0).abs() < 8.0, "frame {}", (f - b).length());
    }

    #[test]
    fn it_rides_and_turns() {
        let mut d = Dike::new(1000.0, 0.0, 1000.0, &Flat);
        d.step(&Flat);
        d.control = ACCEL;
        for _ in 0..40 {
            d.step(&Flat);
        }
        let (f, b) = d.wheels();
        assert!(f.z > 1400.0, "rode to {f}");
        assert!(((f - b).length() - 192.0).abs() < 16.0, "frame {}", (f - b).length());
        // Bars to the right turn it.
        let yaw = d.yaw;
        d.control = ACCEL | RIGHT;
        for _ in 0..20 {
            d.step(&Flat);
        }
        let turned = (d.yaw - yaw).rem_euclid(2048);
        assert!(turned > 50 && turned < 1024, "turned {turned}");
        assert!(d.wheels().0.is_finite());
    }
}
