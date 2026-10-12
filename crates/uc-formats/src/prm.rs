//! `.prm` static meshes ("prims": lampposts, bins, desks, ...): `load_prim_object`
//! (fallen/Source/io.cpp:888). Face layouts are the PC ones from fallen/Editor/Headers/Prim.h.

use crate::reader::{Reader, Result};

/// `PRIM_START_SAVE_TYPE` (Editor/Headers/Prim.h:21). save_type - this == 1 means 6-byte points.
pub const PRIM_START_SAVE_TYPE: u16 = 5793;
const FACE_FLAG_FIXED: u16 = 1 << 5;
/// `FACE_PAGE_OFFSET` (DDEngine/Headers/texture.h:20).
const FACE_PAGE_OFFSET: u32 = 8 * 64;

#[derive(Debug, Clone)]
pub struct Face {
    /// Indices into `Prim::points` (already rebased to 0).
    pub points: Vec<u16>,
    /// Raw on-disk UVs: one per point.
    pub uv: Vec<[u8; 2]>,
    pub texture_page: u8,
    pub draw_flags: u8,
    pub face_flags: u16,
}

impl Face {
    /// Texture page and normalised UVs, following `TEXTURE_fix_prim_textures`
    /// (DDEngine/Source/texture.cpp:2481). UVs on disk address a 256x256 sheet split into
    /// an 8x8 grid of 32-pixel cells; the cell the face sits in picks the page.
    pub fn texture(&self) -> (u32, Vec<[f32; 2]>) {
        let n = self.uv.len() as i32;
        if self.face_flags & FACE_FLAG_FIXED != 0 {
            let page = self.texture_page as u32 | (((self.uv[0][0] & 0xC0) as u32) << 2);
            let uv = self
                .uv
                .iter()
                .enumerate()
                .map(|(i, &[u, v])| {
                    let u = if i == 0 { u & 0x3F } else { u };
                    [u as f32 / 32.0, v as f32 / 32.0]
                })
                .collect();
            return (page + FACE_PAGE_OFFSET, uv);
        }
        let av_u = self.uv.iter().map(|t| t[0] as i32).sum::<i32>() / n / 32;
        let av_v = self.uv.iter().map(|t| t[1] as i32).sum::<i32>() / n / 32;
        let (base_u, base_v) = (av_u * 32, av_v * 32);
        let uv = self
            .uv
            .iter()
            .map(|&[u, v]| {
                let fix = |t: i32| {
                    let t = t.clamp(0, 32);
                    if t == 31 { 32 } else { t }
                };
                [fix(u as i32 - base_u) as f32 / 32.0, fix(v as i32 - base_v) as f32 / 32.0]
            })
            .collect();
        let page = av_u + av_v * 8 + self.texture_page as i32 * 64;
        (page.max(0) as u32, uv)
    }
}

#[derive(Debug, Clone)]
pub struct Prim {
    pub save_type: u16,
    pub name: String,
    /// Model-space points in world units.
    pub points: Vec<[i16; 3]>,
    pub faces: Vec<Face>,
    pub coltype: u8,
    pub damage: u8,
    pub shadowtype: u8,
    pub flag: u8,
}

impl Prim {
    /// Parses either the new (`nprimNNN.prm`) or the old (`primNNN.prm`) layout.
    pub fn parse(data: &[u8]) -> Result<Prim> {
        let mut r = Reader::new(data);
        let first = r.u16()?;
        let mut r = Reader::new(data);
        let (save_type, name, ranges, extra);
        if first == PRIM_START_SAVE_TYPE || first == PRIM_START_SAVE_TYPE + 1 {
            save_type = r.u16()?;
            name = r.cstr(32)?;
            ranges = prim_ranges(&mut r)?;
            extra = [r.u8()?, r.u8()?, r.u8()?, r.u8()?];
        } else {
            // PrimObjectOld: name, ranges, coltype/damage/shadowtype/flag, Dummy[4] (save type in [3]).
            name = r.cstr(32)?;
            ranges = prim_ranges(&mut r)?;
            extra = [r.u8()?, r.u8()?, r.u8()?, r.u8()?];
            r.skip(6)?;
            save_type = r.u16()?;
        }
        let (sp, ep, s4, e4, s3, e3) = ranges;
        let npts = ep.saturating_sub(sp) as usize;
        let mut points = Vec::with_capacity(npts);
        for _ in 0..npts {
            if save_type.wrapping_sub(PRIM_START_SAVE_TYPE) == 1 {
                points.push([r.i16()?, r.i16()?, r.i16()?]);
            } else {
                points.push([r.i32()? as i16, r.i32()? as i16, r.i32()? as i16]);
            }
        }
        let mut faces = Vec::new();
        for _ in 0..(e3 - s3).max(0) {
            faces.push(face(&mut r, 3, sp)?);
        }
        for _ in 0..e4.saturating_sub(s4) {
            faces.push(face(&mut r, 4, sp)?);
        }
        r.expect_end()?;
        for f in &faces {
            if f.points.iter().any(|&p| p as usize >= points.len()) {
                return r.error("face refers to a point outside the prim");
            }
        }
        Ok(Prim { save_type, name, points, faces, coltype: extra[0], damage: extra[1], shadowtype: extra[2], flag: extra[3] })
    }
}

/// (StartPoint, EndPoint, StartFace4, EndFace4, StartFace3, EndFace3)
fn prim_ranges(r: &mut Reader) -> Result<(u16, u16, u16, u16, i16, i16)> {
    Ok((r.u16()?, r.u16()?, r.u16()?, r.u16()?, r.i16()?, r.i16()?))
}

pub(crate) fn face(r: &mut Reader, n: usize, start_point: u16) -> Result<Face> {
    let texture_page = r.u8()?;
    let draw_flags = r.u8()?;
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        points.push(r.u16()?.wrapping_sub(start_point));
    }
    let mut uv = Vec::with_capacity(n);
    for _ in 0..n {
        uv.push([r.u8()?, r.u8()?]);
    }
    r.skip(2 * n)?; // Bright[n] (or colour for building faces)
    r.skip(4)?; // ThingIndex, Col2
    let face_flags = r.u16()?;
    r.skip(2)?; // Type, ID
    Ok(Face { points, uv, texture_page, draw_flags, face_flags })
}
