//! The AI's guns run dry (#403, Person.cpp:573 and 6034; pcom.cpp:9005): a pistol's 15
//! rounds and no more, then a click and the gun put away to come on hand to hand; an AK
//! never runs out. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::AI_GANG;
use uc_game::{Controls, Sfx, Weapon};
use uc_world::character::person as ptype;

mod common;

/// Someone in the fight school made a gang member with `w`, coming for her from 500 off:
/// the shots fired in a minute and a half, and what's in their hand at the end.
fn shoot_it_out(w: Weapon) -> Option<(usize, Weapon)> {
    let mut r = common::start("fight2")?;
    r.heal = true;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let me = r.g.people[pi].fig.pos;
    let i = r.g.people.iter().position(|q| !q.player && !q.is_dead() && !q.removed).expect("nobody");
    let q = &mut r.g.people[i];
    (q.ai, q.kind, q.mv, q.fake, q.bent, q.weapon, q.skill) = (AI_GANG, ptype::THUG_RED, 1, false, 0, w, 10);
    q.fig.pos = me + vec3(500.0, 0.0, 0.0);
    q.fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
    q.foe = Some(pi);
    q.go_for();
    let mut shots = 0;
    r.g.take_sounds();
    for _ in 0..(90.0 / common::DT) as usize {
        r.play(common::DT, Controls::default());
        // Keep them where they were, so they shoot rather than close in.
        let q = &mut r.g.people[i];
        if q.weapon.is_gun() {
            q.fig.pos = me + vec3(500.0, 0.0, 0.0);
        }
        shots += r.g.take_sounds().iter().filter(|(s, _)| matches!(s, Sfx::Shot(_))).count();
    }
    Some((shots, r.g.people[i].weapon))
}

#[test]
fn a_pistol_runs_dry() {
    let Some((shots, w)) = shoot_it_out(Weapon::Pistol) else { return };
    assert_eq!(shots, 15, "{shots} shots from a pistol");
    assert_eq!(w, Weapon::None, "still holding a {w:?}");
}

#[test]
fn an_ak_doesnt() {
    let Some((shots, w)) = shoot_it_out(Weapon::Ak47) else { return };
    assert!(shots > 15, "only {shots} shots from an AK");
    assert_eq!(w, Weapon::Ak47);
}
