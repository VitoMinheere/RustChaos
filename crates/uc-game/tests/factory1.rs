//! Transmission Terminated (`factory1`): Darci blows up the Wildcats' transmitter on the roof
//! and clears the factory of them without losing a hostage. Checks the script end to end, with
//! the walking done by putting Darci where she has to be and the fighting by knocking people
//! down, as the other mission tests do. Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Controls, Outcome, Weapon};

mod common;
use common::{Run, DT};

fn start(mission: &str) -> Option<Run> {
    let mut r = Run::new(common::load(mission)?);
    r.play(1.0, Controls::default());
    Some(r)
}

/// The transmitter (point 288 watches it) goes up and colour 2 is wiped out; the hostages
/// (colour 3) must all come through.
#[test]
fn transmission_terminated_script_runs_to_a_win() {
    let Some(mut r) = start("factory1") else { return };
    assert!(r.g.unsupported().is_empty(), "unsupported: {:?}", r.g.unsupported());

    r.fetch(238);
    r.blow_up(288);
    assert!(r.g.point_active(288), "the transmitter didn't go up: {:?}", r.seen);

    for _ in 0..60 {
        if r.g.outcome.is_some() {
            break;
        }
        let gang: Vec<usize> = (0..r.g.people.len())
            .filter(|&i| {
                let p = &r.g.people[i];
                !p.player && !p.removed && !p.is_dead() && p.way != 0 && p.colour == 2
            })
            .collect();
        for i in gang {
            let from = r.g.people[i].fig.pos;
            r.stand(from + vec3(160.0, 0.0, 0.0));
            r.play(DT, Controls::default());
            let from = r.g.people[i].fig.pos;
            r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
