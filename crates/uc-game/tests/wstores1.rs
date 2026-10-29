//! Arms Breaker (`wstores1`): Darci blows up the Wildcats' two arms caches, then sees off
//! the team that comes in a van to collect. Checks the script end to end, with the walking done
//! by putting people where they have to be and the fighting by knocking them down, as the
//! other mission tests do. Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Charge, Controls, Outcome, Weapon};

mod common;
use common::{Run, DT};

fn start(mission: &str) -> Option<Run> {
    let mut r = Run::new(common::load(mission)?);
    r.play(1.0, Controls::default());
    Some(r)
}

impl Run {
    /// Picks up the explosives point `way` puts down.
    fn fetch(&mut self, way: usize) {
        let at = self.g.items.iter().find(|it| it.way == way).expect("explosives lying about").pos;
        self.stand(at);
        self.play(0.5, Controls::default());
    }

    /// Takes the explosives out and sets one down where she stands.
    fn set_charge(&mut self) {
        self.take_out(Weapon::Explosives);
        self.play(DT, Controls { action: true, ..Default::default() });
    }
}

impl Run {
    /// Sets a charge beside what point `way` watches and waits, well clear, for it to go up.
    fn blow_up(&mut self, way: usize) {
        self.stand(self.point(way) + vec3(150.0, 0.0, 0.0));
        self.set_charge();
        self.stand(self.point(way) + vec3(0.0, 0.0, 2500.0));
        self.play(Charge::FUSE + 1.5, Controls::default());
    }
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
