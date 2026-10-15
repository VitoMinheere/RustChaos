//! The mission's motorbikes: their physics a turn at a time, and drawn.

use crate::bike::{Bike, Road, WHEEL_COL};
use crate::*;

/// `anim_chunk[9]`: the bike on its own (parked, and being got on and off).
pub(crate) const BIKE_CHUNK: &str = "data\\anim009.all";

/// What a bike sees of the level.
struct Level2<'a>(&'a Level);

impl Road for Level2<'_> {
    fn height(&self, x: f32, z: f32) -> f32 {
        self.0.ground.surface(&self.0.map, x, z, f32::MAX / 2.0)
    }

    fn collide(&self, from: Vec2, to: Vec2, y: f32) -> (Vec2, bool) {
        let at = self.0.ground.slide(from, y, to - from, WHEEL_COL / 2.0);
        (at, (at - to).length() > 0.5)
    }

    /// Every surface as tarmac: the original sorts each floor texture into tarmac, grass,
    /// dirt or slippery by looking at its pixels (`TEXTURE_looks_like`), which isn't here yet.
    fn friction(&self, _: f32, _: f32) -> i64 {
        200
    }
}

/// The bike's anim prim, if the install has it.
pub(crate) fn bike_chunk(level: &Level) -> Option<std::sync::Arc<uc_formats::all::AnimChunk>> {
    let path = uc_formats::resolve(&level.game, BIKE_CHUNK)?;
    let chunk = uc_formats::all::AnimChunk::parse(&std::fs::read(path).ok()?).ok()?;
    Some(std::sync::Arc::new(chunk))
}

impl Game {
    /// `BIKE_create` for a `VT_BIKE` point: parked where it is, facing its way.
    pub(crate) fn add_bike(&mut self, level: &Level, way: usize, pos: Vec3, angle: f32) {
        let yaw = (angle * 2048.0 / std::f32::consts::TAU) as i64;
        let bike = Bike::new(way, pos.x, pos.z, yaw, &Level2(level));
        let fig = bike_chunk(level).and_then(|c| Figure::anim_prim(c, bike.pos(), angle, 1));
        self.bikes.push((bike, fig));
    }

    /// `BIKE_process_normal` for each bike, at 20 turns a second.
    pub(crate) fn run_bikes(&mut self, dt: f32, level: &Level) {
        if self.bikes.is_empty() {
            return;
        }
        self.bike_clock += dt;
        while self.bike_clock >= bike::TICK {
            self.bike_clock -= bike::TICK;
            for (b, _) in &mut self.bikes {
                b.tick(&Level2(level));
            }
        }
        for (b, fig) in &mut self.bikes {
            if let Some(f) = fig {
                f.pos = b.pos() - Vec3::new(0.0, (0x18 + 0x10) as f32, 0.0);
                f.angle = b.yaw as f32 * std::f32::consts::TAU / 2048.0;
                f.advance(dt);
            }
        }
    }

    pub(crate) fn draw_bikes(&self, out: &mut BTreeMap<u32, Batch>, level: &Level) {
        let lighter = level.lighter();
        for (_, fig) in &self.bikes {
            if let Some(f) = fig {
                f.append_batches(out, lighter.as_ref());
            }
        }
    }
}
