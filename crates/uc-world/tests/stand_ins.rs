//! What stands in for the textures an install doesn't have: subdock1's world 6 (walls and
//! pages from the downtown world, its harbour the island maps' sea), and the roof faces
//! textured with the square under them, as the original does for all but warehouses.
//! Skipped without an install.

use uc_world::level::WATER_PAGE;
use uc_world::{Layer, Level, LoadOptions, UNTEXTURED};

fn load(mission: &str) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap())
}

/// Triangles of `layer` drawn plain, with no page.
fn untextured(geo: &uc_world::WorldGeometry, layer: Layer) -> usize {
    geo.chunks.iter().filter_map(|c| c.batches.get(&(layer, UNTEXTURED))).map(|b| b.indices.len() / 3).sum()
}

#[test]
fn subdock1_has_walls_roofs_and_a_harbour() {
    let Some(level) = load("subdock1") else { return };
    assert_eq!(level.map.texture_set, Some(6));
    let geo = level.build();
    assert!(geo.count(Layer::Walls) > 10_000, "{} wall triangles", geo.count(Layer::Walls));
    assert_eq!(untextured(&geo, Layer::Roofs), 0, "dark roofs");
    let missing: Vec<u32> = geo.pages().into_iter().filter(|&p| level.texture(p).is_none()).collect();
    assert!(missing.len() * 20 < geo.pages().len(), "too many pages without a texture: {missing:?}");
    // The harbour moves with the waves.
    let sea = level.sea();
    assert!(sea.len() > 5000, "{} sea squares", sea.len());
    assert!(level.texture(WATER_PAGE as u32).is_some(), "no water texture");
}

#[test]
fn only_warehouse_roofs_are_dark() {
    // Arms Breaker's café is no warehouse: its roof has the page under it, which its clump
    // has. RTA's warehouses stay dark.
    let Some(level) = load("wstores1") else { return };
    assert_eq!(untextured(&level.build(), Layer::Roofs), 0);
    let Some(level) = load("police1") else { return };
    assert!(untextured(&level.build(), Layer::Roofs) > 0, "RTA's warehouse roofs aren't dark");
}
