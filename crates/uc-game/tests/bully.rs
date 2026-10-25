//! Bullies (#243, `PCOM_AI_BULLY`): they pick on whoever's about that they can see within
//! 0x600, a passer-by as soon as the player, and see them off; one who's
//! `PCOM_BENT_ONLY_KILL_PLAYER` only ever picks on the player. In RTA, with one of its
//! crowd made a bully and Darci out of the way. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_BULLY, AI_CIV, BENT_ONLY_KILL_PLAYER, MOVE_WANDER};
use uc_game::Controls;
use uc_world::character::person as ptype;

mod common;
use common::Run;

/// RTA with a bully and a passer-by four squares in front of him, where he can see them, and
/// Darci thirty squares off. Returns (bully, passer-by).
fn setup(bent: u8) -> Option<(Run, usize, usize)> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let crowd: Vec<usize> = r.g.people.iter().enumerate().filter(|(_, q)| q.fake && q.ai == AI_CIV && !q.is_dead()).map(|(i, _)| i).collect();
    let (b, v) = (crowd[0], crowd[1]);
    let eye = vec3(0.0, 96.0, 0.0);
    // Somewhere open, by Darci's start.
    let me = r.g.people[r.player()].fig.pos;
    let (at, dir) = (0..16)
        .map(|k| k as f32 * std::f32::consts::TAU / 16.0)
        .map(|a| (me, glam::vec2(-a.sin(), -a.cos())))
        .find(|(at, f)| r.level.ground.line_of_sight(*at + eye, *at + vec3(f.x, 0.0, f.y) * 1024.0 + eye))
        .expect("nowhere open");
    let put = |r: &mut Run, i: usize, p: glam::Vec3, angle: f32| {
        let y = r.level.ground.surface(&r.level.map, p.x, p.z, p.y + 64.0);
        r.g.people[i].fig.pos = vec3(p.x, y, p.z);
        r.g.people[i].fig.angle = angle;
    };
    let facing = uc_world::character::angle_of(dir);
    put(&mut r, b, at, facing);
    let q = &mut r.g.people[b];
    (q.ai, q.kind, q.mv, q.fake, q.bent) = (AI_BULLY, ptype::THUG_RASTA, 1, false, bent);
    put(&mut r, v, at + vec3(dir.x, 0.0, dir.y) * 1024.0, facing);
    let q = &mut r.g.people[v];
    (q.mv, q.fake) = (MOVE_WANDER, false);
    let pi = r.player();
    r.g.people[pi].fig.pos = at - vec3(dir.x, 0.0, dir.y) * 30.0 * 256.0;
    Some((r, b, v))
}

#[test]
fn a_bully_picks_on_a_passer_by() {
    let Some((mut r, b, v)) = setup(0) else { return };
    r.play(1.0, Controls::default());
    assert_eq!(r.g.people[b].foe, Some(v), "he should be after the passer-by");
    assert!(r.g.people[b].aggro());
}

#[test]
fn one_who_only_picks_on_the_player_leaves_them_be() {
    let Some((mut r, b, _)) = setup(BENT_ONLY_KILL_PLAYER) else { return };
    r.play(2.0, Controls::default());
    assert_eq!(r.g.people[b].foe, None, "with the player far off he has nobody to pick on");
    assert!(!r.g.people[b].aggro());
}

/// A cop in sight of a bully shooting at someone comes after him (#292, `do_arrests`).
#[test]
fn a_cop_who_sees_it_comes_for_the_bully() {
    let Some((mut r, b, v)) = setup(0) else { return };
    let cop = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead() && ![b, v].contains(&r.g.people.iter().position(|o| std::ptr::eq(o, q)).unwrap()))
        .expect("someone for a cop");
    // Down the street towards the passer-by, a little to one side: in the bully's view (the
    // street is clear that far) and within 0x800 of him.
    let (at, to) = (r.g.people[b].fig.pos, r.g.people[v].fig.pos);
    let along = (to - at).normalize();
    let spot = at + along * 700.0 + vec3(along.z, 0.0, -along.x) * 60.0;
    let y = r.level.ground.surface(&r.level.map, spot.x, spot.z, spot.y + 64.0);
    let q = &mut r.g.people[cop];
    (q.ai, q.kind, q.mv, q.fake, q.home) = (uc_game::person::AI_COP, ptype::COP, 1, false, None);
    q.fig.pos = vec3(spot.x, y, spot.z);
    // A gun, so he opens fire on the passer-by: a villain shooting calls in the police.
    r.g.people[b].give(uc_game::Weapon::Pistol);
    r.g.people[b].weapon = uc_game::Weapon::Pistol;
    let mut came = false;
    for _ in 0..(10.0 / common::DT) as usize {
        r.play(common::DT, Controls::default());
        if r.g.people[cop].foe == Some(b) && r.g.people[cop].aggro() {
            came = true;
            break;
        }
    }
    assert!(came, "the cop let him be");
    assert!(r.g.people[b].felon, "not made a felon");
}
