//! The mission's motorbikes: their physics a turn at a time, and drawn.

use crate::bike::{Bike, Road, WHEEL_COL};
use crate::*;

/// `anim_chunk[9]`: the bike on its own (parked, and being got on and off).
pub(crate) const BIKE_CHUNK: &str = "data\\anim009.all";

/// What a bike sees of the level.
struct Level2<'a>(&'a Level);

impl Road for Level2<'_> {
    fn height(&self, x: f32, z: f32) -> f32 {
        self.0.ground.surface(&self.0.map, x, z, f32::MAX / 2.0)
    }

    fn collide(&self, from: Vec2, to: Vec2, y: f32) -> (Vec2, bool) {
        let at = self.0.ground.slide(from, y, to - from, WHEEL_COL / 2.0);
        (at, (at - to).length() > 0.5)
    }

    /// Every surface as tarmac: the original sorts each floor texture into tarmac, grass,
    /// dirt or slippery by looking at its pixels (`TEXTURE_looks_like`), which isn't here yet.
    fn friction(&self, _: f32, _: f32) -> i64 {
        200
    }
}

/// The bike's anim prim, if the install has it.
pub(crate) fn bike_chunk(level: &Level) -> Option<std::sync::Arc<uc_formats::all::AnimChunk>> {
    let path = uc_formats::resolve(&level.game, BIKE_CHUNK)?;
    let chunk = uc_formats::all::AnimChunk::parse(&std::fs::read(path).ok()?).ok()?;
    Some(std::sync::Arc::new(chunk))
}

impl Game {
    /// `BIKE_create` for a `VT_BIKE` point: parked where it is, facing its way.
    pub(crate) fn add_bike(&mut self, level: &Level, way: usize, pos: Vec3, angle: f32) {
        let yaw = (angle * 2048.0 / std::f32::consts::TAU) as i64;
        let bike = Bike::new(way, pos.x, pos.z, yaw, &Level2(level));
        let fig = bike_chunk(level).and_then(|c| Figure::anim_prim(c, bike.pos(), angle, 1));
        self.bikes.push((bike, fig));
    }

    /// `BIKE_process_normal` for each bike, at 20 turns a second.
    pub(crate) fn run_bikes(&mut self, dt: f32, level: &Level) {
        if self.bikes.is_empty() {
            return;
        }
        self.bike_clock += dt;
        while self.bike_clock >= bike::TICK {
            self.bike_clock -= bike::TICK;
            for (b, _) in &mut self.bikes {
                b.tick(&Level2(level));
            }
        }
        for (b, fig) in &mut self.bikes {
            if let Some(f) = fig {
                f.pos = b.pos() - Vec3::new(0.0, (0x18 + 0x10) as f32, 0.0);
                f.angle = b.yaw as f32 * std::f32::consts::TAU / 2048.0;
                f.advance(dt);
            }
            // The bike's own getting on and off, played out (`advance_keyframe`).
            let done = fig.as_ref().is_none_or(|f| f.finished());
            match b.mode {
                bike::Mode::Mounting if done => b.mode = bike::Mode::Driving,
                bike::Mode::Dismounting if done => {
                    b.mode = bike::Mode::Parked;
                    b.driver = None;
                    if let Some(f) = fig {
                        f.restart_now(1, uc_world::character::Play::Loop);
                    }
                }
                _ => {}
            }
        }
    }

    /// The bike the player is on, if any.
    pub fn riding(&self) -> Option<&Bike> {
        match self.player().map(|p| &p.state) {
            Some(State::Bike { bike, .. }) => self.bikes.get(*bike).map(|(b, _)| b),
            _ => None,
        }
    }

    /// `BIKE_person_can_mount` and `set_person_mount_bike`: a parked bike within a square,
    /// got on from its side (`position_person_for_mounting_bike`: 60 to its left and 30
    /// along, facing across it), the bike playing its own getting-on (`anim_chunk[9]`'s 2).
    /// Darci's set has no `ANIM_BIKE_MOUNT`; she sits down as she does into a car.
    pub(crate) fn mount_near(&mut self, pi: usize) -> bool {
        let at = self.people[pi].fig.pos;
        let Some(k) = self.bikes.iter().position(|(b, _)| {
            b.mode == bike::Mode::Parked && b.driver.is_none() && (b.pos() - at).length() < 256.0
        }) else { return false };
        let (b, fig) = &mut self.bikes[k];
        b.mode = bike::Mode::Mounting;
        b.driver = Some(pi);
        if let Some(f) = fig {
            if !f.restart_now(2, uc_world::character::Play::Once) {
                b.mode = bike::Mode::Driving;
            }
        }
        let heading = (b.yaw + 1024) as f32 * std::f32::consts::TAU / 2048.0;
        let (vx, vz) = (heading.sin(), heading.cos());
        let mid = b.pos();
        let p = &mut self.people[pi];
        p.fig.pos = Vec3::new(mid.x + vz * 60.0 + vx * 30.0, p.fig.pos.y, mid.z - vx * 60.0 + vz * 30.0);
        p.fig.angle = (b.yaw - 512) as f32 * std::f32::consts::TAU / 2048.0;
        p.fig.restart_now(uc_world::character::anim::ENTER_CAR, uc_world::character::Play::Once);
        p.state = State::Bike { bike: k, phase: 0 };
        true
    }

    /// `set_person_dismount_bike`: the bike plays its getting-off (`anim_chunk[9]`'s 5) and
    /// she `ANIM_BIKE_DISMOUNT`.
    pub(crate) fn dismount(&mut self, pi: usize, k: usize) {
        let (b, fig) = &mut self.bikes[k];
        if b.speed() > 2 {
            return;
        }
        b.mode = bike::Mode::Dismounting;
        b.accel = 0;
        if let Some(f) = fig {
            if !f.restart_now(5, uc_world::character::Play::Once) {
                b.mode = bike::Mode::Parked;
            }
        }
        let p = &mut self.people[pi];
        p.fig.restart_now(uc_world::character::anim::BIKE_DISMOUNT, uc_world::character::Play::Once);
        p.state = State::Bike { bike: k, phase: 2 };
    }

    /// The rider's side of it each frame: on once the bike's done getting on
    /// (`SUB_STATE_MOUNTING_BIKE` to `_RIDING_BIKE`), the controls while riding
    /// (`BIKE_control_set`: forward the throttle, back the brake, left and right to steer,
    /// jump for a wheelie), kept on the bike, and off once getting off is done.
    pub(crate) fn ride(&mut self, input: &Controls, level: &Level) {
        for pi in 0..self.people.len() {
            let State::Bike { bike: k, phase } = self.people[pi].state else { continue };
            let Some((b, fig)) = self.bikes.get_mut(k) else { continue };
            match phase {
                0 => {
                    let _ = fig;
                    if b.mode == bike::Mode::Driving {
                        self.people[pi].state = State::Bike { bike: k, phase: 1 };
                    }
                }
                1 => {
                    if self.people[pi].player {
                        let d = input.drive;
                        let accel = if d.throttle > 0.3 { 1 } else { 0 };
                        // The original's left is +1.
                        let steer = if d.steer > 0.3 { -1 } else if d.steer < -0.3 { 1 } else { 0 };
                        b.control(accel, steer, d.throttle < -0.3, d.handbrake);
                    }
                    let (pos, yaw) = (b.pos(), b.yaw);
                    let p = &mut self.people[pi];
                    p.fig.pos = pos - Vec3::new(0.0, 40.0, 0.0);
                    p.fig.angle = yaw as f32 * std::f32::consts::TAU / 2048.0;
                }
                _ => {
                    if self.people[pi].fig.finished() {
                        // Stood beside it, on the ground.
                        let heading = (b.yaw + 1024) as f32 * std::f32::consts::TAU / 2048.0;
                        let (vx, vz) = (heading.sin(), heading.cos());
                        let mid = b.pos();
                        let (x, z) = (mid.x + vz * 60.0 + vx * 30.0, mid.z - vx * 60.0 + vz * 30.0);
                        let y = level.ground.surface(&level.map, x, z, mid.y + 64.0);
                        let p = &mut self.people[pi];
                        p.state = State::Move;
                        p.fig.pos = Vec3::new(x, y, z);
                        p.fig.restart_now(uc_world::character::anim::BREATHE, uc_world::character::Play::Loop);
                    }
                }
            }
        }
    }

    pub(crate) fn draw_bikes(&self, out: &mut BTreeMap<u32, Batch>, level: &Level) {
        let lighter = level.lighter();
        for (_, fig) in &self.bikes {
            if let Some(f) = fig {
                f.append_batches(out, lighter.as_ref());
            }
        }
    }
}
