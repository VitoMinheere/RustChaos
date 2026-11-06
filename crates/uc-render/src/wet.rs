//! Wet ground (#522): not in the original, which draws only its puddles.
//!
//! On a wet night the floor (not the walls or the roofs) is drawn by `fs_wet`: darker, with
//! the sky's sheen at a glancing look, a sheen where the dynamic lights reach, and a streak
//! of each bright light nearby: the lamps, the lit windows and the headlights. A streak is a
//! highlight stretched towards the camera and narrow across, as on real wet tarmac. It's
//! worked out in the shader from where the light is, so there's no reflection pass.
//!
//! The streaks are of the [`GLINTS`] lights nearest in front of the camera: half of them
//! from the lamps and headlights the caller gives, half from the lit windows and the other
//! walls and props that light themselves, which come from the level's geometry. A light near the edge of what's picked fades out, so none
//! pops in or out as the camera moves.

use glam::Vec3;
use std::collections::HashMap;
use uc_formats::txc::Texture;
use uc_world::{Layer, SelfLit, WorldGeometry};

/// How many lights' streaks the shader takes.
pub const GLINTS: usize = 32;

/// How wet the ground is and what shines in it.
#[derive(Debug, Clone, Default)]
pub struct Wet {
    /// From 0, dry, to 1, soaked.
    pub amount: f32,
    /// The lamps and headlights (square units).
    pub glints: Vec<Glint>,
}

/// A bright light whose streak shows on wet ground: where it is (square units), its colour
/// (0 to 1), and the way it shines (zero for every way).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glint {
    pub pos: Vec3,
    pub rgb: [f32; 3],
    pub facing: Vec3,
}

/// No further off than this (squares): the fog has them by then.
const REACH: f32 = 40.0;

/// Up to `n` of `from`, the nearest to the camera at `eye` looking along `look`: within 60
/// degrees of the way it looks, along the ground (a streak lies between the camera and its
/// light), and shining its way (one shining away has its streak behind itself). Each with
/// how much it shows: 1, fading to 0 over the last third of the way to the nearest left out.
pub fn pick(from: &[Glint], eye: Vec3, look: Vec3, n: usize) -> Vec<(Glint, f32)> {
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    let ahead = flat(look);
    // Looking straight down, every way is in view.
    let in_view = |at: Vec3| ahead == Vec3::ZERO || flat(at - eye).dot(ahead) > 0.5;
    let mut near: Vec<(f32, &Glint)> = from.iter()
        .map(|g| ((g.pos - eye).length(), g))
        .filter(|&(d, g)| d < REACH && in_view(g.pos) && g.facing.dot(eye - g.pos) >= 0.0)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    let cut = near.get(n).map_or(REACH, |&(d, _)| d);
    near.truncate(n);
    near.into_iter().map(|(d, g)| (*g, ((cut - d) / (cut / 3.0)).clamp(0.0, 1.0))).collect()
}

/// A page's glow, for one that lights itself or the lit windows over one: its colour where
/// it isn't clear, times how much of it isn't, times `boost`, no more than 1.
pub fn window_glow(t: &Texture, boost: f32) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    for px in t.rgba.chunks(4) {
        let a = px[3] as f32 / 255.0;
        for k in 0..3 {
            sum[k] += px[k] as f32 / 255.0 * a;
        }
    }
    let n = (t.rgba.len() / 4).max(1) as f32;
    sum.map(|c| (c / n * boost).min(1.0))
}

/// The walls and props in the level that light themselves or have lit windows: the middle
/// of each of their quads, facing its way if it's one-sided, with its page's glow. Not the
/// floors, which would shine in themselves, or the roofs, which shine at the sky. Those
/// within the same half square are one, at their middle, as bright as the brightest: a lamp
/// is many little faces.
pub fn windows(geo: &WorldGeometry, self_lit: &SelfLit, glow: &HashMap<u32, [f32; 3]>) -> Vec<Glint> {
    let mut cells: HashMap<[i32; 3], (Vec3, usize, Glint)> = HashMap::new();
    let mut order = Vec::new();
    for c in &geo.chunks {
        let batches = c.batches.iter().filter(|((l, _), _)| matches!(l, Layer::Walls | Layer::Props))
            .map(|(&(_, p), b)| (p, b))
            .chain(c.crinkled.iter().map(|(&p, b)| (p, b)));
        for (page, b) in batches {
            let shines = self_lit.overlay(page).or(self_lit.unlit.contains(&page).then_some(page));
            let Some(rgb) = shines.and_then(|w| glow.get(&w)) else { continue };
            if rgb.iter().all(|&c| c <= 0.0) {
                continue;
            }
            // Six indices at a time: a quad's two triangles, whose middle is that of their
            // six corners (two of them shared).
            for q in b.indices.as_chunks::<6>().0 {
                let corner = |i: u32| &b.vertices[i as usize];
                let pos = q.iter().map(|&i| Vec3::from(corner(i).pos)).sum::<Vec3>() / 6.0;
                let f = corner(q[0]).facing;
                let facing = Vec3::new(f[0] as f32, f[1] as f32, f[2] as f32).normalize_or_zero();
                let key = (pos * 2.0).floor().as_ivec3().to_array();
                let cell = cells.entry(key).or_insert_with(|| {
                    order.push(key);
                    (Vec3::ZERO, 0, Glint { pos, rgb: [0.0; 3], facing })
                });
                cell.0 += pos;
                cell.1 += 1;
                cell.2.rgb = std::array::from_fn(|k| cell.2.rgb[k].max(rgb[k]));
            }
        }
    }
    order.iter().map(|k| {
        let (sum, n, g) = cells[k];
        Glint { pos: sum / n as f32, ..g }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32, z: f32) -> Glint {
        Glint { pos: Vec3::new(x, 1.0, z), rgb: [1.0; 3], facing: Vec3::ZERO }
    }

    #[test]
    fn the_nearest_in_view_and_none_behind() {
        let away = Glint { facing: Vec3::Z, ..at(0.0, 3.0) };
        let aside = at(5.0, 1.0);
        let from = [at(0.0, 5.0), at(0.0, -2.0), at(0.0, 2.0), at(0.0, 60.0), away, aside, at(1.0, 4.0)];
        let picked = pick(&from, Vec3::new(0.0, 1.0, 0.0), Vec3::Z, 4);
        let zs: Vec<f32> = picked.iter().map(|(g, _)| g.pos.z).collect();
        assert_eq!(zs, [2.0, 4.0, 5.0], "behind the camera, off to the side, shining away or past the fog is left out");
        // With room for every one, none fades till the reach.
        assert!(picked.iter().all(|&(_, w)| w == 1.0));
    }

    #[test]
    fn those_near_the_cut_fade_out() {
        let from: Vec<Glint> = (1..=10).map(|z| at(0.0, z as f32)).collect();
        let picked = pick(&from, Vec3::new(0.0, 1.0, 0.0), Vec3::Z, 6);
        assert_eq!(picked.len(), 6);
        // The seventh, at 7, is left out: from 7 * 2 / 3 on they fade.
        let w: Vec<f32> = picked.iter().map(|&(_, w)| w).collect();
        assert_eq!(&w[..4], &[1.0; 4]);
        assert!(w[4] < 1.0 && w[5] < w[4] && w[5] > 0.0, "{w:?}");
    }

    #[test]
    fn a_windows_glow_is_its_lit_panes() {
        let clear = [0, 0, 0, 0];
        let pane = [255, 200, 100, 255];
        let rgba = [pane, clear, clear, clear].concat();
        let g = window_glow(&Texture { width: 2, height: 2, has_alpha: true, rgba }, 2.0);
        assert!((g[0] - 0.5).abs() < 1e-3 && (g[2] - 100.0 / 255.0 / 2.0).abs() < 1e-3, "{g:?}");
    }
}
