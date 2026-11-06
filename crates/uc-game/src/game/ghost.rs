//! Ghost races (#186): a run as it looked, turn by turn, raced on the next try as a
//! see-through Darci (or squad car) that nothing touches or sees.
//!
//! A ghost keeps what the player looked like each turn (where, which way, the two keyframes of
//! the pose; or the car's place, tilt, steering and wheels), not the inputs that made it: a
//! replay of inputs only plays back on the build that recorded it, while keyframes are the
//! game's own data and mean the same in every build. It also keeps the turn each checkpoint
//! was passed on (a point the script sets off by someone getting somewhere), for the splits.

use crate::*;

/// The file's first bytes, with the format's version in the last.
const MAGIC: &[u8; 4] = b"UCG1";

/// The player as one turn left them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pose {
    /// Nobody to show (no player yet).
    None,
    /// On foot: where the figure is drawn (feet, less any lift), its angle, and the pose
    /// between keyframes `cur` and `next`, `tween` (0..256) from the first.
    Foot { pos: Vec3, angle: f32, cur: u32, next: u32, tween: f32 },
    /// Driving car `vehicle` (its index in the mission's vehicles).
    Car { vehicle: u32, pos: Vec3, yaw: f32, pitch: f32, roll: f32, steer: f32, spin: f32 },
    /// Riding bike `bike` (its index in the mission's bikes): the bike where it's drawn, its
    /// heading, and the rider as [`Pose::Foot`] has them.
    Bike { bike: u32, pos: Vec3, yaw: f32, rider: Vec3, angle: f32, cur: u32, next: u32, tween: f32 },
}

/// A recorded run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Track {
    /// The mission clock when it was won.
    pub time: f32,
    /// The player after each turn: `poses[t]` after `t` turns.
    pub poses: Vec<Pose>,
    /// The checkpoints passed: (event point, the turn after which it had gone off).
    pub splits: Vec<(u32, u32)>,
}

/// The shortest way round from angle `a` to `b`, `t` of the way.
fn turn(a: f32, b: f32, t: f32) -> f32 {
    let d = (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    a + d * t
}

impl Track {
    /// The pose `t` turns in (fractions between turns blend the two), held at the end.
    pub fn pose_at(&self, t: f32) -> Pose {
        let Some(last) = self.poses.len().checked_sub(1) else { return Pose::None };
        let t = t.clamp(0.0, last as f32);
        let k = (t.floor() as usize).min(last);
        let (a, b, f) = (self.poses[k], self.poses[(k + 1).min(last)], t - k as f32);
        match (a, b) {
            (Pose::Foot { pos, angle, cur, next, tween }, Pose::Foot { pos: p2, angle: a2, cur: c2, next: n2, tween: t2 }) => {
                // Blending the pose only within one pair of keyframes; past it, the first's.
                let tween = if (cur, next) == (c2, n2) { tween + (t2 - tween) * f } else { tween };
                // A jump of more than two squares (a respawn) isn't blended.
                let pos = if pos.distance(p2) > 512.0 { pos } else { pos.lerp(p2, f) };
                Pose::Foot { pos, angle: turn(angle, a2, f), cur, next, tween }
            }
            (Pose::Car { vehicle, pos, yaw, pitch, roll, steer, spin }, Pose::Car { vehicle: v2, pos: p2, yaw: y2, pitch: pi2, roll: r2, steer: s2, spin: w2 }) if vehicle == v2 => {
                Pose::Car {
                    vehicle, pos: pos.lerp(p2, f), yaw: turn(yaw, y2, f), pitch: pitch + (pi2 - pitch) * f,
                    roll: roll + (r2 - roll) * f, steer: steer + (s2 - steer) * f, spin: turn(spin, w2, f),
                }
            }
            (Pose::Bike { bike, pos, yaw, rider, angle, cur, next, tween }, Pose::Bike { bike: b2, pos: p2, yaw: y2, rider: r2, angle: a2, .. }) if bike == b2 => {
                // A reset (more than two squares) isn't blended.
                if pos.distance(p2) > 512.0 {
                    return a;
                }
                Pose::Bike { bike, pos: pos.lerp(p2, f), yaw: turn(yaw, y2, f), rider: rider.lerp(r2, f), angle: turn(angle, a2, f), cur, next, tween }
            }
            _ => a,
        }
    }

    /// How far ahead (negative) or behind (positive) of this run, in seconds, a run is that
    /// passes the `nth` time through `point` (0 the first) after `turns` turns.
    pub fn split(&self, point: usize, nth: usize, turns: u64) -> Option<f32> {
        let (_, at) = self.splits.iter().filter(|s| s.0 as usize == point).nth(nth)?;
        Some((turns as f32 - *at as f32) * TURN)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        let f = |out: &mut Vec<u8>, x: f32| out.extend_from_slice(&x.to_le_bytes());
        let u = |out: &mut Vec<u8>, x: u32| out.extend_from_slice(&x.to_le_bytes());
        f(&mut out, self.time);
        u(&mut out, self.poses.len() as u32);
        for p in &self.poses {
            match *p {
                Pose::None => out.push(0),
                Pose::Foot { pos, angle, cur, next, tween } => {
                    out.push(1);
                    for x in [pos.x, pos.y, pos.z, angle] {
                        f(&mut out, x);
                    }
                    u(&mut out, cur);
                    u(&mut out, next);
                    f(&mut out, tween);
                }
                Pose::Car { vehicle, pos, yaw, pitch, roll, steer, spin } => {
                    out.push(2);
                    u(&mut out, vehicle);
                    for x in [pos.x, pos.y, pos.z, yaw, pitch, roll, steer, spin] {
                        f(&mut out, x);
                    }
                }
                Pose::Bike { bike, pos, yaw, rider, angle, cur, next, tween } => {
                    out.push(3);
                    u(&mut out, bike);
                    for x in [pos.x, pos.y, pos.z, yaw, rider.x, rider.y, rider.z, angle] {
                        f(&mut out, x);
                    }
                    u(&mut out, cur);
                    u(&mut out, next);
                    f(&mut out, tween);
                }
            }
        }
        u(&mut out, self.splits.len() as u32);
        for &(point, at) in &self.splits {
            u(&mut out, point);
            u(&mut out, at);
        }
        out
    }

    /// A track from [`Track::to_bytes`]; None if it isn't one, or is cut short.
    pub fn from_bytes(bytes: &[u8]) -> Option<Track> {
        let mut r = Reader(bytes.strip_prefix(MAGIC)?);
        let time = r.f()?;
        let n = r.u()? as usize;
        let mut poses = Vec::with_capacity(n.min(1 << 20));
        for _ in 0..n {
            poses.push(match r.byte()? {
                0 => Pose::None,
                1 => {
                    let (x, y, z, angle) = (r.f()?, r.f()?, r.f()?, r.f()?);
                    Pose::Foot { pos: Vec3::new(x, y, z), angle, cur: r.u()?, next: r.u()?, tween: r.f()? }
                }
                2 => {
                    let vehicle = r.u()?;
                    let (x, y, z) = (r.f()?, r.f()?, r.f()?);
                    Pose::Car { vehicle, pos: Vec3::new(x, y, z), yaw: r.f()?, pitch: r.f()?, roll: r.f()?, steer: r.f()?, spin: r.f()? }
                }
                3 => {
                    let bike = r.u()?;
                    let (x, y, z, yaw) = (r.f()?, r.f()?, r.f()?, r.f()?);
                    let (rx, ry, rz, angle) = (r.f()?, r.f()?, r.f()?, r.f()?);
                    Pose::Bike { bike, pos: Vec3::new(x, y, z), yaw, rider: Vec3::new(rx, ry, rz), angle, cur: r.u()?, next: r.u()?, tween: r.f()? }
                }
                _ => return None,
            });
        }
        let n = r.u()? as usize;
        let splits = (0..n).map(|_| Some((r.u()?, r.u()?))).collect::<Option<Vec<_>>>()?;
        Some(Track { time, poses, splits })
    }
}

/// Reads a track's little-endian fields in turn.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let (&b, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(b)
    }

    fn word(&mut self) -> Option<[u8; 4]> {
        let (w, rest) = self.0.split_first_chunk::<4>()?;
        self.0 = rest;
        Some(*w)
    }

    fn u(&mut self) -> Option<u32> {
        self.word().map(u32::from_le_bytes)
    }

    fn f(&mut self) -> Option<f32> {
        self.word().map(f32::from_le_bytes)
    }
}

impl Game {
    /// Starts keeping this run as a ghost, from the pose it's in now.
    pub fn record_ghost(&mut self) {
        let mut t = Track::default();
        t.poses.push(self.pose());
        self.recording = Some(t);
    }

    /// The run kept so far, with the mission clock as its time.
    pub fn recorded(&self) -> Option<Track> {
        self.recording.as_ref().map(|t| Track { time: self.time, ..t.clone() })
    }

    /// The checkpoints this run has passed: (event point, the turn after which it had).
    pub fn checkpoints(&self) -> &[(usize, u64)] {
        &self.passed
    }

    /// The player as they are now, for a ghost.
    fn pose(&self) -> Pose {
        let Some(p) = self.player() else { return Pose::None };
        if let (Some(v), State::InCar { vehicle }) = (self.player_vehicle(), &p.state) {
            return Pose::Car {
                vehicle: *vehicle as u32, pos: v.pos, yaw: v.yaw, pitch: v.pitch, roll: v.roll, steer: v.steer, spin: v.wheel_spin,
            };
        }
        let (cur, next, tween) = p.fig.keyframes();
        if let State::Bike { bike, .. } = p.state {
            if let Some((b, _)) = self.bikes.get(bike) {
                let yaw = uc_formats::units::radians(b.yaw as f32);
                return Pose::Bike { bike: bike as u32, pos: b.pos(), yaw, rider: p.fig.drawn_pos(), angle: p.fig.angle, cur: cur as u32, next: next as u32, tween };
            }
        }
        Pose::Foot { pos: p.fig.drawn_pos(), angle: p.fig.angle, cur: cur as u32, next: next as u32, tween }
    }

    /// After a turn: its checkpoints, and the pose, if this run is being kept.
    pub(crate) fn keep_ghost(&mut self) {
        let turns = self.turns;
        let passed = self.script.as_mut().map_or(Vec::new(), |s| s.take_passed());
        self.passed.extend(passed.iter().map(|&p| (p, turns)));
        if let Some(mut t) = self.recording.take() {
            t.poses.push(self.pose());
            t.splits.extend(passed.iter().map(|&p| (p as u32, turns as u32)));
            self.recording = Some(t);
        }
    }

    /// `track` where it was this far into the run (between the last two turns, as everything
    /// else is drawn), see-through: drawn from the player's own figure, or from the car it
    /// drove. It fades out as the player comes close, so it doesn't get in the way.
    pub fn ghost_batches(&self, track: &Track, level: &Level, out: &mut BTreeMap<u32, Batch>) {
        self.ghost_batches_from(track, 0, level, out);
    }

    /// As [`Game::ghost_batches`], with the track raced from turn `from` of this run: a time
    /// trial's best lap, from the start of the lap under way (#519).
    pub fn ghost_batches_from(&self, track: &Track, from: u64, level: &Level, out: &mut BTreeMap<u32, Batch>) {
        let t = (self.turns.saturating_sub(from) as f32 - 1.0 + self.between()).max(0.0);
        let pose = track.pose_at(t);
        let mut mine = BTreeMap::new();
        let lighter = level.lighter();
        let at = match pose {
            Pose::None => return,
            Pose::Foot { pos, angle, cur, next, tween } => {
                let Some(p) = self.player() else { return };
                let mut fig = p.fig.clone();
                (fig.pos, fig.angle, fig.lift, fig.cancel_rise, fig.pin_root) = (pos, angle, 0.0, false, false);
                fig.show_keyframes(cur as usize, next as usize, tween);
                fig.append_batches(&mut mine, lighter.as_ref());
                pos
            }
            Pose::Car { vehicle, pos, yaw, pitch, roll, steer, spin } => {
                let Some(v) = self.vehicles.get(vehicle as usize) else { return };
                let mut v = v.clone();
                (v.pos, v.yaw, v.pitch, v.roll, v.steer, v.wheel_spin) = (pos, yaw, pitch, roll, steer, spin);
                super::draw::car(&v, level, lighter.as_ref(), &mut mine);
                pos
            }
            Pose::Bike { bike, pos, yaw, rider, angle, cur, next, tween } => {
                // The bike as `run_bikes` draws it, and her on it.
                if let Some(mut f) = self.bikes.get(bike as usize).and_then(|(_, f)| f.clone()) {
                    (f.pos, f.angle) = (pos - Vec3::new(0.0, (0x18 + 0x10) as f32, 0.0), yaw);
                    f.append_batches(&mut mine, lighter.as_ref());
                }
                if let Some(p) = self.player() {
                    let mut fig = p.fig.clone();
                    (fig.pos, fig.angle, fig.lift, fig.cancel_rise, fig.pin_root) = (rider, angle, 0.0, false, false);
                    fig.show_keyframes(cur as usize, next as usize, tween);
                    fig.append_batches(&mut mine, lighter.as_ref());
                }
                pos
            }
        };
        // Mostly there, and gone within a square of the player (or their car).
        let me = self.player_vehicle().map(|v| v.pos).or(self.player().map(|p| p.fig.pos));
        let near = me.map_or(1.0, |me| ((me.distance(at) - 128.0) / 256.0).clamp(0.0, 1.0));
        let alpha = (185.0 * near) as u8;
        if alpha == 0 {
            return;
        }
        for (page, b) in mine {
            if page >= uc_world::GHOST {
                continue;
            }
            let into = out.entry(uc_world::GHOST + page).or_default();
            let base = into.vertices.len() as u32;
            // Cyan, lit as it would be, so it stands out against pale walls too.
            into.vertices.extend(b.vertices.into_iter().map(|mut v| {
                let [r, g, bl, _] = v.color;
                v.color = [r / 4 + 30, (g / 3).saturating_add(165), (bl / 3).saturating_add(185), alpha];
                v
            }));
            into.indices.extend(b.indices.into_iter().map(|i| i + base));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foot(x: f32, angle: f32, tween: f32) -> Pose {
        Pose::Foot { pos: Vec3::new(x, 0.0, 0.0), angle, cur: 7, next: 8, tween }
    }

    fn track() -> Track {
        let car = Pose::Car { vehicle: 3, pos: Vec3::new(1.0, 2.0, 3.0), yaw: 0.5, pitch: 0.01, roll: -0.02, steer: 0.3, spin: 6.0 };
        Track { time: 41.5, poses: vec![Pose::None, foot(0.0, 0.0, 0.0), foot(100.0, 1.0, 128.0), car], splits: vec![(58, 2), (59, 3), (58, 3)] }
    }

    #[test]
    fn a_track_reads_back_as_written() {
        let t = track();
        let bytes = t.to_bytes();
        assert_eq!(Track::from_bytes(&bytes), Some(t));
        // Cut short, or not a track at all: nothing, rather than half of one.
        assert_eq!(Track::from_bytes(&bytes[..bytes.len() - 1]), None);
        assert_eq!(Track::from_bytes(b"UCG2\0\0\0\0"), None);
        assert_eq!(Track::from_bytes(&[]), None);
    }

    /// On the bike (a time trial's, #519): the bike and its rider, read back as written, and
    /// blended between turns.
    #[test]
    fn a_ride_on_the_bike_reads_back_and_blends() {
        let bike = |x: f32, yaw: f32| Pose::Bike { bike: 0, pos: Vec3::new(x, 5.0, 6.0), yaw, rider: Vec3::new(x, 1.0, 6.0), angle: -0.1, cur: 9, next: 10, tween: 64.0 };
        let t = Track { time: 3.0, poses: vec![bike(0.0, 1.0), bike(100.0, 2.0), bike(2000.0, 2.0)], splits: Vec::new() };
        assert_eq!(Track::from_bytes(&t.to_bytes()), Some(t.clone()));
        match t.pose_at(0.5) {
            Pose::Bike { pos, yaw, rider, .. } => {
                assert!((pos.x - 50.0).abs() < 1e-3 && (rider.x - 50.0).abs() < 1e-3, "{pos} {rider}");
                assert!((yaw - 1.5).abs() < 1e-3, "{yaw}");
            }
            p => panic!("{p:?}"),
        }
        // Sent back to a checkpoint: not blended across the jump.
        assert_eq!(t.pose_at(1.5), t.poses[1]);
    }

    #[test]
    fn between_turns_the_pose_is_blended_and_held_at_the_end() {
        let t = track();
        match t.pose_at(1.5) {
            Pose::Foot { pos, angle, tween, .. } => {
                assert!((pos.x - 50.0).abs() < 1e-3, "{pos}");
                assert!((angle - 0.5).abs() < 1e-3 && (tween - 64.0).abs() < 1e-3, "{angle} {tween}");
            }
            p => panic!("{p:?}"),
        }
        // Foot to car isn't blended; past the end it stays in the car.
        assert_eq!(t.pose_at(2.5), t.poses[2]);
        assert_eq!(t.pose_at(99.0), t.poses[3]);
        assert_eq!(Track::default().pose_at(1.0), Pose::None);
    }

    #[test]
    fn splits_compare_the_same_pass_through_a_checkpoint() {
        let t = track();
        // Point 58 the second time: the ghost was there after 3 turns, this run after 6.
        assert_eq!(t.split(58, 1, 6), Some(3.0 * TURN));
        assert_eq!(t.split(59, 0, 2), Some(-TURN));
        assert_eq!(t.split(58, 2, 9), None);
        assert_eq!(t.split(60, 0, 9), None);
    }
}
