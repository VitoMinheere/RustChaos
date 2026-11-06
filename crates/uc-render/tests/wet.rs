//! Wet roads (#522): the floor drawn wet is darker, a light above it streaks it, and the
//! walls are as they were. Skipped without an install or a GPU.

use glam::{vec3, Vec3};
use uc_render::{Camera, Glint, Overlay, Renderer, Settings, Wet};
use uc_world::{Layer, Level, LoadOptions};

fn load() -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, "police2", &LoadOptions { fullbright: false, character: false, overrides: None }).unwrap())
}

/// police2 from just above the ground near its start, looking along it, with the layers
/// `layers`, wet as `wet`: the picture.
fn shot(level: &Level, layers: &[Layer], wet: Option<Wet>) -> Option<Vec<u8>> {
    let mut r = Renderer::headless(false).ok()?;
    let geo = level.build();
    r.upload_world(&geo, &[], |p| level.texture(p));
    let cam = camera(level);
    let s = Settings { sky: level.sky, fog: None, layers: layers.to_vec(), crinkles: false, lights: Vec::new(), shadows: Vec::new(), relief: None, bloom: 0.0, wet };
    Some(r.render_to_image((320, 240), &cam, &s, &Default::default(), &Overlay::new()))
}

fn camera(level: &Level) -> Camera {
    let at = level.spawn.0 / uc_world::UNIT;
    Camera { pos: at + vec3(0.0, 1.0, -4.0), target: at + vec3(0.0, 0.0, 2.0), fov_y: 60f32.to_radians(), ortho: None }
}

fn mean(px: &[u8]) -> f64 {
    px.chunks(4).map(|p| p[..3].iter().map(|&v| v as f64).sum::<f64>()).sum::<f64>() / (px.len() / 4 * 3) as f64
}

#[test]
fn a_wet_floor_is_darker_and_streaked_under_a_light() {
    let Some(level) = load() else { return };
    let floor = [Layer::Floor];
    let soaked = |glints| Some(Wet { amount: 1.0, glints });
    let (Some(dry), Some(wet)) = (shot(&level, &floor, None), shot(&level, &floor, soaked(Vec::new()))) else { return };
    assert!(mean(&wet) < mean(&dry) * 0.9, "wet {} against dry {}", mean(&wet), mean(&dry));
    // A white light two squares up, ahead of the camera: its streak brightens the floor
    // between them.
    let cam = camera(&level);
    let ahead = cam.target + (cam.target - cam.pos).normalize() * 4.0;
    let lamp = Glint { pos: Vec3::new(ahead.x, cam.target.y + 2.0, ahead.z), rgb: [1.0; 3], facing: Vec3::ZERO };
    let Some(lit) = shot(&level, &floor, soaked(vec![lamp])) else { return };
    let brighter = lit.chunks(4).zip(wet.chunks(4)).filter(|(a, b)| a[0] as i32 > b[0] as i32 + 40).count();
    assert!(brighter > 200, "only {brighter} pixels streaked");
}

#[test]
fn the_walls_stay_dry() {
    let Some(level) = load() else { return };
    let walls = [Layer::Walls];
    let (Some(dry), Some(wet)) = (shot(&level, &walls, None), shot(&level, &walls, Some(Wet { amount: 1.0, glints: Vec::new() }))) else { return };
    assert_eq!(dry, wet);
}
