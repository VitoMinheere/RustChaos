//! Dogs (#110, [`crate::canid`]): an unreleased extra, off unless asked for. The console's
//! `dog` puts one in front of the player, and the viewer's `--dogs N` lets N loose round
//! where she starts.

use crate::*;

impl Game {
    /// The dog's model, loaded once.
    fn dog_figure(&mut self, level: &Level, at: Vec3, angle: f32) -> Option<Figure> {
        if self.dog_chunk.is_none() {
            let path = uc_formats::resolve(&level.game, canid::CHUNK)?;
            let mut chunk = uc_formats::all::AnimChunk::parse(&std::fs::read(path).ok()?).ok()?;
            // The file says 28 elements a keyframe, one per mesh, but holds 15: the legs,
            // hips, body and head. Read 28 and each pose runs into the next (#110).
            if let Some(stride) = chunk.frame_stride() {
                chunk.element_count = chunk.element_count.min(stride);
            }
            self.dog_chunk = Some(std::sync::Arc::new(chunk));
        }
        Figure::anim_prim(self.dog_chunk.clone()?, at, angle, canid::WALK)
    }

    /// A dog at `at` (on the ground under it), facing `angle` (radians). Whether there was
    /// one to make: not without `DOG2.all`.
    pub fn spawn_dog(&mut self, level: &Level, at: Vec3, angle: f32) -> bool {
        let Some(fig) = self.dog_figure(level, at, angle) else { return false };
        let ground = level.map.ground_height(at.x, at.z);
        let dog = canid::Canid::new(fig, at, angle, ground, &mut self.rngs.dog);
        self.dogs.push(dog);
        true
    }

    /// `dog`: a dog three squares in front of the player, facing her. Not while driving.
    pub fn spawn_dog_ahead(&mut self, level: &Level) -> bool {
        let Some(p) = self.player().filter(|p| matches!(p.state, State::Move)) else { return false };
        let f = p.facing_dir();
        let at = p.fig.pos + Vec3::new(f.x, 0.0, f.y) * 3.0 * 256.0;
        // Facing back the way she faces: towards her.
        let angle = p.fig.angle + std::f32::consts::PI;
        self.spawn_dog(level, at, angle)
    }

    /// `--dogs N`: N dogs on open ground (with no roof over it) between three and ten
    /// squares from the player. How many it found room for.
    pub fn let_dogs_out(&mut self, level: &Level, n: usize) -> usize {
        let Some(me) = self.player().map(|p| p.fig.pos) else { return 0 };
        let mut made = 0;
        for _ in 0..n * 40 {
            if made == n {
                break;
            }
            let a = (rand(&mut self.rngs.dog) & 2047) as f32 * std::f32::consts::TAU / 2048.0;
            let r = (3.0 + (rand(&mut self.rngs.dog) % 8) as f32) * 256.0;
            let (x, z) = (me.x + a.sin() * r, me.z + a.cos() * r);
            let ground = level.map.ground_height(x, z);
            let open = level.ground.surface(&level.map, x, z, 1.0e5) <= ground + 1.0
                && level.ground.line_of_sight(Vec3::new(x, ground + 64.0, z), me + Vec3::Y * 64.0);
            if open && self.spawn_dog(level, Vec3::new(x, ground, z), a) {
                made += 1;
            }
        }
        made
    }

    /// The dogs' tick, on the creatures' clock.
    pub(crate) fn dog_tick(&mut self, around: &creature::Around) {
        if self.cheats.freeze {
            return;
        }
        let mut acts = Vec::new();
        for d in &mut self.dogs {
            d.tick(around, &mut self.rngs.dog, &mut acts);
        }
        for a in acts {
            match a {
                canid::Act::Sound { id, at } => self.sounds.push((Sfx::ById { id, replace: false }, at)),
            }
        }
    }
}
