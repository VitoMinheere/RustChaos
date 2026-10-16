//! Tripwires (fallen/Source/trip.cpp): a beam out of a tripwire prop that a `tripwire`
//! point watches for the player to cross.

use uc_formats::units::radians;
use glam::{vec2, vec3, Vec2, Vec3};
use std::collections::{BTreeMap, HashMap};
use uc_formats::ucm::{trigger, Mission};
use uc_world::{Batch, Level};

/// `PRIM_OBJ_TRIPWIRE` (Editor/Headers/Prim.h).
pub const PRIM_TRIPWIRE: u8 = 178;

/// How far from its point a tripwire prop can be (`OB_find_type`'s range, Manhattan).
const FIND_RANGE: f32 = 0x200 as f32;
/// How far the beam reaches past the prop, unless something solid is in the way.
const LENGTH: f32 = 0x500 as f32;
/// How close to the beam the player has to come (`TRIP_collide`).
const NEAR: f32 = 0x20 as f32;
/// The beam only looks for the player within this (Manhattan) of its start.
const RANGE: f32 = 0x500 as f32;
/// A tripped wire stays tripped for 4 of the original's 20 ticks a second.
const HOLD: f32 = 4.0 / 20.0;

pub struct Wire {
    pub y: f32,
    pub a: Vec2,
    pub b: Vec2,
    /// How far along the player broke the beam (0..1), while it's tripped.
    pub along: Option<f32>,
    wait: f32,
    /// Switched off by a `kill_waypoint` on its point (`TRIP_deactivate`): not drawn, never
    /// trips.
    pub off: bool,
}

#[derive(Default)]
pub struct Trips {
    pub wires: Vec<Wire>,
    /// The wire each `tripwire` point watches.
    by_way: HashMap<usize, usize>,
}

impl Trips {
    /// A wire for every `tripwire` point with a tripwire prop near it (`EWAY_create_cond`,
    /// fallen/Source/eway.cpp:1203). Points sharing a prop share its wire (`TRIP_create`).
    pub fn new(level: &Level, mission: &Mission) -> Trips {
        let mut t = Trips::default();
        let Some(prim) = uc_world::level::load_prim(&level.game, PRIM_TRIPWIRE) else { return t };
        let (lo, hi) = uc_world::level::prim_bounds(&prim);
        let radius = prim.points.iter()
            .map(|p| vec3(p[0] as f32, p[1] as f32, p[2] as f32).length())
            .fold(0.0, f32::max);
        for ep in mission.event_points.iter().filter(|e| e.used != 0 && e.trigger == trigger::TRIPWIRE) {
            let at = vec3(ep.x as f32, ep.y as f32, ep.z as f32);
            let ob = level.map.objects.iter()
                .filter(|o| o.prim == PRIM_TRIPWIRE)
                .map(|o| (o, (o.x as f32 - at.x).abs() + (o.y as f32 - at.y).abs() + (o.z as f32 - at.z).abs()))
                .filter(|&(_, d)| d < FIND_RANGE)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            let Some((ob, _)) = ob else { continue };
            // Out of the prop's facing (`FMATRIX_vector(yaw, 0)`), from its edge, halfway up.
            let (s, c) = radians(ob.yaw as f32).sin_cos();
            let dir = vec2(s, c);
            let a = vec2(ob.x as f32, ob.z as f32) - dir * radius;
            let y = ob.y as f32 + ((lo.y + hi.y) / 2.0).floor();
            let mut b = a - dir * LENGTH;
            // Cut short at the first wall (`there_is_a_los`).
            if let Some(f) = level.ground.first_hit(vec3(a.x, y, a.y), vec3(b.x, y, b.y)) {
                b = a + (b - a) * f;
            }
            let same = t.wires.iter().position(|w| w.y == y && w.a == a && w.b == b);
            let k = same.unwrap_or_else(|| {
                t.wires.push(Wire { y, a, b, along: None, wait: 0.0, off: false });
                t.wires.len() - 1
            });
            t.by_way.insert(ep.index, k);
        }
        t
    }

    /// `TRIP_process`: only the player trips wires, between her lower foot and her head.
    pub fn update(&mut self, dt: f32, player: Option<(Vec3, f32, f32)>) {
        for w in self.wires.iter_mut().filter(|w| !w.off) {
            if w.wait > 0.0 {
                w.wait -= dt;
                continue;
            }
            w.along = None;
            let Some((pos, feet, head)) = player else { continue };
            let p = vec2(pos.x, pos.z);
            if (p.x - w.a.x).abs() + (p.y - w.a.y).abs() > RANGE {
                continue;
            }
            let (feet, head) = (feet.min(head), feet.max(head));
            if !(feet - 0x10 as f32..=head + 0x10 as f32).contains(&w.y) {
                continue;
            }
            if let Some(along) = collide(w.a, w.b, p) {
                w.along = Some(along);
                w.wait = HOLD;
            }
        }
    }

    /// Whether the wire point `way` watches is tripped (`TRIP_activated`).
    pub fn tripped(&self, way: usize) -> bool {
        self.by_way.get(&way).is_some_and(|&k| self.wires[k].along.is_some())
    }

    /// `TRIP_deactivate`, from a `kill_waypoint` on point `way`.
    pub fn deactivate(&mut self, way: usize) {
        if let Some(&k) = self.by_way.get(&way) {
            self.wires[k].off = true;
            self.wires[k].along = None;
        }
    }

    /// The beams, dark red (`SHAPE_tripwire`, colour 0x661122), stopping where the player
    /// breaks them.
    pub fn batches(&self, out: &mut BTreeMap<u32, Batch>) {
        const HALF: f32 = 6.0;
        let colour = [0x66, 0x11, 0x22, 255];
        for w in self.wires.iter().filter(|w| !w.off) {
            let along = w.along.unwrap_or(1.0);
            if along <= 0.0 {
                continue;
            }
            let (a, b) = (vec3(w.a.x, w.y, w.a.y), vec3(w.a.x, w.y, w.a.y).lerp(vec3(w.b.x, w.y, w.b.y), along));
            let d = (b - a).normalize_or_zero();
            let side = d.cross(Vec3::Y).normalize_or_zero() * HALF;
            let up = Vec3::Y * HALF;
            // Two crossed strips, glowing across their width only.
            let batch = out.entry(uc_world::GLOW).or_default();
            for o in [side, up] {
                let p = [a - o, a + o, b - o, b + o].map(|p| p / uc_world::UNIT);
                batch.quad(p, [[0.0, 0.5], [1.0, 0.5], [0.0, 0.5], [1.0, 0.5]], [colour; 4]);
            }
        }
    }
}

/// `TRIP_collide`: how far along a..b (0..1) point `p` is, if it's within `NEAR` of the line.
fn collide(a: Vec2, b: Vec2, p: Vec2) -> Option<f32> {
    let (d, e) = (b - a, p - a);
    let len = d.length() + 1.0;
    if (e.perp_dot(d) / len).abs() >= NEAR {
        return None;
    }
    let along = e.dot(d) / len / len;
    (0.0..=1.0).contains(&along).then_some(along)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_the_beam_is_noticed_only_near_it() {
        let (a, b) = (vec2(0.0, 0.0), vec2(1000.0, 0.0));
        assert!(collide(a, b, vec2(500.0, 10.0)).is_some_and(|t| (t - 0.5).abs() < 0.01));
        assert_eq!(collide(a, b, vec2(500.0, 40.0)), None);
        assert_eq!(collide(a, b, vec2(-50.0, 0.0)), None);
        assert_eq!(collide(a, b, vec2(1100.0, 0.0)), None);
    }
}
