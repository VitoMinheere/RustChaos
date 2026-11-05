//! Target UC (`snow2`): Darci blows up the Wildcats' three missile sites before any of them
//! can launch. Checks the script end to end, with the walking done by putting Darci where she
//! has to be, as the other mission tests do. Skipped without an install.

use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start(mission: &str) -> Option<Run> {
    let mut r = Run::new(common::load(mission)?);
    r.play(1.0, Controls::default());
    Some(r)
}

/// A charge at each site (points 48, 46 and 43 watch them).
#[test]
fn target_uc_script_runs_to_a_win() {
    let Some(mut r) = start("snow2") else { return };
    assert!(r.g.unsupported().is_empty(), "unsupported: {:?}", r.g.unsupported());

    for (explosives, site) in [(138, 46), (166, 43), (168, 48)] {
        r.fetch(explosives);
        r.blow_up(site);
        assert!(r.g.point_active(site), "site {site} didn't go up: {:?}", r.seen);
    }
    r.play(5.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
