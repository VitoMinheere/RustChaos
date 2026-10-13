//! Ground mist: the sheets a `create_mist` point lays down. Skipped without an install.

use glam::Vec3;
use uc_game::Game;
use uc_world::{Level, LoadOptions};

fn load(mission: &str) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap())
}

/// Rat Catch lays one sheet, with no trigger, so it is there from the start.
#[test]
fn a_mission_lays_its_mist() {
    let Some(level) = load("Gangorder2") else { return };
    let g = Game::new(&level, true);
    assert_eq!(g.mists.sheets.len(), 1, "Rat Catch has one create_mist point");

    // Centred on the point, 0x800 either side of it (`MIST_SIZE`), at a height of its own
    // rather than on the ground.
    let point = level.mission.event_points.iter().find(|e| e.kind == 47).expect("the mist point");
    let at = g.mists.sheets[0].pos;
    assert_eq!((at.x, at.z), (point.x as f32, point.z as f32), "it sits on its point");
    assert!(at.y > 0.0 && at.y < 256.0, "and floats low over the street, not on it: {}", at.y);

    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let mut quads = 0;
    g.mists.quads(0.0, |p, _| {
        quads += 1;
        for c in p {
            lo = lo.min(c);
            hi = hi.max(c);
        }
    });
    assert_eq!(quads, 16 * 16, "17 points across makes 16 quads");
    // In square units: 0x800 either side is 8 squares, so 16 across.
    assert!((hi.x - lo.x - 16.0).abs() < 0.01, "the sheet is 16 squares across, got {}", hi.x - lo.x);
    assert!((hi.z - lo.z - 16.0).abs() < 0.01, "and 16 deep, got {}", hi.z - lo.z);
}

/// Gatecrasher lays three. Each is at a different height from the one before, so two laid over
/// each other don't fight. `EWAY_DO_CREATE_MIST` only ever uses two heights, mind: it starts at
/// 84 and flips by 11 on the low bit, so it goes 84, 95, 84, 95 and back again.
#[test]
fn each_sheet_sits_clear_of_the_one_before() {
    let Some(level) = load("bball2") else { return };
    let g = Game::new(&level, true);
    assert_eq!(g.mists.sheets.len(), 3, "Gatecrasher has three create_mist points");
    let heights: Vec<f32> = g.mists.sheets.iter().map(|m| m.pos.y).collect();
    assert_eq!(heights, vec![84.0, 95.0, 84.0], "the original's two heights, in turn");
    for pair in heights.windows(2) {
        assert!((pair[0] - pair[1]).abs() > 1.0, "two in a row share a height: {heights:?}");
    }
}

/// The texture crawls rather than sliding: the sheet's edge is pinned so neighbouring patches
/// keep their seam, while the points inside it move.
#[test]
fn the_sheet_shimmers_without_its_edge_moving() {
    let Some(level) = load("Gangorder2") else { return };
    let g = Game::new(&level, true);
    let grab = |secs: f32| {
        let mut uvs = Vec::new();
        g.mists.quads(secs, |_, uv| uvs.push(uv));
        uvs
    };
    let (a, b) = (grab(0.0), grab(2.0));
    assert_eq!(a.len(), b.len());
    // Corner by corner, not quad by quad: a quad on the sheet's edge still has three corners
    // inside it, so every quad has something moving in it.
    let corners = a.iter().flatten().zip(b.iter().flatten());
    let (moved, still) = corners.fold((0, 0), |(m, s), (x, y)| if x == y { (m, s + 1) } else { (m + 1, s) });
    assert!(moved > 0, "nothing moved at all");
    assert!(still > 0, "the sheet's edge should stay put, but every point moved");
}
