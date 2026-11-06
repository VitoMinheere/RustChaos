//! Time trials (#519): a course's checkpoints passed in order and timed lap by lap, the car
//! sent back to the last one after a miss or with the player out of it, traffic on or off,
//! and a lap kept for its ghost. A bot drives by steering at each checkpoint in turn.
//! Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::game::trial::{course_index, courses, Car};
use uc_game::{Controls, Drive, Game, Outcome, State};
use uc_world::Level;

use crate::common;
use common::DT;

/// A time trial on course `id` in `car`, with its map loaded.
fn trial(id: &str, car: Car, traffic: bool) -> Option<(Level, Game)> {
    let course = course_index(id).unwrap();
    let level = common::load(&courses()[course].mission)?;
    let g = Game::new_trial(&level, course, car, traffic);
    Some((level, g))
}

/// The controls that steer the car at the next checkpoint, slowing for the bend after it,
/// and back away from whatever it's stuck against. `stuck` is the seconds stuck so far and
/// the seconds left of backing away.
fn steer(g: &Game, stuck: &mut (f32, f32)) -> Controls {
    let t = g.trial().unwrap();
    let c = t.course();
    let n = c.checkpoints.len();
    let car = g.player_vehicle().unwrap();
    let at = vec2(car.pos.x, car.pos.z);
    let (to, after) = (c.checkpoints[t.next], c.checkpoints[(t.next + 1) % n]);
    let want = (to - at).normalize();
    let f = car.forward();
    // Positive cross: the checkpoint is to the right (steer +1 turns right).
    let cross = f.x * want.y - f.y * want.x;
    let steer = (cross * 3.0).clamp(-1.0, 1.0);
    let bend = 1.0 - want.dot((after - to).normalize()).max(0.0);
    let dist = (to - at).length();
    let target = if f.dot(want) < 0.7 { 900.0 } else if dist < 2500.0 { 3000.0 - bend * 2000.0 } else { 3000.0 };
    let throttle = if car.speed > target + 100.0 { -1.0 } else { 1.0 };
    stuck.0 = if car.speed.abs() < 60.0 && stuck.1 <= 0.0 { stuck.0 + DT } else { 0.0 };
    if stuck.0 > 0.5 {
        *stuck = (0.0, 1.0);
    }
    let drive = if stuck.1 > 0.0 {
        stuck.1 -= DT;
        Drive { throttle: -1.0, steer: -steer.signum(), ..Default::default() }
    } else {
        Drive { throttle, steer, ..Default::default() }
    };
    Controls { drive, ..Default::default() }
}

/// Drives for up to `secs`, until `done`; whether it was.
fn drive(g: &mut Game, level: &Level, secs: f32, done: impl Fn(&Game) -> bool) -> bool {
    let mut stuck = (0.0, 0.0);
    for _ in 0..(secs / DT) as usize {
        if done(g) {
            return true;
        }
        let c = steer(g, &mut stuck);
        g.update(DT, &c, level);
    }
    done(g)
}

/// Puts the car on `at` (squares from the map's corner are world units here), facing
/// `heading`, standing still.
fn put_car(g: &mut Game, level: &Level, at: Vec2, heading: f32) {
    let y = level.ground.surface(&level.map, at.x, at.y, level.map.ground_height(at.x, at.y) + 64.0);
    g.vehicles[0].reset(vec3(at.x, y, at.y), heading);
}

/// Three laps of the oval, every checkpoint in order: each lap is timed from the line, the
/// race is the laps together, and it's won at the finish. Nothing was missed on the way.
#[test]
fn the_oval_is_driven_lap_by_lap_through_its_checkpoints() {
    let Some((level, mut g)) = trial("oval", Car::Police, false) else { return };
    assert!(matches!(g.player().unwrap().state, State::InCar { vehicle: 0 }), "not at the wheel");
    assert_eq!(g.vehicles.len(), 1, "traffic with it off");
    let n = g.trial().unwrap().course().checkpoints.len();
    // The checkpoints passed, as they're passed.
    let mut order = Vec::new();
    let mut stuck = (0.0, 0.0);
    for _ in 0..(240.0 / DT) as usize {
        let before = (g.trial().unwrap().next, g.trial().unwrap().laps.len());
        let c = steer(&g, &mut stuck);
        g.update(DT, &c, &level);
        let t = g.trial().unwrap();
        if (t.next, t.laps.len()) != before {
            order.push(before.0);
        }
        if g.outcome.is_some() {
            break;
        }
    }
    let t = g.trial().unwrap();
    let laps = t.lap_times();
    println!("laps {laps:?}, race {:?}, resets {}, time {:.1}", t.total(), t.resets, g.time);
    assert_eq!(g.outcome, Some(Outcome::Won));
    assert_eq!(t.resets, 0, "sent back on a clean run");
    // The start line, then every checkpoint in turn, three times round.
    let want: Vec<usize> = std::iter::once(0).chain((0..3).flat_map(|_| (1..n).chain(std::iter::once(0)))).collect();
    assert_eq!(order, want);
    assert_eq!(laps.len(), 3);
    assert!(laps.iter().all(|&l| l > 10.0 && l < 60.0), "{laps:?}");
    let total = t.total().unwrap();
    assert!((total - laps.iter().sum::<f32>()).abs() < 1e-3, "{total} {laps:?}");
    // The clock started at the line, not as the car set off.
    assert!(total < g.time, "{total} {}", g.time);
    // The last lap's checkpoints, with how far into the lap, stay for the finish.
    let splits: Vec<usize> = t.splits.iter().map(|s| s.0).collect();
    assert_eq!(splits, (1..n).collect::<Vec<_>>());
    assert!(t.splits.windows(2).all(|w| w[0].1 < w[1].1), "{:?}", t.splits);
}

/// Through the start line and the first checkpoint, then straight into the third: a
/// checkpoint missed, so back on the first, facing the second, still to come.
#[test]
fn missing_a_checkpoint_sends_the_car_back_to_the_last() {
    let Some((level, mut g)) = trial("oval", Car::Sports, false) else { return };
    assert!(drive(&mut g, &level, 30.0, |g| g.trial().unwrap().next == 2), "never passed the first checkpoint");
    let c = courses()[course_index("oval").unwrap()].clone();
    put_car(&mut g, &level, c.checkpoints[3], 0.0);
    g.update(DT, &Controls::default(), &level);
    let t = g.trial().unwrap();
    assert_eq!((t.next, t.last, t.resets), (2, Some(1), 1));
    assert_eq!(t.notice.as_ref().map(|n| n.0.as_str()), Some("Missed a checkpoint"));
    let car = &g.vehicles[0];
    assert!(vec2(car.pos.x, car.pos.z).distance(c.checkpoints[1]) < 1.0, "{:?}", car.pos);
    assert!(car.forward().dot((c.checkpoints[2] - c.checkpoints[1]).normalize()) > 0.99, "not facing the next");
    assert_eq!(car.speed, 0.0);
    // Driving past the next one's line, wide of it, is a miss too.
    let (from, to) = (c.checkpoints[1], c.checkpoints[2]);
    let along = (c.checkpoints[3] - from).normalize();
    let side = vec2(along.y, -along.x) * c.radius * 1.5;
    put_car(&mut g, &level, to + side - along * 300.0, along.x.atan2(along.y));
    g.update(DT, &Controls::default(), &level);
    assert_eq!(g.trial().unwrap().resets, 1, "sent back before the line");
    put_car(&mut g, &level, to + side + along * 100.0, along.x.atan2(along.y));
    g.update(DT, &Controls::default(), &level);
    let t = g.trial().unwrap();
    assert_eq!((t.next, t.resets), (2, 2), "the line crossed wide wasn't a miss");
}

/// Out of the car: straight back at the wheel, on the last checkpoint passed.
#[test]
fn getting_out_sends_her_back_to_the_last_checkpoint() {
    let Some((level, mut g)) = trial("oval", Car::Van, false) else { return };
    assert_eq!(g.vehicles[0].kind, Car::Van.kind());
    assert!(drive(&mut g, &level, 30.0, |g| g.trial().unwrap().next == 2), "never passed the first checkpoint");
    // Stopped, and out.
    for _ in 0..(5.0 / DT) as usize {
        if g.player_vehicle().is_some_and(|v| v.speed.abs() < 100.0) {
            break;
        }
        g.update(DT, &Controls { drive: Drive { throttle: -1.0, ..Default::default() }, ..Default::default() }, &level);
    }
    let pi = g.player.unwrap();
    let mut out = false;
    for _ in 0..10 {
        g.update(DT, &Controls { action: true, ..Default::default() }, &level);
        out |= g.trial().unwrap().resets > 0;
        if out {
            break;
        }
    }
    assert!(out, "not sent back: {:?}", g.people[pi].state);
    assert!(matches!(g.people[pi].state, State::InCar { vehicle: 0 }), "{:?}", g.people[pi].state);
    assert_eq!(g.vehicles[0].driver, Some(pi));
    let t = g.trial().unwrap();
    assert_eq!(t.notice.as_ref().map(|n| n.0.as_str()), Some("Back to the last checkpoint"));
    let c = t.course();
    assert!(vec2(g.vehicles[0].pos.x, g.vehicles[0].pos.z).distance(c.checkpoints[1]) < 1.0);
    // And on she drives: the next checkpoint's still the second.
    assert_eq!(t.next, 2);
    assert!(drive(&mut g, &level, 30.0, |g| g.trial().unwrap().next == 3), "couldn't carry on");
}

/// A lap kept as a ghost: every turn of it, in the car, timed as the lap was.
#[test]
fn a_lap_is_kept_for_its_ghost() {
    let Some((level, mut g)) = trial("oval", Car::Police, false) else { return };
    g.record_ghost();
    assert!(drive(&mut g, &level, 120.0, |g| g.trial().unwrap().laps.len() == 1), "no lap done");
    let t = g.trial().unwrap();
    let (a, b) = t.laps[0];
    let lap = g.trial_lap(0).unwrap();
    assert_eq!(lap.poses.len() as u64, b - a + 1);
    assert!((lap.time - t.lap_times()[0]).abs() < 1e-4);
    assert!(lap.poses.iter().all(|p| matches!(p, uc_game::game::ghost::Pose::Car { vehicle: 0, .. })));
    // Its checkpoints, by their place in the course, from the lap's start.
    let n = t.course().checkpoints.len();
    assert_eq!(lap.splits.iter().map(|s| s.0 as usize).collect::<Vec<_>>(), (1..n).collect::<Vec<_>>());
    assert!(lap.splits.windows(2).all(|w| w[0].1 < w[1].1));
    // The ghost waits for the next lap from its start.
    assert_eq!(g.trial_ghost_from(), b);
}

/// Traffic on: the course's cars on the roads, the player's still the first. Off: none.
#[test]
fn traffic_comes_and_goes_with_the_option() {
    let Some((_, on)) = trial("westblock", Car::Police, true) else { return };
    let Some((_, off)) = trial("westblock", Car::Police, false) else { return };
    let want = courses()[course_index("westblock").unwrap()].traffic as usize;
    println!("on: {} cars, {} people; off: {} cars, {} people", on.vehicles.len(), on.people.len(), off.vehicles.len(), off.people.len());
    assert_eq!(off.vehicles.len(), 1);
    assert_eq!(off.people.len(), 1);
    assert!(on.vehicles.len() > want / 2 && on.vehicles.len() <= want + 1, "{} cars", on.vehicles.len());
    assert!(on.people.len() > on.vehicles.len(), "no one on the pavements");
    assert_eq!(on.vehicles[0].driver, on.player);
}
