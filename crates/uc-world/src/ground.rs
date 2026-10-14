//! Where people can stand and what stops them: terrain, rooftops, walls and ladders.
//!
//! Simpler than the original's walkable-face system (`find_face_for_this_pos`, walkable.cpp):
//! a person stands on the highest surface at their (x, z) that is at most a step above their
//! feet. Surfaces are the terrain, flat `ROOF_EXISTS` squares, sloped roof faces and the
//! tops of walkable props (crates, walkways: `PRIM_FLAG_CONTAINS_WALKABLE_FACES`).
//!
//! Props block as the original's collision models say (`PRIM_COLLIDE_*`, Editor/Headers/Prim.h):
//! boxes as their rotated bounding box, cylinders as an octagon, the rest not at all.

use glam::{vec2, vec3, Vec2, Vec3};
use std::collections::HashMap;
use std::path::Path;
use uc_formats::facet::{collision_segments, door_segments, WallSegment};
use uc_formats::iam::{facet_type, flags, Map, SQUARE};

/// `FACE_FLAG_WALKABLE` (Editor/Headers/Prim.h): a prim face people can stand on.
const FACE_FLAG_WALKABLE: u16 = 1 << 6;

/// The height of triangle `t` at (x, z), if (x, z) is inside it.
fn height_in(t: &[Vec3; 3], x: f32, z: f32) -> Option<f32> {
    let (a, b, c) = (vec2(t[0].x, t[0].z), vec2(t[1].x, t[1].z), vec2(t[2].x, t[2].z));
    let p = vec2(x, z);
    let d = (b - a).perp_dot(c - a);
    if d.abs() < 1e-6 {
        return None;
    }
    let u = (p - a).perp_dot(c - a) / d;
    let v = (b - a).perp_dot(p - a) / d;
    (u >= -1e-3 && v >= -1e-3 && u + v <= 1.0 + 1e-3).then(|| t[0].y + (t[1].y - t[0].y) * u + (t[2].y - t[0].y) * v)
}

/// `ROOF_SHIFT` (Editor/Headers/Prim.h): RoofFace4.DY is in units of 8.
const ROOF_SHIFT: i32 = 3;
/// How far up a person can step without climbing (kerbs are 32).
pub const STEP_HEIGHT: f32 = 48.0;

/// A ladder facet, in world units.
#[derive(Debug, Clone, Copy)]
pub struct Ladder {
    /// Centre of the ladder at its foot (xz).
    pub foot: Vec2,
    /// Unit direction from the wall out towards a climber.
    pub out: Vec2,
    pub base: f32,
    pub top: f32,
}

/// The flat top of a walkable prop: a rotated rectangle at height `top`.
#[derive(Debug, Clone, Copy)]
struct PropTop {
    origin: Vec2,
    /// Rotation (cos, sin) of the prop's yaw.
    rot: Vec2,
    lo: Vec2,
    hi: Vec2,
    top: f32,
}

impl PropTop {
    fn contains(&self, p: Vec2) -> bool {
        // World to prop space: the inverse of (c*x + s*z, -s*x + c*z).
        let d = p - self.origin;
        let (c, s) = (self.rot.x, self.rot.y);
        let l = vec2(c * d.x - s * d.y, s * d.x + c * d.y);
        l.x >= self.lo.x && l.x <= self.hi.x && l.y >= self.lo.y && l.y <= self.hi.y
    }
}

impl Ground {
    /// Where the platforms' tops are now: (where, yaw in radians, the box in the prop's own
    /// space, the height of the top).
    pub fn set_platforms(&self, tops: impl IntoIterator<Item = (Vec2, f32, Vec2, Vec2, f32)>) {
        *self.moving.borrow_mut() = tops.into_iter()
            .map(|(origin, yaw, lo, hi, top)| {
                let (s, c) = yaw.sin_cos();
                PropTop { origin, rot: vec2(c, s), lo, hi, top }
            })
            .collect();
    }

    /// Whether (x, z) is over platform top `t` (by its place in [`Ground::set_platforms`]),
    /// `margin` either way.
    pub fn over_platform(&self, t: usize, x: f32, z: f32, margin: f32) -> bool {
        self.moving.borrow().get(t).is_some_and(|p| {
            let grown = PropTop { lo: p.lo - Vec2::splat(margin), hi: p.hi + Vec2::splat(margin), ..*p };
            grown.contains(vec2(x, z))
        })
    }
}

/// A cable (`STOREY_TYPE_CABLE`) as a polyline in world units, sag included.
#[derive(Debug, Clone)]
pub struct Cable {
    pub points: Vec<Vec3>,
}

impl Cable {
    /// The closest point of the cable to `p`: (position, distance along the cable 0..1).
    pub fn closest(&self, p: Vec3) -> (Vec3, f32) {
        let n = self.points.len().saturating_sub(1).max(1);
        let mut best = (self.points[0], 0.0, f32::MAX);
        for (i, w) in self.points.windows(2).enumerate() {
            let ab = w[1] - w[0];
            let t = ((p - w[0]).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            let q = w[0] + ab * t;
            let d = (q - p).length();
            if d < best.2 {
                best = (q, (i as f32 + t) / n as f32, d);
            }
        }
        (best.0, best.1)
    }

    /// The point at distance fraction `t` (0..1) along the cable.
    pub fn at(&self, t: f32) -> Vec3 {
        let n = self.points.len() - 1;
        let f = (t.clamp(0.0, 1.0) * n as f32).min(n as f32 - 1e-4);
        let i = f.floor() as usize;
        self.points[i].lerp(self.points[i + 1], f - i as f32)
    }

    /// Length in world units.
    pub fn length(&self) -> f32 {
        self.points.windows(2).map(|w| (w[1] - w[0]).length()).sum()
    }
}

/// The points of cable facet `f` (`cable_draw`, facet.cpp): `Height` segments; the sag follows a
/// cosine whose angle starts at -512 and steps by StyleIndex, then by Building once past -30.
pub fn cable_points(f: &uc_formats::iam::Facet) -> Vec<Vec3> {
    let (x1, z1, x2, z2) = (f.x[0] as f32 * 256.0, f.z[0] as f32 * 256.0, f.x[1] as f32 * 256.0, f.z[1] as f32 * 256.0);
    let count = f.height.max(1) as i32;
    let (y1, y2) = (f.y[0] as f32, f.y[1] as f32);
    let sag = f.fheight as f32 * 64.0;
    let (mut angle, mut dangle) = (-512i32, f.style_index as i16 as i32);
    let mut pts = Vec::new();
    for i in 0..=count {
        let t = i as f32 / count as f32;
        let a = ((angle + 2048) & 2047) as f32 * std::f32::consts::TAU / 2048.0;
        pts.push(vec3(x1 + (x2 - x1) * t, y1 + (y2 - y1) * t - a.cos() * sag, z1 + (z2 - z1) * t));
        angle += dangle;
        if angle >= -30 {
            dangle = f.building as i16 as i32;
        }
    }
    pts
}

pub struct Ground {
    /// Roof corner heights per square, corners (x,z) (x+1,z) (x,z+1) (x+1,z+1).
    roofs: HashMap<(usize, usize), [f32; 4]>,
    /// Walkable prop tops by map square (a prop is listed in every square it covers).
    prop_tops: HashMap<(i32, i32), Vec<PropTop>>,
    /// Walkable prop faces (`FACE_FLAG_WALKABLE`: stair treads, ramps) as world triangles,
    /// by map square.
    prop_tris: HashMap<(i32, i32), Vec<[Vec3; 3]>>,
    pub walls: Vec<WallSegment>,
    /// The map's doors: solid until the mission opens one.
    pub doors: Vec<Door>,
    pub ladders: Vec<Ladder>,
    pub cables: Vec<Cable>,
    /// Fences that can be climbed (chain-link: `STOREY_TYPE_FENCE`, `_FENCE_FLAT`).
    pub fences: Vec<WallSegment>,
    /// The tops of moving platforms where they are now (`WMOVE`): the game moves them, and
    /// they're in a `RefCell` for the same reason the doors' state is in a `Cell`.
    moving: std::cell::RefCell<Vec<PropTop>>,
    /// Which of `walls` each prop's collision outline is: (index into the map's objects,
    /// range of walls).
    pub prop_walls: Vec<(usize, std::ops::Range<usize>)>,
    /// Each prop's prim's `damage` and `flag` bytes, by index into the map's objects: whether
    /// it can be damaged, and whether it blows up when it is (`PRIM_DAMAGE_*`, `PRIM_FLAG_*`).
    pub prop_damage: Vec<(u8, u8)>,
}

/// A door. It blocks like any other wall until a mission opens it, which is the only way one
/// ever opens: `DOOR_open`'s only caller is `EWAY_DO_CONTROL_DOOR` (fallen/Source/eway.cpp),
/// there is no walking into them and no "use" on them.
///
/// Whether it's open belongs to the mission being played rather than to the map, and a `Level`
/// is loaded afresh for each mission, so the flag sits here in a `Cell`. That keeps every
/// collision call taking `&Ground` as it did, instead of threading a set of open doors through
/// each person's movement.
#[derive(Debug)]
pub struct Door {
    pub wall: WallSegment,
    /// The middle of the facet, which `DOOR_find` measures its search from.
    pub mid: Vec2,
    open: std::cell::Cell<bool>,
}

impl Door {
    pub fn is_open(&self) -> bool {
        self.open.get()
    }
}

impl Ground {
    /// `game` is the install, for the prop prims' shapes.
    /// `moving` are the props that are platforms: the game moves those, so they're left out.
    pub fn new(map: &Map, game: &Path, moving: &std::collections::BTreeSet<usize>) -> Ground {
        let mut roofs = HashMap::new();
        for (x, z) in (0..128).flat_map(|x| (0..128).map(move |z| (x, z))) {
            let sq = map.square(x, z);
            if sq.has(flags::ROOF_EXISTS) {
                roofs.insert((x, z), [sq.height as f32 * 64.0; 4]);
            }
        }
        for rf in map.roof_faces.iter().skip(1) {
            let (x, z) = ((rf.rx & 127) as usize, (rf.rz & 127) as usize);
            let y0 = rf.y as f32;
            let d = |i: usize| ((rf.dy[i] as i32) << ROOF_SHIFT) as f32;
            roofs.entry((x, z)).or_insert([y0, y0 + d(0), y0 + d(2), y0 + d(1)]);
        }
        let ladders = map
            .facets
            .iter()
            .skip(1)
            .filter(|f| f.kind == facet_type::LADDER)
            .map(|f| {
                let (a, b) = (vec2(f.x[0] as f32, f.z[0] as f32) * 256.0, vec2(f.x[1] as f32, f.z[1] as f32) * 256.0);
                let d = b - a;
                // Drawn in the middle third, set out from the wall by 1/8 of its length.
                let out = vec2(d.y, -d.x).normalize_or_zero();
                Ladder { foot: (a + b) * 0.5 + out * d.length() / 8.0, out, base: f.y[0] as f32, top: f.y[0] as f32 + f.height as f32 * 64.0 }
            })
            .collect();
        let mut walls = collision_segments(map);
        let doors: Vec<Door> = door_segments(map)
            .into_iter()
            .map(|wall| {
                let mid = (Vec2::from(wall.a) + Vec2::from(wall.b)) * 0.5;
                Door { wall, mid, open: std::cell::Cell::new(false) }
            })
            .collect();
        let mut prop_tops: HashMap<(i32, i32), Vec<PropTop>> = HashMap::new();
        let mut prop_tris: HashMap<(i32, i32), Vec<[Vec3; 3]>> = HashMap::new();
        let mut prims = HashMap::new();
        let mut prop_walls = Vec::new();
        for (i, ob) in map.objects.iter().enumerate() {
            let first_wall = walls.len();
            if moving.contains(&i) {
                continue;
            }
            let Some(prim) = prims.entry(ob.prim).or_insert_with(|| crate::level::load_prim(game, ob.prim)) else { continue };
            if prim.points.is_empty() {
                continue;
            }
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for p in &prim.points {
                let v = vec3(p[0] as f32, p[1] as f32, p[2] as f32);
                lo = lo.min(v);
                hi = hi.max(v);
            }
            let a = ob.yaw as f32 * std::f32::consts::TAU / 2048.0;
            let (s, c) = a.sin_cos();
            let origin = vec2(ob.x as f32, ob.z as f32);
            let place = |x: f32, z: f32| origin + vec2(c * x + s * z, -s * x + c * z);
            let (y0, y1) = (ob.y as f32 + lo.y, ob.y as f32 + hi.y);
            match prim.coltype {
                // PRIM_COLLIDE_BOX, _SMALLBOX: the rotated bounding box.
                0 | 3 => {
                    let k = if prim.coltype == 3 { 0.7 } else { 1.0 };
                    let corners = [place(lo.x * k, lo.z * k), place(hi.x * k, lo.z * k), place(hi.x * k, hi.z * k), place(lo.x * k, hi.z * k)];
                    for i in 0..4 {
                        walls.push(WallSegment { a: corners[i].into(), b: corners[(i + 1) % 4].into(), y0, y1 });
                    }
                }
                // PRIM_COLLIDE_CYLINDER: an octagon around the middle.
                2 => {
                    let r = ((hi.x - lo.x).min(hi.z - lo.z) * 0.5).clamp(12.0, 60.0);
                    let mid = place((lo.x + hi.x) * 0.5, (lo.z + hi.z) * 0.5);
                    let pt = |i: usize| mid + Vec2::from_angle(i as f32 * std::f32::consts::FRAC_PI_4) * r;
                    for i in 0..8 {
                        walls.push(WallSegment { a: pt(i).into(), b: pt(i + 1).into(), y0, y1 });
                    }
                }
                _ => {}
            }
            if walls.len() > first_wall {
                prop_walls.push((i, first_wall..walls.len()));
            }
            // Walkable faces (stair treads, ramps, crate tops): the faces the prim marks walkable
            // that face up. Props flagged walkable without such faces use their top.
            let place3 = |v: [i16; 3]| {
                let q = place(v[0] as f32, v[2] as f32);
                vec3(q.x, ob.y as f32 + v[1] as f32, q.y)
            };
            let mut tris = Vec::new();
            if prim.flag & 0x2 != 0 {
                for f in prim.faces.iter().filter(|f| f.face_flags & FACE_FLAG_WALKABLE != 0) {
                    let p: Vec<Vec3> = f.points.iter().map(|&i| place3(prim.points[i as usize])).collect();
                    let fan: Vec<[Vec3; 3]> = if p.len() == 4 { vec![[p[0], p[1], p[2]], [p[1], p[3], p[2]]] } else { vec![[p[0], p[1], p[2]]] };
                    for t in fan {
                        let n = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
                        if n.y.abs() > 0.7 {
                            tris.push(t);
                        }
                    }
                }
            }
            for t in &tris {
                let (lo, hi) = (t[0].min(t[1]).min(t[2]), t[0].max(t[1]).max(t[2]));
                for x in (lo.x / 256.0).floor() as i32..=(hi.x / 256.0).floor() as i32 {
                    for z in (lo.z / 256.0).floor() as i32..=(hi.z / 256.0).floor() as i32 {
                        prop_tris.entry((x, z)).or_default().push(*t);
                    }
                }
            }
            if prim.flag & 0x2 != 0 && tris.is_empty() {
                let top = PropTop { origin, rot: vec2(c, s), lo: vec2(lo.x, lo.z), hi: vec2(hi.x, hi.z), top: y1 };
                let r = lo.abs().max(hi.abs()).length();
                let (sx0, sz0) = (((origin.x - r) / 256.0).floor() as i32, ((origin.y - r) / 256.0).floor() as i32);
                let (sx1, sz1) = (((origin.x + r) / 256.0).floor() as i32, ((origin.y + r) / 256.0).floor() as i32);
                for x in sx0..=sx1 {
                    for z in sz0..=sz1 {
                        prop_tops.entry((x, z)).or_default().push(top);
                    }
                }
            }
        }
        let cables = map.facets.iter().skip(1).filter(|f| f.kind == facet_type::CABLE).map(|f| Cable { points: cable_points(f) }).collect();
        let fences = map
            .facets
            .iter()
            .skip(1)
            .filter(|f| matches!(f.kind, facet_type::FENCE | facet_type::FENCE_FLAT) && f.flags & uc_formats::iam::facet_flags::INVISIBLE == 0)
            .map(|f| {
                let y0 = f.y[0].min(f.y[1]) as f32;
                WallSegment {
                    a: [f.x[0] as f32 * 256.0, f.z[0] as f32 * 256.0],
                    b: [f.x[1] as f32 * 256.0, f.z[1] as f32 * 256.0],
                    y0,
                    y1: y0 + f.height as f32 * f.block_height as f32 * 4.0,
                }
            })
            .filter(|w| w.y1 > w.y0)
            .collect();
        // Every prop, the moving ones too (Target UC's missiles ride platforms), whose prims
        // the collision above skips.
        let prop_damage = map.objects.iter()
            .map(|ob| {
                prims.entry(ob.prim).or_insert_with(|| crate::level::load_prim(game, ob.prim))
                    .as_ref().map_or((0, 0), |p| (p.damage, p.flag))
            })
            .collect();
        Ground { roofs, prop_tops, prop_tris, walls, doors, ladders, cables, fences, prop_walls, prop_damage, moving: Default::default() }
    }

    fn roof_height(&self, x: f32, z: f32) -> Option<f32> {
        let (mx, mz) = ((x / SQUARE as f32).floor(), (z / SQUARE as f32).floor());
        if mx < 0.0 || mz < 0.0 {
            return None;
        }
        let h = self.roofs.get(&(mx as usize, mz as usize))?;
        let (fx, fz) = (x / SQUARE as f32 - mx, z / SQUARE as f32 - mz);
        Some(if fx + fz < 1.0 {
            h[0] + (h[1] - h[0]) * fx + (h[2] - h[0]) * fz
        } else {
            h[3] + (h[2] - h[3]) * (1.0 - fx) + (h[1] - h[3]) * (1.0 - fz)
        })
    }

    /// Height of the surface under feet at (x, y, z): the highest of terrain and roof that is
    /// no more than a step above `y`. Below `y` means the person should fall.
    pub fn surface(&self, map: &Map, x: f32, z: f32, y: f32) -> f32 {
        let mut best = map.ground_height(x, z);
        if let Some(r) = self.roof_height(x, z) {
            if r <= y + STEP_HEIGHT && r > best {
                best = r;
            }
        }
        let key = ((x / 256.0).floor() as i32, (z / 256.0).floor() as i32);
        for t in self.prop_tops.get(&key).into_iter().flatten() {
            if t.top <= y + STEP_HEIGHT && t.top > best && t.contains(vec2(x, z)) {
                best = t.top;
            }
        }
        for t in self.moving.borrow().iter() {
            if t.top <= y + STEP_HEIGHT && t.top > best && t.contains(vec2(x, z)) {
                best = t.top;
            }
        }
        for t in self.prop_tris.get(&key).into_iter().flatten() {
            if let Some(h) = height_in(t, x, z) {
                if h <= y + STEP_HEIGHT && h > best {
                    best = h;
                }
            }
        }
        best
    }

    /// Moves a circle of `radius` at feet height `y` from `from` by `delta`, sliding along
    /// walls. Walls only block if they reach above a step over the person's feet.
    pub fn slide(&self, from: Vec2, y: f32, delta: Vec2, radius: f32) -> Vec2 {
        let mut p = from + delta;
        // Anything that can't be stepped onto blocks: a jump that lifts the feet to within a
        // step of a ledge's top clears it (and lands on it).
        let waist = y + STEP_HEIGHT - 4.0;
        for _ in 0..2 {
            for w in self.blocking() {
                if waist < w.y0 || waist > w.y1 {
                    continue;
                }
                let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                let closest = a + ab * t;
                let d = p - closest;
                let dist = d.length();
                if dist < radius {
                    let n = if dist > 1e-3 { d / dist } else { vec2(-ab.y, ab.x).normalize_or_zero() };
                    p = closest + n * radius;
                }
            }
        }
        p
    }

    /// Everything solid at the moment: the map's walls, and the doors still shut.
    pub fn blocking(&self) -> impl Iterator<Item = &WallSegment> {
        self.walls.iter().chain(self.doors.iter().filter(|d| !d.is_open()).map(|d| &d.wall))
    }

    /// Opens the door nearest `(x, z)`, as `DOOR_open` does: the closest one whose middle is
    /// within two squares, measured the engine's way (`QDIST2`, the larger axis plus half the
    /// other). Returns whether it found one.
    pub fn open_door_near(&self, x: f32, z: f32) -> bool {
        let at = vec2(x, z);
        let near = |d: &&Door| {
            let v = (d.mid - at).abs();
            v.max_element() + v.min_element() / 2.0
        };
        let found = self
            .doors
            .iter()
            .filter(|d| near(d) < 512.0)
            .min_by(|a, b| near(a).total_cmp(&near(b)));
        if let Some(d) = found {
            d.open.set(true);
        }
        found.is_some()
    }

    /// A ladder whose foot is within `reach` of `pos` (xz) and whose base is near `pos.y`.
    pub fn ladder_near(&self, pos: Vec3, reach: f32) -> Option<usize> {
        self.ladders.iter().position(|l| {
            (l.foot - vec2(pos.x, pos.z)).length() < reach && (pos.y - l.base).abs() < 64.0 && l.top > pos.y + STEP_HEIGHT
        })
    }

    /// A ladder whose foot is within `reach` of `pos` (xz) that someone in the air can catch:
    /// its bottom no more than `grab` above their feet, and its top still above them.
    pub fn ladder_to_catch(&self, pos: Vec3, reach: f32, grab: f32) -> Option<usize> {
        self.ladders.iter().position(|l| {
            (l.foot - vec2(pos.x, pos.z)).length() < reach && l.base - pos.y < grab && l.top > pos.y + STEP_HEIGHT
        })
    }

    /// A climbable fence within `reach` of `pos` that `dir` walks into: (fence, nearest point on
    /// it, unit normal pointing over to the far side).
    pub fn fence_ahead(&self, pos: Vec3, dir: Vec2, reach: f32) -> Option<(usize, Vec2, Vec2)> {
        let p = vec2(pos.x, pos.z);
        self.fences.iter().enumerate().find_map(|(i, w)| {
            let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            let q = a + ab * t;
            let n = (q - p).normalize_or_zero();
            let ok = (q - p).length() < reach && n.dot(dir) > 0.6 && w.y1 > pos.y + STEP_HEIGHT && w.y0 <= pos.y + 64.0
                && w.y1 - pos.y < 600.0;
            ok.then_some((i, q, n))
        })
    }

    /// Whether nothing solid lies between `a` and `b` (eye to eye).
    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        self.first_hit(a, b).is_none()
    }

    /// How far from `a` to `b` (0..1) the first solid wall cuts the line, if one does.
    pub fn first_hit(&self, a: Vec3, b: Vec3) -> Option<f32> {
        let (p, r) = (vec2(a.x, a.z), vec2(b.x - a.x, b.z - a.z));
        self.blocking().filter_map(|w| {
            let (q, s) = (Vec2::from(w.a), Vec2::from(w.b) - Vec2::from(w.a));
            let denom = r.perp_dot(s);
            if denom.abs() < 1e-6 {
                return None;
            }
            let t = (q - p).perp_dot(s) / denom;
            let u = (q - p).perp_dot(r) / denom;
            let y = a.y + (b.y - a.y) * t;
            ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) && y >= w.y0 && y <= w.y1).then_some(t)
        }).min_by(f32::total_cmp)
    }

    /// The cable within `reach` of point `p` (e.g. a person's hands): (cable, fraction along).
    pub fn cable_near(&self, p: Vec3, reach: f32) -> Option<(usize, f32)> {
        self.cables
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (q, t) = c.closest(p);
                (i, t, (q - p).length())
            })
            .filter(|&(_, _, d)| d < reach)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(i, t, _)| (i, t))
    }

    /// Where a climber stands on ladder `i` at height `y`.
    pub fn on_ladder(&self, i: usize, y: f32) -> Vec3 {
        let l = &self.ladders[i];
        let p = l.foot + l.out * 28.0;
        vec3(p.x, y, p.y)
    }
}
