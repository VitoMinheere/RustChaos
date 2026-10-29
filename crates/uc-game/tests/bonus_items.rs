//! Killed, someone skilled leaves a health pack or some ammo behind (`generate_bonus_item`,
//! Person.cpp:16763); the unskilled leave nothing. Skipped without an install.

use uc_game::person::ko;
use uc_game::{Controls, Game, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Combat Silver with Darci and one of its fighters, of skill `skill`, killed.
fn killed(skill: u8) -> Option<(Level, Game, usize, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "fight1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let pi = g.player.unwrap();
    let ei = (0..g.people.len()).find(|&i| i != pi && !g.people[i].removed && g.people[i].hostile())?;
    for (i, p) in g.people.iter_mut().enumerate() {
        if i != pi && i != ei {
            p.removed = true;
        }
    }
    let before = g.items.len();
    let e = &mut g.people[ei];
    e.skill = skill;
    e.fake = false;
    let from = e.fig.pos + glam::vec3(100.0, 0.0, 0.0);
    e.take_hit(from, 10_000, ko::KNOCKDOWN, false, false, Weapon::None);
    assert!(e.knocked_out);
    for _ in 0..150 {
        g.update(DT, &Controls::default(), &level);
    }
    Some((level, g, ei, before))
}

#[test]
fn the_skilled_leave_something_behind() {
    let Some((_level, g, ei, before)) = killed(12) else { return };
    assert_eq!(g.items.len(), before + 1, "one item");
    let it = g.items.last().unwrap();
    assert!(matches!(it.kind, 3 | 10 | 11 | 12), "a health pack or ammo: {}", it.kind);
    let at = g.people[ei].fig.hips();
    assert!(glam::vec2(it.pos.x - at.x, it.pos.z - at.z).length() < 64.0, "by the body");
}

#[test]
fn the_unskilled_leave_nothing() {
    let Some((_level, g, _, before)) = killed(3) else { return };
    assert_eq!(g.items.len(), before);
}
