//! 2D overlay (HUD): text in an 8x8 bitmap font and solid rectangles, in pixel coordinates
//! with (0, 0) at the top left. Build an [`Overlay`] each frame and pass it to
//! `Renderer::render`.

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct OVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

/// The font atlas: 16 x 8 cells of 8 x 8 pixels, one per ASCII code. Cell 0 is solid (for
/// rectangles). Returns (width, height, RGBA) with the glyph in alpha.
pub(crate) fn font_atlas() -> (u32, u32, Vec<u8>) {
    let (w, h) = (128u32, 64u32);
    let mut px = vec![255u8; (w * h * 4) as usize];
    for (c, rows) in font8x8::legacy::BASIC_LEGACY.iter().enumerate() {
        let (cx, cy) = ((c % 16) as u32 * 8, (c / 16) as u32 * 8);
        for (y, row) in rows.iter().enumerate() {
            for x in 0..8 {
                let on = c == 0 || row & (1 << x) != 0;
                px[(((cy + y as u32) * w + cx + x) * 4 + 3) as usize] = if on { 255 } else { 0 };
            }
        }
    }
    (w, h, px)
}

/// The font cell for `ch`. Mission texts are Windows-1252 read byte for byte: their curly
/// quotes and dashes (0x91-0x97) become plain ASCII, and accented letters the letter without
/// the accent; anything else outside ASCII is '?'.
fn glyph(ch: char) -> u8 {
    match ch as u32 {
        0x91 | 0x92 | 0x2018 | 0x2019 => b'\'',
        0x93 | 0x94 | 0x201C | 0x201D => b'"',
        0x96 | 0x97 | 0x2013 | 0x2014 => b'-',
        0x85 | 0x2026 => b'.',
        c if c < 128 => c as u8,
        0xC0..=0xFF => b"AAAAAAACEEEEIIIIDNOOOOOxOUUUUYTsaaaaaaaceeeeiiiidnooooo/ouuuuyty"[ch as usize - 0xC0],
        _ => b'?',
    }
}

/// Horizontal advance of each glyph in font pixels (proportional: lit width + 1).
fn advance(c: u8) -> f32 {
    let rows = &font8x8::legacy::BASIC_LEGACY[(c & 127) as usize];
    let right = rows.iter().map(|r| 8 - r.leading_zeros() as i32).max().unwrap_or(0);
    if right == 0 { 5.0 } else { right as f32 + 1.0 }
}

#[derive(Default)]
pub struct Overlay {
    pub(crate) verts: Vec<OVertex>,
    /// Quads textured with the backdrop image (`Renderer::set_backdrop`), drawn first.
    pub(crate) backdrop: Vec<OVertex>,
    /// Quads textured with the minimap image (`Renderer::set_minimap`), drawn next.
    pub(crate) minimap: Vec<OVertex>,
    /// Quads textured with the two face sheets (`Renderer::set_faces`), drawn last, over the
    /// text and rectangles.
    pub(crate) faces: [Vec<OVertex>; 2],
}

impl Overlay {
    pub fn new() -> Overlay {
        Overlay::default()
    }

    pub fn is_empty(&self) -> bool {
        self.verts.is_empty() && self.backdrop.is_empty() && self.minimap.is_empty() && self.faces.iter().all(|f| f.is_empty())
    }

    /// Part `uv` ([u0, v0, u1, v1]) of the minimap image over a rectangle, tinted by `color`.
    pub fn minimap(&mut self, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], color: [u8; 4]) {
        let start = self.verts.len();
        self.quad(x, y, w, h, uv, color);
        let quad: Vec<OVertex> = self.verts.drain(start..).collect();
        self.minimap.extend(quad);
    }

    /// Part `uv` of face sheet `sheet` (0 or 1) over a rectangle.
    pub fn face(&mut self, sheet: usize, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4]) {
        let start = self.verts.len();
        self.quad(x, y, w, h, uv, [255, 255, 255, 255]);
        let quad: Vec<OVertex> = self.verts.drain(start..).collect();
        self.faces[sheet.min(1)].extend(quad);
    }

    /// The backdrop image stretched over a rectangle, tinted by `color`.
    pub fn backdrop(&mut self, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
        let start = self.verts.len();
        self.quad(x, y, w, h, [0.0, 0.0, 1.0, 1.0], color);
        let quad: Vec<OVertex> = self.verts.drain(start..).collect();
        self.backdrop.extend(quad);
    }

    fn quad(&mut self, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], color: [u8; 4]) {
        let [u0, v0, u1, v1] = uv;
        let v = |px: f32, py: f32, u: f32, vv: f32| OVertex { pos: [px, py], uv: [u, vv], color };
        let (a, b, c, d) = (v(x, y, u0, v0), v(x + w, y, u1, v0), v(x, y + h, u0, v1), v(x + w, y + h, u1, v1));
        self.verts.extend_from_slice(&[a, b, c, b, d, c]);
    }

    /// A solid rectangle.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
        // The middle of the solid cell 0.
        self.quad(x, y, w, h, [2.0 / 128.0, 2.0 / 64.0, 6.0 / 128.0, 6.0 / 64.0], color);
    }

    /// Width of `s` at `scale` (font pixels are `scale` screen pixels).
    pub fn text_width(s: &str, scale: f32) -> f32 {
        s.chars().map(glyph).map(advance).sum::<f32>() * scale
    }

    /// Draws `s` with a drop shadow; returns its width (see `glyph` for non-ASCII).
    pub fn text(&mut self, x: f32, y: f32, scale: f32, color: [u8; 4], s: &str) -> f32 {
        let shadow = [0, 0, 0, color[3]];
        for (dx, col) in [(scale.max(1.0), shadow), (0.0, color)] {
            let mut cx = x + dx;
            for ch in s.chars() {
                let c = glyph(ch);
                if c > b' ' {
                    let (gx, gy) = ((c % 16) as f32 * 8.0, (c / 16) as f32 * 8.0);
                    self.quad(cx, y + dx, 8.0 * scale, 8.0 * scale, [gx / 128.0, gy / 64.0, (gx + 8.0) / 128.0, (gy + 8.0) / 64.0], col);
                }
                cx += advance(c) * scale;
            }
        }
        Overlay::text_width(s, scale)
    }

    /// Splits `s` into lines no wider than `max_width`.
    pub fn wrap(s: &str, max_width: f32, scale: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in s.lines() {
            let mut line = String::new();
            for word in para.split_whitespace() {
                let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && Overlay::text_width(&candidate, scale) > max_width {
                    lines.push(std::mem::replace(&mut line, word.to_string()));
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::glyph;

    #[test]
    fn accented_letters_lose_their_accents() {
        let plain: String = "Sören Übersetzung françaises Stéphan ÿ".chars().map(|c| glyph(c) as char).collect();
        assert_eq!(plain, "Soren Ubersetzung francaises Stephan y");
        assert_eq!(glyph('\u{92}'), b'\'');
        assert_eq!(glyph('漢'), b'?');
    }
}
