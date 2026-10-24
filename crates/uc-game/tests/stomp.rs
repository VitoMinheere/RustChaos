//! Stamping on someone who's down (#266, `set_person_stomp`): a kick at someone lying
//! knocked down in front of Darci is a stamp, which hurts only the fallen, and one that kills
//! leaves them dead where they lie (`ANIM_FIGHT_STOMPED_*`). Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::character::anim;
use uc_world::Level;

mod common;
use common::DT;

/// An endless run in RTA's park, the first of the wave knocked down with `health` left and
/// Darci stood over him.
fn floored(health: i32) -> Option<(Level, Game, usize)> {
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
    for (k, p) in g.people.iter_mut().enumerate() {
        if k != me && k != foe {
            p.removed = true;
        }
    }
    let at = g.people[me].fig.pos;
    let f = g.people[me].fig.facing();
    g.people[foe].fig.pos = at + vec3(f.x, 0.0, f.y) * 90.0;
    g.people[foe].fig.angle = uc_world::character::angle_of(f);
    g.people[foe].health = health + 1;
    g.people[foe].max_health = health + 1;
    // Hit from behind.
    g.people[foe].take_hit(at, 1, ko::KNOCKDOWN, false, false, Weapon::None);
    assert!(matches!(g.people[foe].state, State::Down), "{:?}", g.people[foe].state);
    for _ in 0..(1.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    // She steps up to where he fell, facing him.
    let body = g.people[foe].body_pos();
    g.people[me].fig.pos = body - vec3(f.x, 0.0, f.y) * 80.0;
    g.people[me].fig.angle = uc_world::character::angle_of(f);
    Some((level, g, foe))
}

/// Stamps once: the frames of her move.
fn stamp(level: &Level, g: &mut Game) {
    let me = g.player.unwrap();
    g.update(DT, &Controls { kick: true, ..Default::default() }, level);
    assert_eq!(g.people[me].fig.anim(), anim::FIGHT_STOMP, "she kicked {}", g.people[me].fig.anim());
    for _ in 0..(1.2 / DT) as usize {
        g.update(DT, &Controls::default(), level);
    }
}

#[test]
fn a_kick_at_the_fallen_is_a_stamp() {
    let Some((level, mut g, foe)) = floored(120) else { return };
    assert_eq!(g.people[foe].fig.anim(), anim::KD_FRONT_LAND);
    stamp(&level, &mut g);
    let left = g.people[foe].health;
    assert!((50..=70).contains(&left), "a stamp left {left} of 120");
    assert!(!g.people[foe].knocked_out, "one stamp shouldn't kill him");
}

#[test]
fn a_stamp_can_kill() {
    let Some((level, mut g, foe)) = floored(40) else { return };
    let me = g.player.unwrap();
    stamp(&level, &mut g);
    assert!(g.people[foe].knocked_out, "on {} he's still alive", g.people[foe].health);
    // He went down onto his front (`ANIM_KD_FRONT_LAND`), and dies there face down.
    assert_eq!(g.people[foe].fig.anim(), anim::FIGHT_STOMPED_BACK);
    // Dead, he's not stamped on again: another kick is just a kick.
    for _ in 0..(1.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    g.update(DT, &Controls { kick: true, ..Default::default() }, &level);
    assert_ne!(g.people[me].fig.anim(), anim::FIGHT_STOMP);
}

#[test]
fn standing_people_are_kicked_not_stamped() {
    let Some((level, mut g, foe)) = floored(120) else { return };
    let me = g.player.unwrap();
    // Up again after his two and a half seconds.
    for _ in 0..(4.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(!g.people[foe].is_down(), "{:?}", g.people[foe].state);
    g.update(DT, &Controls { kick: true, ..Default::default() }, &level);
    assert_ne!(g.people[me].fig.anim(), anim::FIGHT_STOMP);
}
