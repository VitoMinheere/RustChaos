//! Dynamic lights (#240): cars light the road ahead at night and not by day, a gun lights its
//! shooter for the moment it goes off, and a fireball lights its surroundings for about a
//! second. Skipped without an install.

use uc_game::{Controls, Game, Item};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// The level with its lighting, which says whether it's night.
fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: false, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..10 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    Some((level, g))
}

#[test]
fn cars_light_the_road_at_night_only() {
    let Some((_, night)) = load("MIB") else { return };
    assert!(night.night);
    let cars = night.vehicles.iter().filter(|v| !v.wrecked()).count();
    let lit = night.lights().iter().filter(|l| l.radius == 800.0).count();
    assert!(cars > 0 && lit >= cars, "{lit} car lights for {cars} cars");
    let Some((_, day)) = load("police1") else { return };
    assert!(!day.night);
    // RTA's wrecked car burns, by day or night; the cars' lights are only on at night.
    assert!(!day.lights().iter().any(|l| l.radius == 800.0), "car lights by day: {:?}", day.lights());
}

#[test]
fn a_shot_lights_up_for_a_moment() {
    let Some((level, mut g)) = load("fight2") else { return };
    let pi = g.player.unwrap();
    let pos = g.people[pi].fig.pos;
    g.items.push(Item { way: 0, kind: 2, pos, rounds: None });
    for _ in 0..5 {
        g.update(DT, &Controls::default(), &level);
    }
    let mut lit = false;
    for _ in 0..20 {
        g.update(DT, &Controls { punch: true, ..Default::default() }, &level);
        if g.lights().iter().any(|l| l.radius == 400.0 && (l.pos - g.people[pi].fig.pos).length() < 200.0) {
            lit = true;
            break;
        }
    }
    assert!(lit, "the shot never lit up");
    for _ in 0..10 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(!g.lights().iter().any(|l| l.radius == 400.0), "the muzzle's light should be gone");
}

#[test]
fn a_fireball_lights_up_and_fades() {
    let Some((level, mut g)) = load("police1") else { return };
    let at = g.people[g.player.unwrap()].fig.pos;
    g.pyro.burst(at, uc_game::pyro::FIREBALL, 400.0);
    let first = g.lights();
    let l = first.iter().find(|l| l.pos == at).expect("the fireball's light");
    assert_eq!(l.radius, 800.0);
    let bright = l.rgb[0];
    for _ in 0..10 {
        g.update(DT, &Controls::default(), &level);
    }
    let later = g.lights().into_iter().find(|l| l.pos == at).map_or(0.0, |l| l.rgb[0]);
    assert!(later < bright, "it should cool: {later} against {bright}");
    for _ in 0..45 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(!g.lights().iter().any(|l| l.pos == at), "it should be out within a second and a half");
}
