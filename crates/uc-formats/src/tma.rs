//! `style.tma`: the wall texture style table for one texture world
//! (`server\textures\world<N>\style.tma`). Loaded by `load_texture_styles`
//! (fallen/Source/io.cpp:361) and turned into page numbers by
//! `TEXTURE_fix_texture_styles` (fallen/DDEngine/Source/texture.cpp:2450).

use crate::reader::{Reader, Result};

pub const STYLES: usize = 200;
pub const PIECES: usize = 5;

/// `TEXTURE_PIECE_*` (fallen/Headers/building.h:12).
pub mod piece {
    pub const LEFT: usize = 0;
    pub const MIDDLE: usize = 1;
    pub const RIGHT: usize = 2;
    pub const MIDDLE1: usize = 3;
    pub const MIDDLE2: usize = 4;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StylePiece {
    /// Texture page (same numbering as clump ids and floor pages).
    pub page: u32,
    /// 0 none, 1 mirror horizontally, 2 mirror vertically, 3 both.
    pub flip: u8,
}

#[derive(Debug, Clone)]
pub struct StyleTable {
    pub pieces: Vec<[StylePiece; PIECES]>,
}

impl StyleTable {
    pub fn parse(data: &[u8]) -> Result<StyleTable> {
        let mut r = Reader::new(data);
        let save_type = r.i32()?;
        if save_type <= 1 {
            return r.error(format!("unsupported style.tma save_type {save_type}"));
        }
        if save_type < 4 {
            let n = r.u16()? as usize;
            r.skip(2 * 8 * 8 * n)?; // TextureInfo
        }
        let (styles, per) = (r.u16()? as usize, r.u16()? as usize);
        if styles != STYLES {
            return r.error(format!("expected {STYLES} styles, found {styles}"));
        }
        let mut pieces = vec![[StylePiece::default(); PIECES]; STYLES];
        for style in pieces.iter_mut() {
            let (skip, read) = if save_type < 5 { (3, per - 3) } else { (0, per) };
            r.skip(4 * skip)?;
            for p in 0..read {
                let (page, tx, ty, flip) = (r.u8()?, r.u8()?, r.u8()?, r.u8()?);
                if let Some(slot) = style.get_mut(p) {
                    // TEXTURE_fix_texture_styles: (Tx*32/32) + (Ty*32/32)*8 + Page*64.
                    *slot = StylePiece { page: tx as u32 + ty as u32 * 8 + page as u32 * 64, flip };
                }
            }
        }
        Ok(StyleTable { pieces })
    }

    /// `dx_textures_xy[style][piece]`; style 0 is treated as 1, like the engine.
    pub fn piece(&self, style: usize, piece: usize) -> StylePiece {
        let style = if style == 0 { 1 } else { style };
        self.pieces.get(style).map(|s| s[piece]).unwrap_or_default()
    }
}
