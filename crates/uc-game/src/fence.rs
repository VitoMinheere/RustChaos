//! Fences rattling when a car scrapes them (`VEH_shake_fences`, fallen/Source/vehicle.cpp:1876).
//! A car rubbing along a wall shakes the fences on its square's edges, and each is drawn
//! with its corners jittering while the shake dies away (`FACET_draw`'s `p_facet->Shake`,
//! DDEngine/Source/facet.cpp:3435). The level leaves the fences out of its static geometry
//! for this ([`uc_world::Level::fences`]).

use glam::{vec3, Vec3};
use std::collections::BTreeMap;
use uc_world::level::FacetQuad;
use uc_world::{Batch, Level};

/// `FACET_draw`'s `shake[]` (facet.cpp:2385), in world units: four rows of each strength,
/// one picked at random for each corner's x and another for its z.
const SHAKE: [f32; 32] = [
    0.0, 0.0, 0.0, 0.0,
    1.0, -2.0, 1.0, -1.0,
    3.0, -3.0, 2.0, -2.0,
    4.0, -3.0, 2.0, -4.0,
    5.0, -5.0, 5.0, -4.0,
    6.0, -4.0, 7.0, -6.0,
    7.0, -6.0, 8.0, -9.0,
    9.0, -7.0, 8.0, -9.0,
];

/// One of the map's fences, as it's drawn.
pub struct Fence {
    /// Its quads, standing still.
    quads: Vec<FacetQuad>,
    /// Its ends, in map squares (`DFacet::x`, `z`).
    x: [u8; 2],
    z: [u8; 2],
    /// `DFacet::Shake`: 255 when a car's just scraped it, dying away.
    pub shake: u8,
}

/// The map's fences.
#[derive(Default)]
pub struct Fences {
    pub fences: Vec<Fence>,
}

impl Fences {
    pub fn new(level: &Level) -> Fences {
        let fences = level.fences().into_iter().map(|(i, quads)| {
            let f = &level.map.facets[i];
            Fence { quads, x: f.x, z: f.z, shake: 0 }
        }).collect();
        Fences { fences }
    }

    /// `VEH_shake_fences`: a car on square (`mx`, `mz`) has hit a wall, so each fence in that
    /// square's 4 by 4 collision block (`PAP_2LO(mx >> 2, mz >> 2).ColVectHead`) that runs
    /// along one of the lines through the square's edges shakes, wherever along the line it
    /// is.
    pub fn shake(&mut self, mx: i32, mz: i32) {
        let (lx, lz) = (mx >> 2, mz >> 2);
        let spans = |a: [u8; 2], l: i32| (a[0].min(a[1]) as i32) <= l * 4 + 4 && (a[0].max(a[1]) as i32) >= l * 4;
        for f in &mut self.fences {
            if !spans(f.x, lx) || !spans(f.z, lz) {
                continue;
            }
            let (x, z) = (f.x[0] as i32, f.z[0] as i32);
            let on = if f.x[0] == f.x[1] {
                x == mx || x == mx + 1
            } else {
                // (Every fence runs along x or z: `VEH_shake_fences` asserts as much.)
                f.z[0] == f.z[1] && (z == mz || z == mz + 1)
            };
            if on {
                f.shake = 255;
            }
        }
    }

    /// Each shaking fence settles a little, each turn as the original does each frame it's
    /// drawn (facet.cpp:3437).
    pub fn process(&mut self) {
        for f in &mut self.fences {
            if f.shake > 0 {
                f.shake -= f.shake >> 2;
                f.shake -= 1;
            }
        }
    }

    /// Each fence, with a shaking one's corners each moved along x and z by a random amount
    /// of its strength (facet.cpp:3485), afresh each `turn`. The corners quads share move
    /// together, so the fence doesn't come apart.
    pub fn batches(&self, out: &mut BTreeMap<u32, Batch>, turn: u32) {
        for f in &self.fences {
            let row = ((f.shake >> 3) & !0x3) as usize;
            for q in &f.quads {
                let p = if f.shake == 0 {
                    q.corners
                } else {
                    q.corners.map(|p| {
                        let h = jitter_hash(p, turn);
                        let dx = SHAKE[row + (h & 3) as usize];
                        let dz = SHAKE[row + ((h >> 2) & 3) as usize];
                        p + vec3(dx, 0.0, dz) / uc_world::UNIT
                    })
                };
                out.entry(q.page).or_default().quad(p, q.uv, q.colour);
            }
        }
    }
}

/// A corner's two random picks (`Random() & 3` each), the same for every quad sharing it,
/// changing each turn. Only for drawing: it leaves the game's own random numbers alone.
fn jitter_hash(p: Vec3, turn: u32) -> u32 {
    let q = (p * 64.0).round().as_ivec3();
    let mut h = (q.x as u32).wrapping_mul(0x9e37_79b1)
        ^ (q.y as u32).wrapping_mul(0x85eb_ca77)
        ^ (q.z as u32).wrapping_mul(0xc2b2_ae3d)
        ^ turn.wrapping_mul(0x27d4_eb2f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fence(x: [u8; 2], z: [u8; 2]) -> Fence {
        Fence { quads: Vec::new(), x, z, shake: 0 }
    }

    /// Only fences on the lines through the car's square's edges shake, and only in its 4 by
    /// 4 block.
    #[test]
    fn a_scrape_shakes_the_fences_on_the_squares_edges() {
        let mut f = Fences {
            fences: vec![
                fence([10, 10], [8, 11]), // along z at x = 10, the square's west edge
                fence([11, 11], [9, 10]), // its east edge
                fence([12, 12], [8, 11]), // a square further east
                fence([8, 11], [9, 9]),   // along x at z = 9, its north edge
                fence([10, 10], [0, 3]),  // on the line, but in another block
            ],
        };
        f.shake(10, 9);
        let shook: Vec<bool> = f.fences.iter().map(|f| f.shake == 255).collect();
        assert_eq!(shook, [true, true, false, true, false]);
    }

    /// `Shake -= Shake >> 2; Shake -= 1`: from 255 it's still in under a second.
    #[test]
    fn the_shake_dies_away() {
        let mut f = Fences { fences: vec![fence([0, 0], [0, 1])] };
        f.fences[0].shake = 255;
        let mut turns = 0;
        while f.fences[0].shake > 0 {
            f.process();
            turns += 1;
        }
        assert!((10..30).contains(&turns), "{turns} turns");
    }
}
