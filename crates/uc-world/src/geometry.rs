//! Renderer-independent geometry: vertices grouped by chunk, layer and texture page.
//! Positions are in "square units": 1.0 = one map square = 256 world units.

use crate::crinkle::{Relief, ReliefChunk, ReliefQuad};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};
use uc_formats::txc::Crinkle;

/// World units per square unit.
pub const UNIT: f32 = 256.0;
/// Chunk size in map squares, for frustum culling (16 x 16 chunks per map).
pub const CHUNK_SQUARES: f32 = 8.0;
/// Page value for untextured (flat-coloured) geometry.
pub const UNTEXTURED: u32 = u32::MAX;
/// Batches with this page are soft round blobs, drawn blended over the scene with a round
/// falloff times the vertex alpha: blob shadows, the blood under a body, and smoke.
pub const SHADOW: u32 = u32::MAX - 1;
/// Batches with this page are the same round blobs added to the scene rather than blended
/// over it: the fire and dust of an explosion.
pub const GLOW: u32 = u32::MAX - 2;
/// Batches with this page are blended over the scene like [`SHADOW`], but textured with a
/// soft cloud instead of a round blob: the sheets of ground mist a mission lays down.
pub const MIST: u32 = u32::MAX - 3;
/// `TEXTURE_page_sky` (`TEXTURE_NUM_STANDARD + 10`): the world's `sky.tga`, wrapped round the
/// skyline. Batches with this page are drawn first, behind everything, with no fog.
pub const SKY_PAGE: u32 = 22 * 64 + 10;
/// `TEXTURE_page_moon` (`TEXTURE_NUM_STANDARD + 1`): `moon.tga`, added over the sky.
pub const MOON_PAGE: u32 = 22 * 64 + 1;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    /// Vertex colour (lighting), multiplied with the texture in gamma space like the original.
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Floor,
    Roofs,
    Walls,
    Props,
    /// Cables, ladders and other small structures.
    Detail,
    Character,
}

impl Layer {
    pub const ALL: [Layer; 6] = [Layer::Floor, Layer::Roofs, Layer::Walls, Layer::Props, Layer::Detail, Layer::Character];
}

#[derive(Debug, Clone, Default)]
pub struct Batch {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Batch {
    pub fn quad(&mut self, p: [Vec3; 4], uv: [[f32; 2]; 4], c: [[u8; 4]; 4]) {
        let base = self.vertices.len() as u32;
        for i in 0..4 {
            self.vertices.push(Vertex { pos: p[i].to_array(), uv: uv[i], color: c[i] });
        }
        // Engine corner order: 0,1 one edge, 2,3 the opposite edge.
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    pub fn tri(&mut self, p: [Vec3; 3], uv: [[f32; 2]; 3], c: [[u8; 4]; 3]) {
        let base = self.vertices.len() as u32;
        for i in 0..3 {
            self.vertices.push(Vertex { pos: p[i].to_array(), uv: uv[i], color: c[i] });
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
}

/// A piece of the world for culling: its bounds and its batches by (layer, page).
#[derive(Debug, Clone)]
pub struct Chunk {
    pub min: Vec3,
    pub max: Vec3,
    pub batches: BTreeMap<(Layer, u32), Batch>,
    /// Walls with a crinkle, drawn flat, by page. They're part of [`Layer::Walls`], but near
    /// the camera they're left out for [`Relief::draw`] to draw instead.
    pub crinkled: BTreeMap<u32, Batch>,
    /// Those walls' quads, to raise.
    pub reliefs: Vec<ReliefQuad>,
}

#[derive(Debug, Clone, Default)]
pub struct WorldGeometry {
    pub chunks: Vec<Chunk>,
    /// The crinkles [`ReliefQuad::crinkle`] refers to.
    pub crinkles: Vec<Crinkle>,
}

impl WorldGeometry {
    pub fn pages(&self) -> BTreeSet<u32> {
        self.chunks
            .iter()
            .flat_map(|c| c.batches.keys().map(|k| k.1).chain(c.crinkled.keys().copied()))
            .filter(|&p| p != UNTEXTURED)
            .collect()
    }

    pub fn count(&self, layer: Layer) -> usize {
        self.chunks
            .iter()
            .flat_map(|c| c.batches.iter())
            .filter(|((l, _), _)| *l == layer)
            .map(|(_, b)| b.indices.len() / 3)
            .sum::<usize>()
            + if layer == Layer::Walls { self.crinkled_count() } else { 0 }
    }

    /// Triangles in the walls with a crinkle, drawn flat.
    pub fn crinkled_count(&self) -> usize {
        self.chunks.iter().flat_map(|c| c.crinkled.values()).map(|b| b.indices.len() / 3).sum()
    }

    /// The walls with a crinkle, to keep for drawing them raised.
    pub fn relief(&self) -> Relief {
        let chunks = self
            .chunks
            .iter()
            .filter(|c| !c.reliefs.is_empty())
            .map(|c| ReliefChunk { min: c.min, max: c.max, quads: c.reliefs.clone() })
            .collect();
        Relief { crinkles: self.crinkles.clone(), chunks }
    }
}

/// Collects primitives into chunks by the position of their first vertex.
#[derive(Default)]
pub struct Builder {
    chunks: BTreeMap<(i32, i32), Parts>,
}

#[derive(Default)]
struct Parts {
    batches: BTreeMap<(Layer, u32), Batch>,
    crinkled: BTreeMap<u32, Batch>,
    reliefs: Vec<ReliefQuad>,
}

impl Builder {
    fn parts(&mut self, at: Vec3) -> &mut Parts {
        let key = ((at.x / CHUNK_SQUARES).floor() as i32, (at.z / CHUNK_SQUARES).floor() as i32);
        self.chunks.entry(key).or_default()
    }

    pub fn batch(&mut self, at: Vec3, layer: Layer, page: u32) -> &mut Batch {
        self.parts(at).batches.entry((layer, page)).or_default()
    }

    /// A wall quad with a crinkle: drawn flat, and kept to raise.
    pub fn crinkled(&mut self, q: ReliefQuad) {
        let parts = self.parts(q.corners[0]);
        parts.crinkled.entry(q.page).or_default().quad(q.corners, q.uv, q.colour);
        parts.reliefs.push(q);
    }

    pub fn finish(self) -> WorldGeometry {
        let chunks = self
            .chunks
            .into_values()
            .filter(|p| !p.batches.is_empty() || !p.crinkled.is_empty())
            .map(|Parts { batches, crinkled, reliefs }| {
                let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for v in batches.values().chain(crinkled.values()).flat_map(|b| &b.vertices) {
                    min = min.min(Vec3::from(v.pos));
                    max = max.max(Vec3::from(v.pos));
                }
                Chunk { min, max, batches, crinkled, reliefs }
            })
            .collect();
        WorldGeometry { chunks, crinkles: Vec::new() }
    }
}
