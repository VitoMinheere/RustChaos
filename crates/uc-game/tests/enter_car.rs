//! Getting into a car (`set_person_enter_vehicle`, fallen/Source/Person.cpp:8254): lined up
//! at the nearer door, `ANIM_ENTER_CAR` at the driver's and `ANIM_ENTER_TAXI` at the
//! passenger's, and in once it's played out. Driving Bronze's unlocked squad car (point 2).
//! Skipped without an install.

use glam::Vec3;
use uc_game::{Controls, Game, State};
use uc_world::character::anim;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load() -> Option<(Level, Game, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "testdrive1a", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    g.view_dist = f32::MAX;
    for _ in 0..(120.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() && g.cutscene.is_none() {
            break;
        }
    }
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
}

#[test]
fn in_at_the_passenger_s_door() {
    let Some((level, mut g, vi)) = load() else { return };
    gets_in(&level, &mut g, vi, -200.0, anim::ENTER_TAXI);
}
