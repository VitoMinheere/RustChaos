//! Arms Breaker (`wstores1`): Darci blows up the Wildcats' two arms caches, then sees off
//! the team that comes in a van to collect. Checks the script end to end, with the walking done
//! by putting people where they have to be and the fighting by knocking them down, as the
//! other mission tests do. Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Controls, Outcome, Weapon};

mod common;
use common::Run;

fn start(mission: &str) -> Option<Run> {
    let mut r = Run::new(common::load(mission)?);
    r.play(1.0, Controls::default());
    Some(r)
}

/// Both caches go up (points 321 and 359), the van comes, and its four are put down.
#[test]
fn arms_breaker_script_runs_to_a_win() {
    let Some(mut r) = start("wstores1") else { return };
    assert!(r.g.unsupported().is_empty(), "unsupported: {:?}", r.g.unsupported());

    // Explosives lie near each cache.
    r.fetch(408);
    r.blow_up(321);
    assert!(r.g.point_active(321), "the first cache didn't go up: {:?}", r.seen);
    r.fetch(410);
    r.blow_up(359);
    assert!(r.g.point_active(359), "the second cache didn't go up: {:?}", r.seen);

    // The van brings the pickup team.
    r.play(8.0, Controls::default());
    for way in [96, 97, 395, 396] {
        let i = r.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("{way} never came: {:?}", r.seen));
        let from = r.g.people[i].fig.pos;
        r.stand(from + vec3(160.0, 0.0, 0.0));
        r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
        r.play(1.0, Controls::default());
    }
    r.play(5.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
