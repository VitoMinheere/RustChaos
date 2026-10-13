//! Warehouse ambience (`warefx`): the police missions give the station its own, Gatecrasher
//! two more, and inside one of them the city goes quiet and its ambience plays instead.
//! Skipped without an install.

use glam::vec3;
use uc_formats::ucm::waypoint;
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..10 {
        g.update(1.0 / 30.0, &Controls::default(), &level);
    }
    Some((level, g))
}

/// Every `warefx` point in the six missions that have them is inside a warehouse, and gives
/// it the ambience it asks for.
#[test]
fn every_warefx_point_is_inside_a_warehouse() {
    for (mission, want) in [("police1", 1), ("police2", 1), ("police3", 1), ("police4", 1), ("westcrime1", 1), ("bball2", 0)] {
        let Some((level, g)) = load(mission) else { return };
        assert!(g.unsupported().is_empty(), "{mission}: unsupported {:?}", g.unsupported());
        for e in level.mission.event_points.iter().filter(|e| e.used != 0 && e.kind == waypoint::WAREFX) {
            let w = g.wares.which_contains(&level.map, e.x >> 8, e.z >> 8);
            let w = w.unwrap_or_else(|| panic!("{mission}: point {} isn't in a warehouse", e.index));
            assert_eq!(g.wares.wares[w].ambience, e.data[0] as u8, "{mission}: point {}", e.index);
            if want != 0 {
                assert_eq!(e.data[0], want, "{mission}: the station's ambience is the police one");
            }
        }
    }
}

/// Inside the station Darci hears it; up on its roof or out in the street she doesn't. RTA
/// starts her in there.
#[test]
fn indoors_is_under_the_roof() {
    let Some((level, mut g)) = load("police1") else { return };
    let e = level.mission.event_points.iter().find(|e| e.index == 269).unwrap();
    let (x, z) = (e.x as f32, e.z as f32);
    let me = g.player.unwrap();
    let step = |g: &mut Game| g.update(1.0 / 30.0, &Controls::default(), &level);
    assert_eq!(g.indoors(), Some(1), "RTA starts in the station");

    let floor = level.map.ground_height(x, z);
    let roof = level.ground.surface(&level.map, x, z, 1.0e5);
    assert!(roof - floor > 128.0, "the station should have a roof over its floor ({floor} .. {roof})");
    g.people[me].fig.pos = vec3(x, floor, z);
    step(&mut g);
    assert_eq!(g.indoors(), Some(1), "inside the station");

    g.people[me].fig.pos = vec3(x, roof, z);
    step(&mut g);
    assert_eq!(g.indoors(), None, "on its roof");

    // RTA starts Darci inside the station, so for the street: the first square away from it
    // that no building stands on.
    let street = (1..40).map(|k| vec3(x + k as f32 * 256.0, 0.0, z))
        .find(|p| !level.map.square((p.x as usize) >> 8, (p.z as usize) >> 8).has(uc_formats::iam::flags::HIDDEN))
        .expect("a street square");
    g.people[me].fig.pos = vec3(street.x, level.map.ground_height(street.x, street.z), street.z);
    step(&mut g);
    assert_eq!(g.indoors(), None, "out in the street");
}

/// The four loops are where the release keeps them (not under the sound table's names).
#[test]
fn the_ambience_recordings_are_there() {
    let Some(game) = uc_formats::default_install() else { return };
    for name in uc_game::ware::AMBIENCE {
        let rel = format!(r"data\sfx\1622\{}", name.replace('/', "\\"));
        assert!(uc_formats::resolve(&game, &rel).is_some(), "{rel} is missing");
    }
}
