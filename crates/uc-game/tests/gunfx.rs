//! Gunfire's sparks and casings (#237): every shot throws sparks from the muzzle and a
//! casing from the hand, three from an AK-47, and the casings lie where they land. Skipped
//! without an install.

use uc_game::{Controls, Game, Item};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load() -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "fight2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    Some((level, g))
}

/// Gives the player gun item `kind` and fires it once at nothing, playing on until the shot
/// has gone off (the gun comes up first).
fn fire(level: &Level, g: &mut Game, kind: u8) {
    let pi = g.player.unwrap();
    let pos = g.people[pi].fig.pos;
    g.items.push(Item { way: 0, kind, pos, rounds: None });
    for _ in 0..5 {
        g.update(DT, &Controls::default(), level);
    }
    // Held until the first round goes: an automatic keeps firing while it's held.
    for _ in 0..40 {
        if !g.gunfx.brass.is_empty() {
            return;
        }
        g.update(DT, &Controls { punch: true, fire: true, ..Default::default() }, level);
    }
}

#[test]
fn a_shot_throws_sparks_and_a_casing() {
    let Some((level, mut g)) = load() else { return };
    fire(&level, &mut g, 2);
    assert!(!g.gunfx.sparks.is_empty(), "no sparks from the muzzle");
    assert_eq!(g.gunfx.brass.len(), 1, "a pistol throws one casing a shot");
    for _ in 0..90 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.gunfx.sparks.is_empty(), "the sparks should be gone");
    let pi = g.player.unwrap();
    let me = g.people[pi].fig.pos;
    let b = g.gunfx.brass[0];
    assert!(b.still, "the casing should have come to rest: {b:?}");
    let floor = level.ground.surface(&level.map, b.pos.x, b.pos.z, b.pos.y + 8.0);
    assert!((b.pos.y - floor).abs() < 1.0, "the casing should lie on the floor: {} against {floor}", b.pos.y);
    assert!((b.pos - me).length() < 256.0, "the casing should land near her: {} from her", (b.pos - me).length());
}

#[test]
fn an_ak47_throws_three_casings() {
    let Some((level, mut g)) = load() else { return };
    fire(&level, &mut g, 6);
    let pi = g.player.unwrap();
    assert_eq!(g.gunfx.brass.len(), 3, "weapon {:?} rounds {:?}", g.people[pi].weapon, g.people[pi].rounds());
}
