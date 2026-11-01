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

/// An override folder is found whatever its case: they're named as the clumps are
/// (`awol1`), not always as the missions (`AWOL1`).
#[test]
fn override_folders_match_whatever_their_case() {
    let dir = std::env::temp_dir().join(format!("uc-overrides-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("awol1")).unwrap();
    std::fs::write(dir.join("awol1").join("page_0007.png"), b"").unwrap();
    let found = uc_world::level::override_path(&dir, "AWOL1", None, 7);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(found, Some(dir.join("awol1").join("page_0007.png")));
}

/// A mission with no clump of its own knows which mission each page it borrowed came from,
/// so that mission's upscaled page is used for it too: every one a clump that has the page.
#[test]
fn borrowed_pages_know_where_they_came_from() {
    let Some(game) = uc_formats::default_install() else { return };
    for mission in ["army1", "e7", "Rooftop1", "upperclass1"] {
        let Some(level) = load(mission) else { return };
        assert!(uc_formats::resolve(&game, &format!("clumps\\{mission}.txc")).is_none(), "{mission} has a clump");
        let pages = level.build().pages();
        for p in pages.into_iter().filter(|&p| level.clump.texture(p).ok().flatten().is_some()) {
            let donor = level.page_donors.get(&p).unwrap_or_else(|| panic!("{mission}: page {p} from nowhere"));
            let clump = uc_formats::resolve(&game, &format!("clumps\\{donor}.txc")).unwrap();
            let clump = uc_formats::txc::Clump::parse(std::fs::read(clump).unwrap()).unwrap();
            assert!(clump.texture(p).ok().flatten().is_some(), "{mission}: page {p} isn't in {donor}'s clump");
        }
    }
}
