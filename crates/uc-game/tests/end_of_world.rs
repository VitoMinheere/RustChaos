//! The end of the world (#234): Day Of Reckoning's `create_item` points 200 and 201 ask for
//! wire cutters, which the loader turns into `EWAY_DO_END_OF_WORLD`. When the eight-minute
//! countdown (point 95) runs out, the city goes up round Darci and the mission is lost.
//! Skipped without an install.

use uc_game::{Controls, Outcome};
use uc_world::character::person as ptype;

mod common;

#[test]
fn the_world_ends_when_day_of_reckoning_runs_out_of_time() {
    let Some(mut r) = common::start("Finale1") else { return };
    // Bane's men are taken off: the fighting isn't what's tested.
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut() {
            if p.hostile() {
                p.removed = true;
            }
            if matches!(p.kind, ptype::ROPER) {
                p.health = p.max_health;
            }
        }
    });
    r.play(479.0, Controls::default());
    assert!(r.g.end_of_world.is_none(), "the world ended early");
    assert!(r.g.outcome.is_none(), "the mission ended early: {:?}", r.g.outcome);
    // The bang comes 1.7 seconds in and the end at 4.05, on top of a second or so for the
    // countdown's point to go off.
    r.heal = false;
    r.play(8.0, Controls::default());
    assert!(r.g.end_of_world.is_some(), "the countdown ran out and nothing happened");
    assert_eq!(r.g.outcome, Some(Outcome::Lost("Time ran out".into())));
    assert!(r.g.no_floor, "the floor goes with the world");
    assert!(r.g.items.iter().all(|i| i.kind != 25), "wire cutters were put down as an item");
}
