//! Psycho Park (`park2`): gang after gang, each brought on by putting down the one before,
//! until the kingpin's own crew (colour 14) is gone. Checks the script end to end, with the
//! fighting done by knocking people down, as the other mission tests do. Skipped without an
//! install.

use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    common::start("park2")
}

impl Run {

}

/// Everyone who comes at Darci goes down, round after round; Roper (163) must come through.
#[test]
fn psycho_park_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let mut downed = 0;
    for _ in 0..240 {
        if r.g.outcome.is_some() {
            break;
        }
        let hostile: Vec<usize> = (0..r.g.people.len())
            .filter(|&i| {
                let p = &r.g.people[i];
                !p.player && !p.removed && !p.is_dead() && p.way != 0 && p.hostile()
            })
            .collect();
        for i in hostile {
            r.look_at(i, 160.0);
            r.play(DT, Controls::default());
            let from = r.g.people[i].fig.pos;
            r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
            downed += 1;
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s after {downed} down", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
