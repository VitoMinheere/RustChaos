//! Bloom (#523): what lights itself glows past its edges; with it off, nothing changes.
//! Skipped without an install or a GPU.

use glam::vec3;
use std::collections::BTreeMap;
use uc_render::{Camera, Overlay, Renderer, Settings};
use uc_world::{Layer, Level, LoadOptions, GLOW};

/// police3 (at night) with a lamp's glow (`GLOW`) in the middle of the view, and the bloom
/// at `bloom`: the picture, 320x240.
fn shot(level: &Level, bloom: f32) -> Option<Vec<u8>> {
    let mut r = Renderer::headless(false).ok()?;
    let geo = level.build();
    r.upload_world(&geo, &[], |p| level.texture(p));
    let at = level.spawn.0 / uc_world::UNIT;
    let cam = Camera { pos: at + vec3(0.0, 1.0, -3.0), target: at + vec3(0.0, 1.0, 0.0), fov_y: 60f32.to_radians(), ortho: None };
    let mut b = uc_world::Batch::default();
    let q = |x: f32, y: f32| at + vec3(x, 1.0 + y, -2.0);
    b.quad([q(0.1, 0.1), q(-0.1, 0.1), q(-0.1, -0.1), q(0.1, -0.1)], [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], [[255, 255, 200, 255]; 4]);
    let glows = BTreeMap::from([(GLOW, b)]);
    let s = Settings {
        sky: level.sky, fog: None, layers: vec![Layer::Floor, Layer::Walls, Layer::Roofs, Layer::Character],
        crinkles: false, lights: Vec::new(), shadows: Vec::new(), relief: None, bloom, wet: None,
    };
    Some(r.render_to_image((320, 240), &cam, &s, &glows, &Overlay::new()))
}

fn px(img: &[u8], x: usize, y: usize) -> u32 {
    let k = (y * 320 + x) * 4;
    img[k..k + 3].iter().map(|&v| v as u32).sum()
}

#[test]
fn a_glow_blooms_past_its_edges() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police3", &LoadOptions { fullbright: false, character: false, overrides: None }).unwrap();
    let (Some(off), Some(on)) = (shot(&level, 0.0), shot(&level, 2.4)) else { return };
    // Never darker anywhere.
    assert!(off.chunks(4).zip(on.chunks(4)).all(|(a, b)| (0..3).all(|k| b[k] >= a[k])), "the bloom darkened something");
    // Brighter round the glow, beyond the quad (a tenth of a square across, about 14 pixels
    // at 2 squares off): 20 pixels to the side.
    let (cx, cy) = (160, 120);
    assert!(px(&on, cx + 20, cy) > px(&off, cx + 20, cy) + 6, "no halo: {} -> {}", px(&off, cx + 20, cy), px(&on, cx + 20, cy));
    // Off draws as before there was a bloom: a second render the same.
    assert_eq!(shot(&level, 0.0).unwrap(), off);
}
