//! Damaged props drawn bent (#296, `OB_damage`, ob.cpp:1511, and `OB_find`, ob.cpp:353):
//! lampposts and signs lean, the rest crumple. Their own chunks in the static world are left
//! out (`Renderer::hide_props`) and they're drawn here instead.

use crate::*;

/// How a damaged prop is drawn.
#[derive(Debug, Clone)]
pub struct Bend {
    pub prim: uc_formats::prm::Prim,
    /// Where it stands and the way it faces (radians).
    pub at: Vec3,
    pub yaw: f32,
    /// Leaning: tipped over this far (radians, `pitch` and `roll` of `MESH_draw_poly`).
    pub pitch: f32,
    pub roll: f32,
    /// Crumpled: how badly, the original's flag bits (`oo->flags >> 6`), 0 for not.
    pub crumple: u8,
}

/// `MESH_NUM_CRUMPLES` and `_CRUMPVALS` (mesh.cpp:51).
const CRUMPLES: u8 = 5;
const CRUMPVALS: usize = 8;

impl Bend {
    /// Where point `v` of the prim goes (`MESH_draw_poly`: world = transpose(`MATRIX_calc`
    /// (yaw, pitch, roll)) times the point, crumpled by `MESH_crumple`).
    pub fn place(&self, v: Vec3) -> Vec3 {
        let v = v + self.crumpled(v);
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let (sr, cr) = self.roll.sin_cos();
        let m = [
            cy * cr + sy * sp * sr, -cp * sr, -sy * cr + cy * sp * sr,
            cy * sr - sy * sp * cr, cp * cr, -sy * sr - cy * sp * cr,
            sy * cp, sp, cy * cp,
        ];
        self.at + Vec3::new(
            m[0] * v.x + m[3] * v.y + m[6] * v.z,
            m[1] * v.x + m[4] * v.y + m[7] * v.z,
            m[2] * v.x + m[5] * v.y + m[8] * v.z,
        )
    }

    /// Point `v`'s crumple: one of eight random offsets, up to 2.5 a step of how badly it's
    /// crumpled (`MESH_init`, mesh.cpp:118), drawn one more than kept (`OB_find`). The
    /// original takes them in turn down the prim's points; here by where the point is, so
    /// it's the same for every face that shares it.
    fn crumpled(&self, v: Vec3) -> Vec3 {
        if self.crumple == 0 {
            return Vec3::ZERO;
        }
        let amp = (self.crumple + 1).min(CRUMPLES - 1) as f32 * 2.5;
        let k = (v.x as i32).wrapping_mul(73_856_093) ^ (v.y as i32).wrapping_mul(19_349_663) ^ (v.z as i32).wrapping_mul(83_492_791);
        let j = (k as u32) % CRUMPVALS as u32;
        let r = |n: u32| {
            let h = (j * 3 + n).wrapping_mul(2_654_435_761) ^ 0x5bd1_e995;
            (h >> 8 & 0xffff) as f32 / 32768.0 - 1.0
        };
        Vec3::new(r(0), r(1), r(2)) * amp
    }
}

impl Game {
    /// Prop `i` damaged by a blow from `from`: a lamppost or a sign leans (one way or the
    /// other on two axes by which side `from` is on, 40 of 2048 plus a little by which prop
    /// it is); anything else crumples a step further, 1 to 4 and then between 3 and 4.
    pub(crate) fn bend_prop(&mut self, level: &Level, i: usize, lean: bool, from: Vec3) {
        let o = &level.map.objects[i];
        if !self.bent.contains_key(&i) {
            let Some(prim) = uc_world::level::load_prim(&level.game, o.prim) else { return };
            let at = Vec3::new(o.x as f32, o.y as f32, o.z as f32);
            let yaw = uc_formats::units::radians(o.yaw as f32);
            self.bent.insert(i, Bend { prim, at, yaw, pitch: 0.0, roll: 0.0, crumple: 0 });
        }
        let Some(b) = self.bent.get_mut(&i) else { return };
        if lean {
            // `dprod` and `cprod` of the way it faces with `from`, which the original passes
            // as it is: a position, not a direction.
            let (lx, lz) = (b.yaw.sin(), b.yaw.cos());
            let dprod = lx * from.x + lz * from.z;
            let cprod = lx * from.z - lz * from.x;
            const DLEAN: [i32; 4] = [25, -16, -28, 15];
            let unit = std::f32::consts::TAU / 2048.0;
            b.roll = ((if cprod > 0.0 { 40 } else { -40 }) + DLEAN[i & 3]) as f32 * unit;
            b.pitch = ((if dprod > 0.0 { 40 } else { -40 }) + DLEAN[(i + 1) & 3]) as f32 * unit;
        } else {
            b.crumple = if b.crumple + 1 == 5 { 3 } else { b.crumple + 1 };
        }
    }

    /// The props drawn bent, whose own chunks the static world leaves out.
    pub fn bent_props(&self) -> impl Iterator<Item = usize> + '_ {
        self.bent.keys().copied()
    }
}
