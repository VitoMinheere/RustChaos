//! The original's unused physics (#111) in a level: in RTA's police station, sphere-matter,
//! a flag, a dike and a hypermatter sign made where Darci stands come to rest on its floor,
//! not on its roof, and the dike rides off on the throttle. Skipped without an install.

use glam::vec2;
use uc_game::{dike, Controls};

mod common;

#[test]
fn the_extras_rest_on_the_floor_indoors() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let at = r.g.people[r.player()].fig.pos;
    r.g.sphere_matter_test();
    assert!(r.g.flag_test());
    r.g.dike_test(&r.level);
    assert!(r.g.hypermatter_test(&r.level));
    r.play(5.0, Controls::default());
    // The sign has landed on the floor (it takes a while longer to topple and settle).
    let sign = r.g.hm.get(0).unwrap();
    assert!(sign.points().iter().all(|p| p.pos.is_finite() && p.pos.y > at.y - 8.0), "sign below the floor");
    assert!(sign.points().iter().any(|p| p.pos.y < at.y + 32.0), "sign not down: {:?}", sign.cog());
    for (p, radius, _) in r.g.sm.spheres() {
        assert!(p.is_finite() && p.y >= at.y + radius - 2.0 && p.y < at.y + 160.0, "sphere at {p}, Darci at {at}");
    }
    let c = &r.g.cloths.cloths[0];
    assert!(c.pos.iter().all(|p| p.is_finite() && p.y >= at.y && p.y < at.y + 400.0), "flag at {:?}", c.pos[0]);
    let d = r.g.dike.as_ref().unwrap();
    let (front, back) = d.wheels();
    assert!((front.y - at.y - 80.0).abs() < 2.0 && (back.y - at.y - 80.0).abs() < 2.0, "{front} {back}");
    // Throttle on: it rides off.
    r.g.dike_control = dike::ACCEL;
    r.play(2.0, Controls::default());
    let moved = vec2(r.g.dike.as_ref().unwrap().wheels().1.x - back.x, r.g.dike.as_ref().unwrap().wheels().1.z - back.z).length();
    assert!(moved > 100.0, "rode {moved}");
}
