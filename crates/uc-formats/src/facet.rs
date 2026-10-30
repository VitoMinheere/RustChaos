//! Wall geometry and texturing for building facets, ported from the engine's normal-facet
//! path: `FACET_draw_rare` / `MakeFacetPoints` / `FillFacetPoints` / `texture_quad` /
//! `facet_rand` (fallen/DDEngine/Source/facet.cpp). Renderer-independent: it produces
//! world-space quads with a texture page and UVs.
//!
//! Simplifications compared with the original (fine for a viewer):
//! - The engine picks a different style row when it sees a two-sided facet from behind
//!   (`facet_backwards`, view-dependent). Here every facet uses its front style.
//! - Foundations (`FHeight`) that follow the terrain, barbed wire and per-vertex lighting
//!   are ignored. Doors are built shut: the game swings them open.

use crate::iam::{facet_flags, facet_type, Facet, Map};
use crate::tma::{piece, StyleTable};

/// One textured wall quad. Corner order matches the engine's `quad[]`:
/// 0 = top far, 1 = top near, 2 = bottom far, 3 = bottom near ("near" = segment start).
#[derive(Debug, Clone, Copy)]
pub struct WallQuad {
    pub corners: [[f32; 3]; 4],
    pub uv: [[f32; 2]; 4],
    pub page: u32,
    /// Index of the facet this quad came from.
    pub facet: usize,
}

/// `facet_rand`: a per-facet LCG seeded from the facet's position.
struct FacetRand(u32);

impl FacetRand {
    fn new(f: &Facet) -> Self {
        FacetRand((f.x[0] as i32 * f.z[0] as i32 + f.y[0] as i32) as u32)
    }
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(69069).wrapping_add(1);
        self.0 >> 7
    }
}

/// Which facet kinds this module builds as textured walls.
pub fn is_wall(f: &Facet) -> bool {
    matches!(
        f.kind,
        facet_type::NORMAL | facet_type::WALL | facet_type::DOOR | facet_type::OUTSIDE_DOOR
            | facet_type::TRENCH | facet_type::FENCE | facet_type::FENCE_BRICK | facet_type::FENCE_FLAT
    )
}

/// `texture_quad`'s page and flip choice for one wall segment.
fn segment_texture(map: &Map, styles: &StyleTable, style_value: i16, pos: usize, count: usize,
                   flipx: bool, rand: u32) -> (u32, u8) {
    let texture_piece = if pos == 0 {
        if flipx { piece::LEFT } else { piece::RIGHT }
    } else if pos + 2 == count {
        if flipx { piece::RIGHT } else { piece::LEFT }
    } else {
        [piece::MIDDLE, piece::MIDDLE, piece::MIDDLE1, piece::MIDDLE2][(rand & 3) as usize]
    };
    let mut style = style_value as i32;
    if style < 0 {
        // A painted storey: per-segment pages in paint_mem, falling back to the storey's style.
        if let Some(storey) = map.storeys.get((-style) as usize) {
            if (pos as i32) < storey.count as i32 {
                let painted = map.paint_mem.get(storey.index as usize + pos).copied().unwrap_or(0);
                let flip = if painted & 0x80 != 0 { 1 } else { 0 };
                if painted & 0x7F != 0 {
                    return ((painted & 0x7F) as u32, flip);
                }
            }
            style = storey.style as i32;
        } else {
            style = 1;
        }
    }
    let p = styles.piece(style.max(0) as usize, texture_piece);
    (p.page, p.flip)
}

/// Builds the textured quads for facet `index`. Returns nothing for non-wall facets.
pub fn wall_quads(map: &Map, styles: &StyleTable, index: usize) -> Vec<WallQuad> {
    let f = &map.facets[index];
    let mut out = Vec::new();
    if !is_wall(f) || f.flags & facet_flags::INVISIBLE != 0 {
        return out;
    }
    let (x0, z0) = (f.x[0] as f32 * 256.0, f.z[0] as f32 * 256.0);
    let (dx, dz) = ((f.x[1] as i32 - f.x[0] as i32) * 256, (f.z[1] as i32 - f.z[0] as i32) * 256);
    // Segment count: one per 256 units along the wall (QDIST2 for diagonals).
    let (adx, adz) = (dx.abs(), dz.abs());
    let len = if adx > adz { adx + (adz >> 1) } else { adz + (adx >> 1) };
    let segments = ((len >> 8).max(1)) as usize;
    let count = segments + 1; // points per row, as in the engine
    let (sdx, sdz) = (dx as f32 / segments as f32, dz as f32 / segments as f32);

    let is_fence = matches!(f.kind, facet_type::FENCE | facet_type::FENCE_BRICK | facet_type::FENCE_FLAT);
    let (rows, block_height) = if is_fence {
        // Fences are one row: Height * BlockHeight * 4 tall (facet.cpp:2268).
        (1usize, f.height as f32 * f.block_height as f32 * 4.0)
    } else {
        ((f.height / 4) as usize, (f.block_height as f32) * 16.0)
    };
    if rows == 0 || block_height <= 0.0 {
        return out;
    }

    // Style index bookkeeping from the STOREY_TYPE_NORMAL case.
    let hug = f.flags & facet_flags::HUG_FLOOR != 0;
    let mut start = f.style_index as i32;
    let reverse = f.flags & facet_flags::INSIDE != 0;
    if reverse || f.flags & facet_flags::TWO_TEXTURED != 0 {
        start -= 1;
    }
    let step = if !hug && f.flags & (facet_flags::TWO_TEXTURED | facet_flags::TWO_SIDED) != 0 { 2 } else { 1 };

    let mut rng = FacetRand::new(f);
    let (y0, y1) = (f.y[0] as f32, f.y[1] as f32);
    for row in 0..rows {
        let style_idx = start + (row as i32 + 1) * step - 1;
        let style_value = map.dstyles.get(style_idx.max(0) as usize).copied().unwrap_or(1);
        let bottom = row as f32 * block_height;
        let top = bottom + block_height;
        for c0 in 0..segments {
            let rand = rng.next();
            let pos = if reverse { count - 2 - c0 } else { c0 };
            let (page, flip) = segment_texture(map, styles, style_value, pos, count, reverse, rand);
            let t0 = c0 as f32 / segments as f32;
            let t1 = (c0 + 1) as f32 / segments as f32;
            let p = |seg: usize, t: f32, h: f32| {
                [x0 + sdx * seg as f32, y0 + (y1 - y0) * t + h, z0 + sdz * seg as f32]
            };
            // quad[0]=top c0+1, quad[1]=top c0, quad[2]=bottom c0+1, quad[3]=bottom c0
            let corners = [p(c0 + 1, t1, top), p(c0, t0, top), p(c0 + 1, t1, bottom), p(c0, t0, bottom)];
            let mut uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
            if flip & 1 != 0 {
                for t in uv.iter_mut() {
                    t[0] = 1.0 - t[0];
                }
            }
            if flip & 2 != 0 {
                for t in uv.iter_mut() {
                    t[1] = 1.0 - t[1];
                }
            }
            if is_fence {
                // A fence row may be shorter than a full texture: crop like the engine (v = 0.25 .. 1).
                let v_top = 1.0 - (block_height / 256.0).min(1.0);
                for i in [0, 1] {
                    uv[i][1] = if flip & 2 != 0 { 1.0 - v_top } else { v_top };
                }
            }
            out.push(WallQuad { corners, uv, page, facet: index });
        }
    }
    out
}

/// A wall as seen from above, for simple collision: the segment (x, z) a..b in world units,
/// solid between heights `y0` and `y1`.
#[derive(Debug, Clone, Copy)]
pub struct WallSegment {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub y0: f32,
    pub y1: f32,
}

/// Solid walls and fences of the map. Cables, ladders and invisible facets are left out (the
/// game handles those with their own rules), and so are doors: they block too, but a mission
/// can open one, so [`door_segments`] keeps them apart.
pub fn collision_segments(map: &Map) -> Vec<WallSegment> {
    segments(map, false).into_iter().map(|(_, w)| w).collect()
}

/// The map's doors, in the order they appear, as the shapes they block with while shut.
///
/// A door here is `STOREY_TYPE_OUTSIDE_DOOR`, which is the one kind `DOOR_find` looks for
/// (fallen/Source/door.cpp) and the one a mission can open. Maps carry a handful each, and
/// the count matches the mission's door points: botanicc has two of both.
/// Each comes with its facet's index.
pub fn door_segments(map: &Map) -> Vec<(usize, WallSegment)> {
    segments(map, true)
}

fn segments(map: &Map, doors: bool) -> Vec<(usize, WallSegment)> {
    let mut out = Vec::new();
    for (i, f) in map.facets.iter().enumerate().skip(1) {
        let is_door = f.kind == facet_type::OUTSIDE_DOOR;
        // `STOREY_TYPE_DOOR` is a doorway rather than a door: nothing opens one and the
        // original walks straight through, so it is drawn and never collided with.
        if !is_wall(f) || is_door != doors || f.kind == facet_type::DOOR || f.flags & facet_flags::INVISIBLE != 0 {
            continue;
        }
        let is_fence = matches!(f.kind, facet_type::FENCE | facet_type::FENCE_BRICK | facet_type::FENCE_FLAT);
        let height = if is_fence {
            f.height as f32 * f.block_height as f32 * 4.0
        } else {
            (f.height / 4) as f32 * f.block_height as f32 * 16.0
        };
        if height <= 0.0 {
            continue;
        }
        let y0 = (f.y[0].min(f.y[1])) as f32;
        out.push((i, WallSegment {
            a: [f.x[0] as f32 * 256.0, f.z[0] as f32 * 256.0],
            b: [f.x[1] as f32 * 256.0, f.z[1] as f32 * 256.0],
            y0,
            y1: y0 + height,
        }));
    }
    out
}
