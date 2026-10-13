//! `make_person_pee`: Semtex sets a thug peeing from the start, and RTA asks for it of a
//! civilian whose point comes after the pee point, so it never happens, as in the original.
//! Skipped without an install.

use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    Some((level, g))
}

fn play(level: &Level, g: &mut Game, secs: f32) {
    for _ in 0..(secs / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
    }
}

#[test]
fn semtexs_thug_pees() {
    let Some((level, mut g)) = load("Semtex") else { return };
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    play(&level, &mut g, 2.0);
    let i = g.people.iter().position(|p| p.way == 425).expect("the thug point 425 makes");
    assert!(g.people[i].peeing, "point 426 should have set him going");
    let near: Vec<_> = g.drops.drops.iter().filter(|d| (d.pos - g.people[i].fig.pos).length() < 256.0).collect();
    assert!(!near.is_empty(), "no stream in front of him");
    // They fall, and land: a couple of seconds of it doesn't pile up.
    assert!(near.iter().all(|d| d.pos.y < g.people[i].fig.hips().y), "a drop went up");
    assert!(g.drops.drops.len() < 2 * 20 * 2, "drops aren't landing: {}", g.drops.drops.len());
}

#[test]
fn rtas_pee_point_comes_too_early() {
    let Some((level, mut g)) = load("police1") else { return };
    play(&level, &mut g, 1.0);
    let i = g.people.iter().position(|p| p.way == 238).expect("the civilian point 238 makes");
    assert!(!g.people[i].peeing, "point 178 fires before he exists, so he never starts");
    assert!(g.drops.drops.is_empty());
}
