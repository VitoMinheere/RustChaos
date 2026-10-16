//! Trouble in the Park (`police4`): Darci clears the Wildcats out of the park and meets Roper.
//! Checks the script end to end, with the walking done by putting people where they have to
//! be and the fighting by knocking them down, as the other mission tests do. Skipped without
//! an install.

use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start() -> Option<Run> {
    common::start("police4")
}

impl Run {
    /// Puts down, for good, whoever the points in `ways` created and is still standing.
    fn put_down(&mut self, ways: &[usize]) -> usize {
        let mut n = 0;
        for &way in ways {
            if let Some(i) = self.g.people.iter().position(|p| p.way == way && !p.is_dead() && !p.removed) {
                self.look_at(i, 160.0);
                let from = self.g.people[i].fig.pos;
                self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
                n += 1;
                self.play(1.0, Controls::default());
            }
        }
        n
    }
}

/// The disturbance is put down, the second gang that turns up after it too, and Roper has his
/// word with Darci.
#[test]
fn trouble_in_the_park_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    // The three Wildcats making the disturbance.
    assert_eq!(r.put_down(&[54, 137, 138]), 3, "the disturbance wasn't there");
    r.play(8.0, Controls::default());
    assert!(r.g.people.iter().any(|p| p.way == 158), "Roper never showed: {:?}", r.seen);

    // The second lot.
    assert_eq!(r.put_down(&[143, 145, 147, 149]), 4, "the second gang never turned up");

    // Darci sees Roper, and he has his say.
    let roper = r.who(158);
    for _ in 0..90 {
        if r.g.outcome.is_some() {
            break;
        }
        if !r.g.in_conversation() {
            r.look_at(roper, 300.0);
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(!r.g.people[roper].is_dead(), "Roper mustn't come to harm");
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
