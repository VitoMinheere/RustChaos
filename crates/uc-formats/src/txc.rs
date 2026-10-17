//! `.txc` texture clumps: `FileClump` (fallen/DDLibrary/Source/FileClump.cpp:12) holding
//! textures compressed by `TGA_write_compressed`/`WriteSquished` (fallen/DDLibrary/Source/Tga.cpp).
//! The clump id is the texture page number. A page's crinkle, if it has one, is at
//! [`CRINKLE_BASE`] + page.

use crate::reader::{Reader, Result};
use std::collections::BTreeMap;

pub struct Clump {
    pub max_id: u32,
    entries: BTreeMap<u32, (usize, usize)>,
    data: Vec<u8>,
}

/// A decoded texture, RGBA8, top row first.
#[derive(Debug, Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub rgba: Vec<u8>,
}

impl Clump {
    pub fn parse(data: Vec<u8>) -> Result<Clump> {
        let mut r = Reader::new(&data);
        let max_id = r.u32()?;
        let mut offsets = Vec::with_capacity(max_id as usize);
        for _ in 0..max_id {
            offsets.push(r.u32()? as usize);
        }
        let mut entries = BTreeMap::new();
        for (id, &off) in offsets.iter().enumerate() {
            let len = r.u32()? as usize;
            if off == 0 {
                continue;
            }
            if off + len > data.len() {
                return r.error(format!("entry {id} runs past the end of the file"));
            }
            entries.insert(id as u32, (off, len));
        }
        Ok(Clump { max_id, entries, data })
    }

    /// A clump with nothing in it, to fill from others.
    pub fn empty() -> Clump {
        Clump { max_id: 0, entries: BTreeMap::new(), data: Vec::new() }
    }

    /// Copies in the entries of `other` this clump lacks and `take` accepts. Returns how
    /// many it copied.
    pub fn fill_from(&mut self, other: &Clump, take: impl Fn(u32) -> bool) -> usize {
        let mut n = 0;
        for (&id, &(off, len)) in &other.entries {
            if self.entries.contains_key(&id) || !take(id) {
                continue;
            }
            self.entries.insert(id, (self.data.len(), len));
            self.data.extend_from_slice(&other.data[off..off + len]);
            self.max_id = self.max_id.max(id + 1);
            n += 1;
        }
        n
    }

    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.keys().copied()
    }

    pub fn raw(&self, id: u32) -> Option<&[u8]> {
        self.entries.get(&id).map(|&(o, n)| &self.data[o..o + n])
    }

    /// The crinkle of texture page `page`, if it has one (`TEXTURE_load_page`,
    /// DDEngine/Source/texture.cpp:1057). Only pages below 512 can have one.
    pub fn crinkle(&self, page: u32) -> Result<Option<Crinkle>> {
        if page >= CRINKLE_PAGES {
            return Ok(None);
        }
        match self.raw(CRINKLE_BASE + page) {
            Some(blob) => Crinkle::parse(blob).map(Some),
            None => Ok(None),
        }
    }

    /// Decodes entry `id`. Returns `Ok(None)` if it is absent or not an image (width 0).
    pub fn texture(&self, id: u32) -> Result<Option<Texture>> {
        match self.raw(id) {
            Some(blob) => decode(blob),
            None => Ok(None),
        }
    }
}

/// The clump id of page 0's crinkle: `TEXTURE_MAX_TEXTURES` (texture.cpp:1057).
pub const CRINKLE_BASE: u32 = 1568;
/// Pages below this can have a crinkle (`64 * 8`, texture.cpp:1028 and facet.cpp:1718).
pub const CRINKLE_PAGES: u32 = 512;

/// The texture page an entry belongs to: itself, or the page whose crinkle it is.
pub fn entry_page(id: u32) -> u32 {
    if (CRINKLE_BASE..CRINKLE_BASE + CRINKLE_PAGES).contains(&id) { id - CRINKLE_BASE } else { id }
}

/// A crinkle: relief that stands in for a flat wall quad up close (DDEngine/Source/Crinkle.cpp).
#[derive(Debug, Clone, PartialEq)]
pub struct Crinkle {
    pub points: Vec<CrinklePoint>,
    pub faces: Vec<[u16; 3]>,
}

/// One point of a crinkle (`CRINKLE_Point`, Crinkle.cpp:54).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrinklePoint {
    /// Across the quad from corner 0 towards corner 1, 0 to 1.
    pub u: f32,
    /// Across the quad from corner 0 towards corner 2, 0 to 1.
    pub v: f32,
    /// How far the point stands out of the wall.
    pub out: f32,
    /// How much of each corner's colour it takes, out of 128.
    pub w: [u8; 4],
}

impl Crinkle {
    /// `CRINKLE_read_bin` (Crinkle.cpp:365): `num_points`, `num_faces` and two stale
    /// pointers, then the points and the faces.
    pub fn parse(blob: &[u8]) -> Result<Crinkle> {
        let mut r = Reader::new(blob);
        let (np, nf) = (r.i32()?, r.i32()?);
        r.skip(8)?;
        if np < 0 || nf < 0 || np as usize * 16 + nf as usize * 6 > r.remaining() {
            return r.error(format!("crinkle of {np} points and {nf} faces doesn't fit"));
        }
        let f32 = |r: &mut Reader| r.u32().map(f32::from_bits);
        let mut points = Vec::with_capacity(np as usize);
        for _ in 0..np {
            let (u, v, out) = (f32(&mut r)?, f32(&mut r)?, f32(&mut r)?);
            let w = [r.u8()?, r.u8()?, r.u8()?, r.u8()?];
            points.push(CrinklePoint { u, v, out, w });
        }
        let mut faces = Vec::with_capacity(nf as usize);
        for _ in 0..nf {
            let f = [r.u16()?, r.u16()?, r.u16()?];
            if f.iter().any(|&i| i as usize >= points.len()) {
                return r.error("crinkle face points past its points");
            }
            faces.push(f);
        }
        Ok(Crinkle { points, faces })
    }
}

/// `ReadSquished` + `TGA_read_compressed`.
pub fn decode(blob: &[u8]) -> Result<Option<Texture>> {
    let mut r = Reader::new(blob);
    let mut words = Vec::with_capacity(blob.len() / 2);
    while r.remaining() >= 2 {
        words.push(r.u16()?);
    }
    if words.len() < 3 {
        return r.error("texture blob too short");
    }
    let (header, pixels): ([u16; 3], Vec<u16>) = if words[0] == 0xFFFF {
        if words.len() < 5 {
            return r.error("squished texture header too short");
        }
        let header = [words[1], words[2], words[3]];
        let total = words[4] as usize;
        let mapping = words.get(5..5 + total).ok_or_else(|| err(&r, "palette runs past end"))?;
        let mut bits = 1;
        while total > (1 << bits) {
            bits += 1;
        }
        let n = header[1] as usize * header[2] as usize;
        let mut out = Vec::with_capacity(n);
        let (mut pos, mut acc, mut nacc) = (5 + total, 0u32, 0u32);
        for _ in 0..n {
            while nacc < bits {
                let w = *words.get(pos).ok_or_else(|| err(&r, "packed indices run past end"))?;
                acc = (acc << 16) | w as u32;
                pos += 1;
                nacc += 16;
            }
            nacc -= bits;
            let idx = ((acc >> nacc) & ((1 << bits) - 1)) as usize;
            acc &= (1u32 << nacc) - 1;
            out.push(*mapping.get(idx).ok_or_else(|| err(&r, "palette index out of range"))?);
        }
        (header, out)
    } else {
        ([words[0], words[1], words[2]], words[3..].to_vec())
    };
    let (alpha, width, height) = (header[0] != 0, header[1] as u32, header[2] as u32);
    if width == 0 || height == 0 {
        return Ok(None);
    }
    let n = (width * height) as usize;
    if pixels.len() < n {
        return r.error("not enough pixel data");
    }
    let mut rgba = Vec::with_capacity(n * 4);
    for &p in &pixels[..n] {
        if alpha {
            // ARGB4444, expanded like C4to8.
            let x4 = |v: u16| ((v & 15) as u8) * 17;
            rgba.extend_from_slice(&[x4(p >> 8), x4(p >> 4), x4(p), x4(p >> 12)]);
        } else {
            // RGB565, expanded like C5to8/C6to8.
            let (r5, g6, b5) = ((p >> 11) as u8, ((p >> 5) & 63) as u8, (p & 31) as u8);
            rgba.extend_from_slice(&[(r5 << 3) | (r5 >> 2), (g6 << 2) | (g6 >> 4), (b5 << 3) | (b5 >> 2), 255]);
        }
    }
    Ok(Some(Texture { width, height, has_alpha: alpha, rgba }))
}

fn err(r: &Reader, msg: &str) -> crate::reader::ParseError {
    crate::reader::ParseError { offset: r.pos(), message: msg.into() }
}
