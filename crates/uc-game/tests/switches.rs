//! Switches (`switch` points): in subdock1, one of the two unfinished missions with them, a
//! switch (point 23) opens a door (`activate_prim` point 24) and films it (camera point 32).
//! Action next to it flips it, as the cut code in interfac.cpp did. Takeover1's switches hang
//! in the air over an empty patch of its map (jumper1, which lacks the buildings they were
//! made for), so they can't be reached. Skipped without an install.

use glam::vec3;
use uc_game::Controls;

mod common;
use common::Run;

#[test]
fn action_at_a_switch_sets_off_its_point() {
    let Some(mut r) = common::start("subdock1") else { return };
    r.stop_at_outcome = false;
    r.play(1.0, Controls::default());
    let at = r.g.switches.0.iter().find(|s| (s.pos - r.point(23)).abs().element_sum() < 512.0).expect("no switch by point 23").pos;
    // Pressing action out of reach does nothing.
    r.stand(at + vec3(600.0, 0.0, 0.0));
    r.play(0.5, Controls { action: true, ..Default::default() });
    r.play(0.5, Controls::default());
    assert!(!r.g.point_active(23) && !r.g.point_active(24));
    // Under it, it flips, and what waits on it goes.
    r.stand(at - vec3(0.0, 100.0, 0.0));
    let feet = r.g.people[r.player()].fig.pos;
    assert!((at - feet - vec3(0.0, 64.0, 0.0)).abs().element_sum() < 256.0, "can't reach it from {feet}");
    r.play(1.0 / 30.0, Controls { action: true, ..Default::default() });
    r.play(0.5, Controls::default());
    assert!(r.g.point_active(23), "switch point not set off");
    assert!(r.g.point_active(24), "door not opened");
}

#[test]
fn takeover1_loads_its_switches() {
    let Some(level) = common::load("Takeover1") else { return };
    let r = Run::new(level);
    assert!(r.g.unsupported().is_empty(), "{:?}", r.g.unsupported());
    assert_eq!(r.g.switches.0.len(), 3);
}
