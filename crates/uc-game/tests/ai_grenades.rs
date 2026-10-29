//! Enemies with grenades (#405, `PCOM_HAS_GRENADE`, pcom.cpp:5551; Person.cpp:6060): three
//! to a man, in hand, thrown at her from range, and once they're gone, out with the pistol
//! if he's one. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::AI_GANG;
use uc_game::{Controls, Weapon};
use uc_world::character::person as ptype;

mod common;

#[test]
fn police2_has_grenadiers() {
    let Some(mut r) = common::start("police2") else { return };
    r.play(1.0, Controls::default());
    assert!(r.g.people.iter().any(|p| p.weapon == Weapon::Grenade && p.grenades == 3), "nobody with grenades in hand");
}

#[test]
fn they_throw_all_three_then_draw_the_pistol() {
    let Some(mut r) = common::start("fight2") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let me = r.g.people[pi].fig.pos;
    let i = r.g.people.iter().position(|q| !q.player && !q.is_dead() && !q.removed).expect("nobody");
    let q = &mut r.g.people[i];
    (q.ai, q.kind, q.mv, q.fake, q.bent, q.skill) = (AI_GANG, ptype::THUG_RED, 1, false, 0, 10);
    q.give(Weapon::Pistol);
    (q.weapon, q.grenades) = (Weapon::Grenade, 3);
    q.fig.pos = me + vec3(700.0, 0.0, 0.0);
    q.fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
    q.foe = Some(pi);
    q.invulnerable = true;
    q.go_for();
    let mut thrown = 0;
    for _ in 0..(30.0 / common::DT) as usize {
        let before = r.g.grenades.len();
        r.play(common::DT, Controls::default());
        thrown += r.g.grenades.len().saturating_sub(before);
        let q = &mut r.g.people[i];
        q.fig.pos = me + vec3(700.0, 0.0, 0.0);
        if q.weapon != Weapon::Grenade {
            break;
        }
    }
    assert_eq!(thrown, 3, "{thrown} grenades thrown");
    assert_eq!(r.g.people[i].weapon, Weapon::Pistol);
}
