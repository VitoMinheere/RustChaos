//! Dodging bullets (#254, Person.cpp:6340): someone skilled enough (7 and over) shot at may
//! flip out of the way, unhurt; the unskilled never do. One of RTA's crowd made a gang member
//! in front of Darci, her pistol on him. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, AI_GANG, BENT_FIGHT_BACK};
use uc_game::{Controls, Game, State, Weapon};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Darci shooting for `secs` at one of RTA's crowd made a gang member of `skill`: whether he
/// ever flipped.
fn flips(skill: u8, secs: f32) -> Option<bool> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let p = g.player.unwrap();
    g.people[p].give(Weapon::Pistol);
    g.people[p].weapon = Weapon::Pistol;
    let i = g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("crowd");
    let me = g.people[p].fig.pos;
    let f = g.people[p].facing_dir();
    let at = me + vec3(f.x, 0.0, f.y) * 300.0;
    let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
    let q = &mut g.people[i];
    (q.ai, q.kind, q.mv, q.fake, q.skill, q.max_health) = (AI_GANG, ptype::THUG_RED, 1, false, skill, 10_000);
    q.health = 10_000;
    // One who fights back, so doesn't run from her gun (#304).
    q.bent = BENT_FIGHT_BACK;
    q.fig.pos = vec3(at.x, y, at.z);
    let mut flipped = false;
    for _ in 0..(secs / DT) as usize {
        g.target = Some(i);
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &Controls { punch: true, ..Default::default() }, &level);
        flipped |= matches!(g.people[i].state, State::Flip { .. });
    }
    Some(flipped)
}

#[test]
fn the_skilled_flip_out_of_the_way() {
    let Some(f) = flips(15, 20.0) else { return };
    assert!(f, "never dodged a shot");
}

#[test]
fn the_unskilled_dont() {
    let Some(f) = flips(5, 10.0) else { return };
    assert!(!f, "dodged with skill 5");
}
