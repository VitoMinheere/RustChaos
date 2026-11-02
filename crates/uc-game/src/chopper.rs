//! Police helicopters (`VT_HELICOPTER`): `CHOPPER_create` and `CHOPPER_fn_normal`
//! (fallen/Source/chopper.cpp), drawn as `CHOPPER_draw_chopper` (drawxtra.cpp) does.
//!
//! It waits on the ground until it has someone to follow (a `VMT_TRACK_TARGET` point's
//! victim) or Darci comes within four squares, spins its rotors up and climbs, then flies
//! after its target with its searchlight on them, keeping well above the rooftops and
//! steering clear of buildings. Chasing the player more than 32 squares from where it
//! started sends it home, where it picks her up again if she comes near.
//!
//! The player can fly one too (#465), which the original never lets them: a chopper parked
//! for them ([`Chopper::parked`], in Explore and Beat Cop) waits on the ground for them to
//! get in, and is flown with a [`Stick`] ([`Chopper::pilot_tick`]). The ones the missions
//! make stay the AI's.
//!
//! It runs on the original's 20 ticks a second in the original's own units: positions in
//! 256ths of a world unit (`WorldPos`), speeds in those a tick, angles 2048 to a turn.

use uc_formats::units::radians;
use glam::Vec3;

pub const TICK: f32 = 1.0 / 20.0;

/// `HARDWIRED_RADIUS`: how far from home it chases the player.
const HARDWIRED_RADIUS: i64 = 8192;
/// `DETECTION_RADIUS`: how near the player has to come to be noticed.
const DETECTION_RADIUS: i64 = 1024;
/// `IGNORAMUS_RADIUS`: how near home it has to be to notice her again.
const IGNORAMUS_RADIUS: i64 = 6144;
/// `PRIM_OBJ_CHOPPER` and `PRIM_OBJ_CHOPPER_BLADES`.
pub const BODY_PRIM: u8 = 74;
pub const ROTOR_PRIM: u8 = 75;
/// `S_HELI`.
pub const HELI_SOUND: usize = 109;

/// `CHOPPER_substate_*` (no patrolling: nothing sets it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Takeoff,
    Landing,
    Landed,
    Tracking,
    Homing,
    /// Someone at the controls, or the rotors still running down after they've got out.
    Piloted,
}

/// The pilot's controls (#465), each -1 to 1: forward, to the right, turning right, and up.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Stick {
    pub ahead: f32,
    pub side: f32,
    pub turn: f32,
    pub climb: f32,
}

/// The rotors' speed it lifts at, of the 500 they run up to.
const LIFT: i32 = 400;
/// The fastest the pilot goes along the ground, in 256ths a tick: the AI's own top speed
/// (`CHOPPER_limit`'s 12288), 960 units a second.
const TOP_SPEED: f32 = 12288.0;
/// How fast it picks up speed with the stick right over (256ths a tick, a tick).
const ACCEL: f32 = 320.0;
/// The fastest it climbs or sinks (256ths a tick): 160 units a second.
const CLIMB: f32 = 2048.0;
/// The highest it goes over the ground or roof under it (world units).
pub const CEILING: f32 = 2560.0;
/// The highest the player can drop out from (world units over the ground under it): a
/// storey.
pub const DROP: f32 = 256.0;
/// How far out from its middle the body and rotors keep clear of walls (world units).
const RADIUS: f32 = 192.0;
/// How far up a wall it can be pushed onto, as onto a kerb or a low roof (world units).
const STEP: f32 = 48.0;

/// What it follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A person, by index.
    Person(usize),
}

/// What the helicopter needs to know about the world each tick.
pub trait Sky {
    /// `PAP_calc_map_height_at`: the ground or the roof at (x, z), world units.
    fn map_height(&self, x: f32, z: f32) -> f32;
    /// `MAV_inside`: (x, y, z) is inside a building.
    fn inside(&self, x: f32, y: f32, z: f32) -> bool;
    /// Where person `i` is (world units), if they're about.
    fn person(&self, i: usize) -> Option<Vec3>;
    /// Whether person `i` is the player (`PlayerID > 0`).
    fn is_player(&self, i: usize) -> bool;
    /// The person event point `way` made, once it has (`EWAY_get_person`).
    fn made_by(&self, way: usize) -> Option<usize>;
}

/// `Chopper`.
#[derive(Debug, Clone)]
pub struct Chopper {
    /// The event point that made it.
    pub way: usize,
    /// `WorldPos`.
    pub x: i32,
    pub y: i32,
    pub z: i32,
    home: (i32, i32, i32),
    dx: i32,
    dy: i32,
    dz: i32,
    /// The way it faces (`DrawMesh.Angle`).
    pub angle: i32,
    ry: i32,
    counter: i32,
    pub rotors: i32,
    rotorspeed: i32,
    speed: i32,
    pub state: State,
    /// The searchlight's brightness, 0 to 250.
    pub light: i32,
    since_takeoff: i32,
    /// The event point whose person it's to follow, until it picks them up.
    pub victim: Option<usize>,
    pub target: Option<Target>,
    /// Where the searchlight is pointed (`spotx`, `spotz`, in 256ths) and how it's moving.
    spot: (i32, i32),
    spot_d: (i32, i32),
    /// The searchlight's way and reach (world units), from [`Chopper::aim_light`].
    pub beam: Vec3,
    /// Parked for the player to fly: it never takes off by itself (#465).
    pub parked: bool,
    /// Who's flying it, if anyone.
    pub pilot: Option<usize>,
}

/// `SDIST2` of a 256ths offset, in world units.
fn dist2(dx: i32, dz: i32) -> i64 {
    let (dx, dz) = ((dx >> 8) as i64, (dz >> 8) as i64);
    dx * dx + dz * dz
}

fn sin(a: i32) -> i32 {
    (radians((a & 2047) as f32).sin() * 65536.0) as i32
}

fn cos(a: i32) -> i32 {
    sin(a + 512)
}

impl Chopper {
    /// `EWAY_SUBTYPE_VEHICLE_HELICOPTER`: 265 above the point, on the ground and still.
    pub fn new(way: usize, at: Vec3, victim: Option<usize>) -> Chopper {
        let (x, y, z) = ((at.x * 256.0) as i32, ((at.y + 265.0) * 256.0) as i32, (at.z * 256.0) as i32);
        Chopper {
            way, x, y, z, home: (x, y, z), dx: 0, dy: 0, dz: 0, angle: 0, ry: 0, counter: 0, rotors: 0, rotorspeed: 0,
            speed: 2048, state: State::Landed, light: 0, since_takeoff: 0, victim, target: None,
            spot: (x, z), spot_d: (0, 0), beam: Vec3::NEG_Y, parked: false, pilot: None,
        }
    }

    /// One parked for the player (#465) at `at` on the ground, its nose along (sin, cos) of
    /// `angle` (2048 to a turn).
    pub fn parked(at: Vec3, angle: i32) -> Chopper {
        // On its skids, 250 up (the mission's are put 265 up).
        let c = Chopper::new(0, at - Vec3::new(0.0, 15.0, 0.0), None);
        Chopper { parked: true, angle: angle & 2047, home: (c.x, c.y, c.z), ..c }
    }

    /// On the ground or a roof, for getting in or out.
    pub fn grounded(&self, sky: &impl Sky) -> bool {
        self.height(sky) <= 24.0
    }

    /// How high its skids are over the ground or roof under it, world units. (Not
    /// `CHOPPER_altitude`, which takes the ground's height in world units from the
    /// helicopter's in 256ths, as the AI's flying is tuned to.)
    pub fn height(&self, sky: &impl Sky) -> f32 {
        self.y as f32 / 256.0 - 250.0 - sky.map_height(self.x as f32 / 256.0, self.z as f32 / 256.0)
    }

    /// The way its nose points, in world xz.
    pub fn nose(&self) -> glam::Vec2 {
        let a = radians(self.angle as f32);
        glam::Vec2::new(a.sin(), a.cos())
    }

    /// The rotors' speed, 0 (still) to 1 (flat out), for their sound.
    pub fn rotor_speed(&self) -> f32 {
        self.rotorspeed as f32 / 500.0
    }

    /// One tick of a parked chopper (#465): with a pilot, the rotors run up and it lifts off
    /// once they're fast enough, climbing or sinking, turning, and going along its nose or to
    /// the side as `stick` says, easing to a hover when let go. It settles on whatever's under
    /// it, roofs as well as the street, and walls stop it. With nobody at the controls it
    /// sinks to the ground and its rotors run down.
    pub fn pilot_tick(&mut self, stick: Stick, sky: &impl Sky) {
        let flown = self.pilot.is_some();
        self.rotors += self.rotorspeed >> 2;
        self.rotorspeed = if flown { (self.rotorspeed + 10).min(500) } else { (self.rotorspeed - 4).max(0) };
        let lift = self.rotorspeed >= LIFT;
        let on_ground = self.grounded(sky);
        let stick = if flown { stick } else { Stick::default() };
        let clamp = |v: f32| v.clamp(-1.0, 1.0);
        // Turning: right turns the nose to the right, as a car's steering does.
        let want_ry = if lift { -clamp(stick.turn) * 24.0 } else { 0.0 };
        self.ry = ((self.ry as f32 * 3.0 + want_ry) / 4.0).round() as i32;
        self.angle = (self.angle + self.ry) & 2047;
        // Along the ground: only off it.
        let (dx, dz) = (self.dx as f32, self.dz as f32);
        let (mut dx, mut dz) = if lift && !on_ground {
            let a = radians(self.angle as f32);
            let (fwd, right) = (glam::Vec2::new(a.sin(), a.cos()), glam::Vec2::new(-a.cos(), a.sin()));
            let push = (fwd * clamp(stick.ahead) + right * clamp(stick.side)) * ACCEL;
            let (mut dx, mut dz) = (dx + push.x, dz + push.y);
            if push == glam::Vec2::ZERO {
                dx *= 0.94;
                dz *= 0.94;
            }
            let v = glam::Vec2::new(dx, dz);
            let v = if v.length() > TOP_SPEED { v.normalize() * TOP_SPEED } else { v };
            (v.x, v.y)
        } else {
            (dx * 0.6, dz * 0.6)
        };
        // Up and down.
        let want_dy = if !lift {
            if on_ground { 0.0 } else { -CLIMB / 2.0 }
        } else if !flown {
            -CLIMB / 2.0
        } else {
            clamp(stick.climb) * CLIMB
        };
        let mut dy = (self.dy as f32 * 3.0 + want_dy) / 4.0;
        // Walls: each way on its own, so it slides along them.
        let (x, z) = (self.x as f32 / 256.0, self.z as f32 / 256.0);
        let bottom = self.y as f32 / 256.0 - 250.0;
        let blocked = |x: f32, z: f32| sky.map_height(x, z) > bottom + STEP;
        if dx != 0.0 && blocked(x + dx / 256.0 + RADIUS * dx.signum(), z) {
            dx = -dx * 0.25;
        }
        if dz != 0.0 && blocked(x, z + dz / 256.0 + RADIUS * dz.signum()) {
            dz = -dz * 0.25;
        }
        let (nx, nz) = (x + dx / 256.0, z + dz / 256.0);
        // The ground or roof under it: it rests on it, and never higher than the ceiling.
        let floor = sky.map_height(nx, nz) + 250.0;
        let mut ny = self.y as f32 / 256.0 + dy / 256.0;
        if ny <= floor {
            ny = floor;
            dy = dy.max(0.0);
        }
        if ny > floor + CEILING {
            ny = floor + CEILING;
            dy = dy.min(0.0);
        }
        (self.x, self.y, self.z) = ((nx * 256.0) as i32, (ny * 256.0) as i32, (nz * 256.0) as i32);
        (self.dx, self.dy, self.dz) = (dx as i32, dy as i32, dz as i32);
        self.light = 0;
        self.state = if !flown && self.rotorspeed == 0 && self.grounded(sky) { State::Landed } else { State::Piloted };
    }

    /// Where it is, world units.
    pub fn pos(&self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32) / 256.0
    }

    /// Its tilt and roll as it flies (`CHOPPER_draw_chopper`): leaning into its way.
    pub fn tilt_roll(&self) -> (i32, i32) {
        ((-self.dz / 64) & 2047, (self.dx / 64) & 2047)
    }

    /// The rotors are turning: heard and blowing dirt about.
    pub fn flying(&self) -> bool {
        self.state != State::Landed
    }

    /// `CHOPPER_altitude`: how high it is above the ground or roof under it, in 256ths.
    fn altitude(&self, sky: &impl Sky) -> i32 {
        (self.y - (250 << 8)) - sky.map_height(self.x as f32 / 256.0, self.z as f32 / 256.0) as i32
    }

    fn set_state(&mut self, state: State, sky: &impl Sky) {
        self.counter = 0;
        match state {
            State::Takeoff => self.since_takeoff = 0,
            State::Tracking => {
                self.pick_up_victim(sky);
                if self.target.is_none() {
                    // "Tracking failed, no target".
                    return self.set_state(State::Idle, sky);
                }
            }
            State::Homing => self.target = None,
            _ => {}
        }
        self.state = state;
    }

    /// The point's person is who it follows, once they're made (`EWAY_get_person`).
    fn pick_up_victim(&mut self, sky: &impl Sky) {
        if let Some(i) = self.victim.and_then(|way| sky.made_by(way)) {
            self.victim = None;
            self.target = Some(Target::Person(i));
        }
    }

    fn target_pos(&self, sky: &impl Sky) -> Option<Vec3> {
        match self.target? {
            Target::Person(i) => sky.person(i),
        }
    }

    /// `CHOPPER_home`: steer for `to` (256ths), turning to face it when it's far off.
    fn home(&mut self, to: (i32, i32), sky: &impl Sky) {
        let (dx, dz) = (to.0 - self.x, to.1 - self.z);
        let dist = dist2(dx, dz);
        let mut accel = 0;
        if dist > 128 * 128 {
            accel = if dist > 512 * 512 { 96 } else { 64 };
        }
        if self.since_takeoff < 255 {
            accel = self.since_takeoff >> 2;
            self.since_takeoff = (self.since_takeoff + 5).min(255);
        }
        self.speed = (dist / 64).clamp(2048, 12288) as i32;
        if accel != 0 {
            self.dx += accel * (to.0 - self.x).signum();
            self.dz += accel * (to.1 - self.z).signum();
        }
        if dist > 512 * 512 || self.counter != 0 {
            // `-Arctan(dx, dz)`: the nose along the way to go.
            let angle = ((dx as f32).atan2(dz as f32) * 2048.0 / std::f32::consts::TAU) as i32 & 2047;
            let mut dangle = angle - self.angle;
            if dangle > 1024 {
                dangle -= 2048;
            }
            if dangle < -1024 {
                dangle += 2048;
            }
            self.ry = (dangle >> 3).clamp(-10, 10);
            if dist > 512 * 512 {
                self.counter = 128;
            }
        }
        if self.counter != 0 {
            self.counter -= 1;
        }
        self.angle = (self.angle + self.ry) & 2047;
        self.ry = (self.ry * 204) >> 8;
        // Avoid buildings, looking up to four squares each way at a square below it.
        let (mx, my, mz) = (self.x as f32 / 256.0, self.y as f32 / 256.0 - 255.0, self.z as f32 / 256.0);
        const AVOID: i32 = 64;
        for d in (128..=0x400).step_by(128).map(|d| d as f32) {
            if sky.inside(mx + d, my, mz) {
                self.dx -= AVOID;
            }
            if sky.inside(mx - d, my, mz) {
                self.dx += AVOID;
            }
            if sky.inside(mx, my, mz + d) {
                self.dz -= AVOID;
            }
            if sky.inside(mx, my, mz - d) {
                self.dz += AVOID;
            }
        }
    }

    /// `CHOPPER_limit`: ease back towards its speed.
    fn limit(&mut self) {
        let s = self.speed;
        if self.dx > s {
            self.dx = (3 * self.dx + s) >> 2;
        }
        if self.dz > s {
            self.dz = (3 * self.dz + s) >> 2;
        }
        if self.dx < -s {
            self.dx = (3 * self.dx - s) >> 2;
        }
        if self.dz < -s {
            self.dz = (3 * self.dz - s) >> 2;
        }
    }

    /// `CHOPPER_damp`.
    fn damp(&mut self) {
        self.dx = (self.dx * 230) >> 8;
        self.dz = (self.dz * 230) >> 8;
    }

    /// `CHOPPER_predict_altitude`: climb or sink towards 675 over the ground or roof ahead
    /// (and at least 50 above its target).
    fn predict_altitude(&mut self, sky: &impl Sky) {
        if self.dx == 0 && self.dz == 0 {
            return;
        }
        let (dx, dz) = (self.dx >> 3, self.dz >> 3);
        let (tx, tz) = ((self.x >> 8) + dx, (self.z >> 8) + dz);
        let mut altitude = (675 << 8) + ((sky.map_height(tx as f32, tz as f32) as i32) << 8);
        if let Some(t) = self.target_pos(sky) {
            let over = ((t.y + 50.0) * 256.0) as i32;
            if altitude < over {
                altitude = over;
            }
        }
        altitude -= self.altitude(sky);
        let dist = (((dx * dx + dz * dz) as f32).sqrt() as i32).max(1);
        altitude /= dist;
        altitude /= 32;
        if self.since_takeoff != 255 {
            altitude = altitude.clamp(-6, 6);
        }
        self.dy += altitude;
        self.dy -= self.dy / 32;
    }

    /// `CHOPPER_fn_normal`, one tick. `player` is the player (Darci).
    pub fn tick(&mut self, player: Option<usize>, sky: &impl Sky) {
        // A parked one is the player's, and never goes off by itself.
        if self.parked {
            return self.pilot_tick(Stick::default(), sky);
        }
        self.rotors += self.rotorspeed >> 2;
        self.x += self.dx;
        self.y += self.dy;
        self.z += self.dz;
        let altitude = self.altitude(sky);
        let me = self.pos();
        // `CHOPPER_radius_broken` against Darci.
        let near = |at: Option<Vec3>, r: i64| at.is_some_and(|p| {
            let (dx, dz) = ((p.x - me.x) as i64, (p.z - me.z) as i64);
            dx * dx + dz * dz <= r * r
        });
        let darci = player.and_then(|p| sky.person(p));
        match self.state {
            State::Idle => {
                self.damp();
                if self.victim.is_some() {
                    self.set_state(State::Tracking, sky);
                } else if near(darci, DETECTION_RADIUS) {
                    if self.target.is_none() {
                        self.target = player.map(Target::Person);
                    }
                    self.set_state(State::Tracking, sky);
                }
            }
            State::Takeoff => {
                self.damp();
                if self.rotorspeed < 500 {
                    self.rotorspeed += 10;
                    if self.rotorspeed > 150 {
                        self.dy += (self.rotorspeed - 150) >> 6;
                        self.dy += 2;
                    }
                } else if altitude < 170 << 8 {
                    self.dy += 10;
                    if self.light < 250 {
                        self.light += 5;
                    }
                } else {
                    self.set_state(State::Idle, sky);
                }
            }
            State::Landing => {
                for _ in 0..4 {
                    self.damp();
                }
                if altitude > 0 {
                    self.dy = (((3 * self.dy - (altitude >> 6)) as f32) * 0.25 - 1.0) as i32;
                } else {
                    self.dy = 0;
                    if self.light > 3 {
                        self.light -= 3;
                    }
                    if self.light > 0 {
                        self.light -= 1;
                    }
                    if self.rotorspeed > 200 {
                        self.rotorspeed -= 1;
                    }
                    if self.rotorspeed > 0 {
                        self.rotorspeed -= 1;
                    } else {
                        self.set_state(State::Landed, sky);
                    }
                }
            }
            State::Landed => {
                if self.victim.is_some() || near(darci, DETECTION_RADIUS) {
                    self.set_state(State::Takeoff, sky);
                }
            }
            State::Tracking => {
                self.pick_up_victim(sky);
                let Some(t) = self.target_pos(sky) else {
                    // Its target is gone: back home.
                    return self.set_state(State::Homing, sky);
                };
                self.predict_altitude(sky);
                self.home(((t.x * 256.0) as i32, (t.z * 256.0) as i32), sky);
                self.limit();
                if let Some(Target::Person(i)) = self.target {
                    let (hx, hz) = ((self.x - self.home.0) as i64 >> 8, (self.z - self.home.2) as i64 >> 8);
                    if sky.is_player(i) && hx * hx + hz * hz > HARDWIRED_RADIUS * HARDWIRED_RADIUS {
                        self.set_state(State::Homing, sky);
                    }
                }
            }
            // Only a parked one is flown, by `pilot_tick`.
            State::Piloted => {}
            State::Homing => {
                self.predict_altitude(sky);
                self.home((self.home.0, self.home.2), sky);
                self.limit();
                let (hx, hz) = ((self.x - self.home.0) as i64 >> 8, (self.z - self.home.2) as i64 >> 8);
                if hx * hx + hz * hz <= IGNORAMUS_RADIUS * IGNORAMUS_RADIUS && near(darci, DETECTION_RADIUS) {
                    self.target = player.map(Target::Person);
                    self.set_state(State::Tracking, sky);
                }
            }
        }
    }

    /// Where the searchlight falls (`CHOPPER_draw_chopper`): drifting after the target's
    /// pelvis `at` (or straight down with none), and how far down it reaches. Moves the spot
    /// on, as the original does each time it draws; here it's each tick.
    pub fn aim_light(&mut self, at: Option<Vec3>) -> Vec3 {
        let me = self.pos();
        let Some(t) = at else {
            self.spot = (self.x, self.z);
            self.spot_d = (0, 0);
            return Vec3::NEG_Y;
        };
        let (tx, ty, tz) = ((t.x * 256.0) as i32, (t.y * 256.0) as i32, (t.z * 256.0) as i32);
        let max = self.speed << 6;
        let pull = self.since_takeoff >> 1;
        self.spot_d.0 += pull * (tx - self.spot.0).signum();
        self.spot_d.1 += pull * (tz - self.spot.1).signum();
        self.spot_d = (self.spot_d.0.clamp(-max, max), self.spot_d.1.clamp(-max, max));
        self.spot.0 += self.spot_d.0 + self.dx;
        self.spot.1 += self.spot_d.1 + self.dz;
        self.spot.0 = ((self.spot.0 as f32 * 24.0 + tx as f32) * 0.04) as i32;
        self.spot.1 = ((self.spot.1 as f32 * 24.0 + tz as f32) * 0.04) as i32;
        // It won't aim up at anyone higher than it is.
        let dy = if ty > self.y { 0.0 } else { (ty - self.y) as f32 / 256.0 };
        Vec3::new(self.spot.0 as f32 / 256.0 - me.x, dy, self.spot.1 as f32 / 256.0 - me.z)
    }

    /// The dirt its rotor wash blows about (`DIRT_gust` twice a tick while low): pairs of
    /// (from, to) in world units, or none if it's high up or the rotors are still.
    pub fn wash(&self, sky: &impl Sky) -> Vec<((i32, i32), (i32, i32))> {
        let altitude = self.altitude(sky);
        if self.rotorspeed <= 0 || altitude >= 1200 << 8 {
            return Vec::new();
        }
        let mag = self.rotorspeed - (altitude >> 10);
        if mag <= 0 {
            return Vec::new();
        }
        let from = (self.x >> 8, self.z >> 8);
        [self.rotors & 2047, (self.rotors + 1024) & 2047]
            .map(|r| (from, ((self.x + ((mag * sin(r)) >> 10)) >> 8, (self.z + ((mag * cos(r)) >> 10)) >> 8)))
            .to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat ground with a tall building over x 40..44 squares.
    struct Open {
        people: Vec<Vec3>,
    }

    impl Sky for Open {
        fn map_height(&self, x: f32, _: f32) -> f32 {
            if (40.0 * 256.0..44.0 * 256.0).contains(&x) { 2000.0 } else { 0.0 }
        }
        fn inside(&self, x: f32, y: f32, z: f32) -> bool {
            self.map_height(x, z) > y
        }
        fn person(&self, i: usize) -> Option<Vec3> {
            self.people.get(i).copied()
        }
        fn is_player(&self, i: usize) -> bool {
            i == 0
        }
        /// Point 7 makes person 1.
        fn made_by(&self, way: usize) -> Option<usize> {
            (way == 7).then_some(1)
        }
    }

    fn run(c: &mut Chopper, sky: &Open, ticks: usize) {
        for _ in 0..ticks {
            c.tick(Some(0), sky);
        }
    }

    #[test]
    fn waits_until_darci_comes_near() {
        let mut sky = Open { people: vec![Vec3::new(20.0 * 256.0, 0.0, 20.0 * 256.0)] };
        let mut c = Chopper::new(1, Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0), None);
        run(&mut c, &sky, 100);
        assert_eq!(c.state, State::Landed);
        sky.people[0] = Vec3::new(31.0 * 256.0, 0.0, 30.0 * 256.0);
        run(&mut c, &sky, 1);
        assert_eq!(c.state, State::Takeoff);
        run(&mut c, &sky, 400);
        assert_eq!(c.state, State::Tracking);
        assert!(c.light >= 200, "light {}", c.light);
        assert!(c.pos().y > 400.0, "only {} up", c.pos().y);
    }

    /// A victim's helicopter takes off at once and follows them about, above them: it
    /// circles them a few squares out rather than hanging still: it accelerates straight
    /// at them along each axis, up to 48 units a tick, and nothing slows it while it tracks.
    #[test]
    fn follows_its_victim() {
        let mut sky = Open { people: vec![Vec3::new(10.0 * 256.0, 0.0, 10.0 * 256.0), Vec3::new(20.0 * 256.0, 0.0, 30.0 * 256.0)] };
        let mut c = Chopper::new(1, Vec3::new(20.0 * 256.0, 0.0, 20.0 * 256.0), Some(7));
        run(&mut c, &sky, 1);
        assert_eq!(c.state, State::Takeoff);
        run(&mut c, &sky, 400);
        assert_eq!(c.target, Some(Target::Person(1)));
        let flat = |v: Vec3| glam::vec2(v.x, v.z);
        // They move on; it follows, passing over them and never far off.
        for at in [Vec3::new(20.0 * 256.0, 0.0, 30.0 * 256.0), Vec3::new(30.0 * 256.0, 0.0, 10.0 * 256.0)] {
            sky.people[1] = at;
            run(&mut c, &sky, 300);
            let (mut closest, mut furthest) = (f32::MAX, 0f32);
            for _ in 0..600 {
                run(&mut c, &sky, 1);
                let d = flat(c.pos()).distance(flat(at));
                (closest, furthest) = (closest.min(d), furthest.max(d));
                assert!(c.pos().y > 500.0);
            }
            assert!(closest < 6.0 * 256.0, "never closer than {}", closest / 256.0);
            assert!(furthest < 16.0 * 256.0, "as far as {}", furthest / 256.0);
        }
    }

    fn fly(c: &mut Chopper, sky: &Open, stick: Stick, ticks: usize) {
        for _ in 0..ticks {
            c.pilot_tick(stick, sky);
        }
    }

    const UP: Stick = Stick { ahead: 0.0, side: 0.0, turn: 0.0, climb: 1.0 };

    /// A parked one stays put for anyone but its pilot, who has to wait for the rotors
    /// before it lifts; then it climbs, no higher than the ceiling.
    #[test]
    fn the_pilot_takes_off_once_the_rotors_are_up() {
        let sky = Open { people: vec![Vec3::new(31.0 * 256.0, 0.0, 30.0 * 256.0)] };
        let mut c = Chopper::parked(Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0), 0);
        for _ in 0..100 {
            c.pilot_tick(UP, &sky);
            c.tick(Some(0), &sky);
        }
        assert_eq!(c.state, State::Landed, "took off with nobody in it");
        c.pilot = Some(0);
        fly(&mut c, &sky, UP, 20);
        assert!(c.height(&sky) < 1.0, "lifted before the rotors were up: {}", c.height(&sky));
        fly(&mut c, &sky, UP, 60);
        assert!(c.height(&sky) > 100.0, "only {} up", c.height(&sky));
        fly(&mut c, &sky, UP, 1000);
        assert!((c.height(&sky) - CEILING).abs() < 1.0, "{} up", c.height(&sky));
        assert_eq!(c.state, State::Piloted);
    }

    /// Forward goes along the nose, turning turns it, and letting go it slows to a hover.
    #[test]
    fn it_goes_where_it_is_pointed() {
        let sky = Open { people: Vec::new() };
        let mut c = Chopper::parked(Vec3::new(20.0 * 256.0, 0.0, 10.0 * 256.0), 0);
        c.pilot = Some(0);
        fly(&mut c, &sky, UP, 100);
        let from = c.pos();
        fly(&mut c, &sky, Stick { ahead: 1.0, ..Default::default() }, 60);
        let moved = c.pos() - from;
        assert!(moved.z > 1000.0 && moved.x.abs() < 10.0, "nose (0, 1) went {moved}");
        let before = c.angle;
        fly(&mut c, &sky, Stick { turn: 1.0, ..Default::default() }, 20);
        assert!((before - c.angle) & 2047 > 100 && (before - c.angle) & 2047 < 1024, "{before} to {}", c.angle);
        fly(&mut c, &sky, Stick::default(), 100);
        let at = c.pos();
        fly(&mut c, &sky, Stick::default(), 10);
        assert!(c.pos().distance(at) < 20.0, "drifting on {}", c.pos().distance(at));
        // To the right of a nose along +z is -x.
        c.angle = 0;
        let at = c.pos();
        fly(&mut c, &sky, Stick { side: 1.0, ..Default::default() }, 30);
        assert!(c.pos().x < at.x - 200.0, "{} to {}", at.x, c.pos().x);
    }

    /// The tall building stops it below its roof, and it lands on the roof from above.
    #[test]
    fn walls_stop_it_and_roofs_hold_it() {
        let sky = Open { people: Vec::new() };
        let mut c = Chopper::parked(Vec3::new(36.0 * 256.0, 0.0, 30.0 * 256.0), 512);
        c.pilot = Some(0);
        fly(&mut c, &sky, UP, 100);
        fly(&mut c, &sky, Stick { climb: 0.0, ..Default::default() }, 10);
        assert!(c.pos().y < 1500.0);
        // Nose along +x, at the wall at 40 squares.
        fly(&mut c, &sky, Stick { ahead: 1.0, ..Default::default() }, 200);
        assert!(c.pos().x < 40.0 * 256.0, "through the wall to {}", c.pos().x / 256.0);
        // Up over it and onto the roof.
        fly(&mut c, &sky, UP, 300);
        fly(&mut c, &sky, Stick { ahead: 1.0, ..Default::default() }, 30);
        fly(&mut c, &sky, Stick::default(), 60);
        assert!((40.0 * 256.0..44.0 * 256.0).contains(&c.pos().x), "not over the roof: {}", c.pos().x / 256.0);
        fly(&mut c, &sky, Stick { climb: -1.0, ..Default::default() }, 400);
        assert!(c.grounded(&sky) && c.pos().y > 2200.0, "at {} over the roof", c.height(&sky));
        // Got out of, the rotors run down and it's landed.
        c.pilot = None;
        fly(&mut c, &sky, Stick::default(), 200);
        assert_eq!(c.state, State::Landed);
    }

    /// It won't chase the player more than 32 squares from home.
    #[test]
    fn gives_up_on_the_player_far_from_home() {
        let mut sky = Open { people: vec![Vec3::new(30.0 * 256.0, 0.0, 10.0 * 256.0)] };
        let mut c = Chopper::new(1, Vec3::new(30.0 * 256.0, 0.0, 10.5 * 256.0), None);
        run(&mut c, &sky, 600);
        assert_eq!(c.state, State::Tracking);
        sky.people[0] = Vec3::new(30.0 * 256.0, 0.0, 100.0 * 256.0);
        let mut homed = false;
        for _ in 0..3000 {
            c.tick(Some(0), &sky);
            homed |= c.state == State::Homing;
        }
        assert!(homed, "never turned for home");
        assert!((c.pos().z - 10.5 * 256.0).abs() < 4.0 * 256.0, "not home: {}", c.pos().z / 256.0);
    }
}
