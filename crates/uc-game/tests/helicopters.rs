//! Helicopters (`VT_HELICOPTER`): the three missions that make one get a helicopter, not a
//! squad car, and Psycho Park's takes off after the player and keeps over her with its
//! searchlight on. Skipped without an install.

use uc_game::chopper::{State, Target};
use uc_game::{Controls, Game};
use uc_world::Level;

mod common;
use common::DT;

fn run(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / DT) as usize {
        g.skip_cutscene();
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &Controls::default(), level);
    }
}

/// Each mission's helicopter point makes a helicopter and no car.
#[test]
fn helicopters_not_squad_cars() {
    for (mission, way) in [("park2", 151), ("e3", 306)] {
        let Some((level, mut g)) = common::begin(mission) else { return };
        run(&mut g, &level, 5.0);
        assert!(g.choppers.iter().any(|c| c.way == way), "{mission}: no helicopter from point {way}");
        assert!(g.vehicles.iter().all(|v| v.way != way), "{mission}: point {way} made a car too");
    }
}

/// Psycho Park's helicopter tracks the player (point 1): up, over her, light on.
#[test]
fn psycho_park_s_helicopter_watches_darci() {
    let Some((level, mut g)) = common::begin("park2") else { return };
    run(&mut g, &level, 30.0);
    let c = g.choppers.iter().find(|c| c.way == 151).expect("no helicopter");
    let p = g.player.unwrap();
    assert_eq!(c.target, Some(Target::Person(p)));
    assert_eq!(c.state, State::Tracking);
    let me = g.people[p].fig.pos;
    assert!(c.pos().y > me.y + 400.0, "only {} above her", c.pos().y - me.y);
    let flat = (glam::vec2(c.pos().x, c.pos().z) - glam::vec2(me.x, me.z)).length();
    assert!(flat < 16.0 * 256.0, "{} squares off", flat / 256.0);
    assert!(c.light > 200);
    assert!(g.chopper_noise(me).is_some());
}
