//! What she picks up and when (#397, Special.cpp:1359-1407): a health pack only once she's
//! down 100 or more, anything within 160 summing the three axes, and nothing in its first
//! second. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Item};

mod common;

/// Drops item `kind` `off` from her, made `age` seconds ago, and gives her half a second:
/// whether she took it.
fn takes(kind: u8, off: glam::Vec3, age: f32, hurt: i32) -> Option<bool> {
    let mut r = common::start("fight2")?;
    r.heal = false;
    r.play(2.0, Controls::default());
    let pi = r.player();
    let p = &mut r.g.people[pi];
    p.health = p.max_health - hurt;
    let pos = p.fig.pos + off;
    let born = r.g.time - age;
    r.g.items.push(Item { way: 0, kind, pos, rounds: None, born });
    let n = r.g.items.len();
    r.play(0.5, Controls::default());
    Some(r.g.items.len() < n)
}

#[test]
fn health_only_when_down_a_hundred() {
    let Some(scratched) = takes(3, vec3(0.0, 0.0, 0.0), 5.0, 50) else { return };
    assert!(!scratched, "a health pack went on a scratch");
    assert_eq!(takes(3, vec3(0.0, 0.0, 0.0), 5.0, 120), Some(true));
}

#[test]
fn within_a_hundred_and_sixty() {
    let Some(near) = takes(2, vec3(120.0, 0.0, 0.0), 5.0, 0) else { return };
    assert!(near, "a pistol 120 off wasn't picked up");
    assert_eq!(takes(2, vec3(130.0, 0.0, 60.0), 5.0, 0), Some(false));
}

#[test]
fn not_in_its_first_second() {
    let Some(fresh) = takes(2, vec3(0.0, 0.0, 0.0), 0.0, 0) else { return };
    assert!(!fresh, "a pistol was picked up the moment it landed");
}
