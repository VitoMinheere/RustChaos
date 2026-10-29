//! A damaged car smokes from under its bonnet (`draw_car`, vehicle.cpp:1309): from under 128 of
//! its 300 health, more the worse it gets; a whole car doesn't. Skipped without an install.

use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Smoke puffs by a parked car of `health` over three seconds.
fn smoke(health: i32) -> Option<usize> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "testdrive1a", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let vi = (0..g.vehicles.len()).find(|&v| g.vehicles[v].driver.is_none()).expect("a parked car");
    g.vehicles[vi].health = health;
    let at = g.vehicles[vi].pos;
    let mut puffs = 0;
    for _ in 0..(3.0 / DT) as usize {
        let before = g.pyro.puffs.len();
        g.update(DT, &Controls::default(), &level);
        let new = g.pyro.puffs.len().saturating_sub(before);
        puffs += g.pyro.puffs.iter().rev().take(new).filter(|p| (p.pos - at).length() < 400.0).count();
    }
    Some(puffs)
}

#[test]
fn damaged_cars_smoke() {
    let Some(whole) = smoke(300) else { return };
    let (hurt, bad) = (smoke(100).unwrap(), smoke(10).unwrap());
    println!("smoke: whole {whole}, at 100 {hurt}, at 10 {bad}");
    assert_eq!(whole, 0, "a whole car smoked");
    assert!(hurt > 0 && bad > hurt, "worse damage, more smoke: {hurt} then {bad}");
}
