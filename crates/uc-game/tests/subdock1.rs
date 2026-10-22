//! subdock1 (Dock Extraction, unreleased): with the dock operator (point 37) out, the police
//! helicopter (41) comes in on a roof at the south end with reinforcements, and getting to it
//! (point 42) wins. Darci is moved rather than walked, and the operator knocked out rather
//! than fought. Skipped without an install.

use glam::vec3;
use uc_game::{person::ko, Controls, Outcome, Weapon};

mod common;

#[test]
fn the_operator_down_and_darci_at_the_chopper_wins() {
    let Some(mut r) = common::start("subdock1") else { return };
    r.play(1.0, Controls::default());
    assert!(r.said("Terminate dock operator"), "{:?}", r.seen);
    assert!(!r.g.point_active(41), "chopper too soon");
    let op = r.who(37);
    let from = r.g.people[r.player()].fig.pos;
    r.g.people[op].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    assert!(r.until(10.0, |r| r.g.point_active(41)), "no chopper");
    assert!(r.until(5.0, |r| r.said("Choppers comin in")), "{:?}", r.seen);
    // The rendezvous is up on a roof: there's a floor there.
    let at = r.point(42);
    let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    assert!((y - at.y).abs() < 64.0, "nothing to stand on at {at}: {y}");
    r.stand(vec3(at.x, at.y, at.z));
    assert!(r.until(5.0, |r| r.g.outcome.is_some()), "not over");
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
