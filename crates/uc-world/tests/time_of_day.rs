//! Time of day (#515) on the install's maps.

use uc_formats::lgt::FLAG_DAYTIME;
use uc_world::{Level, LoadOptions, TimeOfDay};

fn load(mission: &str, fullbright: bool) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, &LoadOptions { fullbright, character: false, overrides: None }).unwrap())
}

/// Gangland's variants are the files the game shipped, the dawn hazed grey as the mission
/// that uses it is; back to its own, the mission's lighting again.
#[test]
fn gangland_uses_its_shipped_variants() {
    let Some(mut level) = load("gang1", false) else { return };
    let own = (level.lighting.as_ref().unwrap().ambient, level.sky);
    assert_eq!(own.0, [35, 35, 17]);
    level.set_time(Some(TimeOfDay::Night));
    assert_eq!(level.lighting.as_ref().unwrap().ambient, [15, 23, 33]);
    assert_eq!(level.sky, [0, 0, 26]);
    level.set_time(Some(TimeOfDay::Dawn));
    assert_eq!(level.lighting.as_ref().unwrap().ambient, [78, 69, 55]);
    assert_eq!(level.sky, [196, 196, 204], "the haze rule");
    // Its day is derived from the dawn: daylight, brighter, the day's sky.
    level.set_time(Some(TimeOfDay::Day));
    let l = level.lighting.as_ref().unwrap();
    assert_ne!(l.flags & FLAG_DAYTIME, 0);
    assert!(l.ambient.iter().sum::<i32>() > 78 + 69 + 55 - 10, "{:?}", l.ambient);
    assert_eq!(level.sky, [210, 200, 240]);
    level.set_time(None);
    assert_eq!((level.lighting.as_ref().unwrap().ambient, level.sky), own);
    assert_eq!(level.time, None);
}

/// A day map with no variants gets its night derived: dark, not daylight (so it rains, the
/// moon's up and the cars have their lights on), the lampposts lit.
#[test]
fn a_day_map_gets_a_derived_night() {
    let Some(mut level) = load("police1", false) else { return };
    assert_ne!(level.lighting.as_ref().unwrap().flags & FLAG_DAYTIME, 0, "police1 is a day mission");
    let lights = level.lighting.as_ref().unwrap().lights.len();
    level.set_time(Some(TimeOfDay::Night));
    let l = level.lighting.as_ref().unwrap();
    assert_eq!(l.flags & FLAG_DAYTIME, 0);
    assert!(l.ambient.iter().all(|&c| c < 40), "{:?}", l.ambient);
    assert_eq!(l.lights.len(), lights);
    assert_eq!(level.sky, [0, 0, 26]);
    assert_eq!(level.time, Some(TimeOfDay::Night));
    assert!(level.lighter().unwrap().sources().len() > lights, "the lampposts lit");
}

/// Full bright stays full bright.
#[test]
fn full_bright_has_no_time() {
    let Some(mut level) = load("police1", true) else { return };
    let sky = level.sky;
    level.set_time(Some(TimeOfDay::Night));
    assert!(level.lighting.is_none());
    assert_eq!(level.sky, sky);
}
