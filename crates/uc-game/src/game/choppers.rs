//! The mission's helicopters: flown a tick at a time, drawn with their rotors and
//! searchlight, and heard. And the one parked for the player (#465): got into and out of
//! with action, and flown with [`Controls::fly`].

use uc_formats::units::radians;
use crate::chopper::{Sky, Target};
use glam::Quat;
use crate::*;

/// What a helicopter sees of the game.
struct World<'a> {
    game: &'a Game,
    level: &'a Level,
}

impl Sky for World<'_> {
    fn map_height(&self, x: f32, z: f32) -> f32 {
        self.level.ground.surface(&self.level.map, x, z, f32::MAX / 2.0)
    }

    fn inside(&self, x: f32, y: f32, z: f32) -> bool {
        self.map_height(x, z) > y
    }

    fn person(&self, i: usize) -> Option<Vec3> {
        let p = self.game.people.get(i)?;
        (!p.removed).then_some(p.fig.pos)
    }

    fn is_player(&self, i: usize) -> bool {
        self.game.player == Some(i)
    }

    fn made_by(&self, way: usize) -> Option<usize> {
        self.game.by_way.get(&way).copied()
    }
}

/// Where [`Game::park_chopper`] puts one: the nearest open, flat place to `start`.
fn parking(level: &Level, start: Vec3) -> Option<Vec3> {
    use uc_formats::iam::{flags, MAP_SIZE};
    let n = MAP_SIZE as i32;
    let (sx, sz) = ((start.x / 256.0) as i32, (start.z / 256.0) as i32);
    let props: Vec<(i32, i32)> = level.map.objects.iter().map(|o| (o.x >> 8, o.z >> 8)).collect();
    let open = |x: i32, z: i32, y: f32| {
        if !(1..n - 1).contains(&x) || !(1..n - 1).contains(&z) {
            return false;
        }
        let sq = level.map.square(x as usize, z as usize);
        if sq.has(flags::HIDDEN) || sq.has(flags::WATER) || sq.page() == uc_world::level::WATER_PAGE || crate::road::is_road(&level.map, x, z) {
            return false;
        }
        let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
        let here = level.ground.surface(&level.map, cx, cz, y + 64.0);
        let roof = level.ground.map_height(&level.map, cx, cz);
        (here - y).abs() <= 16.0 && roof <= here + 16.0 && !props.contains(&(x, z))
    };
    for r in 3..40i32 {
        let mut best: Option<(i32, Vec3)> = None;
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dz.abs()) != r {
                    continue;
                }
                let (x, z) = (sx + dx, sz + dz);
                if !(1..n - 1).contains(&x) || !(1..n - 1).contains(&z) {
                    continue;
                }
                let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
                let y = level.ground.surface(&level.map, cx, cz, start.y + 256.0);
                if (-1..=1).all(|i| (-1..=1).all(|j| open(x + i, z + j, y))) {
                    let d = dx * dx + dz * dz;
                    if best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, Vec3::new(cx, y, cz)));
                    }
                }
            }
        }
        if let Some((_, at)) = best {
            return Some(at);
        }
    }
    None
}

/// How long someone caught in the searchlight stays on the radar after (seconds).
pub const LIT: f32 = 5.0;

impl Game {
    /// `CHOPPER_fn_normal` for each helicopter, at 20 ticks a second.
    pub(crate) fn fly_choppers(&mut self, dt: f32, level: &Level) {
        if self.choppers.is_empty() {
            return;
        }
        self.clocks.chopper += dt;
        while self.clocks.chopper >= crate::TICK {
            self.clocks.chopper -= crate::TICK;
            let mut choppers = std::mem::take(&mut self.choppers);
            let mut lit = Vec::new();
            {
                let world = World { game: self, level };
                for c in &mut choppers {
                    if c.parked {
                        let stick = if c.pilot.is_some() && c.pilot == self.player { self.stick } else { chopper::Stick::default() };
                        c.pilot_tick(stick, &world);
                        lit.extend(c.pilot.filter(|_| c.light > 100).map(|pi| (pi, c.pos() + c.beam, c.height(&world))));
                        continue;
                    }
                    c.tick(self.player, &world);
                    let aim = match c.target {
                        Some(Target::Person(i)) => self.people.get(i).filter(|p| !p.removed).map(|p| p.fig.hips()),
                        None => None,
                    };
                    c.beam = c.aim_light(aim);
                }
            }
            // Low over the ground the rotor wash blows the dirt about (`DIRT_gust` twice a
            // tick, chopper.cpp:545).
            for c in &choppers {
                let world = World { game: self, level };
                let wash = c.wash(&world);
                for ((x1, z1), (x2, z2)) in wash {
                    self.dirt.gust(x1, z1, x2, z2);
                }
            }
            self.choppers = choppers;
            for p in &mut self.people {
                p.lit = (p.lit - crate::TICK).max(0.0);
            }
            for (pilot, spot, height) in lit {
                self.searchlight(pilot, spot, height);
            }
        }
        // The pilot goes where it goes, sat in it.
        for (k, c) in self.choppers.iter().enumerate() {
            if let Some(pi) = c.pilot {
                if matches!(self.people[pi].state, State::Flying { chopper } if chopper == k) {
                    self.people[pi].fig.pos = c.pos() - Vec3::new(0.0, 200.0, 0.0);
                    self.people[pi].fig.angle = radians(c.angle as f32) + std::f32::consts::PI;
                }
            }
        }
    }

    /// The player's searchlight on the spot `spot` from `height` up (#466; not the
    /// original's, whose beam is only drawn): anyone wanted in it shows on the radar while
    /// they're lit and for [`LIT`] seconds after; a thug caught in it gives up one time in
    /// three, lying down to be cuffed, and otherwise runs from it.
    fn searchlight(&mut self, pilot: usize, spot: Vec3, height: f32) {
        let reach = 160.0 + height * 0.15;
        for i in 0..self.people.len() {
            let p = &self.people[i];
            let off = p.fig.pos - spot;
            if i == pilot || !p.on_feet() || p.aboard() || Vec2::new(off.x, off.z).length() > reach || off.y.abs() > 200.0 {
                continue;
            }
            if !p.is_guilty() {
                continue;
            }
            let caught = p.lit == 0.0;
            let thug = matches!(p.kind, ptype::THUG_RASTA | ptype::THUG_GREY | ptype::THUG_RED) || p.ai == person::AI_GANG;
            self.people[i].lit = LIT;
            if caught && thug && !self.people[i].player {
                if self.rand().is_multiple_of(3) {
                    self.people[i].get_down();
                } else if self.people[i].scared.is_none() {
                    self.people[i].scared = Some(person::Scare { by: Some(pilot), at: spot, time: 0.0, goal: None });
                }
            }
        }
    }

    /// The helicopter the player is flying.
    pub fn player_chopper(&self) -> Option<&chopper::Chopper> {
        match self.player().map(|p| &p.state) {
            Some(State::Flying { chopper }) => self.choppers.get(*chopper),
            _ => None,
        }
    }

    /// How high the player's helicopter is over what's under it (world units).
    pub fn chopper_height(&self, level: &Level) -> Option<f32> {
        let c = self.player_chopper()?;
        Some(c.height(&World { game: self, level }))
    }

    /// A parked helicopter with nobody in it, on the ground, within reach of `pi`'s feet:
    /// 300 out from its middle and no more than 300 up or down.
    fn chopper_near(&self, pi: usize, level: &Level) -> Option<usize> {
        let at = self.people[pi].fig.pos;
        let world = World { game: self, level };
        self.choppers.iter().position(|c| {
            let d = c.pos() - at;
            c.parked && c.pilot.is_none() && c.grounded(&world) && Vec2::new(d.x, d.z).length() < 300.0 && d.y.abs() < 300.0
        })
    }

    /// Into the helicopter by the player (#465), if there's one to get into.
    pub(crate) fn board_chopper(&mut self, pi: usize, level: &Level) -> bool {
        let Some(k) = self.chopper_near(pi, level) else { return false };
        self.choppers[k].pilot = Some(pi);
        let p = &mut self.people[pi];
        p.state = State::Flying { chopper: k };
        p.cues.push(Sfx::CarDoor);
        true
    }

    /// Out of the helicopter: on the ground (or a roof) beside it, out of the door on the
    /// left; or, off the ground but no more than a storey up, dropping from it, to land as
    /// any fall does. Higher up, nobody gets out.
    pub(crate) fn leave_chopper(&mut self, pi: usize, k: usize, level: &Level) -> bool {
        let c = &self.choppers[k];
        let height = c.height(&World { game: self, level });
        if height > chopper::DROP {
            return false;
        }
        let side = Vec2::new(c.nose().y, -c.nose().x) * 200.0;
        let (pos, at) = (c.pos(), Vec2::new(c.pos().x, c.pos().z) + side);
        self.choppers[k].pilot = None;
        let floor = level.ground.surface(&level.map, at.x, at.y, pos.y - 200.0);
        let p = &mut self.people[pi];
        p.cues.push(Sfx::CarDoor);
        if height <= 24.0 {
            p.fig.pos = Vec3::new(at.x, floor, at.y);
            p.state = State::Move;
            p.stand_still();
        } else {
            // Straight down from under it.
            p.fig.pos = Vec3::new(pos.x, pos.y - 250.0, pos.z);
            p.fall(Vec2::ZERO);
        }
        true
    }

    /// Explore and Beat Cop (#465): a police helicopter parked in the open near the player's
    /// start, for them to fly. The nearest place, three or more squares off, with a 3x3 of
    /// squares round it that are out of doors, not water, not road and flat (the ground no
    /// more than 16 up or down across them), and no prop on them.
    pub(crate) fn park_chopper(&mut self, level: &Level) {
        let Some(start) = self.player().map(|p| p.fig.pos) else { return };
        if let Some(at) = parking(level, start) {
            // Facing away from her.
            let away = flat(at, start);
            let angle = (away.x.atan2(away.y) * 2048.0 / std::f32::consts::TAU) as i32;
            self.choppers.push(chopper::Chopper::parked(at, angle));
        }
    }

    /// The nearest helicopter in the air to `at`, for its rotors' sound (`S_HELI`, looped
    /// while it isn't landed).
    pub fn chopper_noise(&self, at: Vec3) -> Option<Vec3> {
        self.choppers.iter().filter(|c| c.flying()).map(|c| c.pos())
            .min_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at)))
    }

    /// `CHOPPER_draw_chopper`: the body and the rotors (turned by `rotors`), leaning into
    /// the way it flies, and the searchlight: a cone five squares long added to the scene,
    /// bright yellow at the lamp and fading out, pointed at its target's pelvis.
    pub(crate) fn draw_choppers(&self, out: &mut BTreeMap<u32, Batch>, level: &Level) {
        let lighter = level.lighter();
        let angle = |a: i32| radians(a as f32);
        for c in &self.choppers {
            let at = c.pos();
            let (tilt, roll) = c.tilt_roll();
            let lean = Quat::from_rotation_x(angle(tilt)) * Quat::from_rotation_z(angle(roll));
            for (prim, spin) in [(chopper::BODY_PRIM, 0), (chopper::ROTOR_PRIM, c.rotors)] {
                if let Some(prim) = level.item_prims.get(&prim) {
                    // Half a turn round: the model's nose is along -z, and the original's
                    // `Arctan` (which the angle is, `-Arctan(dx, dz)` drawn at `-Angle`) is half
                    // a turn off `atan2`, as the people's facing is.
                    let rot = Quat::from_rotation_y(angle(c.angle + 1024 + spin)) * lean;
                    uc_world::level::prim_batches(prim, |v| at + rot * v, lighter.as_ref(), out);
                }
            }
            if c.light == 0 {
                continue;
            }
            let dir = c.beam.normalize_or(Vec3::NEG_Y);
            // The player's reaches the ground, as wide as the spot that picks people out
            // (#466); the AI's is five squares long, as the original's.
            let (len, wide) = if c.parked {
                (c.beam.length(), 160.0 + 0.15 * (-c.beam.y - 250.0).max(0.0))
            } else {
                (256.0 * 5.0, 256.0)
            };
            let end = at + dir * len;
            let side = dir.cross(Vec3::Y).normalize_or(Vec3::X) * wide;
            let up = side.cross(dir).normalize_or(Vec3::Z) * wide;
            let bright = (0x66 * c.light / 255) as u8;
            let lamp = [bright, bright, 0, 255];
            const STEPS: usize = 12;
            let glow = out.entry(uc_world::GLOW).or_default();
            for k in 0..STEPS {
                let (a0, a1) = (k as f32, (k + 1) as f32);
                let ring = |a: f32| {
                    let a = a * std::f32::consts::TAU / STEPS as f32;
                    end + side * a.cos() + up * a.sin()
                };
                glow.tri(
                    [at / uc_world::UNIT, ring(a0) / uc_world::UNIT, ring(a1) / uc_world::UNIT],
                    [[0.5, 0.5]; 3],
                    [lamp, [0, 0, 0, 255], [0, 0, 0, 255]],
                );
            }
        }
    }
}
