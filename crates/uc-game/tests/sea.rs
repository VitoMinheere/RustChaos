//! The sea (`MAV_SPARE_FLAG_WATER` points, aeng.cpp:7628): El Cossa Island's water is floor
//! squares of page 454 whose points rise and fall with a rolling wave, up to 16 units. The
//! level leaves them out of its static floor for the game to draw. Skipped without an install.

use std::collections::BTreeMap;
use uc_game::{Controls, Game};
use uc_world::level::WATER_PAGE;
use uc_world::{Level, LoadOptions};

#[test]
fn el_cossa_island_s_sea_moves() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "jung3", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let sea = level.sea();
    assert!(sea.len() > 1000, "only {} sea squares", sea.len());
    assert!(sea.iter().any(|s| s.page == WATER_PAGE as u32));
    // None of them in the static floor as well.
    let geo = level.build();
    assert!(!geo.pages().contains(&(WATER_PAGE as u32)), "water squares drawn twice");

    let mut g = Game::new(&level, true);
    let heights = |g: &Game| {
        let mut out = BTreeMap::new();
        g.draw_sea(&mut out);
        out.values().flat_map(|b| b.vertices.iter().map(|v| v.pos[1] * uc_world::UNIT)).collect::<Vec<f32>>()
    };
    let before = heights(&g);
    for _ in 0..30 {
        g.update(1.0 / 30.0, &Controls::default(), &level);
    }
    let after = heights(&g);
    assert_eq!(before.len(), after.len());
    let moved = before.iter().zip(&after).filter(|(a, b)| (*a - *b).abs() > 0.5).count();
    assert!(moved > before.len() / 4, "only {moved} of {} points moved", before.len());
    let biggest = before.iter().zip(&after).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
    assert!(biggest <= 32.5, "a wave of {biggest}");
}

/// A city with no sea has nothing left out of its floor.
#[test]
fn no_sea_in_the_city() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    assert!(level.sea().is_empty());
}
