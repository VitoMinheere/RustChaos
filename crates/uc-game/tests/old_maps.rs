//! The three maps in the older layout (`jumper1`, `submiss1a`, `supply1`): their walls and
//! roofs are read, and Takeover1, the one mission on one of them, has floors under the
//! people, switches and items it puts up on its buildings. Skipped without an install.

use uc_world::{Level, LoadOptions};

#[test]
fn old_maps_have_walls_and_roofs() {
    let Some(game) = uc_formats::default_install() else { return };
    for name in ["jumper1", "submiss1a", "supply1"] {
        let map = uc_formats::iam::Map::parse(&std::fs::read(game.join(format!("data/{name}.iam"))).unwrap()).unwrap();
        assert_eq!(map.walkables_layout, uc_formats::iam::WalkablesLayout::Old, "{name}");
        let w = map.walkables.as_ref().expect("walkables");
        assert!(w.faces.len() > 1000, "{name}: {} walkable faces", w.faces.len());
        // Walls along whole squares, inside the map.
        let walls: Vec<_> = map.facets.iter().filter(|f| f.kind == uc_formats::iam::facet_type::NORMAL).collect();
        assert!(walls.len() > 400, "{name}: {} walls", walls.len());
        assert!(walls.iter().all(|f| f.x.iter().chain(&f.z).all(|&v| v < 128) && f.block_height > 0), "{name}");
    }
}

#[test]
fn takeover1_stands_on_its_buildings() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "Takeover1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    // The people, items and switches it puts up on its roofs (not the platforms' paths and
    // the cameras, which are in the air).
    use uc_formats::ucm::{trigger, waypoint};
    let raised: Vec<_> = level.mission.event_points.iter()
        .filter(|e| e.used != 0 && e.y > 100)
        .filter(|e| matches!(e.kind, waypoint::CREATE_PLAYER | waypoint::CREATE_ENEMIES | waypoint::CREATE_ITEM) || e.trigger == trigger::SWITCH)
        .collect();
    let on = |x: f32, y: f32, z: f32| (level.ground.surface(&level.map, x, z, y + 40.0) - y).abs() < 40.0;
    let off: Vec<usize> = raised.iter().filter(|e| !on(e.x as f32, e.y as f32, e.z as f32)).map(|e| e.index).collect();
    assert!(raised.len() > 30, "{} raised", raised.len());
    assert!(off.is_empty(), "points {off:?} of {} have nothing under them", raised.len());
}

