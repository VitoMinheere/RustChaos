//! A gun pointed at them (#304, `PCOM_set_person_ai_navtokill`): going for Darci with her
//! gun on them, someone unarmed runs for it half the time; with nothing drawn they come
//! on. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::{AI_CIV, AI_GANG};
use uc_game::{Controls, Weapon};
use uc_world::character::person as ptype;

mod common;

/// Six of RTA's crowd made gang members in a line in front of her, all coming for her.
fn line_up(r: &mut common::Run, gun: bool) -> Vec<usize> {
    r.heal = true;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let me = r.g.people[pi].fig.pos;
    r.g.people[pi].fig.angle = uc_world::character::angle_of(vec2(1.0, 0.0));
    if gun {
        r.g.people[pi].give(Weapon::Pistol);
        r.g.people[pi].weapon = Weapon::Pistol;
    }
    let crowd: Vec<usize> = r.g.people.iter().enumerate().filter(|(_, q)| q.fake && q.ai == AI_CIV && !q.is_dead()).map(|(i, _)| i).take(6).collect();
    for (k, &i) in crowd.iter().enumerate() {
        let q = &mut r.g.people[i];
        (q.ai, q.kind, q.mv, q.fake, q.bent) = (AI_GANG, ptype::THUG_RED, 1, false, 0);
        q.fig.pos = me + vec3(300.0, 0.0, (k as f32 - 2.5) * 40.0);
        q.fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
        q.foe = Some(pi);
        q.go_for();
    }
    crowd
}

fn fled(r: &mut common::Run, who: &[usize]) -> usize {
    let mut out = vec![false; who.len()];
    for _ in 0..(2.0 / common::DT) as usize {
        r.play(common::DT, Controls::default());
        for (k, &i) in who.iter().enumerate() {
            out[k] |= r.g.people[i].scared.is_some();
        }
    }
    out.iter().filter(|&&f| f).count()
}

#[test]
fn with_her_gun_on_them_some_run() {
    let Some(mut r) = common::start("police1") else { return };
    let who = line_up(&mut r, true);
    let n = fled(&mut r, &who);
    assert!(n >= 1, "nobody ran from her gun");
}

#[test]
fn with_nothing_drawn_they_come_on() {
    let Some(mut r) = common::start("police1") else { return };
    let who = line_up(&mut r, false);
    assert_eq!(fled(&mut r, &who), 0, "they ran from an empty hand");
}
