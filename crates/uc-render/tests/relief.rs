//! The relief (#474): walls, floors and roofs lit by their pages' normal maps, and people,
//! cars and props too (#521). A flat normal map draws as no normal map does; a bumpy one
//! changes the picture but not how bright it is overall. Skipped without an install or a GPU.

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
    let s = Settings { sky: level.sky, fog: None, layers, crinkles: false, lights: Vec::new(), shadows: Vec::new(), relief, bloom: 0.0, wet: None };
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

/// A person's page (Darci's, 600) on a wall-sized quad in front of the camera, drawn as people
/// are, among the moving things, with normal map `n` and the relief on: the picture.
fn person_shot(level: &Level, n: impl Fn(usize, usize) -> [u8; 3]) -> Option<Vec<u8>> {
    const PAGE: u32 = 600;
    let mut r = Renderer::headless(false).ok()?;
    let geo = level.build();
    let size = 16;
    let map = uc_formats::txc::Texture {
        width: size as u32,
        height: size as u32,
        has_alpha: false,
        rgba: (0..size * size).flat_map(|k| { let [r, g, b] = n(k % size, k / size); [r, g, b, 255] }).collect(),
    };
    r.upload_world_relief(&geo, &[PAGE], |p| level.texture(p), |p| (p == PAGE).then(|| map.clone()));
    let at = level.spawn.0 / uc_world::UNIT;
    let cam = Camera { pos: at + vec3(0.0, 1.0, -3.0), target: at + vec3(0.0, 1.0, 0.0), fov_y: 60f32.to_radians(), ortho: None };
    let mut b = uc_world::geometry::Batch::default();
    let q = |x: f32, y: f32| at + vec3(x, y, -2.0);
    b.quad([q(2.0, 3.0), q(-2.0, 3.0), q(-2.0, -1.0), q(2.0, -1.0)], [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], [[160, 160, 160, 255]; 4]);
    let dynamic = std::collections::BTreeMap::from([(PAGE, b)]);
    let relief = Some(Relief { strength: 3.0, ambient_slope: level.lighter()?.ambient_slope(), lamps: Vec::new() });
    let s = Settings { sky: level.sky, fog: None, layers: vec![Layer::Floor, Layer::Walls, Layer::Character], crinkles: false, lights: Vec::new(), shadows: Vec::new(), relief, bloom: 0.0, wet: None };
    Some(r.render_to_image((320, 240), &cam, &s, &dynamic, &Overlay::new()))
}

#[test]
fn people_and_cars_are_lit_by_their_normal_maps_too() {
    let Some(level) = load() else { return };
    let ridges = |x: usize, _| if (x / 2).is_multiple_of(2) { [200, 128, 230] } else { [56, 128, 230] };
    let (Some(flat), Some(on)) = (person_shot(&level, |_, _| [128, 128, 255]), person_shot(&level, ridges)) else { return };
    let changed = flat.chunks(4).zip(on.chunks(4)).filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 4)).count();
    assert!(changed > flat.len() / 4 / 10, "only {changed} pixels changed");
}
