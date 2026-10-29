//! Laying footprints, skid marks and tyre tracks ([`crate::tracks`]), and drawing them.

use crate::tracks::{self, Foot, Kind, Surface};
use crate::*;

/// `SUB_OBJECT_LEFT_FOOT`, `_RIGHT_FOOT`.
const LEFT_FOOT: usize = 3;
const RIGHT_FOOT: usize = 14;

impl Game {
    /// Someone below a quarter of their health, and not in a car, now and then leaves a
    /// splash of blood by a foot (Person.cpp:4466, `TRACKS_Bleed`, tracks.cpp:317): the
    /// less health left, the more often. A splash is 1 to 32 across, at any angle, a few
    /// units off the foot and just over the ground.
    pub(crate) fn bleed(&mut self, level: &Level) {
        // The odds are the original's per tick, 20 a second, not per turn here.
        if (self.time * 20.0) as u64 == ((self.time - TURN) * 20.0) as u64 {
            return;
        }
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.removed || p.is_dead() || p.health <= 0 || p.health >= p.max_health / 4 || matches!(p.state, State::InCar { .. }) {
                continue;
            }
            let health = p.health;
            if (self.rand() & 0x7f) as i32 <= health || self.rand() & 1 == 0 {
                continue;
            }
            let size = 1 + (self.rand() & 0x1f) as i32;
            let foot = if self.rand() & 1 != 0 { LEFT_FOOT } else { RIGHT_FOOT };
            let a = (self.rand() & 2047) as f32 * std::f32::consts::TAU / 2048.0;
            // `(rand() & 0247) - 1024` across: an octal slip in the original, so always a few
            // units one way; `(rand() & 2047) - 1024` along.
            let (ox, oz) = (((self.rand() & 0o247) as f32 - 1024.0) / 256.0, ((self.rand() & 2047) as f32 - 1024.0) / 256.0);
            let Some((_, at)) = self.people[i].fig.part_world(foot) else { continue };
            let (x, z) = (at.x + ox, at.z + oz);
            let y = level.map.ground_height(x, z) + 1.0;
            let d = Vec3::new(a.sin(), 0.0, a.cos()) * size as f32;
            self.tracks.add_quad(Vec3::new(x, y, z), d, uc_world::geometry::BLOODSPLAT_PAGE, [255, 255, 255], 0, size as f32, None);
        }
    }

    /// Someone landed dead from a plunge: a splash of blood spreading out round them
    /// (`PYRO_SPLATTERY`, pyro.cpp:1146). Its counter goes up 18 a frame to 255; each frame
    /// lays four times five splashes, `counter` out from the middle and shrinking from 31 to
    /// nothing as it goes.
    pub(crate) fn splatter(&mut self, level: &Level) {
        for i in 0..self.people.len() {
            if !std::mem::take(&mut self.people[i].splattered) {
                continue;
            }
            let at = self.people[i].fig.pos;
            let mut counter = 0u32;
            while counter <= 254 {
                for _ in 0..4 {
                    let size = ((255 - counter) >> 3) as f32;
                    let a = (self.rand() & 2047) as f32 * std::f32::consts::TAU / 2048.0;
                    let d = Vec3::new(a.sin(), 0.0, a.cos()) * size.max(1.0);
                    let (x, z) = (at.x - d.x / 2.0, at.z - d.z / 2.0);
                    let turn = (self.rand() & 0xff) as f32;
                    for b in 0..5 {
                        let a = (b as f32 * 340.0 + turn) * std::f32::consts::TAU / 2048.0;
                        let (ox, oz) = (a.sin() * counter as f32, a.cos() * counter as f32);
                        let y = level.map.ground_height(x + ox, z + oz) + 1.0;
                        self.tracks.add_quad(Vec3::new(x + ox, y, z + oz), d, uc_world::geometry::BLOODSPLAT_PAGE, [255, 255, 255], 0, size.max(1.0), None);
                    }
                }
                counter += 18;
            }
        }
    }

    /// What each footstep this turn sounds like (`footstep_wave`, Person.cpp:1195): soft for
    /// the player in a warehouse, wet in a puddle, else the floor texture's own.
    pub(crate) fn step_sounds(&mut self, level: &Level) {
        let indoors = self.indoors.is_some();
        for p in self.people.iter_mut() {
            let at = p.fig.pos;
            let range = if indoors && p.player {
                steps::SOFT
            } else if self.puddles.in_puddle(at.x, at.z) {
                steps::PUDDLE
            } else {
                let (mx, mz) = ((at.x / 256.0) as i32, (at.z / 256.0) as i32);
                if (0..128).contains(&mx) && (0..128).contains(&mz) {
                    self.steps.on_page(level.map.square(mx as usize, mz as usize).page() as u32)
                } else {
                    steps::ROAD
                }
            };
            for c in p.cues.iter_mut() {
                if let Sfx::Footstep { sound: s @ None, .. } = c {
                    *s = Some(range);
                }
            }
        }
    }

    /// On snow, everyone standing about but the Men in Black breathes out a puff from their
    /// head, once a breath as the animation passes its fifth frame (`fn_person_idle`,
    /// Person.cpp:11791, `MagicFrameCheck`). The original stands people in `ANIM_STAND_READY`;
    /// here they breathe (`ANIM_BREATHE`), which is only two frames, so the puff comes at its
    /// last.
    pub(crate) fn breathe_out(&mut self, level: &Level) {
        if dirt::Ground::of(level.map.texture_set) != dirt::Ground::Snow {
            return;
        }
        for p in self.people.iter_mut() {
            let standing = matches!(p.state, State::Move)
                && (person::idle_anim(p.fig.anim()) || matches!(p.fig.anim(), uc_world::character::anim::IDLE_SCRATCH1 | uc_world::character::anim::IDLE_SCRATCH2));
            if p.removed || p.is_mib() || !standing {
                p.breathed = false;
                continue;
            }
            if p.fig.frame().unwrap_or(0) < 5.min(p.fig.frames().saturating_sub(1)) {
                p.breathed = false;
            } else if !p.breathed {
                p.breathed = true;
                if let Some((_, at)) = p.fig.part_world(11) {
                    self.pyro.breath(at);
                }
            }
        }
    }

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
                // A foot in a puddle drips (`DRIP_create_if_in_puddle`, Person.cpp:1420).
                self.puddles.splash(Vec3::new(at.x, p.fig.pos.y, at.z));
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
        self.smoke_damaged();
        self.slide_dust();
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

    /// A sliding tackle kicks up dust round the left foot while it slides (`ANIM_SLIDER_HOLD`,
    /// Person.cpp:16056): three puffs a turn, seven while still going fast, the colour of tyre
    /// smoke (`0xc9b7a3`), up to 16 either way of the foot.
    fn slide_dust(&mut self) {
        if self.turns % 3 == 2 {
            return;
        }
        for i in 0..self.people.len() {
            let State::Slide { phase: 1, speed, .. } = self.people[i].state else { continue };
            let Some((_, foot)) = self.people[i].fig.part_world(3) else { continue };
            for _ in 0..if speed > 500.0 { 7 } else { 3 } {
                let (dx, dz) = ((self.rand() & 31) as f32 - 16.0, (self.rand() & 31) as f32 - 16.0);
                self.pyro.tyre_smoke(foot + Vec3::new(dx, 0.0, dz));
            }
        }
    }

    /// A damaged car smokes from under its bonnet (`draw_car`, vehicle.cpp:1309): each of the
    /// original's turns (two of our three), when a number up to 127 comes out over its health,
    /// so from under 128 of its 300, and more the worse it gets. A wreck smokes from the back
    /// too.
    fn smoke_damaged(&mut self) {
        if self.turns % 3 == 2 {
            return;
        }
        for vi in 0..self.vehicles.len() {
            let health = self.vehicles[vi].health;
            if health >= 127 || (self.rand() & 0x7f) as i32 <= health {
                continue;
            }
            let v = &self.vehicles[vi];
            let front = vehicle::TYPES[v.kind as usize].3;
            let vel = Vec3::new(v.forward().x, 0.0, v.forward().y) * v.speed;
            let ends: Vec<Vec2> = if v.wrecked() { vec![v.to_world(0.0, front), v.to_world(0.0, -front)] } else { vec![v.to_world(0.0, front)] };
            let y = v.pos.y + 20.0;
            for at in ends {
                let (dx, dz) = ((self.rand() & 63) as f32 - 32.0, (self.rand() & 63) as f32 - 32.0);
                self.pyro.engine_smoke(Vec3::new(at.x + dx, y, at.y + dz), vel);
            }
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
