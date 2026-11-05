//! Grim Gardens (`botanicc`): Darci blows up what the Wildcats keep in the gardens, one lot
//! at each end, and the cameras show each going up. Checks the script end to end, with the
//! walking done by putting Darci where she has to be, as the other mission tests do. Skipped
//! without an install.

use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start(mission: &str) -> Option<Run> {
    let mut r = Run::new(common::load(mission)?);
    r.play(1.0, Controls::default());
    Some(r)
}

/// A charge at each end (points 131 and 138 watch the props), each followed by its fly-by
/// (whose last waypoints, 212 and 210, are what the win waits on).
#[test]
fn grim_gardens_script_runs_to_a_win() {
    let Some(mut r) = start("botanicc") else { return };
    assert!(r.g.unsupported().is_empty(), "unsupported: {:?}", r.g.unsupported());

    // Two lots of explosives lie together (points 129 and 213).
    r.fetch(129);
    r.fetch(213);
    r.blow_up(131);
    assert!(r.g.point_active(131) || r.g.point_active(164), "the first lot didn't go up: {:?}", r.seen);
    r.blow_up(138);
    assert!(r.g.point_active(138) || r.g.point_active(345), "the second lot didn't go up: {:?}", r.seen);

    r.play(10.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
