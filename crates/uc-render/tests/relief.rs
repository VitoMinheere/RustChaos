//! The relief (#474): walls, floors and roofs lit by their pages' normal maps. A flat normal
//! map draws as no normal map does; a bumpy one changes the picture but not how bright it is
//! overall. Skipped without an install or a GPU.

use glam::vec3;
use uc_render::{Camera, Overlay, Relief, Renderer, Settings};
use uc_world::{Layer, Level, LoadOptions};

/// police1 from above its start, with every page given the normal map `n` (RGB, 16x16),
/// and the relief on or off: the picture.
fn shot(level: &Level, n: impl Fn(usize, usize) -> [u8; 3], relief: bool) -> Option<Vec<u8>> {
    let mut r = Renderer::headless(false).ok()?;
    let geo = level.build();
    let size = 16;
    let map = uc_formats::txc::Texture {
        width: size as u32,
        height: size as u32,
        has_alpha: false,
        rgba: (0..size * size).flat_map(|k| { let [r, g, b] = n(k % size, k / size); [r, g, b, 255] }).collect(),
    };
    r.upload_world_relief(&geo, &[], |p| level.texture(p), |_| Some(map.clone()));
    let at = level.spawn.0 / uc_world::UNIT;
    let cam = Camera { pos: at + vec3(-6.0, 4.0, -6.0), target: at, fov_y: 60f32.to_radians(), ortho: None };
    let lighter = level.lighter()?;
    let relief = relief.then(|| Relief {
        strength: 3.0,
        ambient_slope: lighter.ambient_slope(),
        lamps: lighter.sources().into_iter()
            .map(|(p, rad, rgb)| uc_render::Light { pos: glam::Vec3::from(p) / uc_world::UNIT, radius: rad / uc_world::UNIT, rgb })
            .collect(),
    });
    let layers = vec![Layer::Floor, Layer::Walls, Layer::Roofs];
    let s = Settings { sky: level.sky, fog: None, layers, crinkles: false, lights: Vec::new(), shadows: Vec::new(), relief };
    Some(r.render_to_image((320, 240), &cam, &s, &Default::default(), &Overlay::new()))
}

fn load() -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, "police1", &LoadOptions { fullbright: false, character: false, overrides: None }).unwrap())
}

fn mean(px: &[u8]) -> f64 {
    px.chunks(4).map(|p| p[..3].iter().map(|&v| v as f64).sum::<f64>()).sum::<f64>() / (px.len() / 4 * 3) as f64
}

#[test]
fn a_flat_normal_map_changes_nothing() {
    let Some(level) = load() else { return };
    let flat = |_, _| [128, 128, 255];
    let (Some(off), Some(on)) = (shot(&level, flat, false), shot(&level, flat, true)) else { return };
    let worst = off.iter().zip(&on).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    // (128 is a hair off 0 in 0..255, and the derivatives' frame isn't exact.)
    assert!(worst <= 3, "a flat normal map changed a pixel by {worst}");
}

#[test]
fn a_bumpy_normal_map_lights_the_relief() {
    let Some(level) = load() else { return };
    // Ridges across the page: normals leaning one way and then the other.
    let ridges = |x: usize, _| if (x / 2).is_multiple_of(2) { [200, 128, 230] } else { [56, 128, 230] };
    let (Some(off), Some(on)) = (shot(&level, |_, _| [128, 128, 255], true), shot(&level, ridges, true)) else { return };
    let changed = off.chunks(4).zip(on.chunks(4)).filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 4)).count();
    assert!(changed > off.len() / 4 / 20, "only {changed} pixels changed");
    // Lighter on one side of a ridge, darker on the other: no brighter or darker overall.
    let (a, b) = (mean(&off), mean(&on));
    assert!((a - b).abs() < a * 0.05, "the relief changed the brightness: {a:.1} -> {b:.1}");
}
