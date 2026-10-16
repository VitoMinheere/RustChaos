//! Darci's kick between the legs (#142, `set_person_kick_near`): kicking as she steps in on
//! someone close doubles him over (`ANIM_KICK_NAD_TAKE`, `_STUNNED`, `_RECOVER`), for more
//! than a kick's damage; while he's bent double, another kick is an ordinary one. Skipped
//! without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::{Controls, Game};
use uc_world::character::anim;
use uc_world::Level;

mod common;
use common::DT;

/// An endless run in RTA's park, with the first of the wave stood just in front of Darci.
fn face_off() -> Option<(Level, Game, usize)> {
    let level = common::load("police1")?;
    let mut g = Game::new_endless(&level, 5, Some(vec2(8827.0, 10274.0)), 9.0);
    let me = g.player.unwrap();
    let mut foe = None;
    for _ in 0..(10.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        foe = g.people.iter().position(|p| !p.player && p.hostile() && !p.removed);
        if foe.is_some() {
            break;
        }
    }
    let foe = foe.expect("nobody came");
    // Everyone else out of it.
    for (k, p) in g.people.iter_mut().enumerate() {
        if k != me && k != foe {
            p.removed = true;
        }
    }
    let at = g.people[me].fig.pos;
    let f = g.people[me].fig.facing();
    g.people[foe].fig.pos = at + vec3(f.x, 0.0, f.y) * 100.0;
    g.people[foe].fig.angle = uc_world::character::angle_of(-f);
    Some((level, g, foe))
}

/// Towards him, kicking.
fn step_in_kick(g: &Game, foe: usize) -> Controls {
    let me = g.player().unwrap().fig.pos;
    let to = g.people[foe].fig.pos - me;
    Controls { wish: Vec2::new(to.x, to.z).normalize(), kick: true, ..Default::default() }
}

#[test]
fn stepping_in_kicks_between_the_legs() {
    let Some((level, mut g, foe)) = face_off() else { return };
    let me = g.player.unwrap();
    let before = g.people[foe].health;
    let c = step_in_kick(&g, foe);
    g.update(DT, &c, &level);
    assert_eq!(g.people[me].fig.anim(), anim::KICK_NAD, "she kicked {}", g.people[me].fig.anim());
    // He doubles over, stays bent, then straightens up.
    let mut seen = Vec::new();
    for _ in 0..(6.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        let a = g.people[foe].fig.anim();
        if seen.last() != Some(&a) && matches!(a, anim::KICK_NAD_TAKE | anim::KICK_NAD_STUNNED | anim::KICK_NAD_RECOVER) {
            seen.push(a);
        }
    }
    assert_eq!(seen, [anim::KICK_NAD_TAKE, anim::KICK_NAD_STUNNED, anim::KICK_NAD_RECOVER], "didn't double over");
    let lost = before - g.people[foe].health;
    assert!(lost >= 50, "only {lost} for a kick between the legs");
}

#[test]
fn no_second_one_while_he_is_doubled_up() {
    let Some((level, mut g, foe)) = face_off() else { return };
    let me = g.player.unwrap();
    g.update(DT, &step_in_kick(&g, foe), &level);
    for _ in 0..(1.5 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        if g.people[foe].doubled() && !matches!(g.people[me].state, uc_game::person::State::Attack { .. }) {
            break;
        }
    }
    assert!(g.people[foe].doubled(), "not doubled up");
    g.update(DT, &step_in_kick(&g, foe), &level);
    assert_ne!(g.people[me].fig.anim(), anim::KICK_NAD, "kicked there again while he was bent double");
}

/// From further off it's her ordinary kick.
#[test]
fn from_further_off_it_is_a_normal_kick() {
    let Some((level, mut g, foe)) = face_off() else { return };
    let me = g.player.unwrap();
    let at = g.people[me].fig.pos;
    let f = g.people[me].fig.facing();
    g.people[foe].fig.pos = at + vec3(f.x, 0.0, f.y) * 180.0;
    g.update(DT, &step_in_kick(&g, foe), &level);
    assert_ne!(g.people[me].fig.anim(), anim::KICK_NAD);
}
