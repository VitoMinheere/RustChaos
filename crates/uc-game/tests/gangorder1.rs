//! Assassin (`Gangorder1`): Mako takes out Corrola for the Boss. Checks the script end to end,
//! with the walking done by putting people where they have to be and the fighting by knocking
//! them down, as the other mission tests do. Skipped without an install.

use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start() -> Option<Run> {
    common::start("Gangorder1")
}

/// Radar gives the job, Corrola goes down, and Mako gets back to the hideout.
#[test]
fn assassin_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    // Radar, up on his roof, points Mako at Corrola.
    r.walk_up_and_talk(177, 70.0);
    assert!(r.g.point_active(179), "Radar never gave the job: {:?}", r.seen);

    // Corrola and his bodyguards turn up; put him down for good.
    r.play(2.0, Controls::default());
    let corrola = r.who(113);
    r.look_at(corrola, 160.0);
    let from = r.g.people[corrola].fig.pos;
    r.g.people[corrola].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    assert!(r.said("SMOKED CORROLA"), "nobody noticed: {:?}", r.seen);
    assert_eq!(r.g.outcome, None);

    // Back to the hideout.
    r.stand(r.point(116));
    r.play(8.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
