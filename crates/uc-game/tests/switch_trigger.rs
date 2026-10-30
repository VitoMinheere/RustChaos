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

/// Pressed, the switch turns green (`PRIM_OBJ_SWITCH_ON`, interfac.cpp:1765): the game
/// draws it that way, and it's no switch to press again.
#[test]
fn pressed_it_turns_green() {
    let Some(mut r) = common::start("Assault1") else { return };
    // Pressing it wins the mission; play on after.
    r.stop_at_outcome = false;
    r.play(1.0, Controls::default());
    let ob = r.g.point_switch(POINT).unwrap();
    // The level leaves it to the game, which draws it off.
    let on = uc_world::level::load_prim(&r.level.game, uc_world::level::SWITCH_ON).unwrap();
    let off = uc_world::level::load_prim(&r.level.game, uc_world::level::SWITCH_OFF).unwrap();
    let faces = |p: &uc_formats::prm::Prim| p.faces.iter().map(|f| f.texture()).collect::<Vec<_>>();
    assert_ne!(faces(&on), faces(&off), "the switch looks the same on as off");
    let drawn = |r: &common::Run, p: &uc_formats::prm::Prim| {
        let b = r.g.batches(&r.level, glam::Vec3::ZERO);
        p.faces.iter().all(|f| b.contains_key(&f.texture().0))
    };
    assert!(drawn(&r, &off));
    assert!(!r.g.pressed.contains(&ob));
    assert!(r.stand_at_switch(POINT));
    press(&mut r);
    assert!(r.g.pressed.contains(&ob), "pressing didn't turn it on");
    assert!(drawn(&r, &on));
    // Back at it once she's done, action presses nothing.
    r.play(2.0, Controls::default());
    let p = r.player();
    assert!(matches!(r.g.people[p].state, uc_game::State::Move));
    assert!(r.stand_at_switch(POINT));
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(!matches!(r.g.people[p].state, uc_game::State::Press { .. }), "pressed it again");
}

/// Once worked, a switch's point stays triggered (`EWAY_FLAG_TRIGGERED`, eway.cpp:7999):
/// botanic1's point 40 is `active_while`, and stays on after the press.
#[test]
fn it_stays_pressed() {
    const WHILE: usize = 40;
    let Some(mut r) = common::start("botanic1") else { return };
    r.stop_at_outcome = false;
    r.play(1.0, Controls::default());
    assert!(r.stand_at_switch(WHILE));
    press(&mut r);
    r.play(3.0, Controls::default());
    assert!(r.g.point_active(WHILE), "the point went off again");
}
