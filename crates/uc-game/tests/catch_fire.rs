//! Catching fire (#297, collide.cpp:6237): standing in a fire burns, 30 health a tick, until
//! it kills; a civilian near one runs from it. RTA's crashed taxi is burning. Skipped
//! without an install.

use glam::vec3;
use uc_game::person::AI_CIV;
use uc_game::Controls;

mod common;

#[test]
fn standing_in_a_fire_burns_to_death() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let (fire, _) = r.g.pyro.fires().next().expect("RTA's taxi burns");
    let me = r.player();
    r.heal = false;
    let before = r.g.people[me].health;
    let mut lost = 0;
    for _ in 0..(3.0 / common::DT) as usize {
        let y = r.level.ground.surface(&r.level.map, fire.x, fire.z, fire.y + 64.0);
        r.g.people[me].fig.pos = vec3(fire.x + 30.0, y, fire.z);
        r.play(common::DT, Controls::default());
        lost = before - r.g.people[me].health;
        if r.g.people[me].knocked_out {
            break;
        }
    }
    assert!(lost > 60, "only {lost} lost in the fire");
    assert!(r.g.people[me].knocked_out, "still alive in the fire on {}", r.g.people[me].health);
}

#[test]
fn a_civilian_runs_from_a_fire() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let (fire, _) = r.g.pyro.fires().next().expect("RTA's taxi burns");
    let i = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("crowd");
    let at = fire + vec3(150.0, 0.0, 0.0);
    let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    r.g.people[i].fig.pos = vec3(at.x, y, at.z);
    r.play(0.2, Controls::default());
    assert!(r.g.people[i].scared.is_some(), "a civilian stood by the fire");
}

/// Once alight, stepping out of the fire doesn't put it out (`PYRO_IMMOLATE` on her): she
/// screams as she catches, and burns on away from it until it kills.
#[test]
fn stepping_out_of_a_fire_still_burns() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let (fire, _) = r.g.pyro.fires().next().expect("RTA's taxi burns");
    let me = r.player();
    r.heal = false;
    r.g.take_sounds();
    for _ in 0..(0.2 / common::DT) as usize {
        let y = r.level.ground.surface(&r.level.map, fire.x, fire.z, fire.y + 64.0);
        r.g.people[me].fig.pos = vec3(fire.x + 30.0, y, fire.z);
        r.play(common::DT, Controls::default());
    }
    assert!(r.g.people[me].alight.is_some(), "not alight in the fire");
    let cues: Vec<uc_game::Sfx> = r.g.take_sounds().into_iter().map(|(s, _)| s).collect();
    let darci = uc_world::character::person::DARCI;
    assert_eq!(cues.iter().filter(|c| **c == uc_game::Sfx::OnFire { kind: darci, female: true }).count(), 1, "{cues:?}");
    r.g.people[me].health = r.g.people[me].max_health;
    let away = r.ground_at(fire + vec3(1500.0, 0.0, 0.0));
    r.g.people[me].fig.pos = away;
    let before = r.g.people[me].health;
    r.play(0.5, Controls::default());
    assert!(r.g.people[me].health < before, "the flames went out away from the fire");
    assert!(r.until(5.0, |r| r.g.people[r.player()].knocked_out), "burned on at {}", r.g.people[me].health);
    assert!(r.g.people[me].alight.is_some(), "no flames on the body");
}

/// Water puts the flames out: someone alight on El Cossa Island's sea stops burning.
#[test]
fn water_puts_the_flames_out() {
    let Some(level) = common::load("jung3") else { return };
    let mut r = common::Run::new(level);
    let water = (0..128 * 128).map(|i| (i % 128, i / 128)).find(|&(x, z)| {
        r.level.map.square(x, z).page() == uc_world::level::WATER_PAGE
    });
    let Some((x, z)) = water else { panic!("jung3 has no water") };
    let me = r.player();
    r.heal = false;
    r.g.people[me].alight = Some(0.0);
    r.g.people[me].fig.pos = r.ground_at(vec3(x as f32 * 256.0 + 128.0, 0.0, z as f32 * 256.0 + 128.0));
    r.play(0.2, Controls::default());
    assert!(r.g.people[me].alight.is_none(), "still alight in the water");
    assert!(!r.g.people[me].knocked_out);
}
