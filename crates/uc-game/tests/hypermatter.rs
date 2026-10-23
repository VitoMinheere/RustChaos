//! Hypermatter (fallen/Source/hm.cpp), the soft-body prototype the shipped game never runs:
//! prim 6 dropped from 512 above the ground, as the commented-out debug key did
//! (Controls.cpp:4269), lands and comes to rest in one piece. Skipped without an install.

use glam::{vec3, Vec3, Vec3Swizzles};
use uc_game::hm::{self, HmWorld, Object, STEPS_PER_TURN};

/// Flat ground at a fixed height.
struct Flat(f32);

impl HmWorld for Flat {
    fn height(&self, _: f32, _: f32) -> f32 {
        self.0
    }
}

#[test]
fn prim_6_falls_lands_and_settles() {
    let Some(game) = uc_formats::default_install() else { return };
    let prim = uc_world::level::load_prim(&game, 6u16).expect("prim 6");
    let (lo, hi) = uc_world::level::prim_bounds(&prim);
    let ground = Flat(64.0);
    let at = vec3(8000.0, ground.0 + 512.0, 9000.0);
    let mut o = Object::new(&prim, 6, at, [0, 0, 0], Vec3::ZERO, &hm::primgrid(6), hm::DEBUG_KEY).expect("made");
    assert_eq!(o.mesh_len(), prim.points.len());

    let mut landed = None;
    let mut still = None;
    for turn in 0..20 * 30 {
        for _ in 0..STEPS_PER_TURN {
            o.process(&ground);
        }
        assert!(o.points().iter().all(|p| p.pos.is_finite()), "turn {turn}");
        if landed.is_none() && o.points().iter().any(|p| p.pos.y < ground.0 + 8.0) {
            landed = Some(turn);
        }
        if landed.is_some() && o.stationary() {
            still = Some(turn);
            break;
        }
    }
    let landed = landed.expect("never landed");
    // From 512 up at 0.015 a step squared, the lowest point (the prim's base is at its
    // origin) takes about 32 turns to fall.
    assert!((20..60).contains(&landed), "landed at turn {landed}");
    assert!(still.is_some(), "never came to rest");

    // The lattice is on the ground, none of it under.
    let ys = o.points().iter().map(|p| p.pos.y);
    assert!(ys.clone().all(|y| y > ground.0 - 1.0), "under the ground");
    assert!(ys.fold(f32::MAX, f32::min) < ground.0 + 8.0, "not on the ground");

    // The sign drawn through it: about the size it was, about where it was dropped. It
    // falls over, and the corners that land are held where they touch (friction 0 keeps
    // none of their sideways speed), so it ends a little squashed. Its lowest points are
    // inside the lattice, a little off the ground.
    let pts: Vec<Vec3> = (0..o.mesh_len()).map(|i| o.mesh_point(i)).collect();
    assert!(pts.iter().all(|p| p.is_finite()));
    let (mlo, mhi) = pts.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(a, b), &p| (a.min(p), b.max(p)));
    assert!(mlo.y > ground.0 - 16.0, "under the ground: {mlo}");
    assert!(mlo.y < ground.0 + 64.0, "not on the ground: {mlo}");
    let (size, now) = (hi - lo, mhi - mlo);
    let longest = size.max_element();
    assert!(now.max_element() > longest * 0.7 && now.max_element() < longest * 1.4, "{size} became {now}");
    let centre = (mlo.xz() + mhi.xz()) / 2.0;
    let dropped = (lo.xz() + hi.xz()) / 2.0 + at.xz();
    // Lying down, it covers ground up to its height away.
    assert!(centre.distance(dropped) < longest, "{centre} from {dropped}");
    // `place` and `mesh_point` agree.
    for (i, p) in prim.points.iter().enumerate().step_by(7) {
        let v = vec3(p[0] as f32, p[1] as f32, p[2] as f32);
        assert!((o.place(v) - pts[i]).length() < 0.01);
    }
}
