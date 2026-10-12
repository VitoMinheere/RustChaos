//! Builds real levels from an install (skipped when none is found; see uc-formats tests).

use uc_world::character::{anim, person};
use uc_world::{Layer, Level, LoadOptions, Play};

#[test]
fn police1_builds_with_people() {
    let Some(game) = uc_formats::default_install() else {
        eprintln!("no Urban Chaos install found (set UC_GAME_DIR); skipping");
        return;
    };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: false, character: true, overrides: None }).unwrap();
    let geo = level.build();
    for layer in [Layer::Floor, Layer::Roofs, Layer::Walls, Layer::Props, Layer::Detail] {
        assert!(geo.count(layer) > 1000, "{layer:?} has only {} triangles", geo.count(layer));
    }
    assert_eq!(geo.chunks.len(), 256);
    let missing: Vec<u32> = geo.pages().into_iter().filter(|&p| level.texture(p).is_none()).collect();
    assert!(missing.len() * 20 < geo.pages().len(), "too many pages without a texture: {missing:?}");

    // Every person type can be posed and drawn with Darci's animations.
    let people = level.people.as_ref().unwrap();
    for t in 0..=person::MIB3 {
        let mut f = people.figure(t, 0, level.spawn.0, level.spawn.1).unwrap();
        assert!(f.play(anim::RUN, Play::Loop));
        f.advance(0.5);
        let mut batches = Default::default();
        f.append_batches(&mut batches, level.lighter().as_ref());
        assert!(!batches.is_empty(), "person type {t} drew nothing");
    }
}

#[test]
fn one_shot_animations_finish() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "ftutor1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut f = level.people.as_ref().unwrap().figure(person::DARCI, 0, level.spawn.0, 0.0).unwrap();
    for a in [anim::PUNCH_COMBO1, anim::KICK_COMBO3, anim::KD_FRONT_MID, anim::PULL_UP, anim::ARREST_CROUTCH] {
        assert!(f.restart(a, Play::Once), "animation {a} missing");
        let mut t = 0.0;
        while !f.finished() && t < 10.0 {
            f.advance(1.0 / 30.0);
            t += 1.0 / 30.0;
        }
        assert!(f.finished(), "animation {a} never finished");
        assert!((f.progress() - 1.0).abs() < 1e-6);
    }
    // The tutorial's rooftop is reachable: three ladders, roofs at 640.
    assert_eq!(level.ground.ladders.len(), 3);
    assert_eq!(level.ground.surface(&level.map, 55.6 * 256.0, 55.0 * 256.0, 640.0), 640.0);
}
