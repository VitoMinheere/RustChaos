//! `.lgt` baked lighting: `NIGHT_load_ed_file` (fallen/Source/night.cpp:3979) and the
//! static-light evaluation of `NIGHT_ambient_at_point` / `NIGHT_get_light_at`
//! (night.cpp:112, :2880), so any renderer can light its vertices like the game.

use crate::iam::Object;
use crate::reader::{count, Reader, Result};

/// `NIGHT_FLAG_*`.
pub const FLAG_LIGHTS_UNDER_LAMPOSTS: u32 = 1 << 0;
pub const FLAG_DAYTIME: u32 = 1 << 2;

/// `ED_Light` (fallen/Ledit/Headers/ed.h): a static point light.
#[derive(Debug, Clone, Copy)]
pub struct Light {
    pub range: u8,
    pub rgb: [i8; 3],
    pub pos: [i32; 3],
}

#[derive(Debug, Clone)]
pub struct Lighting {
    pub version: u16,
    pub lights: Vec<Light>,
    pub flags: u32,
    /// Ambient colour as stored (before the engine's scaling).
    pub ambient: [i32; 3],
    pub lampost_rgb: [i8; 3],
    pub lampost_radius: i32,
    pub sky: [u8; 3],
}

/// The ambient light direction the engine always uses ("These constants crop up everywhere!").
const AMB_NORM: [f32; 3] = [110.0, -148.0, -177.0];

impl Lighting {
    pub fn parse(data: &[u8]) -> Result<Lighting> {
        let mut r = Reader::new(data);
        let head = r.i32()?;
        let (version, size) = ((head >> 16) as u16, (head & 0xFFFF) as usize);
        let n = r.i32()?;
        let n = count(&mut r, n)?;
        r.i32()?; // sizeof(NIGHT_Colour)
        if size != 20 {
            return r.error(format!("unexpected ED_Light size {size}"));
        }
        let mut lights = Vec::new();
        for _ in 0..n {
            let range = r.u8()?;
            let rgb = [r.i8()?, r.i8()?, r.i8()?];
            let (_next, used, _flags, _pad) = (r.u8()?, r.u8()?, r.u8()?, r.u8()?);
            let pos = [r.i32()?, r.i32()?, r.i32()?];
            if used != 0 {
                lights.push(Light { range, rgb, pos });
            }
        }
        r.i32()?; // ed_light_free
        let flags = r.u32()?;
        r.u32()?; // amb_d3d_colour
        r.u32()?; // amb_d3d_specular
        let ambient = [r.i32()?, r.i32()?, r.i32()?];
        let lampost_rgb = [r.i8()?, r.i8()?, r.i8()?];
        r.u8()?;
        let lampost_radius = r.i32()?;
        let sky = [r.u8()?, r.u8()?, r.u8()?];
        // 32 of the 61 retail files carry one trailing pad byte; the loader ignores it.
        if r.remaining() > 1 {
            return r.error(format!("{} bytes left over", r.remaining()));
        }
        Ok(Lighting { version, lights, flags, ambient, lampost_rgb, lampost_radius, sky })
    }

    /// `NIGHT_amb_*`: the file's ambient scaled by `* 820 >> 8` (load) then `* 80 >> 8` (NIGHT_ambient).
    fn amb(&self) -> [f32; 3] {
        self.ambient.map(|c| ((((c * 820) >> 8) * 80) >> 8) as f32)
    }
}

/// Lights world-space points the way the game's static lighting does.
pub struct Lighter<'a> {
    lighting: &'a Lighting,
    lamps: Vec<[f32; 3]>,
    /// For each [`CELL`]-sized square of the map, the lights that can reach into it: indices
    /// into `lighting.lights`, then `lights.len()` plus an index into `lamps`, in that order
    /// (the order [`Lighter::light`] adds them up in). A light reaches no further than its
    /// radius along x or z (`QDIST3` is never less than the largest of them), so a point only
    /// needs its own square's.
    cells: Vec<Vec<u16>>,
}

/// The side of a [`Lighter`] cell in world units: four map squares.
const CELL: f32 = 1024.0;
/// Cells along each side of the map (128 squares of 256).
const CELLS: usize = 32;

impl<'a> Lighter<'a> {
    /// `objects` are used for lamppost lights when the file enables them
    /// (prims 0-4 are lampposts, `NIGHT_get_light_at`).
    pub fn new(lighting: &'a Lighting, objects: &[Object]) -> Self {
        let mut lamps = Vec::new();
        if lighting.flags & FLAG_LIGHTS_UNDER_LAMPOSTS != 0 {
            for o in objects.iter().filter(|o| o.prim < 5) {
                // The light hangs off the lamp's arm: -FMATRIX_vector(yaw) >> 9, i.e. 128 units.
                let a = o.yaw as f32 * std::f32::consts::TAU / 2048.0;
                lamps.push([o.x as f32 - a.sin() * 128.0, o.y as f32 + 256.0, o.z as f32 - a.cos() * 128.0]);
            }
        }
        let mut cells = vec![Vec::new(); CELLS * CELLS];
        let lamp_r = lighting.lampost_radius as f32 * 4.0;
        let reach = lighting.lights.iter().map(|l| (l.pos.map(|v| v as f32), l.range as f32 * 4.0))
            .chain(lamps.iter().map(|&p| (p, lamp_r)));
        for (k, (pos, r)) in reach.enumerate() {
            if r <= 0.0 {
                continue;
            }
            let span = |v: f32| (((v - r) / CELL).floor().max(0.0) as usize, (((v + r) / CELL).floor().max(0.0) as usize).min(CELLS - 1));
            let ((x0, x1), (z0, z1)) = (span(pos[0]), span(pos[2]));
            for z in z0..=z1.max(z0) {
                for x in x0..=x1.max(x0) {
                    if x < CELLS && z < CELLS {
                        cells[z * CELLS + x].push(k as u16);
                    }
                }
            }
        }
        Lighter { lighting, lamps, cells }
    }

    pub fn sky(&self) -> [u8; 3] {
        self.lighting.sky
    }

    /// Vertex colour (0..1 per channel, to multiply with the texture) at a world-space point
    /// with the given surface normal (any length).
    pub fn light(&self, p: [f32; 3], normal: [f32; 3]) -> [f32; 3] {
        let amb = self.lighting.amb();
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt().max(1e-6);
        let n = normal.map(|v| v / len * 256.0);
        let dprod = (n[0] * AMB_NORM[0] + n[1] * AMB_NORM[1] + n[2] * AMB_NORM[2]) / 1024.0 + 192.0;
        let mut c = amb.map(|a| a * dprod / 256.0);
        let mut add = |pos: [f32; 3], radius: f32, rgb: [f32; 3]| {
            let (dx, dy, dz) = ((pos[0] - p[0]).abs(), (pos[1] - p[1]).abs(), (pos[2] - p[2]).abs());
            let dist = qdist3(dx, dy, dz);
            if radius > 0.0 && dist <= radius {
                let bright = 128.0 - dist * 128.0 / radius;
                for i in 0..3 {
                    c[i] += rgb[i] * bright / 256.0;
                }
            }
        };
        let lamp_rgb = self.lighting.lampost_rgb.map(|v| v as f32);
        let lamp_r = self.lighting.lampost_radius as f32 * 4.0;
        let cell = |v: f32| ((v / CELL).floor().max(0.0) as usize).min(CELLS - 1);
        let n = self.lighting.lights.len();
        for &k in &self.cells[cell(p[2]) * CELLS + cell(p[0])] {
            let k = k as usize;
            if k < n {
                let l = &self.lighting.lights[k];
                add(l.pos.map(|v| v as f32), l.range as f32 * 4.0, l.rgb.map(|v| (v as i32 >> 1) as f32));
            } else {
                add(self.lamps[k - n], lamp_r, lamp_rgb);
            }
        }
        // NIGHT_get_d3d_colour: * (256 / NIGHT_MAX_BRIGHT), clamped to a byte.
        c.map(|v| (v * 4.0).clamp(0.0, 255.0) / 255.0)
    }
}

/// `QDIST3`: the engine's approximate 3D distance.
fn qdist3(x: f32, y: f32, z: f32) -> f32 {
    let mut v = [x, y, z];
    v.sort_by(|a, b| b.partial_cmp(a).unwrap());
    v[0] + v[1] / 4.0 + v[2] / 4.0
}
