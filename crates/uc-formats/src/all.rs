//! `.all` character files: body meshes plus keyframe animation.
//! `load_anim_system` / `load_insert_a_multi_prim` / `read_a_prim` / `load_insert_game_chunk`
//! (fallen/Source/io.cpp:2660, :2631, :1968, :2385). Posing follows `FIGURE_draw` and
//! `FIGURE_draw_prim_tween` (fallen/DDEngine/Source/figure.cpp:7148, :3062).
//!
//! A character is a set of rigid body-part meshes. Each keyframe stores, for every part,
//! a rotation (a `CMatrix33`: three rows of three 10-bit fixed-point numbers) and an offset.
//! Between keyframes the rotation is slerped (`CQuaternion::BuildTween`) and the offset lerped.

use crate::prm::{face, Face, Prim};
use crate::reader::{count, Reader, Result};

/// One body (a "multi-prim"): the meshes for every part and variant, e.g. `pelvis00`,
/// `torso00`, `torso01` (alternative torsos).
#[derive(Debug, Clone)]
pub struct Body {
    pub parts: Vec<Prim>,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyFrame {
    /// Tween speed: the tween (0..256) advances `tween_step * 2` per game tick (15 per second).
    pub tween_step: u8,
    pub flags: i16,
    /// Index of this frame's first element in `AnimChunk::elements`; one element per part.
    pub first_element: Option<usize>,
    pub prev: Option<usize>,
    pub next: Option<usize>,
}

/// `ANIM_FLAG_LAST_FRAME` (Editor/Headers/Anim.h:28).
pub const FLAG_LAST_FRAME: i16 = 1 << 0;

/// One body part's pose in one keyframe.
#[derive(Debug, Clone, Copy)]
pub struct Element {
    /// Rotation, row-major (`m[row][col]`), already decoded from `CMatrix33`.
    pub rotation: [[f32; 3]; 3],
    /// Offset of the part from the character origin, in world units.
    pub offset: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct AnimChunk {
    pub bodies: Vec<Body>,
    /// Number of parts in a pose (15 for people).
    pub element_count: usize,
    /// `BodyDef`: per person variant, which mesh object to use for each part.
    pub people_types: Vec<[u8; 20]>,
    pub keyframes: Vec<KeyFrame>,
    pub elements: Vec<Element>,
    /// First keyframe of each animation, by animation id (`ANIM_*`).
    pub anims: Vec<Option<usize>>,
}

/// Decodes one `CMatrix33` row: three 10-bit numbers in bits 20-29, 10-19, 0-9, each /512,
/// wrapping values >= 1.0 to negative (`cmat_to_fmat`, DDEngine/Source/Quaternion.cpp:183).
fn cmat_row(v: u32) -> [f32; 3] {
    let f = |bits: u32| {
        let x = bits as f32 / 512.0;
        if x >= 1.0 { x - 2.0 } else { x }
    };
    [f((v & 0x3FF0_0000) >> 20), f((v & 0x000F_FC00) >> 10), f(v & 0x3FF)]
}

fn index(v: i32, len: usize) -> Option<usize> {
    (v >= 0 && (v as usize) < len).then_some(v as usize)
}

impl AnimChunk {
    pub fn parse(data: &[u8]) -> Result<AnimChunk> {
        let mut r = Reader::new(data);
        let save_type = r.i32()?;
        let nbodies = if save_type > 2 { let v = r.i32()?; count(&mut r, v)? } else { 1 };
        let mut bodies = Vec::with_capacity(nbodies);
        for _ in 0..nbodies {
            let multi_type = r.i32()?;
            let (so, eo) = (r.i32()?, r.i32()?);
            let mut parts = Vec::new();
            for _ in so..eo {
                parts.push(read_a_prim(&mut r, multi_type)?);
            }
            bodies.push(Body { parts });
        }

        let chunk_type = r.i32()?;
        if chunk_type <= 4 {
            return r.error(format!("keyframe chunk save_type {chunk_type} stores raw pointers; not supported"));
        }
        let element_count = { let v = r.i32()?; count(&mut r, v)? };
        let max_people = r.i16()?.max(0) as usize;
        let max_anims = r.i16()?.max(0) as usize;
        let max_elements = { let v = r.i32()?; count(&mut r, v)? };
        let max_keyframes = r.i16()?.max(0) as usize;
        let max_fights = r.i16()?.max(0) as usize;
        let sentinel = |r: &mut Reader| -> Result<()> {
            let v = r.u16()?;
            if v != 666 { r.error(format!("missing 666 sentinel (found {v})")) } else { Ok(()) }
        };

        let mut people_types = Vec::with_capacity(max_people);
        for _ in 0..max_people {
            let b = r.take(20)?;
            let mut a = [0u8; 20];
            a.copy_from_slice(b);
            people_types.push(a);
        }
        sentinel(&mut r)?;

        r.u32()?; // addr1 (original pointer base, unused for save_type > 4)
        let mut raw_frames = Vec::with_capacity(max_keyframes);
        for _ in 0..max_keyframes {
            let (_xyz, tween_step, flags) = (r.u8()?, r.u8()?, r.i16()?);
            let (first, prev, next, _fight) = (r.i32()?, r.i32()?, r.i32()?, r.i32()?);
            raw_frames.push((tween_step, flags, first, prev, next));
        }
        sentinel(&mut r)?;

        r.u32()?; // addr2
        let mut elements = Vec::with_capacity(max_elements);
        for _ in 0..max_elements {
            let rows = [r.u32()?, r.u32()?, r.u32()?];
            let offset = [r.i16()? as f32, r.i16()? as f32, r.i16()? as f32];
            r.u16()?; // Pad
            elements.push(Element { rotation: rows.map(cmat_row), offset });
        }
        sentinel(&mut r)?;

        let mut anims = Vec::with_capacity(max_anims);
        for _ in 0..max_anims {
            anims.push(index(r.i32()?, max_keyframes));
        }
        sentinel(&mut r)?;

        r.u32()?; // addr3
        r.skip(max_fights * 16)?; // GameFightCol
        sentinel(&mut r)?;
        r.expect_end()?;

        let keyframes = raw_frames
            .into_iter()
            .map(|(tween_step, flags, first, prev, next)| KeyFrame {
                tween_step,
                flags,
                first_element: index(first, max_elements),
                prev: index(prev, max_keyframes),
                next: index(next, max_keyframes),
            })
            .collect();
        Ok(AnimChunk { bodies, element_count, people_types, keyframes, elements, anims })
    }

    /// The mesh for part `i` of body `body`, as `FIGURE_draw` picks it:
    /// `start_object + PeopleTypes[person].BodyPart[i]`.
    pub fn part_mesh(&self, body: usize, person: usize, part: usize) -> Option<&Prim> {
        let offset = *self.people_types.get(person)?.get(part)? as usize;
        self.bodies.get(body)?.parts.get(offset)
    }

    /// The elements of keyframe `frame`, one per part.
    pub fn pose(&self, frame: usize) -> Option<&[Element]> {
        let first = self.keyframes.get(frame)?.first_element?;
        self.elements.get(first..first + self.element_count)
    }
}

/// `read_a_prim`: one mesh inside a multi-prim. Face point indices are rebased to 0.
fn read_a_prim(r: &mut Reader, save_type: i32) -> Result<Prim> {
    let name = r.cstr(32)?.trim().to_string();
    let (sp, ep) = (r.i32()?, r.i32()?);
    let mut points = Vec::new();
    for _ in sp..ep {
        if save_type > 3 {
            points.push([r.i16()?, r.i16()?, r.i16()?]);
        } else {
            points.push([r.i32()? as i16, r.i32()? as i16, r.i32()? as i16]);
        }
    }
    let base = sp as u16;
    let mut faces: Vec<Face> = Vec::new();
    let (sf3, ef3) = (r.i32()?, r.i32()?);
    for _ in sf3..ef3 {
        faces.push(face(r, 3, base)?);
    }
    let (sf4, ef4) = (r.i32()?, r.i32()?);
    for _ in sf4..ef4 {
        faces.push(face(r, 4, base)?);
    }
    if faces.iter().any(|f| f.points.iter().any(|&p| p as usize >= points.len())) {
        return r.error(format!("{name}: face refers to a point outside the mesh"));
    }
    Ok(Prim { save_type: save_type as u16, name, points, faces, coltype: 0, damage: 0, shadowtype: 0, flag: 0 })
}
