//! Ribbons (fallen/Source/ribbon.cpp): strips of triangles through the last few points
//! something has left behind, drawn by `RIBBON_draw_ribbon` (drawxtra.cpp:2549). The bike's
//! exhaust is two of them (bike.cpp:1621).
//!
//! Each ribbon keeps its points in a ring. Every three points in a row make a triangle, so
//! points laid in pairs make a strip. A fading ribbon is clear at its oldest point and solid
//! [`Ribbon::fade`] points on. Its texture slides along it as it goes.

use glam::Vec3;
use std::collections::BTreeMap;
use uc_world::Batch;

/// `MAX_RIBBONS`.
pub const MAX_RIBBONS: usize = 64;
/// `MAX_RIBBON_SIZE`: the most points a ribbon keeps.
pub const MAX_SIZE: usize = 32;

/// `RIBBON_FLAG_FADE`: clear at the oldest end, else all [`Ribbon::fade`] alpha.
pub const FADE: u8 = 2;
/// `RIBBON_FLAG_SLIDE`.
pub const SLIDE: u8 = 4;
/// `RIBBON_FLAG_CONVECT`: its points rise 22 a turn.
pub const CONVECT: u8 = 16;
/// `RIBBON_FLAG_IALPHA`: the fade goes up from the oldest point rather than down.
pub const IALPHA: u8 = 32;

/// `Ribbon`.
#[derive(Debug, Clone)]
pub struct Ribbon {
    pub flags: u8,
    /// `POLY_PAGE`: the texture page it's drawn with.
    pub page: u32,
    /// Turns left, or -1 for ever.
    pub life: i32,
    pub rgb: [u8; 3],
    /// The ring's length (`Size`): it holds one point fewer.
    size: usize,
    /// Where the next point goes.
    head: usize,
    /// The oldest point.
    tail: usize,
    /// How far the texture has slid along it, in 64ths (`Scroll`).
    pub scroll: u8,
    /// Over how many points it fades in (`FadePoint`), or its alpha if it doesn't fade.
    pub fade: u8,
    /// Added to [`Ribbon::scroll`] each turn (`SlideSpeed`).
    pub slide: u8,
    /// How many times the texture goes across it (`TextureU`).
    tex_u: u8,
    /// How many points one pass of the texture along it takes (`TextureV`).
    tex_v: u8,
    points: [Vec3; MAX_SIZE],
}

impl Ribbon {
    /// Its points, oldest first.
    pub fn points(&self) -> impl Iterator<Item = Vec3> + '_ {
        let n = (self.head + self.size - self.tail) % self.size;
        (0..n).map(move |k| self.points[(self.tail + k) % self.size])
    }

    /// `RIBBON_draw_ribbon` (drawxtra.cpp:2549): a triangle for every three points in a row,
    /// oldest first. Across, the texture's u alternates 0 and [`Ribbon::tex_u`] by the point's
    /// place in the ring; along, its v is that place over [`Ribbon::tex_v`], slid on by
    /// [`Ribbon::scroll`].
    fn draw(&self, out: &mut Batch) {
        let n = (self.head + self.size - self.tail) % self.size;
        // `RIBBON_draw`: only once it has two points (`RIBBON_length(i) > 2`).
        if n < 2 {
            return;
        }
        let (vo, vs) = (self.scroll as f32 / 64.0, 1.0 / self.tex_v as f32);
        let [r, g, b] = self.rgb;
        let mut verts = Vec::with_capacity(n);
        for ctr in 0..n {
            let i = (self.tail + ctr) % self.size;
            let alpha = if self.flags & FADE == 0 {
                self.fade
            } else {
                let id = (256 * ctr as i32) / self.fade.max(1) as i32;
                let id = if self.flags & IALPHA != 0 { id } else { 255 - id };
                id.clamp(0, 255) as u8
            };
            let u = if i & 1 == 1 { 0.0 } else { self.tex_u as f32 };
            verts.push((self.points[i] / uc_world::UNIT, [u, i as f32 * vs + vo], [r, g, b, alpha]));
        }
        for w in verts.windows(3) {
            out.tri([w[0].0, w[1].0, w[2].0], [w[0].1, w[1].1, w[2].1], [w[0].2, w[1].2, w[2].2]);
        }
    }
}

/// Every ribbon (`Ribbons`).
#[derive(Debug, Clone)]
pub struct Ribbons {
    list: Vec<Option<Ribbon>>,
    /// Where `RIBBON_alloc` looks for a free one next (`NextRibbon`).
    next: usize,
}

impl Default for Ribbons {
    fn default() -> Ribbons {
        Ribbons { list: vec![None; MAX_RIBBONS], next: 0 }
    }
}

impl Ribbons {
    /// `RIBBON_alloc`: a ribbon of up to `size` points on `page`, living `life` turns (-1 for
    /// ever), fading over `fade` points, its texture sliding `slide` 64ths a turn, `u` times
    /// across and once along every `v` points (0 for all of it). None if there's no room.
    #[allow(clippy::too_many_arguments)]
    pub fn alloc(&mut self, flags: u8, size: usize, page: u32, life: i32, fade: u8, slide: u8, u: u8, v: u8, rgb: [u8; 3]) -> Option<usize> {
        let k = (0..MAX_RIBBONS).map(|k| (self.next + k) % MAX_RIBBONS).find(|&k| self.list[k].is_none())?;
        self.next = (k + 1) % MAX_RIBBONS;
        let size = size.min(MAX_SIZE);
        let tex_v = if v == 0 { size as u8 } else { v };
        self.list[k] = Some(Ribbon {
            flags, page, life, rgb, size, head: 0, tail: 0, scroll: 0, fade, slide, tex_u: u, tex_v,
            points: [Vec3::ZERO; MAX_SIZE],
        });
        Some(k)
    }

    /// `RIBBON_free`.
    pub fn free(&mut self, k: usize) {
        self.list[k] = None;
    }

    pub fn get(&self, k: usize) -> Option<&Ribbon> {
        self.list.get(k)?.as_ref()
    }

    /// `RIBBON_extend`: a new point at the head, the oldest dropping off once it's full.
    pub fn extend(&mut self, k: usize, at: Vec3) {
        let Some(Some(r)) = self.list.get_mut(k) else { return };
        r.points[r.head] = at;
        r.head = (r.head + 1) % r.size;
        if r.head == r.tail {
            r.tail = (r.tail + 1) % r.size;
        }
    }

    /// `RIBBON_life`.
    pub fn set_life(&mut self, k: usize, life: i32) {
        if let Some(Some(r)) = self.list.get_mut(k) {
            r.life = life;
        }
    }

    /// `RIBBON_process`, a turn: each slides its texture on, rises if it convects, and ages,
    /// going once its life is up.
    pub fn process(&mut self) {
        for slot in &mut self.list {
            let Some(r) = slot else { continue };
            r.scroll = r.scroll.wrapping_add(r.slide);
            if r.flags & CONVECT != 0 {
                for p in &mut r.points[..r.size] {
                    p.y += 22.0;
                }
            }
            if r.life > 0 {
                r.life -= 1;
            }
            if r.life == 0 {
                *slot = None;
            }
        }
    }

    /// `RIBBON_draw`: each into its page's batch.
    pub fn draw(&self, out: &mut BTreeMap<u32, Batch>) {
        for r in self.list.iter().flatten() {
            r.draw(out.entry(r.page).or_default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ribbon(r: &mut Ribbons) -> usize {
        r.alloc(FADE | SLIDE | IALPHA, 10, 0, -1, 8, 6, 1, 0, [0x6f; 3]).unwrap()
    }

    /// A ribbon of ten keeps the last nine points, oldest first.
    #[test]
    fn keeps_the_last_points() {
        let mut r = Ribbons::default();
        let k = ribbon(&mut r);
        for x in 0..20 {
            r.extend(k, Vec3::new(x as f32, 0.0, 0.0));
        }
        let xs: Vec<f32> = r.get(k).unwrap().points().map(|p| p.x).collect();
        assert_eq!(xs, (11..20).map(|x| x as f32).collect::<Vec<_>>());
    }

    /// Clear at the oldest point, solid from the eighth on; three points make a triangle.
    #[test]
    fn fades_in_from_the_oldest() {
        let mut r = Ribbons::default();
        let k = ribbon(&mut r);
        for x in 0..20 {
            r.extend(k, Vec3::new(x as f32, (x % 2) as f32 * 70.0, 0.0));
        }
        let mut out = BTreeMap::new();
        r.draw(&mut out);
        let b = &out[&0];
        assert_eq!(b.indices.len(), 7 * 3);
        let alphas: Vec<u8> = b.vertices.iter().step_by(3).map(|v| v.color[3]).collect();
        assert_eq!(alphas, vec![0, 32, 64, 96, 128, 160, 192]);
        assert_eq!(b.vertices.last().unwrap().color[3], 255);
    }

    /// The texture slides along each turn; a ribbon with a life goes when it's up.
    #[test]
    fn slides_and_ages() {
        let mut r = Ribbons::default();
        let k = ribbon(&mut r);
        let short = r.alloc(0, 4, 0, 2, 255, 0, 1, 0, [255; 3]).unwrap();
        assert_ne!(k, short);
        r.process();
        assert_eq!(r.get(k).unwrap().scroll, 6);
        assert!(r.get(short).is_some());
        r.process();
        assert!(r.get(short).is_none());
        assert!(r.get(k).is_some());
    }

    /// No more than [`MAX_RIBBONS`] at once.
    #[test]
    fn runs_out() {
        let mut r = Ribbons::default();
        for _ in 0..MAX_RIBBONS {
            ribbon(&mut r);
        }
        assert!(r.alloc(0, 4, 0, -1, 0, 0, 1, 0, [0; 3]).is_none());
        r.free(5);
        assert_eq!(r.alloc(0, 4, 0, -1, 0, 0, 1, 0, [0; 3]), Some(5));
    }
}
