//! Urban Shakedown (`westcrime1`): the Wildcats come in waves, each one brought on by wiping
//! out the one before, against the clock. Checks the script end to end, with the fighting done
//! by knocking them down, as the other mission tests do. Skipped without an install.

use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start() -> Option<Run> {
    common::start("westcrime1")
}

impl Run {

}

/// The waves are the colours 7, 8, 9, 10 and 12; when the last is gone, the district is safe.
#[test]
fn urban_shakedown_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let mut downed = 0;
    for _ in 0..120 {
        if r.g.outcome.is_some() {
            break;
        }
        let wave: Vec<usize> = (0..r.g.people.len())
            .filter(|&i| {
                let p = &r.g.people[i];
                !p.player && !p.removed && !p.is_dead() && p.way != 0 && [7, 8, 9, 10, 12].contains(&p.colour)
            })
            .collect();
        for i in wave {
            r.look_at(i, 160.0);
            let from = r.g.people[i].fig.pos;
            r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
            downed += 1;
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s after {downed} down", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
