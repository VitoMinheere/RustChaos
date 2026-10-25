//! Reflections in the puddles (#268, `FIGURE_draw_reflection`, `SKY_draw_moon_reflection`):
//! someone standing on a puddle near the camera is drawn again upside down in it, mirrored in
//! its surface, and at night the moon is too. The renderer shows them only where the puddles
//! do. Skipped without an install.

use glam::Vec3;
use uc_game::{Controls, Game};
use uc_world::{reflect_page, Level, LoadOptions, MOON_PAGE, REFLECT, UNIT};

/// The Fallen, a rainy night, with Darci stood in the middle of the puddle nearest her.
fn in_a_puddle() -> Option<(Level, Game, f32)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "MIB", &LoadOptions { fullbright: false, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    g.skip_cutscene();
    g.update(1.0 / 30.0, &Controls::default(), &level);
    let me = g.player.unwrap();
    let at = g.people[me].fig.pos;
    let pd = g.puddles.puddles.iter()
        .min_by(|a, b| {
            let d = |p: &uc_game::puddle::Puddle| ((p.a.0 + p.b.0) / 2.0 - at.x).hypot((p.a.1 + p.b.1) / 2.0 - at.z);
            d(a).total_cmp(&d(b))
        })
        .expect("a rainy night has puddles");
    let (mid, y) = (Vec3::new((pd.a.0 + pd.b.0) / 2.0, pd.y, (pd.a.1 + pd.b.1) / 2.0), pd.y);
    g.people[me].fig.pos = mid;
    Some((level, g, y))
}

/// Every vertex of the reflections of people, world units.
fn reflected(g: &Game, level: &Level, eye: Vec3) -> Vec<Vec3> {
    g.batches(level, eye).iter()
        .filter(|(&p, _)| reflect_page(p).is_some_and(|own| own != MOON_PAGE))
        .flat_map(|(_, b)| b.vertices.iter().map(|v| Vec3::from(v.pos) * UNIT))
        .collect()
}

#[test]
fn someone_in_a_puddle_is_upside_down_in_it() {
    let Some((level, g, y)) = in_a_puddle() else { return };
    let me = g.player.unwrap();
    let eye = g.people[me].fig.pos + Vec3::new(0.0, 300.0, -600.0);
    let all = reflected(&g, &level, eye);
    assert!(!all.is_empty(), "no reflection of Darci");
    // All of her under the puddle's surface, head down: as far below it as she stands above.
    assert!(all.iter().all(|v| v.y <= y + 1.0), "a reflection above the puddle");
    let lowest = all.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    assert!(lowest < y - 120.0, "only {} deep", y - lowest);
    // Too far from the camera (0x600, across plus along), there's none.
    assert!(reflected(&g, &level, eye + Vec3::new(0x600 as f32, 0.0, 0.0)).is_empty());
}

#[test]
fn off_the_puddles_nothing_is_reflected() {
    let Some((level, mut g, _)) = in_a_puddle() else { return };
    let me = g.player.unwrap();
    // Somewhere a square from every puddle.
    let clear = |p: Vec3| g.puddles.puddles.iter().all(|pd| {
        p.x < pd.a.0.min(pd.b.0) - 512.0 || p.x > pd.a.0.max(pd.b.0) + 512.0 || p.z < pd.a.1.min(pd.b.1) - 512.0 || p.z > pd.a.1.max(pd.b.1) + 512.0
    });
    let start = g.people[me].fig.pos;
    let spot = (1..40).flat_map(|k| [Vec3::new(k as f32 * 128.0, 0.0, 0.0), Vec3::new(0.0, 0.0, k as f32 * 128.0)])
        .map(|d| start + d)
        .find(|&p| clear(p))
        .expect("somewhere dry");
    g.people[me].fig.pos = spot;
    let eye = spot + Vec3::new(0.0, 300.0, -600.0);
    assert!(reflected(&g, &level, eye).is_empty(), "a reflection with no puddle under her");
}

#[test]
fn the_moon_is_in_the_puddles_at_night() {
    let Some((level, g, _)) = in_a_puddle() else { return };
    let me = g.player.unwrap();
    let eye = g.people[me].fig.pos + Vec3::new(0.0, 300.0, -600.0);
    let all = g.batches(&level, eye);
    let moon = all.get(&MOON_PAGE).expect("the moon at night");
    let under = all.get(&(REFLECT + MOON_PAGE)).expect("the moon in the puddles");
    // The moon is up in the sky; its reflection as far under the camera's half height.
    let mid = |b: &uc_world::Batch| b.vertices.iter().map(|v| Vec3::from(v.pos) * UNIT).sum::<Vec3>() / b.vertices.len() as f32;
    let (up, down) = (mid(moon), mid(under));
    assert!(up.y > eye.y && down.y < 0.0, "moon at {up}, reflection at {down}");
    assert!(((up.y - eye.y * 0.5) + (down.y - eye.y * 0.5)).abs() < 1.0, "not mirrored: {up} and {down}");
    // Squashed to half its height.
    let span = |b: &uc_world::Batch| {
        let ys = b.vertices.iter().map(|v| v.pos[1] * UNIT);
        ys.clone().fold(f32::MIN, f32::max) - ys.fold(f32::MAX, f32::min)
    };
    assert!((span(under) / span(moon) - 0.5).abs() < 0.05, "{} against {}", span(under), span(moon));
}

#[test]
fn a_splash_sets_the_puddle_rippling_and_her_reflection_wobbling() {
    let Some((level, mut g, _)) = in_a_puddle() else { return };
    let me = g.player.unwrap();
    let at = g.people[me].fig.pos;
    let eye = at + Vec3::new(0.0, 300.0, -600.0);
    let k = g.puddles.puddles.iter().position(|p| p.contains(at.x, at.z)).expect("her puddle");
    g.puddles.puddles[k].ripple = (0, 0);
    let calm = reflected(&g, &level, eye);
    g.puddles.ripple(at);
    assert_eq!(g.puddles.puddles[k].ripple, (40, 45), "no ripple");
    let rippled = reflected(&g, &level, eye);
    assert_eq!(calm.len(), rippled.len());
    let moved = calm.iter().zip(&rippled).map(|(a, b)| (*a - *b).length()).fold(0.0, f32::max);
    assert!(moved > 1.0 && moved < 40.0, "the reflection moved {moved}");
    // It dies down a tick at a time: gone in under three seconds.
    for _ in 0..90 {
        g.puddles.update(1.0 / 30.0, false, None);
    }
    assert_eq!(g.puddles.puddles[k].ripple, (0, 0));
}
