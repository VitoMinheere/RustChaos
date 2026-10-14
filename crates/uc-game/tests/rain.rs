//! Rain (`GF_RAINING`): every night mission rains, out of doors, a fresh scatter of drops in
//! front of the camera each frame; no day mission does. Skipped without an install.

use glam::vec3;
use std::collections::BTreeMap;
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: false, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    Some((level, g))
}

/// The drops drawn this frame, with Darci's camera a little behind and above her.
fn drops(g: &Game, level: &Level) -> Vec<glam::Vec3> {
    let me = g.player().unwrap().fig.pos;
    let eye = me + vec3(0.0, 300.0, -600.0);
    let mut out = BTreeMap::new();
    g.rain(level, eye, me - eye, &mut out);
    out.get(&uc_world::SHADOW).map_or(Vec::new(), |b| b.vertices.iter().map(|v| glam::Vec3::from(v.pos) * uc_world::UNIT).collect())
}

#[test]
fn it_rains_at_night() {
    // `police2` is at night; RTA (`police1`) by day.
    let Some((level, mut g)) = load("police2") else { return };
    assert!(Game::raining(&level));
    let first = drops(&g, &level);
    assert!(first.len() >= Game::RAINDROPS * 3 / 2, "only {} vertices", first.len());
    // In front of the camera and within eight squares or so of it.
    let me = g.player().unwrap().fig.pos;
    let eye = me + vec3(0.0, 300.0, -600.0);
    for v in &first {
        assert!((*v - eye).length() < 256.0 * 12.0, "a drop {} away", (*v - eye).length());
    }
    // Somewhere else the next frame.
    g.update(1.0 / 30.0, &Controls::default(), &level);
    assert_ne!(drops(&g, &level), first);
}

#[test]
fn not_by_day() {
    let Some((level, g)) = load("police1") else { return };
    assert!(!Game::raining(&level));
    assert!(drops(&g, &level).is_empty());
}
