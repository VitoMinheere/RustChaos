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
