//! Puddles and drips (#239, fallen/Source/puddle.cpp, drip.cpp): a rainy night's city has
//! puddles along its kerbs and walls, the rain drips into the ones near the player, and a
//! foot coming down in one drips too. A dry level has none. Skipped without an install.

use uc_formats::iam::flags;
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// The level with its lighting, which says whether it's night (and so raining).
fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: false, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    Some((level, g))
}

#[test]
fn a_rainy_night_has_puddles_on_flat_open_ground() {
    let Some((level, g)) = load("MIB") else { return };
    assert!(Game::raining(&level), "The Fallen is a night mission");
    let n = g.puddles.puddles.len();
    assert!(n > 50, "only {n} puddles");
    for p in &g.puddles.puddles {
        let mid = ((p.a.0 + p.b.0) / 2.0, (p.a.1 + p.b.1) / 2.0);
        let sq = level.map.square((mid.0 / 256.0) as usize, (mid.1 / 256.0) as usize);
        assert!(!sq.has(flags::WATER), "a puddle on water at {mid:?}");
        let ground = level.map.ground_height(mid.0, mid.1);
        assert!((p.y - ground).abs() <= 24.0, "a puddle at {} over ground at {ground}", p.y);
        assert!(g.puddles.in_puddle(mid.0, mid.1), "its own middle isn't in it");
    }
}

#[test]
fn the_rain_drips_into_the_puddles_near_her() {
    let Some((level, mut g)) = load("MIB") else { return };
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    assert!(!g.puddles.drips.is_empty(), "no drips");
    let me = g.people[g.player.unwrap()].fig.pos;
    for d in &g.puddles.drips {
        assert!(g.puddles.in_puddle(d.pos.x, d.pos.z), "a drip outside the puddles at {}", d.pos);
        // The puddle's middle is within 24 squares; the drip anywhere in it.
        let reach = 24.0 * 256.0 + 512.0;
        assert!((d.pos.x - me.x).abs() <= reach && (d.pos.z - me.z).abs() <= reach, "a drip far off at {}", d.pos);
    }
}

#[test]
fn a_foot_in_a_puddle_drips() {
    let Some((_, mut g)) = load("MIB") else { return };
    let p = g.puddles.puddles[0];
    let mid = glam::vec3((p.a.0 + p.b.0) / 2.0, p.y, (p.a.1 + p.b.1) / 2.0);
    let before = g.puddles.drips.len();
    // Several goes: the drip lands within 15 of the foot, which may be off the puddle's edge.
    for _ in 0..8 {
        g.puddles.splash(mid);
    }
    assert!(g.puddles.drips.len() > before, "stepping in a puddle should drip");
}

#[test]
fn a_dry_level_has_none() {
    let Some((level, mut g)) = load("police1") else { return };
    assert!(!Game::raining(&level));
    assert!(g.puddles.puddles.is_empty());
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.puddles.drips.is_empty());
}
