//! Time trials (#519): laps of a course against the clock, in a car of the player's choice.
//! Not in the original. A course is a map and a lap of checkpoints, hand-placed in
//! `data/courses.txt`. The checkpoints are shown and passed as the driving tests' are: a
//! navigation beacon on the next one (`MAP_beacon_create`, DDEngine/Source/map.cpp:1181), passed
//! by coming within its radius (`EWAY_COND_PROXIMITY`). Driving past one, out of the car, or
//! the car wrecked: back to the last one passed, in a whole car.
//!
//! The first checkpoint is the start and finish line. The clock starts as the car first
//! crosses it, and each lap ends there.

use crate::*;
use std::sync::OnceLock;

/// A course, as `data/courses.txt` has it, in world units.
#[derive(Debug, Clone, PartialEq)]
pub struct Course {
    pub id: String,
    pub name: String,
    /// The mission whose map it's on (loaded for the map; its script isn't run).
    pub mission: String,
    pub laps: u32,
    /// Where the car starts, and its heading (its nose along (sin, cos)).
    pub start: Vec2,
    pub heading: f32,
    /// How near a checkpoint counts, unless it has its own ([`Course::radius_of`]).
    pub radius: f32,
    /// The cars on the roads with traffic on.
    pub traffic: u8,
    /// Gold, silver and bronze: seconds for the whole race.
    pub medals: [f32; 3],
    /// The checkpoints in order, the first the start and finish.
    pub checkpoints: Vec<Vec2>,
    /// Each checkpoint's own radius, where it has one (a tight turn, a dead end to back
    /// out of).
    pub radii: Vec<Option<f32>>,
}

impl Course {
    /// How near checkpoint `k` counts.
    pub fn radius_of(&self, k: usize) -> f32 {
        self.radii.get(k).copied().flatten().unwrap_or(self.radius)
    }
}

/// The courses, in the order the file has them.
pub fn courses() -> &'static [Course] {
    static COURSES: OnceLock<Vec<Course>> = OnceLock::new();
    COURSES.get_or_init(|| parse(include_str!("../../data/courses.txt")).expect("data/courses.txt"))
}

/// Course `id`'s place in [`courses`].
pub fn course_index(id: &str) -> Option<usize> {
    courses().iter().position(|c| c.id.eq_ignore_ascii_case(id))
}

/// The courses in `text` (see `data/courses.txt`), or what's wrong on which line.
pub fn parse(text: &str) -> Result<Vec<Course>, String> {
    let mut out: Vec<Course> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = |why: &str| format!("line {}: {why}: {line}", n + 1);
        let mut f = line.split_whitespace();
        let word = f.next().unwrap_or_default();
        let nums: Vec<f32> = if word == "course" { Vec::new() } else { f.clone().map(|x| x.parse::<f32>()).collect::<Result<_, _>>().map_err(|_| bad("not a number"))? };
        let want = |k: usize| if nums.len() == k { Ok(()) } else { Err(bad(&format!("wants {k} numbers"))) };
        if word == "course" {
            let (Some(id), Some(mission), Some(laps)) = (f.next(), f.next(), f.next()) else { return Err(bad("wants an id, a mission and laps")) };
            let laps = laps.parse().ok().filter(|&l| l > 0).ok_or_else(|| bad("laps"))?;
            let name = f.collect::<Vec<_>>().join(" ");
            out.push(Course {
                id: id.to_string(), name, mission: mission.to_string(), laps, start: Vec2::ZERO, heading: 0.0, radius: 4.0 * 256.0,
                traffic: 0, medals: [0.0; 3], checkpoints: Vec::new(), radii: Vec::new(),
            });
            continue;
        }
        let c = out.last_mut().ok_or_else(|| bad("before any course"))?;
        match word {
            "start" => {
                want(3)?;
                (c.start, c.heading) = (vec2(nums[0], nums[1]) * 256.0, nums[2].to_radians());
            }
            "radius" => {
                want(1)?;
                c.radius = nums[0] * 256.0;
            }
            "traffic" => {
                want(1)?;
                c.traffic = nums[0] as u8;
            }
            "medals" => {
                want(3)?;
                c.medals = [nums[0], nums[1], nums[2]];
            }
            "cp" => {
                if !(2..=3).contains(&nums.len()) {
                    return Err(bad("wants 2 or 3 numbers"));
                }
                c.checkpoints.push(vec2(nums[0], nums[1]) * 256.0);
                c.radii.push(nums.get(2).map(|r| r * 256.0));
            }
            _ => return Err(bad("unknown")),
        }
    }
    match out.iter().find(|c| c.checkpoints.len() < 2) {
        Some(c) => Err(format!("{}: fewer than two checkpoints", c.id)),
        None => Ok(out),
    }
}

/// What to race in: a car, or the motorbike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Car {
    Police,
    Sports,
    Van,
    Bike,
}

impl Car {
    pub const ALL: [Car; 4] = [Car::Police, Car::Sports, Car::Van, Car::Bike];

    /// Its `VEH_TYPE_*` (fallen/Headers/vehicle.h): the police car, the car (`ENGINE_CAR`,
    /// a little slower than a cop car but lighter on its feet) and the van. None for the
    /// bike, which isn't one of the vehicles (`BIKE_create`).
    pub fn kind(self) -> Option<u8> {
        match self {
            Car::Police => Some(3),
            Car::Sports => Some(1),
            Car::Van => Some(0),
            Car::Bike => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Car::Police => "Police car",
            Car::Sports => "Sports car",
            Car::Van => "Van",
            Car::Bike => "Motorbike",
        }
    }

    /// A word for it, for file names and keys.
    pub fn id(self) -> &'static str {
        match self {
            Car::Police => "police",
            Car::Sports => "sports",
            Car::Van => "van",
            Car::Bike => "bike",
        }
    }

    /// Its place in [`Car::ALL`], and back.
    pub fn index(self) -> u8 {
        Car::ALL.iter().position(|&c| c == self).unwrap_or(0) as u8
    }

    pub fn from_index(i: u8) -> Car {
        Car::ALL.get(i as usize).copied().unwrap_or(Car::Police)
    }

    pub fn from_id(id: &str) -> Option<Car> {
        Car::ALL.into_iter().find(|c| c.id().eq_ignore_ascii_case(id))
    }
}

/// Seconds a notice ("Missed a checkpoint") stays up.
const NOTICE: f32 = 2.5;
/// How far to the side of a checkpoint (in its radii) driving past it counts as missing it.
/// Further off, it's another part of the course that happens to cross its line.
const MISS_WIDTH: f32 = 3.0;

/// A time trial under way.
#[derive(Debug, Clone)]
pub struct Trial {
    /// The course's place in [`courses`].
    pub course: usize,
    pub car: Car,
    pub traffic: bool,
    /// The car's place in the game's vehicles (always the first), or the bike's in its bikes.
    pub vehicle: usize,
    /// The checkpoint to reach next.
    pub next: usize,
    /// The last checkpoint passed, if any since the start.
    pub last: Option<usize>,
    /// The turn the lap under way started on (none before the start line is crossed).
    pub lap_from: Option<u64>,
    /// The laps done: the turns each started and ended on.
    pub laps: Vec<(u64, u64)>,
    /// This lap's checkpoints passed: (checkpoint, turns into the lap).
    pub splits: Vec<(usize, u64)>,
    /// Sent back to a checkpoint, this many times.
    pub resets: u32,
    /// Something to say, and the seconds it has left on screen.
    pub notice: Option<(String, f32)>,
    /// Every checkpoint passed after the start line: (lap, checkpoint, turn).
    passes: Vec<(usize, usize, u64)>,
    /// Where a reset puts the car: on the last checkpoint passed, facing the next.
    back_to: (Vec2, f32),
    /// Where the car was last turn.
    was: Vec2,
}

impl Trial {
    pub fn course(&self) -> &'static Course {
        &courses()[self.course]
    }

    /// The whole race's time, once it's over: seconds from the start line to the finish.
    pub fn total(&self) -> Option<f32> {
        let (from, to) = (self.laps.first()?.0, self.laps.last()?.1);
        (self.laps.len() as u32 >= self.course().laps).then(|| (to - from) as f32 * TURN)
    }

    /// Each lap's time.
    pub fn lap_times(&self) -> Vec<f32> {
        self.laps.iter().map(|&(a, b)| (b - a) as f32 * TURN).collect()
    }

    /// The lap under way (1 the first; 0 before the start line).
    pub fn lap(&self) -> u32 {
        if self.lap_from.is_some() { self.laps.len() as u32 + 1 } else { self.laps.len() as u32 }
    }

    /// Over: every lap done.
    pub fn finished(&self) -> bool {
        self.total().is_some()
    }

    /// The medal the race earned: 0 gold, 1 silver, 2 bronze.
    pub fn medal(&self) -> Option<usize> {
        let t = self.total()?;
        self.course().medals.iter().position(|&m| t <= m)
    }
}

/// The ground under `at` on the map (its road, not the roof of a bridge over it).
fn ground(level: &Level, at: Vec2) -> f32 {
    let base = level.map.ground_height(at.x, at.y);
    level.ground.surface(&level.map, at.x, at.y, base + 64.0)
}

/// The heading from `a` to `b`.
fn heading(a: Vec2, b: Vec2) -> f32 {
    let d = b - a;
    d.x.atan2(d.y)
}

impl Game {
    /// A time trial on `course` (its place in [`courses`]) in `car`, with traffic on the roads
    /// or not: the course's map with no script, the player at the wheel at its start.
    pub fn new_trial(level: &Level, course: usize, car: Car, traffic: bool) -> Game {
        let mut g = Game::build(level, false, false);
        let c = &courses()[course.min(courses().len() - 1)];
        let at = c.start;
        // The car first: the ghost of a lap is drawn from the car it was driven in.
        let y = ground(level, at);
        let vehicle = match car.kind() {
            Some(kind) => {
                g.vehicles.push(Vehicle::new(0, kind, Vec3::new(at.x, y, at.y), c.heading, crate::vehicle::Key::None));
                let v = g.vehicles.len() - 1;
                if let Some(pi) = g.player {
                    g.seat_in(pi, v);
                }
                v
            }
            None => {
                g.add_bike(level, 0, Vec3::new(at.x, y, at.y), c.heading);
                let k = g.bikes.len() - 1;
                if let Some(pi) = g.player {
                    g.place_on_bike(level, k, pi, at, c.heading);
                }
                k
            }
        };
        if traffic {
            g.add_civs(level);
            g.add_traffic(level, c.traffic);
        }
        g.trial = Some(Trial {
            course, car, traffic, vehicle, next: 0, last: None, lap_from: None, laps: Vec::new(), splits: Vec::new(),
            resets: 0, notice: None, passes: Vec::new(), back_to: (at, c.heading), was: at,
        });
        g
    }

    /// The time trial, if this is one.
    pub fn trial(&self) -> Option<&Trial> {
        self.trial.as_ref()
    }

    /// The turn a ghost of the best lap is raced from: the lap under way's start, or now
    /// before the start line (so it waits on the line).
    pub fn trial_ghost_from(&self) -> u64 {
        self.trial.as_ref().and_then(|t| t.lap_from.or(t.laps.last().map(|l| l.0))).unwrap_or(self.turns)
    }

    /// Lap `n` (0 the first) of the run kept as a ghost, as a track of its own: its poses
    /// from the turn it started on, its checkpoints by their place in the course, and its time.
    pub fn trial_lap(&self, n: usize) -> Option<game::ghost::Track> {
        let t = self.trial.as_ref()?;
        let rec = self.recording.as_ref()?;
        let (a, b) = *t.laps.get(n)?;
        let poses = rec.poses.get(a as usize..=b as usize)?.to_vec();
        let splits = t.passes.iter().filter(|s| s.0 == n).map(|&(_, k, at)| (k as u32, (at - a) as u32)).collect();
        Some(game::ghost::Track { time: (b - a) as f32 * TURN, poses, splits })
    }

    /// Puts person `i` at the wheel of car `v`, as if they'd got in.
    fn seat_in(&mut self, i: usize, v: usize) {
        self.entering.retain(|&(k, _)| k != i);
        let pos = self.vehicles[v].pos;
        self.vehicles[v].driver = Some(i);
        let p = &mut self.people[i];
        (p.knocked_out, p.out_cold, p.health) = (false, false, p.max_health);
        p.stand_still();
        p.fig.pos = pos;
        p.state = State::InCar { vehicle: v };
    }

    /// Every turn: the next checkpoint passed or missed, and the player back on the last one
    /// if she's out of the car.
    pub(crate) fn run_trial(&mut self, dt: f32, level: &Level) {
        let Some(mut t) = self.trial.take() else { return };
        if let Some((_, left)) = &mut t.notice {
            *left -= dt;
            if *left <= 0.0 {
                t.notice = None;
            }
        }
        if self.outcome.is_none() {
            self.trial_turn(&mut t, level);
        }
        self.trial = Some(t);
    }

    fn trial_turn(&mut self, t: &mut Trial, level: &Level) {
        let Some(pi) = self.player else { return };
        let c = t.course();
        let n = c.checkpoints.len();
        // This turn ends on the next.
        let now = self.turns + 1;
        let Some(at) = self.trial_aboard(t, pi) else {
            self.trial_reset(t, level, "Back to the last checkpoint");
            return;
        };
        let was = std::mem::replace(&mut t.was, at);
        let cp = c.checkpoints[t.next];
        let r = c.radius_of(t.next);
        if at.distance(cp) < r {
            self.trial_passed(t, now);
            return;
        }
        // Past the next one's line (square to the course there) but wide of it.
        let along = (c.checkpoints[(t.next + 1) % n] - c.checkpoints[(t.next + n - 1) % n]).normalize_or_zero();
        let crossed = (was - cp).dot(along) < 0.0 && (at - cp).dot(along) >= 0.0;
        let wide = (at - cp).perp_dot(along).abs();
        let past = crossed && wide >= r && wide < r * MISS_WIDTH;
        // Or through a checkpoint further on, once under way. Not one already passed: backing
        // out of a dead end (the Advanced Circuit's) goes back past them.
        let other = t.lap_from.is_some() && t.next != 0
            && (t.next + 1..n).any(|k| at.distance(c.checkpoints[k]) < c.radius_of(k));
        if past || other {
            self.trial_reset(t, level, "Missed a checkpoint");
        }
    }

    /// Where the player's car or bike is, if she's still on it and it's whole.
    fn trial_aboard(&self, t: &Trial, pi: usize) -> Option<Vec2> {
        let pos = match self.people[pi].state {
            State::InCar { vehicle } if t.car != Car::Bike && vehicle == t.vehicle && !self.vehicles[vehicle].wrecked() => self.vehicles[vehicle].pos,
            State::Bike { bike, phase: 0 | 1 } if t.car == Car::Bike && bike == t.vehicle => self.bikes[bike].0.pos(),
            _ => return None,
        };
        Some(vec2(pos.x, pos.z))
    }

    /// Checkpoint `t.next` reached at turn `now`.
    fn trial_passed(&mut self, t: &mut Trial, now: u64) {
        let c = t.course();
        let n = c.checkpoints.len();
        let k = t.next;
        if k == 0 {
            if let Some(from) = t.lap_from {
                t.laps.push((from, now));
            }
            if t.laps.len() as u32 >= c.laps {
                t.lap_from = None;
                self.outcome = Some(Outcome::Won);
            } else {
                t.lap_from = Some(now);
                t.splits.clear();
            }
        } else if let Some(from) = t.lap_from {
            t.splits.push((k, now - from));
            t.passes.push((t.laps.len(), k, now));
        }
        t.last = Some(k);
        t.next = (k + 1) % n;
        t.back_to = (c.checkpoints[k], heading(c.checkpoints[k], c.checkpoints[t.next]));
    }

    /// Back on the last checkpoint passed (or the start), facing the next, in a whole car.
    fn trial_reset(&mut self, t: &mut Trial, level: &Level, why: &str) {
        let Some(pi) = self.player else { return };
        let (at, yaw) = t.back_to;
        if t.car == Car::Bike {
            self.place_on_bike(level, t.vehicle, pi, at, yaw);
        } else {
            let y = ground(level, at);
            let v = &mut self.vehicles[t.vehicle];
            v.reset(Vec3::new(at.x, y, at.y), yaw);
            (v.health, v.stalled, v.hurt_by, v.door_swing) = (300, false, None, None);
            // Whole again, it can be wrecked again.
            self.tally.blown.remove(&t.vehicle);
            self.seat_in(pi, t.vehicle);
        }
        t.was = at;
        t.resets += 1;
        t.notice = Some((why.to_string(), NOTICE));
    }

    /// The beacons: the next checkpoint, numbered, or the start and finish line.
    pub(crate) fn trial_beacons(&self) -> Vec<(String, Vec3)> {
        let Some(t) = &self.trial else { return Vec::new() };
        if self.outcome.is_some() {
            return Vec::new();
        }
        let c = t.course();
        let at = c.checkpoints[t.next];
        let label = match (t.next, t.lap_from) {
            (0, None) => "Start line".to_string(),
            (0, Some(_)) if t.laps.len() as u32 + 1 >= c.laps => "Finish line".to_string(),
            (0, Some(_)) => "Lap line".to_string(),
            (k, _) => format!("Checkpoint {k}/{}", c.checkpoints.len() - 1),
        };
        vec![(label, Vec3::new(at.x, 0.0, at.y))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_courses_file_reads() {
        let all = courses();
        assert!(all.len() >= 2);
        let oval = &all[course_index("oval").unwrap()];
        assert_eq!((oval.mission.as_str(), oval.laps, oval.name.as_str()), ("testdrive1a", 3, "Oval Circuit"));
        assert_eq!(oval.checkpoints.len(), 9);
        assert_eq!(oval.start, vec2(59.5, 33.1) * 256.0);
        assert!(oval.medals[0] < oval.medals[1] && oval.medals[1] < oval.medals[2]);
        // The ids are told apart.
        let mut ids: Vec<_> = all.iter().map(|c| c.id.to_ascii_lowercase()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
    }

    #[test]
    fn a_bad_line_is_said_where() {
        assert!(parse("cp 1 2").unwrap_err().contains("line 1"));
        let e = parse("course a m 2 A\nstart 1 2\n").unwrap_err();
        assert!(e.contains("line 2") && e.contains("3 numbers"), "{e}");
        assert!(parse("course a m 2 A\ncp 1 2\n").unwrap_err().contains("fewer than two"));
        assert!(parse("course a m 2 A\ncp 1 2 3 4\n").unwrap_err().contains("2 or 3 numbers"));
    }

    #[test]
    fn a_checkpoint_can_have_its_own_radius() {
        let c = &parse("course a m 2 A\nradius 4\ncp 1 2\ncp 3 4 2.5\n").unwrap()[0];
        assert_eq!((c.radius_of(0), c.radius_of(1)), (4.0 * 256.0, 2.5 * 256.0));
        // The Advanced Circuit's dead ends are tighter than the rest.
        let silver = &courses()[course_index("silver").unwrap()];
        assert_eq!(silver.radius_of(8), 2.0 * 256.0);
    }

    #[test]
    fn cars_go_by_index_and_id() {
        for c in Car::ALL {
            assert_eq!(Car::from_index(c.index()), c);
            assert_eq!(Car::from_id(c.id()), Some(c));
        }
    }
}
