//! The Missile Silos (`snow1`, in uc-viewer's `UNRELEASED`): Darci blows up the Wildcats'
//! three missiles before the men by them can launch them. It has neither a beacon nor a way
//! for two of the missiles to be damaged of its own (`uc_world::level::finish_unfinished`,
//! `missiles_first`). The walking is done by putting Darci where she has to be. Skipped
//! without an install.

use glam::vec3;
use uc_game::{Charge, Controls, Outcome, Weapon};

mod common;
use common::{Run, DT};

impl Run {
    /// Picks up the explosives point `way` puts down.
    fn fetch(&mut self, way: usize) {
        let at = self.g.items.iter().find(|it| it.way == way).expect("explosives lying about").pos;
        self.stand(at);
        self.play(0.5, Controls::default());
    }

    /// Sets a charge beside the missile point `way` watches and waits, well clear, for it to
    /// go up.
    fn blow_up(&mut self, way: usize) {
        self.stand(self.point(way) + vec3(150.0, 0.0, 0.0));
        self.take_out(Weapon::Explosives);
        self.play(DT, Controls { action: true, ..Default::default() });
        self.stand(self.point(way) + vec3(0.0, 0.0, 2500.0));
        self.play(Charge::FUSE + 1.5, Controls::default());
    }

    fn labels(&self) -> Vec<String> {
        self.g.beacons().into_iter().map(|(l, _)| l).collect()
    }
}

/// A beacon on each missile and on the explosives by the bikes; three charges from there,
/// one by each missile (points 43, 46 and 48 watch them), and it's won.
#[test]
fn missile_silos_run_to_a_win() {
    let Some(mut r) = common::start("snow1") else { return };
    r.play(1.0, Controls::default());
    assert_eq!(r.labels(), ["Warhead", "Warhead", "Warhead", "Supplies"]);
    for explosives in [167, 168, 170] {
        r.fetch(explosives);
    }
    assert_eq!(r.labels(), ["Warhead", "Warhead", "Warhead"]);
    for (left, site) in [(2, 43), (1, 46), (0, 48)] {
        r.blow_up(site);
        assert!(r.g.point_active(site), "missile {site} didn't go up: {:?}", r.seen);
        assert_eq!(r.labels().len(), left, "{:?}", r.labels());
    }
    assert!(r.until(5.0, |r| r.g.outcome.is_some()));
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}

/// Seen by the man at the silo by the sea (point 34), he runs to launch it, and it's lost.
#[test]
fn missile_silos_launch_loses() {
    let Some(mut r) = common::start("snow1") else { return };
    r.play(1.0, Controls::default());
    let i = r.who(34);
    r.look_at(i, 600.0);
    assert!(r.until(60.0, |r| r.g.outcome.is_some()), "{:?}", r.seen);
    assert!(r.said("Missile launch sequence"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}
