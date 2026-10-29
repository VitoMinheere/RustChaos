//! A gang closing in (#393, `PCOM_new_gang_attack`): four of them coming at Darci from the
//! one side spread round her, each to a quarter of their own, rather than queueing up on
//! that side. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::{AI_CIV, AI_GANG};
use uc_game::Controls;
use uc_world::character::person as ptype;

mod common;

#[test]
fn a_gang_spreads_round_her() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let me = r.g.people[pi].fig.pos;
    let crowd: Vec<usize> = r.g.people.iter().enumerate().filter(|(_, q)| q.fake && q.ai == AI_CIV && !q.is_dead()).map(|(i, _)| i).take(4).collect();
    assert_eq!(crowd.len(), 4);
    for (k, &i) in crowd.iter().enumerate() {
        let q = &mut r.g.people[i];
        (q.ai, q.kind, q.mv, q.fake, q.bent) = (AI_GANG, ptype::THUG_RED, 1, false, 0);
        q.fig.pos = me + vec3(350.0, 0.0, (k as f32 - 1.5) * 50.0);
        q.fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
        q.foe = Some(pi);
        q.go_for();
    }
    let mut best = 0;
    for _ in 0..(8.0 / common::DT) as usize {
        r.play(common::DT, Controls::default());
        let her = r.g.people[pi].fig.pos;
        let mut sides = [false; 4];
        for &i in &crowd {
            let v = r.g.people[i].fig.pos - her;
            if vec2(v.x, v.z).length() < 300.0 {
                sides[((v.x.atan2(v.z) / std::f32::consts::FRAC_PI_2).round() as i32).rem_euclid(4) as usize] = true;
            }
        }
        best = best.max(sides.iter().filter(|&&s| s).count());
    }
    assert!(best >= 3, "they stayed on {best} side(s) of her");
}
