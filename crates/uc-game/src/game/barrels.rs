//! The barrels a turn at a time: knocked by whoever walks into them, and moving.

use crate::barrel::{self, World};
use crate::*;

struct Ground<'a>(&'a Level);

impl World for Ground<'_> {
    fn height(&self, x: f32, z: f32, y: f32) -> f32 {
        self.0.ground.surface(&self.0.map, x, z, y + 16.0)
    }

    fn slide(&self, from: Vec2, to: Vec2, y: f32) -> Vec2 {
        self.0.ground.slide(from, y, to - from, 30.0)
    }
}

impl Game {
    /// Each turn: everyone who moved knocks the barrels round where they are
    /// (`BARREL_hit_with_sphere` 0x50 in `move_thing`, collide.cpp:7103), and the barrels
    /// move ([`barrel::process`]), knocking into things heard.
    pub(crate) fn roll_barrels(&mut self, dt: f32, level: &Level) {
        if self.barrels.is_empty() {
            return;
        }
        self.clocks.barrel += dt;
        while self.clocks.barrel >= barrel::TICK {
            self.clocks.barrel -= barrel::TICK;
            let now: Vec<Vec3> = self.people.iter()
                .map(|p| if p.removed || matches!(p.state, State::InCar { .. }) { Vec3::NAN } else { p.fig.pos })
                .collect();
            let mut seed = self.rngs.barrel;
            for (i, &at) in now.iter().enumerate() {
                let was = self.barrel_marks.get(i).copied().unwrap_or(at);
                if at.is_nan() || was.is_nan() || at == was {
                    continue;
                }
                barrel::hit_with_sphere(&mut self.barrels, at, 0x50 as f32, &mut || rand(&mut seed));
            }
            self.barrel_marks = now;
            for k in barrel::process(&mut self.barrels, &Ground(level), &mut || rand(&mut seed)) {
                let at = self.barrels[k].pos;
                self.sounds.push((Sfx::Cone, at));
            }
            self.rngs.barrel = seed;
        }
    }
}
