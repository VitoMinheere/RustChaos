//! Explosives and `prim_damaged`: in Transmission Terminated (`factory1`) Darci blows up the
//! transmitter on the roof, which point 288 watches; in Media Trouble (`e3`) the props that
//! points 75, 76 and 81 watch explode when they're caught. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::{Charge, Controls, Weapon};

mod common;
use common::{Run, DT};

fn start(mission: &str) -> Option<Run> {
    let mut r = Run::new(common::load(mission)?);
    r.stop_at_outcome = false;
    r.play(1.0, Controls::default());
    Some(r)
}

impl Run {
    /// The prop nearest `p`.
    fn prop(&self, p: Vec3) -> usize {
        let d = |o: &uc_formats::iam::Object| (o.x as f32 - p.x).abs() + (o.y as f32 - p.y).abs() + (o.z as f32 - p.z).abs();
        (0..self.level.map.objects.len()).min_by(|&a, &b| d(&self.level.map.objects[a]).total_cmp(&d(&self.level.map.objects[b]))).unwrap()
    }

    /// Picks up the explosives point `way` puts down.
    fn fetch(&mut self, way: usize) {
        let at = self.g.items.iter().find(|it| it.way == way).expect("explosives lying about").pos;
        self.stand(at);
        self.play(0.5, Controls::default());
    }

    /// Takes the explosives out and sets one down where she stands.
    fn set_charge(&mut self) {
        let i = self.g.player.unwrap();
        while self.g.people[i].weapon != Weapon::Explosives {
            self.play(DT, Controls { switch: true, ..Default::default() });
        }
        self.play(DT, Controls { action: true, ..Default::default() });
    }
}

#[test]
fn the_transmitter_goes_up() {
    let Some(mut r) = start("factory1") else { return };
    assert!(r.g.unsupported().is_empty() || !r.g.unsupported().iter().any(|u| u.contains("prim_damaged")),
            "unsupported: {:?}", r.g.unsupported());
    // Two lots of explosives lie about (points 238 and 239).
    r.fetch(238);
    r.fetch(239);
    let me = r.g.player.unwrap();
    assert_eq!(r.g.people[me].explosives, 2, "both should be carried");

    let transmitter = r.prop(r.point(288));
    assert!(!r.g.point_active(288));
    r.stand(r.point(288) + vec3(150.0, 0.0, 0.0));
    r.set_charge();
    assert_eq!(r.g.charges.len(), 1, "a charge should be set");
    assert_eq!(r.g.people[me].explosives, 1, "and one left");
    assert_eq!(r.g.people[me].weapon, Weapon::Explosives, "still holding the other");

    // Out of the way while it burns down: nothing has happened yet half way through.
    r.stand(r.point(288) + vec3(0.0, 0.0, 2500.0));
    r.play(Charge::FUSE / 2.0, Controls::default());
    assert!(!r.g.damaged.contains(&transmitter), "it went off early");
    r.play(Charge::FUSE / 2.0 + 0.5, Controls::default());
    assert!(r.g.charges.is_empty(), "the charge should have gone off");
    assert!(r.g.damaged.contains(&transmitter), "the transmitter should be damaged");
    r.play(0.5, Controls::default());
    assert!(r.g.point_active(288), "point 288 should have seen it");
    assert!(r.seen.iter().any(|m| m.contains("Transmission knocked out")), "{:?}", r.seen);
}

#[test]
fn standing_on_a_charge_is_a_bad_idea() {
    let Some(mut r) = start("factory1") else { return };
    r.fetch(238);
    r.set_charge();
    let me = r.g.player.unwrap();
    assert_eq!(r.g.people[me].weapon, Weapon::None, "the last charge leaves her hands empty");
    // Keep her alive through the test's own healing, but she still takes the blast.
    r.g.update(Charge::FUSE + 0.1, &Controls::default(), &r.level);
    assert!(r.g.people[me].is_down(), "500 at point blank should floor her");
}

/// Media Trouble's props blow up when caught, and one set off sets off its neighbours.
#[test]
fn media_troubles_props_explode() {
    let Some(mut r) = start("e3") else { return };
    assert!(!r.g.unsupported().iter().any(|u| u.contains("prim_damaged")), "unsupported: {:?}", r.g.unsupported());
    let me = r.g.player.unwrap();
    r.g.people[me].explosives = 1;
    r.stand(r.point(75) + vec3(0.0, 0.0, -120.0));
    r.set_charge();
    r.stand(r.point(75) + vec3(0.0, 0.0, 3000.0));
    r.play(Charge::FUSE + 1.0, Controls::default());
    println!("damaged: {:?}", r.g.damaged);
    assert!(r.g.point_active(75), "point 75 should have seen its prop go");
}
