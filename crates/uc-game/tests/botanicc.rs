//! Grim Gardens (`botanicc`): Darci blows up what the Wildcats keep in the gardens, one lot
//! at each end, and the cameras show each going up. Checks the script end to end, with the
//! walking done by putting Darci where she has to be, as the other mission tests do. Skipped
//! without an install.

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
