//! Seek, Sneak and Seize (`Stealtst1`): lift the disk and meet at the front gates. Skipped
//! without an install.

use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start() -> Option<Run> {
    common::start("Stealtst1")
}

/// The disk is lifted and taken to the gates.
#[test]
fn seek_sneak_and_seize_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());

    // The disk point 26 puts down is a file, which only became something Darci can carry when
    // evidence items did; `item_held` on point 27 is watching for it.
    let disk = r.g.items.iter().find(|it| it.way == 26).expect("the disk").clone();
    assert_eq!(disk.kind, 14, "the disk is a file");
    r.stand(disk.pos);
    r.play(2.0, Controls::default());
    assert!(r.said("You got the disk"), "it was never lifted: {:?}", r.seen);
    assert_eq!(r.g.outcome, None, "the disk alone doesn't finish it");

    // Then the front gates, which point 36 watches as a cuboid.
    r.stand(r.point(36));
    r.play(4.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// Walking out to the gates without the disk proves the two halves are both needed.
#[test]
fn the_gates_alone_arent_enough() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    r.stand(r.point(36));
    r.play(5.0, Controls::default());
    assert_eq!(r.g.outcome, None, "the gates without the disk shouldn't win it");
}

/// Point 92 ends it if the man point 28 creates is killed.
#[test]
fn losing_the_contact_fails_it() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    let Some(i) = r.g.people.iter().position(|p| p.way == 28) else { return };
    let from = r.g.people[i].fig.pos;
    r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "that should lose it: {:?}", r.g.outcome);
}
