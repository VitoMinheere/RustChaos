//! Switches and valves on the map (#295, interfac.cpp:1665): action stood in front of one,
//! facing it, has Darci step up and turn it (`ANIM_VALVE_LOOP`, then `_END`); stood off to
//! one side she doesn't. Day Of Reckoning's fuel pumps are valves. Skipped without an
//! install.

use glam::{vec2, Vec3};
use uc_game::{Controls, State};
use uc_world::character::anim;

mod common;

#[test]
fn she_turns_a_valve_she_stands_at() {
    let Some(mut r) = common::start("Finale1") else { return };
    r.heal = true;
    r.play(40.0, Controls::default());
    let valve = r.level.map.objects.iter().find(|o| o.prim == 196).expect("a valve");
    let (at, yaw) = (Vec3::new(valve.x as f32, valve.y as f32, valve.z as f32), valve.yaw);
    let a = uc_formats::units::radians(yaw as f32);
    let out = Vec3::new(a.sin(), 0.0, a.cos());
    let me = r.player();
    // Off to one side, facing it: nothing.
    let side = Vec3::new(out.z, 0.0, -out.x);
    r.g.people[me].fig.pos = at + out * 192.0 + side * 150.0;
    r.g.people[me].fig.angle = a;
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(!matches!(r.g.people[me].state, State::Press { .. }), "turned it from the side");
    r.play(1.0, Controls::default());
    // In front of it, a little off, facing it near enough: she steps in and turns it.
    r.g.people[me].fig.pos = at + out * 200.0 + side * 20.0;
    r.g.people[me].fig.angle = a + 0.3;
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(matches!(r.g.people[me].state, State::Press { .. }), "{:?}", r.g.people[me].state);
    assert_eq!(r.g.people[me].fig.anim(), anim::VALVE_LOOP);
    let p = r.g.people[me].fig.pos;
    assert!(vec2(p.x - (at.x + out.x * 192.0), p.z - (at.z + out.z * 192.0)).length() < 1.0, "not moved to the valve");
    let mut ended = false;
    for _ in 0..(6.0 / common::DT) as usize {
        r.play(common::DT, Controls::default());
        ended |= r.g.people[me].fig.anim() == anim::VALVE_END;
        if matches!(r.g.people[me].state, State::Move) {
            break;
        }
    }
    assert!(ended, "never let go of it");
    assert!(matches!(r.g.people[me].state, State::Move), "{:?}", r.g.people[me].state);
}
