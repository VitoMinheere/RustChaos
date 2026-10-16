//! The mission's helicopters: flown a tick at a time, drawn with their rotors and
//! searchlight, and heard.

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

impl Game {
    /// `CHOPPER_fn_normal` for each helicopter, at 20 ticks a second.
    pub(crate) fn fly_choppers(&mut self, dt: f32, level: &Level) {
        if self.choppers.is_empty() {
            return;
        }
        self.clocks.chopper += dt;
        while self.clocks.chopper >= chopper::TICK {
            self.clocks.chopper -= chopper::TICK;
            let mut choppers = std::mem::take(&mut self.choppers);
            {
                let world = World { game: self, level };
                for c in &mut choppers {
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
        let angle = |a: i32| a as f32 * std::f32::consts::TAU / 2048.0;
        for c in &self.choppers {
            let at = c.pos();
            let (tilt, roll) = c.tilt_roll();
            let lean = Quat::from_rotation_x(angle(tilt)) * Quat::from_rotation_z(angle(roll));
            for (prim, spin) in [(chopper::BODY_PRIM, 0), (chopper::ROTOR_PRIM, c.rotors)] {
                if let Some(prim) = level.item_prims.get(&prim) {
                    let rot = Quat::from_rotation_y(angle(c.angle + spin)) * lean;
                    uc_world::level::prim_batches(prim, |v| at + rot * v, lighter.as_ref(), out);
                }
            }
            if c.light == 0 {
                continue;
            }
            let dir = c.beam.normalize_or(Vec3::NEG_Y);
            let end = at + dir * 256.0 * 5.0;
            let side = dir.cross(Vec3::Y).normalize_or(Vec3::X) * 256.0;
            let up = side.cross(dir).normalize_or(Vec3::Z) * 256.0;
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
