//! Roper has a body of his own. Skipped without an install.

use std::sync::Arc;
use uc_world::character::person;
use uc_world::{Level, LoadOptions};

fn load(mission: &str) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap())
}

/// He comes out of `roper.all`, not out of the thug file he used to borrow from.
#[test]
fn roper_has_his_own_body() {
    let Some(level) = load("Stealtst1") else { return };
    let people = level.people.as_ref().expect("the character files");
    let roper = people.roper.as_ref().expect("roper.all should be in the install");

    let model = people.model(person::ROPER, 0);
    assert!(Arc::ptr_eq(&model.chunk, roper), "Roper should be modelled from his own file");
    assert!(!Arc::ptr_eq(&model.chunk, &people.thug), "and not from the thugs'");
    assert_eq!(model.body, 0, "his file holds the one body");

    // Everyone else is where they were.
    assert!(Arc::ptr_eq(&people.model(person::THUG_RASTA, 0).chunk, &people.thug));
    assert!(Arc::ptr_eq(&people.model(person::DARCI, 0).chunk, &people.darci));
}

/// His skeleton takes Darci's animations, which is how the original drives him: every
/// character's `global_anim_array` entry is filled from `game_chunk[ANIM_TYPE_DARCI]`, whatever
/// chunk their body came from.
#[test]
fn roper_is_posed_by_darcis_animations() {
    let Some(level) = load("Stealtst1") else { return };
    let people = level.people.as_ref().unwrap();
    let roper = people.roper.as_ref().unwrap();
    assert_eq!(
        roper.element_count, people.darci.element_count,
        "his skeleton has to take her keyframes"
    );
    let fig = people.figure(person::ROPER, 0, glam::Vec3::ZERO, 0.0);
    assert!(fig.is_some(), "he should pose");
}

/// And he turns up in the missions that ask for him, rather than a thug standing in.
#[test]
fn the_missions_that_want_roper_get_him() {
    let Some(level) = load("Stealtst1") else { return };
    let people = level.people.as_ref().unwrap();
    // `ET_ROPER` is 23, which `enemy_person` maps to `PERSON_ROPER`.
    let wants_roper = level
        .mission
        .event_points
        .iter()
        .any(|e| e.kind == uc_formats::ucm::waypoint::CREATE_ENEMIES && (e.data[0] & 0xFFFF) == 23);
    assert!(wants_roper, "Seek, Sneak and Seize has a Roper in it");
    assert!(people.roper.is_some(), "so he has to have a body to turn up in");
}
