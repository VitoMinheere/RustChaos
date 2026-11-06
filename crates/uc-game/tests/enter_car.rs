//! Getting into a car (`set_person_enter_vehicle`, fallen/Source/Person.cpp:8254): lined up
//! at the nearer door, `ANIM_ENTER_CAR` at the driver's and `ANIM_ENTER_TAXI` at the
//! passenger's, and in once it's played out. Driving Bronze's unlocked squad car (point 2).
//! Skipped without an install.

use glam::Vec3;
use uc_game::{Controls, Game, State};
use uc_world::character::anim;
use uc_world::Level;

use crate::common;
use common::DT;

fn load() -> Option<(Level, Game, usize)> {
    let level = common::load("testdrive1a")?;
    let mut g = Game::new(&level, true);
    g.view_dist = f32::MAX;
    common::settle_past_cutscenes(&mut g, &level, 120.0);
    let vi = g.vehicles.iter().position(|v| v.way == 2)?;
    Some((level, g, vi))
}

/// Stands the player beside the car, `side` car-space units to its left (negative: right),
/// and presses action.
fn try_side(level: &Level, g: &mut Game, vi: usize, side: f32) {
    let pi = g.player.unwrap();
    let at = g.vehicles[vi].to_world(side, 20.0);
    let y = level.ground.surface(&level.map, at.x, at.y, g.vehicles[vi].pos.y + 64.0);
    g.people[pi].fig.pos = Vec3::new(at.x, y, at.y);
    g.people[pi].state = State::Move;
    g.update(DT, &Controls { action: true, ..Default::default() }, level);
}

fn gets_in(level: &Level, g: &mut Game, vi: usize, side: f32, expect: usize) {
    try_side(level, g, vi, side);
    let pi = g.player.unwrap();
    assert!(matches!(g.people[pi].state, State::Enter { vehicle } if vehicle == vi), "not getting in: {:?}", g.people[pi].state);
    assert_eq!(g.people[pi].fig.anim(), expect);
    assert!(g.player_vehicle().is_none(), "in before the animation's played");
    // Facing the right way, the animation carries her hips in towards the car.
    let car = g.vehicles[vi].pos;
    let flat = |v: Vec3| glam::vec2(v.x - car.x, v.z - car.z).length();
    let start = flat(g.people[pi].fig.hips());
    let mut closest = start;
    let mut frames = 0;
    while matches!(g.people[pi].state, State::Enter { .. }) {
        closest = closest.min(flat(g.people[pi].fig.hips()));
        g.update(DT, &Controls::default(), level);
        frames += 1;
        assert!(frames < 300, "never got in");
    }
    assert!(closest < start - 50.0, "her hips went from {start} to no nearer than {closest}");
    assert!(frames > 10, "in after {frames} frames");
    assert!(matches!(g.people[pi].state, State::InCar { vehicle } if vehicle == vi));
    assert_eq!(g.vehicles[vi].driver, Some(pi));
}

#[test]
fn in_at_the_driver_s_door() {
    let Some((level, mut g, vi)) = load() else { return };
    gets_in(&level, &mut g, vi, 200.0, anim::ENTER_CAR);
    // Her own squad car isn't stolen (#517).
    assert_eq!((g.vehicles[vi].kind, g.record.cars_stolen), (3, 0));
}

/// Any other car is stolen, counted once however often she gets in (#517).
#[test]
fn another_car_is_stolen_once() {
    let Some((level, mut g, vi)) = load() else { return };
    // The mission has no other car to get into: the squad car made an ordinary one.
    g.vehicles[vi].kind = 1;
    gets_in(&level, &mut g, vi, 200.0, anim::ENTER_CAR);
    assert_eq!(g.record.cars_stolen, 1);
    let pi = g.player.unwrap();
    g.vehicles[vi].driver = None;
    g.people[pi].state = State::Move;
    gets_in(&level, &mut g, vi, 200.0, anim::ENTER_CAR);
    assert_eq!(g.record.cars_stolen, 1);
}

#[test]
fn in_at_the_passenger_s_door() {
    let Some((level, mut g, vi)) = load() else { return };
    gets_in(&level, &mut g, vi, -200.0, anim::ENTER_TAXI);
}
