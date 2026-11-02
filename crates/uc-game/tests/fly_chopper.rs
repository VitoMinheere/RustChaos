//! Flying the police chopper (#465): Explore and Beat Cop park one near the start; the
//! player gets in with action, takes off, flies it about and lands, and gets out on the
//! ground (or drops from low down; higher up the door stays shut). The missions' own
//! choppers stay the AI's. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::chopper::{State as Heli, Stick};
use uc_game::person::State;
use uc_game::{Controls, Game};

mod common;

fn explore(mission: &str) -> Option<common::Run> {
    let level = common::load(mission)?;
    let g = Game::new(&level, false);
    Some(common::Run { level, g, seen: Vec::new(), heal: true, stop_at_outcome: true, each_turn: None })
}

fn fly(stick: Stick) -> Controls {
    Controls { fly: stick, ..Default::default() }
}

/// Stands the player beside the parked chopper and presses action.
fn get_in(r: &mut common::Run) {
    let at = r.g.choppers.iter().find(|c| c.parked).expect("no chopper parked").pos();
    r.stand(at + vec3(150.0, 0.0, 0.0));
    r.play(common::DT, Controls { action: true, ..Default::default() });
}

#[test]
fn darci_flies_the_chopper_and_lands_it() {
    let Some(mut r) = explore("police1") else { return };
    let me = r.player();
    let start = r.g.people[me].fig.pos;
    let c = r.g.choppers.iter().find(|c| c.parked).expect("Explore parks a chopper");
    let off = (c.pos() - start).length();
    assert!((3.0 * 256.0..40.0 * 256.0).contains(&off), "parked {} squares off", off / 256.0);
    assert_eq!(c.state, Heli::Landed);

    get_in(&mut r);
    assert!(matches!(r.g.people[me].state, State::Flying { .. }), "{:?}", r.g.people[me].state);
    assert!(r.g.player_chopper().is_some());

    // Up: the rotors run up first.
    r.play(6.0, fly(Stick { climb: 1.0, ..Default::default() }));
    let h = r.g.chopper_height(&r.level).unwrap();
    assert!(h > 400.0, "only {h} up");
    // Darci goes with it.
    let c = r.g.player_chopper().unwrap().pos();
    assert!((r.g.people[me].fig.pos - c).length() < 300.0);
    // Too high to get out.
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(matches!(r.g.people[me].state, State::Flying { .. }), "got out {h} up");

    // Along its nose.
    let (from, nose) = (r.g.player_chopper().unwrap().pos(), r.g.player_chopper().unwrap().nose());
    r.play(3.0, fly(Stick { ahead: 1.0, ..Default::default() }));
    let moved = r.g.player_chopper().unwrap().pos() - from;
    assert!(moved.x * nose.x + moved.z * nose.y > 500.0, "went {moved} with the nose along {nose}");

    // Down and out, on whatever's under it.
    r.play(15.0, fly(Stick { climb: -1.0, ..Default::default() }));
    assert!(r.g.chopper_height(&r.level).unwrap() < 24.0, "still {} up", r.g.chopper_height(&r.level).unwrap());
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert_eq!(r.g.people[me].state, State::Move);
    let p = r.g.people[me].fig.pos;
    let c = r.g.choppers.iter().find(|c| c.parked).unwrap();
    assert!(c.pilot.is_none());
    assert!((Vec3::new(p.x, 0.0, p.z) - Vec3::new(c.pos().x, 0.0, c.pos().z)).length() < 300.0, "out at {p}, it's at {}", c.pos());
    // Left alone, its rotors run down.
    r.play(10.0, Controls::default());
    assert_eq!(r.g.choppers.iter().find(|c| c.parked).unwrap().state, Heli::Landed);
}

/// A drop from low down: she falls out and lands on her feet.
#[test]
fn she_can_drop_out_from_low_down() {
    let Some(mut r) = explore("police1") else { return };
    let me = r.player();
    get_in(&mut r);
    r.play(5.0, fly(Stick { climb: 1.0, ..Default::default() }));
    // Back down to under a storey.
    while r.g.chopper_height(&r.level).unwrap() > uc_game::chopper::DROP - 40.0 {
        r.play(common::DT, fly(Stick { climb: -1.0, ..Default::default() }));
    }
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(matches!(r.g.people[me].state, State::Fall { .. }), "{:?}", r.g.people[me].state);
    assert!(r.until(3.0, |r| r.g.people[r.player()].state == State::Move), "{:?}", r.g.people[me].state);
}

/// A Beat Cop shift has one parked too.
#[test]
fn beat_cop_parks_one() {
    let Some(level) = common::load("police1") else { return };
    let g = Game::new_beat(&level, 1, None, Some(5));
    assert!(g.choppers.iter().any(|c| c.parked));
}

/// Psycho Park's chopper is the mission's: walking up to it, it takes off as the original's
/// does, and there's nothing to get into.
#[test]
fn a_mission_s_chopper_stays_the_ai_s() {
    let Some(mut r) = common::start("park2") else { return };
    r.play(1.0, Controls::default());
    assert!(r.g.choppers.iter().all(|c| !c.parked));
    let at = r.g.choppers.iter().find(|c| c.way == 151).expect("Psycho Park's chopper").pos();
    r.stand(at + vec3(150.0, 0.0, 0.0));
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(!matches!(r.g.people[r.player()].state, State::Flying { .. }));
}
