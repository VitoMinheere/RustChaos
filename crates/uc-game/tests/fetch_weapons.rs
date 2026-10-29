//! Empty-handed, someone who'd fight goes for a weapon dropped near them
//! (`PCOM_is_there_an_item_i_should_get`, pcom.cpp:4969): within 0x300, in reach, looked for
//! every 128 ticks. What a mission put down is left for the player. Skipped without an install.

use uc_game::{Controls, Game, Item, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Combat Silver with one of its fighters, empty-handed and far from Darci, and a pistol
/// lying a couple of squares from him, put there by the mission (`way`) or dropped (0).
fn setup(way: usize) -> Option<(Level, Game, usize)> {
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
    let at = g.people[ei].fig.pos;
    g.people[pi].fig.pos = at + glam::vec3(0.0, 0.0, 40.0 * 256.0);
    let e = &mut g.people[ei];
    (e.weapon, e.knife, e.bat, e.guns) = (Weapon::None, false, false, 0);
    let eye = glam::vec3(0.0, 64.0, 0.0);
    let spot = (0..16)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            let p = at + glam::vec3(a.sin(), 0.0, a.cos()) * 500.0;
            glam::vec3(p.x, level.ground.surface(&level.map, p.x, p.z, at.y + 64.0), p.z)
        })
        .find(|&p| (p.y - at.y).abs() < 32.0 && level.ground.line_of_sight(at + eye, p + eye))
        .expect("no clear spot");
    g.items.push(Item { way, kind: Weapon::Pistol.item().unwrap(), pos: spot, rounds: Some(5), born: 0.0 });
    Some((level, g, ei))
}

#[test]
fn he_picks_up_a_dropped_pistol() {
    let Some((level, mut g, ei)) = setup(0) else { return };
    for _ in 0..(15.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        if g.people[ei].has(Weapon::Pistol) {
            return;
        }
    }
    panic!("never picked it up: at {:?}", g.people[ei].fig.pos);
}

#[test]
fn what_the_mission_put_down_is_left_alone() {
    let Some((level, mut g, ei)) = setup(999) else { return };
    for _ in 0..(15.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(!g.people[ei].has(Weapon::Pistol));
}
