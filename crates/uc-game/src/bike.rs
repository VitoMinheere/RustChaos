//! Motorbikes (`VT_BIKE`): fallen/Source/bike.cpp, which the shipped game compiles out
//! (`#ifdef BIKE`) but the unreleased missions use. The bike is two wheels a fixed distance
//! apart, each on a sprung suspension: the back one is driven along the bike's heading and
//! slides only a little sideways, the front one is pulled to the right distance from it and
//! pushed along where the bars point. It runs twice a turn while being ridden, a turn being
//! a twentieth of a second, in the original's own units: positions in 256ths of a world
//! unit, angles in 2048ths of a turn.

use glam::{Vec2, Vec3};

pub const TICK: f32 = 1.0 / 20.0;

/// `BIKE_WHEEL_SUS`, `_RADIUS`, `_APART`, `_COL`.
const WHEEL_SUS: i64 = 0x10;
const WHEEL_RADIUS: i64 = 0x18;
const WHEEL_APART: i64 = 0xa0;
/// The radius the wheels collide with walls at.
pub const WHEEL_COL: f32 = 0x50 as f32;
/// `BIKE_GRAVITY`.
const GRAVITY: i64 = -0xc0;

/// `BIKE_MODE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Parked,
    /// Somebody is getting on.
    Mounting,
    Driving,
    /// Somebody is getting off.
    Dismounting,
}

/// What the bike needs to know about the world.
pub trait Road {
    /// `PAP_calc_map_height_at`, world units.
    fn height(&self, x: f32, z: f32) -> f32;
    /// `BIKE_collide_sphere`: a wheel moved from `from` to `to` (world xz), kept out of walls
    /// and fences; where it ends up, and whether it hit anything.
    fn collide(&self, from: Vec2, to: Vec2, y: f32) -> (Vec2, bool);
    /// `BIKE_get_ground_friction`: 256 is all grip (tarmac 200, grass and dirt 80, slippery
    /// ground 32).
    fn friction(&self, x: f32, z: f32) -> i64;
}

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

/// `BIKE_Bike`.
#[derive(Debug, Clone)]
pub struct Bike {
    /// The event point that made it.
    pub way: usize,
    pub yaw: i64,
    pub pitch: i64,
    pub mode: Mode,
    on_ground_front: bool,
    on_ground_back: bool,
    pub accel: i64,
    pub steer: i64,
    back: [i64; 3],
    back_d: [i64; 3],
    front: [i64; 3],
    front_dy: i64,
    wheel_y_back: i64,
    wheel_y_front: i64,
    wheel_dy_back: i64,
    wheel_dy_front: i64,
    pub wheel_rot_front: i64,
    pub wheel_rot_back: i64,
    /// Skidding after braking hard (`SlideTimer`), in the original's 256ths of a turn.
    slide: i64,
    /// Who is riding it (a person index).
    pub driver: Option<usize>,
}

impl Bike {
    /// `BIKE_create`: parked at (x, z), facing `yaw` (2048ths of a turn).
    pub fn new(way: usize, x: f32, z: f32, yaw: i64, road: &impl Road) -> Bike {
        let (x, z) = (x as i64, z as i64);
        let ground = |x: i64, z: i64| road.height(x as f32, z as f32) as i64;
        let back = [x << 8, (ground(x, z) + WHEEL_RADIUS + WHEEL_SUS) << 8, z << 8];
        let (fx, fz) = (x + (sin(yaw) * WHEEL_APART >> 16), z + (cos(yaw) * WHEEL_APART >> 16));
        let front = [fx << 8, (ground(fx, fz) + WHEEL_RADIUS + WHEEL_SUS) << 8, fz << 8];
        Bike {
            way, yaw: yaw & 2047, pitch: 0, mode: Mode::Parked, on_ground_front: true, on_ground_back: true,
            accel: 0, steer: 0, back, back_d: [0; 3], front, front_dy: 0,
            wheel_y_back: back[1] - (WHEEL_SUS << 8), wheel_y_front: front[1] - (WHEEL_SUS << 8),
            wheel_dy_back: 0, wheel_dy_front: 0, wheel_rot_front: 0, wheel_rot_back: 0, slide: 0, driver: None,
        }
    }

    /// Where it is (world units): halfway between the wheels, less a quarter of the way to
    /// the front, so nearer the back wheel.
    pub fn pos(&self) -> Vec3 {
        let p = |k: usize| ((self.front[k] + self.back[k]) / 2 - (self.front[k] - self.back[k]) / 4) as f32 / 256.0;
        Vec3::new(p(0), p(1), p(2))
    }

    /// The wheels' positions (world units).
    pub fn wheels(&self) -> (Vec3, Vec3) {
        let w = |p: [i64; 3]| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 256.0;
        (w(self.back), w(self.front))
    }

    /// `BIKE_get_speed`.
    pub fn speed(&self) -> i64 {
        qdist2(self.back_d[0].abs() >> 8, self.back_d[2].abs() >> 8)
    }

    /// `BIKE_get_roll`: leaning into the turn.
    pub fn roll(&self) -> i64 {
        (-self.steer << 2).clamp(-0x140, 0x140) & 2047
    }

    /// `BIKE_control_set`: throttle (+1, -1 or 0), steering (-1 left .. +1 right), the brake
    /// and a wheelie. Turning is quicker the slower it goes.
    pub fn control(&mut self, accel: i64, steer: i64, brake: bool, wheelie: bool) {
        let speed = self.speed();
        let tspeed = (32 - speed.abs()).clamp(7, 25);
        let accel = if accel == 0 && brake { -1 } else { accel };
        let mut a = self.accel + accel * 5;
        let s = (self.steer + steer * tspeed).clamp(-80, 80);
        a = if wheelie { a.clamp(0, 127) } else { a.clamp(-40, 50) };
        self.accel = a;
        self.steer = s;
        if (accel < 0 || brake) && speed > 3 {
            self.slide = 1000;
        }
        if wheelie && self.wheel_y_front < self.wheel_y_back + 0x3000 {
            self.wheel_dy_front -= GRAVITY * 4;
        }
    }

    /// `BIKE_process_normal`, one turn (twice over while it's being ridden).
    pub fn tick(&mut self, road: &impl Road) {
        let passes = if self.mode == Mode::Driving { 2 } else { 1 };
        for _ in 0..passes {
            self.pass(road);
        }
    }

    fn pass(&mut self, road: &impl Road) {
        let front_old = (self.front[0], self.front[2]);
        let heading = self.yaw + 1024;
        let (vx, vz) = (sin(heading), cos(heading));
        if self.slide > 0 {
            self.slide = (self.slide - 256).max(0);
        }
        let ridden = self.mode == Mode::Driving;
        if self.on_ground_back && self.slide == 0 {
            let friction = road.friction(self.back[0] as f32 / 256.0, self.back[2] as f32 / 256.0);
            if !ridden {
                self.back_d[0] = 0;
                self.back_d[2] = 0;
            } else {
                // Drive the back wheel, and stop it sliding sideways over the ground.
                self.back_d[0] += ((self.accel * vx >> 8) * (friction + 256)) >> 13;
                self.back_d[2] += ((self.accel * vz >> 8) * (friction + 256)) >> 13;
                let (radx, radz) = (-vz, vx);
                let dprod = (mul64(radx, self.back_d[0]) + mul64(radz, self.back_d[2])) * friction >> 8;
                self.back_d[0] -= radx * dprod >> 16;
                self.back_d[2] -= radz * dprod >> 16;
            }
        }
        // Move the back wheel, out of any wall.
        let old_back = (self.back[0], self.back[2]);
        let to = Vec2::new((self.back[0] + self.back_d[0]) as f32, (self.back[2] + self.back_d[2]) as f32) / 256.0;
        let (at, hit) = road.collide(Vec2::new(old_back.0 as f32, old_back.1 as f32) / 256.0, to, self.back[1] as f32 / 256.0);
        (self.back[0], self.back[2]) = ((at.x * 256.0) as i64, (at.y * 256.0) as i64);
        if hit {
            self.accel >>= 1;
        }
        // The steering straightens, and the throttle eases off.
        self.steer -= self.steer / 4;
        if self.steer.abs() < 4 {
            self.steer = 0;
        }
        if (self.on_ground_front || self.on_ground_back) && self.slide == 0 {
            self.accel -= self.accel / 16;
        }
        // Bikes can't go backwards!
        if self.accel > 0 {
            if self.accel < 4 {
                self.accel = 0;
            }
        } else {
            self.accel = self.accel / 2 + 1;
        }
        if self.slide > 0 || self.on_ground_back {
            for k in [0, 2] {
                self.back_d[k] -= self.back_d[k] / 16;
                if self.back_d[k].abs() < 16 {
                    self.back_d[k] = 0;
                }
            }
        }
        let friction = if self.on_ground_front { road.friction(self.front[0] as f32 / 256.0, self.front[2] as f32 / 256.0) } else { 0 };
        // The front wheel to the right distance from the back, more so the less it grips.
        let (dx, dz) = (self.front[0] - self.back[0], self.front[2] - self.back[2]);
        let dist = qdist2(dx.abs(), dz.abs());
        let ddist = (WHEEL_APART << 8) - dist;
        let along = ddist * (256 - friction) >> 12;
        self.front[0] -= dx * along / ((dist >> 4) + 1);
        self.front[2] -= dz * along / ((dist >> 4) + 1);
        let before = Vec2::new(self.front[0] as f32, self.front[2] as f32) / 256.0;
        let (at, col) = road.collide(Vec2::new(front_old.0 as f32, front_old.1 as f32) / 256.0, before, self.front[1] as f32 / 256.0);
        (self.front[0], self.front[2]) = ((at.x * 256.0) as i64, (at.y * 256.0) as i64);
        if col {
            if self.accel > 12 {
                self.accel >>= 1;
            }
            // Steer away from the wall.
            let (ax, az) = ((at.x - before.x) as i64, (at.y - before.y) as i64);
            let (bx, bz) = ((before.x * 256.0) as i64 - self.back[0] >> 8, (before.y * 256.0) as i64 - self.back[2] >> 8);
            if qdist2(ax.abs(), az.abs()) > 128 {
                if ax * bz - az * bx >= 0 {
                    if self.steer < 100 {
                        self.steer += 20;
                    }
                } else if self.steer > -100 {
                    self.steer -= 20;
                }
            }
        }
        // Push the front wheel along where the bars point, to keep the wheels apart.
        let steer_angle = if self.on_ground_front { heading + (self.steer << 3) } else { heading } & 2047;
        let (sx, sz) = (sin(steer_angle), cos(steer_angle));
        let along = steering_push(self.back[0], self.back[2], self.front[0], self.front[2], sx, sz, WHEEL_APART << 8);
        self.front[0] += mul64(along, sx);
        self.front[2] += mul64(along, sz);
        // Up a step too high: not allowed; bounce back if going fast.
        if col {
            let dy = (road.height(self.front[0] as f32 / 256.0, self.front[2] as f32 / 256.0) as i64) - (self.back[1] >> 8);
            if dy.abs() >= 0x40 {
                (self.back[0], self.back[2]) = old_back;
                (self.front[0], self.front[2]) = front_old;
                if self.back_d[0].abs() + self.back_d[2].abs() > 0x200 {
                    for (k, kk) in [(0, 0), (2, 2)] {
                        self.back[k] -= self.back_d[kk] << 2;
                        self.front[k] -= self.back_d[kk] << 2;
                        self.back_d[kk] = -self.back_d[kk];
                        self.back_d[kk] -= self.back_d[kk] >> 2;
                    }
                    if self.back_d[0].abs() + self.back_d[2].abs() > 0x2000 {
                        self.front_dy += 5000;
                        self.back_d[1] += 3500;
                    }
                }
                self.accel >>= 2;
            }
        }
        // The suspension, front and back.
        let ground = |x: i64, z: i64| (road.height(x as f32 / 256.0, z as f32 / 256.0) as i64) << 8;
        let g = ground(self.front[0], self.front[2]);
        self.on_ground_front = suspension(g, &mut self.front[1], &mut self.front_dy, &mut self.wheel_y_front, &mut self.wheel_dy_front);
        let g = ground(self.back[0], self.back[2]);
        self.on_ground_back = suspension(g, &mut self.back[1], &mut self.back_d[1], &mut self.wheel_y_back, &mut self.wheel_dy_back);
        if self.on_ground_back && !self.on_ground_front {
            self.front_dy -= GRAVITY / 2;
        }
        // Face the way the wheels lie.
        let (dx, dy, dz) = ((self.front[0] - self.back[0]) >> 8, (self.front[1] - self.back[1]) >> 8, (self.front[2] - self.back[2]) >> 8);
        let dxdz = qdist2(dx.abs(), dz.abs());
        if dxdz >= 0x10 {
            self.yaw = (calc_angle(dx, dz) + 1024) & 2047;
        }
        self.pitch = calc_angle(dy, dxdz);
        // The wheels turn as it goes.
        let rspeed = qdist2((self.front[0] - front_old.0).abs() >> 8, (self.front[2] - front_old.1).abs() >> 8) * 3;
        let rspeed = if self.accel < 0 { -rspeed } else { rspeed };
        self.wheel_rot_front += rspeed;
        self.wheel_rot_back += rspeed;
    }
}

/// `BIKE_steering_push`: how far along the steering vector (sx, sz) (16.16 unit) to push the
/// front wheel to put it `clen` from the back one.
fn steering_push(bx: i64, bz: i64, fx: i64, fz: i64, sx: i64, sz: i64, clen: i64) -> i64 {
    let (ax, az) = (fx - bx, fz - bz);
    let alen = qdist2(ax.abs(), az.abs()) + 1;
    let (nax, naz) = (div64(ax, alen), div64(az, alen));
    let (npx, npz) = (-naz, nax);
    let adots = mul64(nax, sx) + mul64(naz, sz);
    let pdots = mul64(npx, sx) + mul64(npz, sz);
    let a = mul64(adots, adots) + mul64(pdots, pdots);
    let b = 2 * mul64(adots, alen);
    let c = mul64(alen, alen) - mul64(clen, clen);
    if a.abs() < 0x20 {
        return 0;
    }
    let disc = mul64(b, b) - 4 * mul64(a, c);
    let pm = if disc <= 0 { 0 } else { ((disc as f64).sqrt() as i64) << 8 };
    let (x1, x2) = (-b + pm, -b - pm);
    let x = if x1.abs() < x2.abs() { x1 } else { x2 };
    div64(x, 2 * a)
}

/// `BIKE_process_suspension`: a frame on a spring over a wheel, both falling, the wheel kept
/// out of the ground. Returns whether the wheel is (all but) on the ground.
fn suspension(ground: i64, fy: &mut i64, fdy: &mut i64, wy: &mut i64, wdy: &mut i64) -> bool {
    *fdy += GRAVITY;
    *wdy += GRAVITY;
    let sdist = *fy - *wy;
    let sddist = (WHEEL_SUS << 8) - sdist;
    *fdy += sddist / 8;
    *wdy -= sddist / 8;
    *fy += *fdy;
    *wy += *wdy;
    let ddy = *fdy - *wdy;
    *wdy += ddy / 128;
    *fdy -= ddy / 128;
    let on = *wy <= ground + ((WHEEL_RADIUS + 2) << 8);
    if *wy <= ground + (WHEEL_RADIUS << 8) {
        *wdy = wdy.abs() - (wdy.abs() >> 2);
        *wy = ground + (WHEEL_RADIUS << 8);
    }
    if *fy < *wy + (WHEEL_SUS << 6) {
        *fy = *wy + (WHEEL_SUS << 6);
    }
    *fdy -= *fdy / 512;
    *wdy -= *wdy / 512;
    on
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat tarmac with a wall along x = 40 squares.
    struct Flat;

    impl Road for Flat {
        fn height(&self, _: f32, _: f32) -> f32 {
            0.0
        }
        fn collide(&self, from: Vec2, to: Vec2, _: f32) -> (Vec2, bool) {
            let wall = 40.0 * 256.0 - WHEEL_COL;
            let _ = from;
            if to.x > wall { (Vec2::new(wall, to.y), true) } else { (to, false) }
        }
        fn friction(&self, _: f32, _: f32) -> i64 {
            200
        }
    }

    fn bike() -> Bike {
        let mut b = Bike::new(1, 20.0 * 256.0, 20.0 * 256.0, 0, &Flat);
        b.mode = Mode::Driving;
        b
    }

    #[test]
    fn parked_it_settles_on_its_springs_and_stays_put() {
        let mut b = Bike::new(1, 20.0 * 256.0, 20.0 * 256.0, 0, &Flat);
        let start = b.pos();
        for _ in 0..200 {
            b.control(1, 0, false, false);
            b.tick(&Flat);
        }
        let moved = b.pos() - start;
        assert!(Vec2::new(moved.x, moved.z).length() < 1.0, "a parked bike moved {moved:?}");
        assert!(b.pos().y > 0.0 && b.pos().y < 64.0, "rests at {}", b.pos().y);
    }

    #[test]
    fn the_throttle_moves_it_along_its_heading() {
        let mut b = bike();
        let start = b.pos();
        let (back, front) = b.wheels();
        let ahead = Vec2::new(front.x - back.x, front.z - back.z).normalize();
        for _ in 0..40 {
            b.control(1, 0, false, false);
            b.tick(&Flat);
        }
        let moved = b.pos() - start;
        let along = Vec2::new(moved.x, moved.z).dot(ahead);
        assert!(along > 256.0, "went {along} ahead");
        assert!(Vec2::new(moved.x, moved.z).perp_dot(ahead).abs() < along * 0.1, "drifted sideways");
        let (back, front) = b.wheels();
        assert!(((front - back).length() - 160.0).abs() < 8.0, "wheels {} apart", (front - back).length());
    }

    #[test]
    fn steering_turns_it() {
        let mut b = bike();
        let yaw = b.yaw;
        for _ in 0..40 {
            b.control(1, 1, false, false);
            b.tick(&Flat);
        }
        let turned = ((b.yaw - yaw + 1024) & 2047) - 1024;
        assert!(turned.abs() > 128, "turned only {turned}");
        assert!(b.roll() != 0);
    }

    /// "Bikes can't go backwards!": held in reverse it only creeps.
    #[test]
    fn it_cant_go_backwards() {
        let mut b = bike();
        for _ in 0..40 {
            b.control(-1, 0, false, false);
            b.tick(&Flat);
        }
        assert!(b.accel <= 1 && b.accel > -40);
        assert!(b.speed() < 4, "speed {}", b.speed());
    }
}
