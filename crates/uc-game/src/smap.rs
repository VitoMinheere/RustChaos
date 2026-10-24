//! People's own shadows (#241, smap.cpp, aeng.cpp:7748-8300).
//!
//! The four people nearest the camera's focus, within six squares of it, each cast their own
//! shape: `SMAP_person` draws their figure seen from the light into a 32-pixel square map,
//! and `SMAP_project_onto_poly` lays that map across the ground around them, the walls beside
//! them and the faces they walk on, fading out from 64 to 256 off them
//! (`AENG_add_projected_fadeout_shadow_poly`). The map darkens what it falls on by half its
//! coverage (`GetShadowPixelMapping`: "not too shadowy").
//!
//! Here a map is made a frame at a time from the triangles the figure is drawn with; the
//! renderer draws it over the polygons given here, and leaves off where they run past it.

use glam::Vec3;

/// Where the light comes from: the direction it shines (`SMAP_person`'s 147, -148, -147).
pub const LIGHT: Vec3 = Vec3::new(147.0, -148.0, -147.0);
/// The map's size, pixels a side (`AENG_AA_BUF_SIZE`).
pub const RES: usize = 32;
/// How many people cast one (`AENG_NUM_SHADOWS`).
pub const MAX: usize = 4;
/// How far from the camera's focus, across plus along (`0x60000 >> 8`).
pub const RANGE: f32 = 0x600 as f32;

/// One person's shadow map.
#[derive(Debug, Clone)]
pub struct Smap {
    /// Coverage, 0 clear to 255 solid: `RES` rows of `RES`.
    pub pixels: Vec<u8>,
    /// The plane facing the light (`SMAP_plane_u`, `_v`, `_n`), through the world's origin.
    u: Vec3,
    v: Vec3,
    n: Vec3,
    /// The figure's extent along each.
    lo: Vec3,
    hi: Vec3,
    /// Where the shadow fades out from (`AENG_project_fadeout_x`, `_z`).
    focus: Vec3,
}

impl Smap {
    /// The map of a figure drawn with `tris` (world units), standing at `focus`.
    pub fn new(tris: &[[Vec3; 3]], focus: Vec3) -> Option<Smap> {
        // `SMAP_init`: the plane faces the light; u lies flat, across it.
        let n = (-LIGHT).normalize();
        let u = Vec3::new(-LIGHT.z, 0.0, LIGHT.x).normalize();
        let v = n.cross(u);
        let along = |p: Vec3| Vec3::new(p.dot(u), p.dot(v), p.dot(n));
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in tris.iter().flatten() {
            let a = along(*p);
            lo = lo.min(a);
            hi = hi.max(a);
        }
        if !(hi.x > lo.x && hi.y > lo.y) {
            return None;
        }
        let mut map = Smap { pixels: vec![0; RES * RES], u, v, n, lo, hi, focus };
        // Each pixel's coverage out of 4 by 4 samples (`AA_draw`'s antialiasing).
        const SUB: usize = 4;
        let size = RES * SUB;
        let mut hit = vec![false; size * size];
        for t in tris {
            let s = t.map(|p| {
                let [x, y] = map.uv(p);
                (x * size as f32, y * size as f32)
            });
            let edge = |a: (f32, f32), b: (f32, f32), p: (f32, f32)| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
            let area = edge(s[0], s[1], s[2]);
            if area.abs() < 1e-6 {
                continue;
            }
            let x0 = s.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
            let x1 = (s.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as usize).min(size);
            let y0 = s.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
            let y1 = (s.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as usize).min(size);
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = (x as f32 + 0.5, y as f32 + 0.5);
                    let (w0, w1, w2) = (edge(s[1], s[2], p), edge(s[2], s[0], p), edge(s[0], s[1], p));
                    // Either way round: the silhouette is the same.
                    if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                        hit[y * size + x] = true;
                    }
                }
            }
        }
        for y in 0..RES {
            for x in 0..RES {
                let mut n = 0;
                for sy in 0..SUB {
                    for sx in 0..SUB {
                        n += hit[(y * SUB + sy) * size + x * SUB + sx] as usize;
                    }
                }
                map.pixels[y * RES + x] = (n * 255 / (SUB * SUB)) as u8;
            }
        }
        Some(map)
    }

    /// Where `p` falls on the map, 0 to 1 across it: the figure a pixel in from each edge
    /// (`SMAP_point_finished`, `SMAP_convert_uvs`). Outside 0 to 1, it's off the map.
    pub fn uv(&self, p: Vec3) -> [f32; 2] {
        let res = RES as f32;
        let (du, dv) = ((res - 2.0) / (self.hi.x - self.lo.x), (res - 2.0) / (self.hi.y - self.lo.y));
        [(1.0 + (p.dot(self.u) - self.lo.x) * du) / res, (1.0 + (p.dot(self.v) - self.lo.y) * dv) / res]
    }

    /// Whether the map falls on the polygon `poly`, whose face is `normal`
    /// (`SMAP_project_onto_poly`): some of it is under the figure seen from the light, it
    /// faces the light, and it isn't on the light's side of the figure (`SMAP_wrong_side`:
    /// its middle nearer the light than three quarters of the figure).
    pub fn falls_on(&self, poly: &[Vec3], normal: Vec3) -> bool {
        let along: Vec<(f32, f32)> = poly.iter().map(|p| (p.dot(self.u), p.dot(self.v))).collect();
        let off = along.iter().all(|a| a.0 < self.lo.x) || along.iter().all(|a| a.0 > self.hi.x)
            || along.iter().all(|a| a.1 < self.lo.y) || along.iter().all(|a| a.1 > self.hi.y);
        if off || normal.dot(self.n) <= 0.0 {
            return false;
        }
        let mid = poly.iter().copied().sum::<Vec3>() / poly.len() as f32;
        mid.dot(self.n) < self.lo.z + (self.hi.z - self.lo.z) * 0.75
    }

    /// How strongly the shadow falls at `p`: all of it within 64 of the person (across plus
    /// along), none past 256.
    pub fn fade(&self, p: Vec3) -> u8 {
        let d = (p.x - self.focus.x).abs() + (p.z - self.focus.z).abs();
        (255.0 - ((d - 64.0) * 255.0 / 192.0).clamp(0.0, 255.0)) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box `w` wide and `h` tall standing at `at`, as triangles.
    fn post(at: Vec3, w: f32, h: f32) -> Vec<[Vec3; 3]> {
        let c = |x: f32, y: f32, z: f32| at + Vec3::new(x * w, y * h, z * w);
        let corners = [c(-0.5, 0.0, -0.5), c(0.5, 0.0, -0.5), c(0.5, 0.0, 0.5), c(-0.5, 0.0, 0.5)];
        let tops = corners.map(|p| p + Vec3::new(0.0, h, 0.0));
        let mut tris = Vec::new();
        for i in 0..4 {
            let j = (i + 1) % 4;
            tris.push([corners[i], corners[j], tops[j]]);
            tris.push([corners[i], tops[j], tops[i]]);
        }
        tris.push([tops[0], tops[1], tops[2]]);
        tris.push([tops[0], tops[2], tops[3]]);
        tris
    }

    #[test]
    fn the_map_is_the_figure_seen_from_the_light() {
        let at = Vec3::new(1000.0, 0.0, 1000.0);
        let m = Smap::new(&post(at, 40.0, 180.0), at).unwrap();
        // Solid in the middle, clear round the edge it's pushed in from.
        let solid = m.pixels.iter().filter(|&&p| p == 255).count();
        assert!(solid > RES * RES / 8, "only {solid} solid pixels");
        assert!((0..RES).all(|k| m.pixels[k] == 0 && m.pixels[k * RES] == 0), "the edge isn't clear");
        // Its own corners land inside it.
        for p in post(at, 40.0, 180.0).iter().flatten() {
            let [u, v] = m.uv(*p);
            assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v), "{p} at {u}, {v}");
        }
    }

    #[test]
    fn the_shadow_falls_away_from_the_light() {
        let at = Vec3::new(1000.0, 0.0, 1000.0);
        let m = Smap::new(&post(at, 40.0, 180.0), at).unwrap();
        // The light shines towards +x and -z: the ground that way is shaded, not the other.
        let ground = |dx: f32, dz: f32| {
            let c = at + Vec3::new(dx, 0.0, dz);
            [c, c + Vec3::new(0.0, 0.0, 64.0), c + Vec3::new(64.0, 0.0, 0.0)]
        };
        let shaded = |dx: f32, dz: f32| {
            let mid = at + Vec3::new(dx + 20.0, 0.0, dz + 20.0);
            let [u, v] = m.uv(mid);
            m.falls_on(&ground(dx, dz), Vec3::Y) && (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v)
                && m.pixels[(v * RES as f32) as usize * RES + (u * RES as f32) as usize] > 0
        };
        assert!(shaded(100.0, -140.0), "nothing on the ground away from the light");
        assert!(!shaded(-200.0, 160.0), "a shadow towards the light");
        // A floor facing down isn't lit, so takes no shadow.
        assert!(!m.falls_on(&ground(0.0, 0.0), -Vec3::Y));
    }

    #[test]
    fn it_fades_out_from_the_person() {
        let at = Vec3::new(1000.0, 0.0, 1000.0);
        let m = Smap::new(&post(at, 40.0, 180.0), at).unwrap();
        assert_eq!(m.fade(at + Vec3::new(30.0, 0.0, 30.0)), 255);
        assert_eq!(m.fade(at + Vec3::new(200.0, 0.0, 100.0)), 0);
        let half = m.fade(at + Vec3::new(160.0, 0.0, 0.0));
        assert!((100..160).contains(&half), "{half}");
    }
}
