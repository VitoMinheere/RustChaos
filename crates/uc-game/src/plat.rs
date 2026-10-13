//! Moving platforms: what a `link_platform` point makes of the prop nearest it
//! (`PLAT_create`, `PLAT_process`, fallen/Source/plat.cpp). It patrols the active points of
//! its colour and group in turn, speeding up to its speed and easing in as it arrives, and
//! waits at each: a simple point says how long in tenths of a second (two seconds at others),
//! and ten seconds there means it stops for good.
//!
//! People stand on its top and ride with it; anyone it comes down on is killed. In Grim
//! Gardens someone jumping onto one is put on top of it rather than into it
//! (`playing_level("botanicc.ucm")`). The flags come in `Data[1]`, but the loader assigns
//! rather than adds them, so only the last of those set counts: Target UC's rockets
//! (lock to an axis, and rocket) only get the rocket's exhaust.
//!
//! It runs on the original's 20 ticks a second: speeds in units a tick, waits in
//! milliseconds.

use glam::{Vec2, Vec3};

pub const TICK: f32 = 1.0 / 20.0;

/// `LP_*` (fallen/Headers/Mission.h), as `PLAT_FLAG_*`.
pub const LOCK_TO_AXIS: i32 = 1;
pub const LOCK_ROTATION: i32 = 2;
pub const ROCKET: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Not made yet: still a prop where the map put it.
    Waiting,
    None,
    Goto,
    Pause,
    Stop,
}

#[derive(Debug, Clone)]
pub struct Platform {
    /// The `link_platform` point, and the prop it moves (index into the map's objects).
    pub way: usize,
    pub ob: usize,
    pub prim: u8,
    pub pos: Vec3,
    /// Radians.
    pub yaw: f32,
    pub state: State,
    colour: u8,
    group: u8,
    /// The one flag that counts ([`LOCK_TO_AXIS`], [`LOCK_ROTATION`] or [`ROCKET`]).
    flag: i32,
    /// Which axis it's locked to this leg, for [`LOCK_TO_AXIS`]: 0 x, 1 y, 2 z.
    axis: Option<usize>,
    /// Units a tick: what it wants and what it's doing.
    wspeed: f32,
    speed: f32,
    /// The point it's heading for (or last reached).
    waypoint: Option<usize>,
    /// Milliseconds left of a pause.
    counter: f32,
}

/// What a platform needs of the script.
pub trait Route {
    /// The active point of `colour` and `group` nearest `at` (`EWAY_find_nearest_waypoint`).
    fn nearest(&self, at: Vec3, colour: u8, group: u8) -> Option<usize>;
    /// The next active point of theirs after `after`, by number, coming round again
    /// (`EWAY_find_waypoint`).
    fn next(&self, after: usize, colour: u8, group: u8) -> Option<usize>;
    fn position(&self, way: usize) -> Option<Vec3>;
    /// How long to wait there, in milliseconds (`EWAY_get_delay`).
    fn delay(&self, way: usize, default: f32) -> f32;
}

impl Platform {
    pub fn new(way: usize, ob: usize, prim: u8, pos: Vec3, yaw: f32) -> Platform {
        Platform {
            way, ob, prim, pos, yaw, state: State::Waiting, colour: 0, group: 0, flag: 0, axis: None,
            wspeed: 0.0, speed: 0.0, waypoint: None, counter: 0.0,
        }
    }

    /// The point has gone off: it starts patrolling. `speed` is `Data[0]` (50 if none),
    /// `flags` `Data[1]`.
    pub fn start(&mut self, colour: u8, group: u8, speed: i32, flags: i32) {
        self.state = State::None;
        (self.colour, self.group) = (colour, group);
        self.wspeed = if speed == 0 { 50.0 } else { speed as f32 };
        // `ed.arg2 = ...` for each in turn: the last one set wins.
        self.flag = [LOCK_TO_AXIS, LOCK_ROTATION, ROCKET].into_iter().rfind(|f| flags & f != 0).unwrap_or(0);
    }

    pub fn rocket(&self) -> bool {
        self.flag == ROCKET && self.state == State::Goto
    }

    /// One tick; returns how far it moved.
    pub fn tick(&mut self, route: &dyn Route) -> Vec3 {
        const MILLISECS: f32 = 50.0;
        const TICKS: f32 = 5.0;
        match self.state {
            State::Waiting | State::Stop => {}
            State::None => {
                self.waypoint = None;
                self.state = State::Pause;
                self.counter = 1000.0;
            }
            State::Pause => {
                if self.counter > MILLISECS {
                    self.counter -= MILLISECS;
                    return Vec3::ZERO;
                }
                self.counter = 0.0;
                if self.waypoint.is_none() {
                    self.waypoint = route.nearest(self.pos, self.colour, self.group);
                }
                let next = route.next(self.waypoint.unwrap_or(0), self.colour, self.group);
                match next.and_then(|w| Some((w, route.position(w)?))) {
                    None => {
                        self.waypoint = None;
                        self.counter = 2000.0;
                    }
                    Some((w, at)) => {
                        self.waypoint = Some(w);
                        self.state = State::Goto;
                        self.speed = 0.0;
                        self.axis = (self.flag == LOCK_TO_AXIS).then(|| {
                            let d = (self.pos - at).abs();
                            if d.x > d.y && d.x > d.z { 0 } else if d.y > d.z && d.y > d.x { 1 } else { 2 }
                        });
                    }
                }
            }
            State::Goto => {
                let Some(at) = self.waypoint.and_then(|w| route.position(w)) else {
                    self.state = State::Pause;
                    return Vec3::ZERO;
                };
                let mut d = at - self.pos;
                match self.axis {
                    Some(0) => (d.y, d.z) = (0.0, 0.0),
                    Some(1) => (d.x, d.z) = (0.0, 0.0),
                    Some(_) => (d.x, d.y) = (0.0, 0.0),
                    None => {}
                }
                let a = d.abs();
                let len = a.max_element() + (a.x + a.y + a.z - a.max_element()) / 4.0 + 1.0;
                // Easing in: a quarter as fast within a square, a sixteenth within half...
                let mut wspeed = self.wspeed;
                for near in [0x100, 0x80, 0x40] {
                    if len < near as f32 {
                        wspeed = (wspeed / 4.0).floor();
                    }
                }
                let wspeed = wspeed.max(3.0);
                self.speed = if (self.speed - wspeed).abs() < TICKS {
                    wspeed
                } else if self.speed > wspeed {
                    self.speed - TICKS
                } else {
                    self.speed + TICKS
                }
                .clamp(0.0, 255.0);
                let step = self.speed;
                if len <= step / 256.0 + 4.0 {
                    self.state = State::Pause;
                    self.counter = route.delay(self.waypoint.unwrap_or(0), 2000.0);
                    if self.counter == 10000.0 {
                        self.state = State::Stop;
                    }
                } else {
                    let mut by = d * (step / len);
                    by.y = by.y.clamp(-8.0, 8.0);
                    self.pos += by;
                    return by;
                }
            }
        }
        Vec3::ZERO
    }
}

/// The top of a platform in its own space, from its prim's bounding box.
pub fn top_box(points: &[[i16; 3]]) -> (Vec2, Vec2, f32, f32) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in points {
        let v = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (Vec2::new(lo.x, lo.z), Vec2::new(hi.x, hi.z), lo.y, hi.y)
}
