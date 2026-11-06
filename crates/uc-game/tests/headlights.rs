//! Headlights at night (vehicle.cpp:1236): a car being driven glows at its headlights, the
//! more the more they point at the camera; not by day, and not parked. Skipped without an
//! install.

use glam::Vec3;
use uc_game::{Controls, Game};
use uc_world::{Level, GLOW};

use crate::common;

/// The glows' vertices with the camera `ahead` of car 0, and whether it's being driven.
fn glows(level: &Level, g: &mut Game, driven: bool, ahead: f32) -> usize {
    g.vehicles[0].driver = driven.then_some(g.player.unwrap());
    let v = &g.vehicles[0];
    let (_, dir) = v.headlights(0.0);
    let eye = v.pos + dir * ahead + Vec3::Y * 100.0;
    g.batches(level, eye).get(&GLOW).map_or(0, |b| b.vertices.len())
}

fn game(mission: &str) -> Option<(Level, Game)> {
    let level = common::load_lit(mission)?;
    let mut g = Game::new(&level, true);
    for _ in 0..10 {
        g.skip_cutscene();
        g.update(1.0 / 30.0, &Controls::default(), &level);
    }
    (!g.vehicles.is_empty()).then_some((level, g))
}

#[test]
fn a_car_driven_at_night_glows_at_its_headlights() {
    // Auto-Destruct is at night.
    let Some((level, mut g)) = game("carbomb1") else { return };
    assert!(g.night);
    let parked = glows(&level, &mut g, false, 2000.0);
    let facing = glows(&level, &mut g, true, 2000.0);
    let behind = glows(&level, &mut g, true, -2000.0);
    assert_eq!(facing, parked + 8, "two glows in front of it");
    assert_eq!(behind, parked, "none from behind");
}

#[test]
fn not_by_day() {
    let Some((level, mut g)) = game("police1") else { return };
    assert!(!g.night);
    assert_eq!(glows(&level, &mut g, true, 2000.0), glows(&level, &mut g, false, 2000.0));
}
