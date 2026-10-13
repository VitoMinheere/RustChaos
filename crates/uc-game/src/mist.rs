//! Ground mist: the sheets a `create_mist` event point lays down.
//!
//! `MIST_create` (fallen/Source/mist.cpp) makes a flat patch of quads `detail` across, at a
//! fixed height above the world rather than following the ground — the call that would have
//! followed it is commented out in `MIST_get_point`. The texture is sampled over half the
//! sheet, from one of four quadrants in turn, so patches laid near each other don't repeat.
//!
//! What gives it life is the wibble in `MIST_get_texture`: every point inside the edge has its
//! texture pulled round a small circle, neighbours half a turn apart, so the sheet shimmers
//! without any of it sliding. The edge points are left alone, which keeps the seam still.
//! `MIST_process` damps each point's drift back towards nothing, which is what settles the
//! sheet after something has pushed through it; the gusts themselves aren't here yet.

use glam::{vec3, Vec3};

/// Quads across a patch: `EWAY_DO_CREATE_MIST` asks for 17.
const DETAIL: usize = 17;
/// Half a patch's width, `MIST_SIZE` in `EWAY_DO_CREATE_MIST`.
const HALF: f32 = 0x800 as f32;
/// How far round the wibble takes a point, over the whole sheet's worth of texture.
const WIBBLE: f32 = 0.1;

/// One sheet of mist.
#[derive(Debug, Clone)]
pub struct Mist {
    /// The middle of the patch, and the height it floats at.
    pub pos: Vec3,
    /// Which quarter of the texture it draws from (`type_cycle & 3`).
    quadrant: u8,
}

impl Mist {
    /// A point of the sheet's grid, in world units.
    fn point(&self, ix: usize, iz: usize) -> Vec3 {
        let step = HALF * 2.0 / (DETAIL - 1) as f32;
        vec3(self.pos.x - HALF + ix as f32 * step, self.pos.y, self.pos.z - HALF + iz as f32 * step)
    }

    /// Where that point reads the texture, `secs` into the mission.
    fn texture(&self, ix: usize, iz: usize, secs: f32) -> [f32; 2] {
        let span = 0.5 / (DETAIL - 1) as f32;
        let base = [(self.quadrant & 1) as f32 * 0.5, (self.quadrant >> 1) as f32 * 0.5];
        let mut uv = [base[0] + ix as f32 * span, base[1] + iz as f32 * span];
        // The edge of the sheet is pinned, so neighbouring patches keep their seam.
        if ix == 0 || iz == 0 || ix == DETAIL - 1 || iz == DETAIL - 1 {
            return uv;
        }
        // Each point goes round a little circle; its neighbours are half a turn behind, which
        // is what makes the sheet crawl rather than slide.
        let radius = WIBBLE / (DETAIL - 1) as f32;
        let turn = secs / (self.quadrant as f32 + 1.0) * 0.7;
        let yaw = turn + if (ix + iz) & 1 == 1 { 0.0 } else { std::f32::consts::PI / 1.75 };
        uv[0] += yaw.sin() * radius;
        uv[1] += yaw.cos() * radius;
        uv
    }
}

/// Every sheet a mission has laid down.
#[derive(Debug, Clone)]
pub struct Mists {
    pub sheets: Vec<Mist>,
    /// The height the next one floats at. `EWAY_DO_CREATE_MIST` starts at 84 and steps it by
    /// 11 each time, up and down about that, so sheets laid over each other don't z-fight.
    next_height: f32,
}

impl Default for Mists {
    fn default() -> Mists {
        Mists { sheets: Vec::new(), next_height: 84.0 }
    }
}

impl Mists {
    /// Lays a sheet centred on `(x, z)`.
    pub fn create(&mut self, x: f32, z: f32) {
        let quadrant = (self.sheets.len() & 3) as u8;
        self.sheets.push(Mist { pos: vec3(x, self.next_height, z), quadrant });
        self.next_height += if self.next_height as i32 & 1 == 1 { -11.0 } else { 11.0 };
    }

    /// The sheets as quads, in square units, for whatever draws them.
    pub fn quads(&self, secs: f32, mut quad: impl FnMut([Vec3; 4], [[f32; 2]; 4])) {
        for m in &self.sheets {
            for ix in 0..DETAIL - 1 {
                for iz in 0..DETAIL - 1 {
                    let corners = [(ix, iz), (ix + 1, iz), (ix, iz + 1), (ix + 1, iz + 1)];
                    quad(
                        corners.map(|(x, z)| m.point(x, z) / uc_world::UNIT),
                        corners.map(|(x, z)| m.texture(x, z, secs)),
                    );
                }
            }
        }
    }
}
