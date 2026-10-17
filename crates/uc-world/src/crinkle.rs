//! Crinkles (#113): relief that stands in for a flat wall quad close to the camera
//! (`CRINKLE_do`, DDEngine/Source/Crinkle.cpp:493). The shipped `config.ini` turns them on
//! (`detail_crinkles`, aeng.cpp:16407).
//!
//! Walls on a page with a crinkle are built flat into each chunk's `crinkled` batches. Each
//! frame, a chunk near the camera has those drawn by [`Relief::draw`] instead: raised for
//! the quads close enough, flat for the rest. The relief grows out of the wall as the camera
//! comes closer, so nothing pops where flat and raised meet.
//!
//! Differences from the original:
//! - How close a quad is: its middle's distance from the camera, not corner 0's depth.
//! - The lighting of a raised face is changed in world space. The original compares a
//!   view-space normal with the world's light direction, so its relief lights up and darkens
//!   as the camera turns.

use crate::geometry::Batch;
use glam::Vec3;
use std::collections::BTreeMap;
use uc_formats::lgt::AMB_NORM;
use uc_formats::txc::Crinkle;

/// The original's draw distance in squares (`CurDrawDistance`, aeng.cpp:510). Its view space
/// is scaled so that depth 1 is this far away.
const VIEW_DIST: f32 = 22.0;
/// How far a point with an `out` of 1 stands out of the wall, in squares: 0.05 in view space
/// (Crinkle.cpp:568).
pub const DEPTH: f32 = 0.05 * VIEW_DIST;
/// Fully raised closer than this, in squares (view-space depth 0.4, facet.cpp:1763).
const FULL: f32 = 0.4 * VIEW_DIST;
/// Flat from this far on, in squares (view-space depth 0.5, facet.cpp:1762).
pub const FLAT: f32 = 0.5 * VIEW_DIST;

/// A wall quad that has a crinkle, as it's drawn flat.
#[derive(Debug, Clone, Copy)]
pub struct ReliefQuad {
    /// In square units, in the engine's corner order (see `uc_formats::facet::WallQuad`).
    pub corners: [Vec3; 4],
    pub uv: [[f32; 2]; 4],
    pub colour: [[u8; 4]; 4],
    pub page: u32,
    /// Index into [`Relief::crinkles`].
    pub crinkle: u32,
}

/// One chunk's quads with a crinkle, and the chunk's bounds.
#[derive(Debug, Clone)]
pub struct ReliefChunk {
    pub min: Vec3,
    pub max: Vec3,
    pub quads: Vec<ReliefQuad>,
}

/// The walls of a level that have crinkles, by chunk, to draw near the camera each frame.
#[derive(Debug, Clone, Default)]
pub struct Relief {
    pub crinkles: Vec<Crinkle>,
    pub chunks: Vec<ReliefChunk>,
}

/// How far a quad `d` squares away is raised, 0 (flat) to 1 (facet.cpp:1726).
pub fn extrude(d: f32) -> f32 {
    ((d - FLAT) / (FULL - FLAT)).clamp(0.0, 1.0)
}

/// Whether a chunk with these bounds has anything to raise for a camera at `cam`. The
/// renderer leaves out such a chunk's `crinkled` batches, for [`Relief::draw`] to draw.
pub fn near(cam: Vec3, min: Vec3, max: Vec3) -> bool {
    cam.clamp(min, max).distance(cam) < FLAT
}

impl Relief {
    /// Draws the quads with a crinkle in the chunks [`near`] the camera into `out`, by page:
    /// raised when close enough, flat otherwise. Returns how many were raised.
    pub fn draw(&self, cam: Vec3, out: &mut BTreeMap<u32, Batch>) -> usize {
        let mut raised = 0;
        for ch in self.chunks.iter().filter(|ch| near(cam, ch.min, ch.max)) {
            for q in &ch.quads {
                let middle = q.corners.iter().copied().sum::<Vec3>() / 4.0;
                let e = extrude(middle.distance(cam));
                let b = out.entry(q.page).or_default();
                match self.crinkles.get(q.crinkle as usize) {
                    Some(c) if e > 0.0 => {
                        raise(c, q, e, b);
                        raised += 1;
                    }
                    _ => b.quad(q.corners, q.uv, q.colour),
                }
            }
        }
        raised
    }
}

/// `CRINKLE_do` (Crinkle.cpp:493): crinkle `c` across quad `q`, raised by `e` (0 to 1).
pub fn raise(c: &Crinkle, q: &ReliefQuad, e: f32, b: &mut Batch) {
    // The wall's normal, the same way round whether or not the texture is mirrored.
    let n = (q.corners[1] - q.corners[0]).cross(q.corners[2] - q.corners[0]).normalize_or_zero() * DEPTH * e;
    // A texture mirrored left to right mirrors its crinkle too: corners 0 and 1 swap, and
    // 2 and 3 (Crinkle.cpp:519).
    let flip = q.uv[1][0] < q.uv[0][0];
    let k = if flip { [1, 0, 3, 2] } else { [0, 1, 2, 3] };
    let (p, uv, col) = (k.map(|i| q.corners[i]), k.map(|i| q.uv[i]), k.map(|i| q.colour[i]));
    let (a, bv) = (p[1] - p[0], p[2] - p[0]);
    let lerp_uv = |u: f32, v: f32| [0, 1].map(|j| uv[0][j] + u * (uv[1][j] - uv[0][j]) + v * (uv[2][j] - uv[0][j]));
    let points: Vec<(Vec3, [f32; 2], [u8; 4])> = c
        .points
        .iter()
        .map(|cp| {
            let pos = p[0] + a * cp.u + bv * cp.v + n * cp.out;
            // Each corner's colour, weighted out of 128 (Crinkle.cpp:639).
            let rgb = [0, 1, 2].map(|ch| {
                let sum: u32 = (0..4).map(|i| col[i][ch] as u32 * cp.w[i] as u32).sum();
                (sum >> 7).min(255) as u8
            });
            (pos, lerp_uv(cp.u, cp.v), [rgb[0], rgb[1], rgb[2], 255])
        })
        .collect();
    let light = Vec3::from(AMB_NORM).normalize();
    for f in &c.faces {
        let t = if flip { [f[0], f[2], f[1]] } else { *f }.map(|i| points[i as usize]);
        let mut colour = t.map(|v| v.2);
        // A face standing out of the wall is lit by which way it faces (Crinkle.cpp:696).
        if f.iter().map(|&i| c.points[i as usize].out.abs()).sum::<f32>() > 0.001 {
            let fnorm = (t[1].0 - t[0].0).cross(t[2].0 - t[0].0).normalize_or_zero();
            let drgb = (fnorm.dot(light) * e * 64.0) as i32;
            for v in &mut colour {
                for ch in &mut v[..3] {
                    *ch = (*ch as i32 + drgb).clamp(0, 255) as u8;
                }
            }
        }
        b.tri(t.map(|v| v.0), t.map(|v| v.1), colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uc_formats::txc::CrinklePoint;

    fn quad() -> ReliefQuad {
        // A wall along x, one square wide and tall, facing +z.
        let corners = [Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)];
        ReliefQuad { corners, uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], colour: [[200; 4]; 4], page: 1, crinkle: 0 }
    }

    fn point(u: f32, v: f32, out: f32) -> CrinklePoint {
        let w = [(1.0 - u) * (1.0 - v), u * (1.0 - v), (1.0 - u) * v, u * v].map(|x| (x * 128.0) as u8);
        CrinklePoint { u, v, out, w }
    }

    #[test]
    fn raises_close_and_flattens_far() {
        assert_eq!(extrude(0.0), 1.0);
        assert_eq!(extrude(FULL), 1.0);
        assert_eq!(extrude(FLAT), 0.0);
        assert!(extrude((FULL + FLAT) / 2.0) > 0.4 && extrude((FULL + FLAT) / 2.0) < 0.6);
        assert!(near(Vec3::ZERO, Vec3::splat(1.0), Vec3::splat(2.0)));
        assert!(!near(Vec3::ZERO, Vec3::splat(FLAT), Vec3::splat(FLAT + 8.0)));
    }

    #[test]
    fn points_follow_the_quad_and_stand_out() {
        let c = Crinkle {
            points: vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.5, 0.5, 0.1)],
            faces: vec![[0, 1, 2]],
        };
        let mut b = Batch::default();
        raise(&c, &quad(), 1.0, &mut b);
        assert_eq!(b.vertices.len(), 3);
        // u runs from corner 0 to corner 1, v from corner 0 to corner 2.
        assert_eq!(b.vertices[0].pos, [1.0, 1.0, 0.0]);
        assert_eq!(b.vertices[1].pos, [0.0, 1.0, 0.0]);
        let mid = b.vertices[2].pos;
        assert!((mid[0] - 0.5).abs() < 1e-5 && (mid[1] - 0.5).abs() < 1e-5);
        assert!((mid[2] - 0.1 * DEPTH).abs() < 1e-5, "out along the normal: {mid:?}");
        assert_eq!(b.vertices[2].uv, [0.5, 0.5]);

        // Mirrored, the same crinkle starts from the other top corner.
        let mut q = quad();
        q.uv = [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let mut b = Batch::default();
        raise(&c, &q, 0.5, &mut b);
        assert_eq!(b.vertices[0].pos, [0.0, 1.0, 0.0]);
        assert!((b.vertices[1].pos[2] - 0.05 * DEPTH).abs() < 1e-5);
    }
}
