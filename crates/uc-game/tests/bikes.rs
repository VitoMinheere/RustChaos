//! Motorbikes (`VT_BIKE`, fallen/Source/bike.cpp, compiled out of the shipped game): the
//! unreleased basic training's bike (`train1`, point 66) is a bike, not a car, standing on
//! its wheels and drawn. Skipped without an install.

use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

#[test]
fn basic_training_has_a_bike() {
    let Some(game) = uc_formats::default_install() else { return };
    let Ok(level) = Level::load(&game, "train1", &LoadOptions { fullbright: true, character: true, overrides: None }) else { return };
    let mut g = Game::new(&level, true);
    for _ in 0..90 {
        g.skip_cutscene();
        g.update(1.0 / 30.0, &Controls::default(), &level);
    }
    let (b, fig) = g.bikes.iter().find(|(b, _)| b.way == 66).expect("no bike from point 66");
    assert!(g.vehicles.iter().all(|v| v.way != 66), "point 66 made a car too");
    assert!(fig.is_some(), "no bike mesh");
    let at = b.pos();
    let floor = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
    println!("bike at {at:?}, floor {floor}, yaw {}", b.yaw);
    assert!(at.y > floor && at.y < floor + 64.0, "the bike is at {} over ground at {floor}", at.y);
    let (back, front) = b.wheels();
    assert!(((front - back).length() - 160.0).abs() < 16.0);
}


/// Getting on it with action, riding it with the throttle, and off again once it's stopped.
#[test]
fn on_off_and_away() {
    use uc_game::{Drive, State};
    let Some(game) = uc_formats::default_install() else { return };
    let Ok(level) = Level::load(&game, "train1", &LoadOptions { fullbright: true, character: true, overrides: None }) else { return };
    let mut g = Game::new(&level, true);
    let dt = 1.0 / 30.0;
    for _ in 0..(60.0 / dt) as usize {
        g.skip_cutscene();
        g.update(dt, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let k = g.bikes.iter().position(|(b, _)| b.way == 66).expect("no bike");
    let p = g.player.unwrap();
    let at = g.bikes[k].0.pos();
    let y = level.ground.surface(&level.map, at.x + 100.0, at.z, at.y + 64.0);
    g.people[p].fig.pos = glam::vec3(at.x + 100.0, y, at.z);
    g.people[p].state = State::Move;
    g.update(dt, &Controls { action: true, ..Default::default() }, &level);
    assert!(matches!(g.people[p].state, State::Bike { phase: 0, .. }), "didn't get on: {:?}", g.people[p].state);
    for _ in 0..90 {
        g.update(dt, &Controls::default(), &level);
    }
    assert!(matches!(g.people[p].state, State::Bike { phase: 1, .. }), "not riding: {:?}", g.people[p].state);
    assert!(g.riding().is_some());
    let from = g.bikes[k].0.pos();
    let go = Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() };
    for _ in 0..90 {
        g.update(dt, &go, &level);
    }
    let gone = (g.bikes[k].0.pos() - from).length();
    assert!(gone > 256.0, "went {gone}");
    // On the seat, facing the way it goes: her hips over its middle, her legs out ahead of
    // them along it.
    let (to, ahead) = (g.bikes[k].0.pos(), (g.bikes[k].0.pos() - from).normalize());
    let hips = g.people[p].fig.part_world(0).unwrap().1;
    assert!(glam::vec2(hips.x - to.x, hips.z - to.z).length() < 24.0, "hips at {hips:?}, the bike at {to:?}");
    let feet = g.people[p].fig.part_world(3).unwrap().1;
    let legs = glam::vec2(feet.x - hips.x, feet.z - hips.z).normalize();
    assert!(legs.dot(glam::vec2(ahead.x, ahead.z).normalize()) > 0.8, "legs {legs:?}, going {ahead:?}");
    // Let it stop, then off.
    for _ in 0..150 {
        g.update(dt, &Controls::default(), &level);
    }
    g.update(dt, &Controls { action: true, ..Default::default() }, &level);
    assert!(matches!(g.people[p].state, State::Bike { phase: 2, .. }), "didn't get off: {:?}", g.people[p].state);
    for _ in 0..150 {
        g.update(dt, &Controls::default(), &level);
    }
    assert!(matches!(g.people[p].state, State::Move));
    assert!(g.riding().is_none());
    assert_eq!(g.bikes[k].0.mode, uc_game::bike::Mode::Parked);
}
