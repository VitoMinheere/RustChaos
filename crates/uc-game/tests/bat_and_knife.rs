//! A bat and a knife both carried (#399, `should_person_get_item`, Special.cpp:284-304):
//! one of each, not two of either. Skipped without an install.

use uc_game::{Controls, Game, Weapon};

mod common;

/// Puts weapon `w` at `at`: a copy of one of the level's items, so it's been there since
/// the start.
fn place(g: &mut Game, w: Weapon, at: glam::Vec3) {
    let mut it = g.items.first().expect("no items").clone();
    (it.way, it.kind, it.pos, it.rounds) = (0, w.item().unwrap(), at, None);
    g.items.push(it);
}

#[test]
fn she_carries_a_bat_and_a_knife() {
    let Some(mut r) = common::start("fight2") else { return };
    r.play(2.0, Controls::default());
    let pi = r.player();
    let at = r.g.people[pi].fig.pos;
    place(&mut r.g, Weapon::Bat, at);
    r.play(0.3, Controls::default());
    place(&mut r.g, Weapon::Knife, at);
    r.play(0.3, Controls::default());
    let p = &r.g.people[pi];
    assert!(p.has(Weapon::Bat) && p.has(Weapon::Knife), "bat {} knife {}", p.has(Weapon::Bat), p.has(Weapon::Knife));
    assert!(p.carried().contains(&Weapon::Knife) && p.carried().contains(&Weapon::Bat));
    // A second knife stays where it is.
    place(&mut r.g, Weapon::Knife, at);
    let n = r.g.items.len();
    r.play(0.3, Controls::default());
    assert_eq!(r.g.items.len(), n, "picked up a second knife");
}
