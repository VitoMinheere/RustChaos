//! `Ground`'s collision and lines of sight look through the walls by map square
//! (`wall_grid`), not at every wall: on every mission, they come out exactly as going through
//! every wall did, for slides and lines all over the map, against walls and props, near its
//! edges and off it. Skipped without an install.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_formats::facet::WallSegment;
use uc_world::ground::{Ground, STEP_HEIGHT};
use uc_world::{Level, LoadOptions};

/// `Ground::slide_odd` as it was, through every wall.
fn slide_every_wall(g: &Ground, from: Vec2, y: f32, delta: Vec2, radius: f32) -> (Vec2, bool) {
    let props = g.prop_walls.first().map_or(usize::MAX, |(_, r)| r.start)..g.walls.len();
    let mut odd = false;
    let mut p = from + delta;
    for _ in 0..2 {
        for (k, w) in g.blocking().enumerate() {
            if y < w.y0 - 64.0 || y >= w.y1 - STEP_HEIGHT {
                continue;
            }
            let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            let closest = a + ab * t;
            let d = p - closest;
            let dist = d.length();
            if dist < radius {
                let n = if dist > 1e-3 { d / dist } else { vec2(-ab.y, ab.x).normalize_or_zero() };
                p = closest + n * radius;
                odd |= props.contains(&k);
            }
        }
    }
    (p, odd)
}

/// `Ground::first_hit` as it was, through every wall.
fn first_hit_every_wall(g: &Ground, a: Vec3, b: Vec3) -> Option<f32> {
    let (p, r) = (vec2(a.x, a.z), vec2(b.x - a.x, b.z - a.z));
    g.blocking().filter_map(|w| {
        let (q, s) = (Vec2::from(w.a), Vec2::from(w.b) - Vec2::from(w.a));
        let denom = r.perp_dot(s);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (q - p).perp_dot(s) / denom;
        let u = (q - p).perp_dot(r) / denom;
        let y = a.y + (b.y - a.y) * t;
        ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) && y >= w.y0 && y <= w.y1).then_some(t)
    }).min_by(f32::total_cmp)
}

struct Rng(u32);

impl Rng {
    /// From `lo` to `hi`.
    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        lo + (hi - lo) * ((self.0 >> 8) as f32 / (1u32 << 24) as f32)
    }
}

/// Somewhere about a wall (in reach of it, at about its height) or, one time in eight,
/// anywhere on the map or a little off it.
fn about(r: &mut Rng, walls: &[WallSegment]) -> Vec3 {
    if walls.is_empty() || r.f(0.0, 1.0) < 0.125 {
        return vec3(r.f(-600.0, 128.0 * 256.0 + 600.0), r.f(-200.0, 3000.0), r.f(-600.0, 128.0 * 256.0 + 600.0));
    }
    let w = &walls[(r.f(0.0, walls.len() as f32) as usize).min(walls.len() - 1)];
    let on = Vec2::from(w.a).lerp(Vec2::from(w.b), r.f(0.0, 1.0));
    let off = vec2(r.f(-160.0, 160.0), r.f(-160.0, 160.0));
    vec3(on.x + off.x, r.f(w.y0 - 120.0, w.y1 + 40.0), on.y + off.y)
}

fn missions(game: &std::path::Path) -> Vec<String> {
    let dir = uc_formats::resolve(game, "levels").expect("no levels folder");
    let mut out: Vec<String> = std::fs::read_dir(dir).unwrap().flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".ucm").map(str::to_string)))
        .collect();
    out.sort();
    out
}

#[test]
fn the_grid_finds_what_every_wall_did() {
    let Some(game) = uc_formats::default_install() else { return };
    let (mut levels, mut slides, mut pushed, mut lines, mut cut) = (0, 0, 0, 0, 0);
    for m in missions(&game) {
        let Ok(level) = Level::load(&game, &m, &LoadOptions { fullbright: true, character: false, overrides: None }) else { continue };
        levels += 1;
        let g = &level.ground;
        let mut r = Rng(m.len() as u32 * 7919 + 1);
        for _ in 0..4000 {
            let at = about(&mut r, &g.walls);
            let delta = vec2(r.f(-40.0, 40.0), r.f(-40.0, 40.0));
            let radius = r.f(10.0, 150.0);
            let (from, y) = (vec2(at.x, at.z), at.y);
            let want = slide_every_wall(g, from, y, delta, radius);
            let got = g.slide_odd(from, y, delta, radius);
            assert_eq!((got.0.x.to_bits(), got.0.y.to_bits(), got.1), (want.0.x.to_bits(), want.0.y.to_bits(), want.1),
                "{m}: slide from {from} at {y} by {delta}, radius {radius}: {got:?}, every wall {want:?}");
            slides += 1;
            pushed += (want.0 != from + delta) as usize;

            let a = about(&mut r, &g.walls);
            // Mostly no further than someone looks (14 squares), now and then right across.
            let reach = if r.f(0.0, 1.0) < 0.9 { 14.0 * 256.0 } else { 128.0 * 256.0 };
            let b = a + vec3(r.f(-reach, reach), r.f(-300.0, 300.0), r.f(-reach, reach));
            let want = first_hit_every_wall(g, a, b);
            let got = g.first_hit(a, b);
            assert_eq!(got.map(f32::to_bits), want.map(f32::to_bits), "{m}: line from {a} to {b}: {got:?}, every wall {want:?}");
            assert_eq!(g.line_of_sight(a, b), want.is_none(), "{m}: line of sight from {a} to {b}");
            lines += 1;
            cut += want.is_some() as usize;
        }
    }
    println!("{levels} missions: {slides} slides ({pushed} pushed), {lines} lines ({cut} cut)");
    assert!(levels > 20, "only {levels} missions loaded");
    // Enough of both that came to something to mean anything.
    assert!(pushed > slides / 10 && cut > lines / 10, "{pushed} of {slides} slides pushed, {cut} of {lines} lines cut");
}
