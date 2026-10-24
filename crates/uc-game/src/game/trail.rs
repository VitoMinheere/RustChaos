//! Laying footprints, skid marks and tyre tracks ([`crate::tracks`]), and drawing them.

use crate::tracks::{self, Foot, Kind, Surface};
use crate::*;

/// `SUB_OBJECT_LEFT_FOOT`, `_RIGHT_FOOT`.
const LEFT_FOOT: usize = 3;
const RIGHT_FOOT: usize = 14;

impl Game {
    /// Prints where a foot came down this turn (`person_splash`, Person.cpp:1374), and the
    /// cars' skid marks.
    pub(crate) fn lay_prints(&mut self, level: &Level) {
        let snow = dirt::Ground::of(level.map.texture_set) == dirt::Ground::Snow;
        for i in 0..self.people.len() {
            let Some(foot) = self.people[i].footfall.take() else { continue };
            let p = &self.people[i];
            if p.removed {
                continue;
            }
            // 20 back from the way she faces (`MATRIX_MUL` of (0, 0, 20) by her yaw).
            let f = p.facing_dir();
            let d = Vec3::new(-f.x, 0.0, -f.y) * 20.0;
            let feet = match foot {
                Foot::Left => vec![(LEFT_FOOT, Kind::LeftPrint)],
                Foot::Right => vec![(RIGHT_FOOT, Kind::RightPrint)],
                Foot::Both => vec![(RIGHT_FOOT, Kind::RightPrint), (LEFT_FOOT, Kind::LeftPrint)],
            };
            let mut mucky = p.mucky;
            for (limb, kind) in feet {
                let Some((_, at)) = p.fig.part_world(limb) else { continue };
                // From twice that ahead of the foot back to it, 1.5 off the floor.
                let from = Vec3::new(at.x - d.x * 2.0, 0.0, at.z - d.z * 2.0);
                if tracks::on_water(&level.map, from.x, from.z) {
                    continue;
                }
                let y = level.ground.surface(&level.map, at.x, at.z, p.fig.pos.y + 64.0) + 1.5;
                let ground = tracks::surface_at(&level.map, from.x, from.z, snow, true);
                mucky = self.tracks.add(Vec3::new(from.x, y, from.z), d * 2.0, kind, mucky, ground);
            }
            self.people[i].mucky = mucky;
        }
        self.lay_skids(level);
    }

    /// A car smoking (`Smokin`, vehicle.cpp:1463), its wheels spinning or sliding: a puff of
    /// smoke at each wheel every one of the original's turns (two of our three), and once
    /// it's moving a dark mark from each back to where it was, every other turn (:1513).
    fn lay_skids(&mut self, level: &Level) {
        self.skid_marks.resize(self.vehicles.len(), None);
        let (puff, mark) = (self.turns % 3 != 2, self.turns & 1 == 0);
        for (v, was) in self.vehicles.iter().zip(&mut self.skid_marks) {
            if !v.skidding() {
                *was = None;
                continue;
            }
            let (wx, fz, rz) = (vehicle::TYPES[v.kind as usize].2, vehicle::TYPES[v.kind as usize].3, vehicle::TYPES[v.kind as usize].4);
            let now = [(-wx, fz), (wx, fz), (-wx, rz), (wx, rz)].map(|(x, z)| v.to_world(x, z));
            if puff {
                for a in now {
                    self.pyro.tyre_smoke(Vec3::new(a.x, v.pos.y + 20.0, a.y));
                }
            }
            // (`speed > 200`: the square of a sixteenth of its speed in 256ths a tick.)
            if !mark || v.speed.hypot(v.slip) <= 17.7 {
                continue;
            }
            if let Some(old) = was {
                for (a, b) in now.iter().zip(old.iter()) {
                    // On the road, not on top of a car: its own roof is walkable ground too.
                    let y = level.ground.road(&level.map, a.x, a.y, v.pos.y + 64.0) + 5.0;
                    let d = *b - *a;
                    self.tracks.add(Vec3::new(a.x, y, a.y), Vec3::new(d.x, 0.0, d.y), Kind::Skid, 0, Surface::None);
                }
            }
            *was = Some(now);
        }
    }

    /// A bike's back wheel, `at` and going `d` a pass: a track behind it on muck (bike.cpp:850).
    /// The original lays it where the bike's frame is over the wheel; here it's on the ground.
    pub(crate) fn lay_tyre(&mut self, level: &Level, k: usize, at: Vec3, d: Vec3) {
        let ground = tracks::surface_at(&level.map, at.x, at.z, false, false);
        let y = level.ground.surface(&level.map, at.x, at.z, at.y) + 1.5;
        let last = self.bikes[k].0.tyre_last;
        self.bikes[k].0.tyre_last = self.tracks.add(Vec3::new(at.x, y, at.z), d, Kind::Tyre, last, ground);
    }

    /// The marks on the ground (`TRACKS_DrawTrack`, drawxtra.cpp:181): each a quad from its
    /// start along `d`, `side` either way, tinted its colour and fading with age. A print takes
    /// its half of the page, the right foot's mirrored.
    pub(crate) fn draw_tracks(&self, out: &mut BTreeMap<u32, Batch>) {
        for t in &self.tracks.list {
            let alpha = self.tracks.alpha(t);
            if alpha == 0 {
                continue;
            }
            let side = Vec3::new(t.side.x, 0.0, t.side.y);
            let p = [t.at + side, t.at - side, t.at + t.d + side, t.at + t.d - side].map(|v| v / uc_world::UNIT);
            let uv = match t.flip {
                Some(true) => [[0.5, 1.0], [0.0, 1.0], [0.5, 0.0], [0.0, 0.0]],
                Some(false) => [[0.0, 1.0], [0.5, 1.0], [0.0, 0.0], [0.5, 0.0]],
                None => [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            };
            let [r, g, b] = t.colour;
            out.entry(t.page).or_default().quad(p, uv, [[r, g, b, alpha]; 4]);
        }
    }
}
