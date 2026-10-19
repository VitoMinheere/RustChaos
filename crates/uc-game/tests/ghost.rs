//! Ghost races (#186): a run kept turn by turn, on foot and driving, its checkpoints for the
//! splits, and the ghost drawn see-through from it on the next run. Skipped without an
//! install.

use glam::vec2;
use std::collections::BTreeMap;
use uc_game::game::ghost::{Pose, Track};
use uc_game::{Controls, Drive, State};

mod common;
use common::{Run, DT};

/// Runs `secs` straight ahead from the start of `mission`, once its opening talk lets the
/// player go, kept as a ghost from the start.
fn run_ahead(mission: &str, secs: f32) -> Option<(Run, Track)> {
    let mut r = Run::new(common::load(mission)?);
    r.g.record_ghost();
    assert!(r.until(1.0, |r| r.g.in_conversation()), "no opening talk");
    assert!(r.until(20.0, |r| !r.g.in_conversation()), "still talking");
    r.play(secs, Controls { wish: vec2(0.0, 1.0), run: true, ..Default::default() });
    let t = r.g.recorded().unwrap();
    Some((r, t))
}

/// The ghost's batches in `r` now, from `track`.
fn ghost(r: &Run, track: &Track) -> BTreeMap<u32, uc_world::Batch> {
    let mut out = BTreeMap::new();
    r.g.ghost_batches(track, &r.level, &mut out);
    out
}

#[test]
fn a_run_is_kept_a_pose_a_turn() {
    let Some((r, t)) = run_ahead("assault1", 3.0) else { return };
    assert_eq!(t.poses.len() as u64, r.g.turns() + 1);
    assert_eq!(t.time, r.g.time);
    let feet: Vec<_> = t.poses.iter().filter_map(|p| match p { Pose::Foot { pos, .. } => Some(*pos), _ => None }).collect();
    assert!(feet.len() > 60, "{} poses on foot", feet.len());
    let went = feet[0].distance(*feet.last().unwrap());
    assert!(went > 300.0, "the ghost went {went}");
    // Running, the pose moves from keyframe to keyframe.
    let frames: std::collections::BTreeSet<_> = t.poses.iter().filter_map(|p| match p { Pose::Foot { cur, .. } => Some(*cur), _ => None }).collect();
    assert!(frames.len() > 5, "{frames:?}");
}

/// On the next run the ghost is drawn where the last one was, see-through, from the
/// ghost pages only; and not at all while it's right where the player is.
#[test]
fn the_ghost_is_drawn_see_through_ahead() {
    let Some((ran, t)) = run_ahead("assault1", 3.0) else { return };
    let mut r = Run::new(common::load("assault1").unwrap());
    // Standing still at the start for as long as the ghost's run went on.
    r.play(ran.g.time, Controls::default());
    let out = ghost(&r, &t);
    assert!(!out.is_empty(), "no ghost drawn");
    assert!(out.keys().all(|&p| uc_world::ghost_page(p).is_some()), "{:?}", out.keys().collect::<Vec<_>>());
    assert!(out.values().flat_map(|b| &b.vertices).all(|v| v.color[3] > 0 && v.color[3] < 255));
    // At the very start both stand on the same spot: the ghost isn't in the way.
    let fresh = Run::new(common::load("assault1").unwrap());
    assert!(ghost(&fresh, &t).is_empty());
}

/// Driving Bronze: in the squad car the ghost is the car; the lap checkpoints are kept with
/// the turn they were passed on, and a later pass compares with the ghost's.
#[test]
fn the_squad_car_and_its_checkpoints_are_kept() {
    let Some(level) = common::load("testdrive1a") else { return };
    let mut r = Run::new(level);
    r.g.record_ghost();
    // Into the car: from the door, as the driving tests do, and wait for the door's animation.
    r.play(1.0, Controls::default());
    let door = r.g.vehicles[0].door();
    let y = r.g.vehicles[0].pos.y;
    r.stand(glam::vec3(door.x, y, door.y));
    r.play(DT, Controls { action: true, ..Default::default() });
    r.until(5.0, |r| r.g.player_vehicle().is_some());
    assert!(matches!(r.g.people[r.player()].state, State::InCar { vehicle: 0 }), "{:?}", r.g.people[r.player()].state);
    r.play(0.5, Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() });
    // The first lap checkpoint: put the car in it.
    let at = r.point(58);
    r.g.vehicles[0].pos = glam::vec3(at.x, r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0), at.z);
    r.play(0.5, Controls::default());
    let passed = r.g.checkpoints().to_vec();
    let (_, turn) = *passed.iter().find(|c| c.0 == 58).unwrap_or_else(|| panic!("checkpoint 58 not passed: {passed:?}"));
    let t = r.g.recorded().unwrap();
    assert!(matches!(t.poses.last(), Some(Pose::Car { vehicle: 0, .. })), "{:?}", t.poses.last());
    assert!(t.splits.contains(&(58, turn as u32)), "{:?}", t.splits);
    // A run two seconds slower through it is two seconds behind.
    let behind = t.split(58, 0, turn + 60).unwrap();
    assert!((behind - 2.0).abs() < 1e-3, "{behind}");
    // Drawn on a fresh run: the car's meshes, as a ghost.
    let mut fresh = Run::new(common::load("testdrive1a").unwrap());
    fresh.play(r.g.time, Controls::default());
    assert!(!ghost(&fresh, &t).is_empty(), "no ghost car");
}
