//! A `player_uses_radius` point with a switch near it (#407, `EWAY_fix_cond`,
//! eway.cpp:1374): action pressed anywhere in its radius does nothing; pressing the switch
//! sets it off. Assault1's point 12 has one 211 off. Skipped without an install.

use glam::vec3;
use uc_game::Controls;

mod common;

const POINT: usize = 12;

fn press(r: &mut common::Run) {
    r.play(common::DT, Controls { action: true, ..Default::default() });
    r.play(1.0, Controls::default());
}

#[test]
fn only_the_switch_sets_it_off() {
    let Some(mut r) = common::start("Assault1") else { return };
    r.play(1.0, Controls::default());
    let ep = r.level.mission.event_points.iter().find(|e| e.index == POINT).unwrap().clone();
    let at = vec3(ep.x as f32, ep.y as f32, ep.z as f32);
    // In the radius, nowhere near where the switch is pressed from.
    r.stand(at);
    press(&mut r);
    assert!(!r.g.point_active(POINT), "set off without the switch");
    // Up to the switch, facing it.
    assert!(r.stand_at_switch(POINT), "point {POINT} isn't on a switch");
    press(&mut r);
    assert!(r.g.point_active(POINT), "pressing the switch didn't set it off");
}
