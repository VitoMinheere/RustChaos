//! `.txc` texture clumps: `FileClump` (fallen/DDLibrary/Source/FileClump.cpp:12) holding
//! textures compressed by `TGA_write_compressed`/`WriteSquished` (fallen/DDLibrary/Source/Tga.cpp).
//! The clump id is the texture page number.

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

    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.keys().copied()
    }

    pub fn raw(&self, id: u32) -> Option<&[u8]> {
        self.entries.get(&id).map(|&(o, n)| &self.data[o..o + n])
    }

    /// Decodes entry `id`. Returns `Ok(None)` if it is absent or not an image (width 0).
    pub fn texture(&self, id: u32) -> Result<Option<Texture>> {
        match self.raw(id) {
            Some(blob) => decode(blob),
            None => Ok(None),
        }
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
