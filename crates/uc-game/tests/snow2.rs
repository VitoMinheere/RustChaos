//! Target UC (`snow2`): Darci blows up the Wildcats' three missile sites before any of them
//! can launch. Checks the script end to end, with the walking done by putting Darci where she
//! has to be, as the other mission tests do. Skipped without an install.

use glam::vec3;
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
