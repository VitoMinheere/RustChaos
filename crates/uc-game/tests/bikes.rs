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

