//! AI sliding tackles (#303, pcom.cpp:8681): a hunter running after someone who's running
//! goes in with a sliding tackle once close. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::{Controls, State};
use uc_world::character::person as ptype;

mod common;

#[test]
fn a_hunter_slides_into_her_as_she_runs() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let h = r.g.people.iter().position(|q| q.fake && q.ai == uc_game::person::AI_CIV && !q.is_dead()).expect("no crowd");
    let me = r.g.people[pi].fig.pos;
    // Facing along +x, with the hunter a few squares behind her.
    r.g.people[pi].fig.angle = uc_world::character::angle_of(vec2(1.0, 0.0));
    {
        let q = &mut r.g.people[h];
        (q.ai, q.kind, q.mv, q.fake, q.skill) = (3, ptype::THUG_RED, 2, false, 8);
        q.fig.pos = me - vec3(500.0, 0.0, 0.0);
        q.foe = Some(pi);
        q.go_for();
    }
    let mut slid = false;
    for k in 0..(12.0 / common::DT) as usize {
        // She runs on and off along +x, turning back now and then so he catches her up.
        let back = (k / 90) % 2 == 1;
        let c = Controls { wish: vec2(if back { -1.0 } else { 1.0 }, 0.0), run: true, ..Default::default() };
        r.play(common::DT, c);
        if matches!(r.g.people[h].state, State::Slide { .. }) {
            slid = true;
            break;
        }
    }
    assert!(slid, "no sliding tackle");
}
