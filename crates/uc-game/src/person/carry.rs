//! Carrying someone, as Roper does the injured.

use super::*;

impl Person {
    /// `fn_person_carry` and the carrying run: pick up, stand, walk at running speed with the
    /// carrying walk, and with action put them down if there's room ahead (192). Nothing else
    /// while carrying: no jumping, fighting, guns or ladders.
    pub(super) fn step_carry(&mut self, victim: usize, phase: u8, c: &Controls, ground: &Ground, map: &Map, dt: f32) {
        let moving = c.wish.length_squared() > 0.01;
        match phase {
            0 | 3 => {
                if self.fig.finished() {
                    if phase == 0 {
                        self.fig.restart(anim::STAND_CARRY, Play::Loop);
                        self.state = State::Carry { victim, phase: 1 };
                    } else {
                        self.state = State::Move;
                        self.idle(false);
                    }
                }
            }
            _ if c.action => {
                let up = vec3(0.0, 64.0, 0.0);
                let f = self.fig.facing();
                let ahead = self.fig.pos + vec3(f.x, 0.0, f.y) * 192.0;
                if ground.line_of_sight(self.fig.pos + up, ahead + up) {
                    self.fig.restart(anim::PUTDOWN_CARRY, Play::Once);
                    self.state = State::Carry { victim, phase: 3 };
                }
            }
            _ if moving => {
                let dir = c.wish.normalize();
                self.fig.turn_towards(dir, 8.0, dt);
                if phase == 1 {
                    self.fig.restart(anim::START_WALK_CARRY, Play::Once);
                    self.state = State::Carry { victim, phase: 2 };
                } else if self.fig.anim() != anim::START_WALK_CARRY || self.fig.finished() {
                    self.fig.play(anim::WALK_CARRY, Play::Loop);
                }
                self.walk(self.fig.facing() * RUN_SPEED * dt, ground, map, false);
            }
            2 => {
                self.fig.restart(anim::STAND_CARRY, Play::Loop);
                self.state = State::Carry { victim, phase: 1 };
            }
            _ => {}
        }
    }

    /// Where the person carried by this one goes, facing back the other way, and how they're
    /// animated to match (`carry_running`, `set_person_stand_carry`).
    pub fn carried_pose(&self) -> Option<(usize, Vec3, f32, usize, Play)> {
        let State::Carry { victim, phase } = self.state else { return None };
        let f = self.fig.facing();
        let at = self.fig.pos + vec3(f.x, 0.0, f.y) * CARRY_AHEAD;
        let (a, play) = match phase {
            0 => (anim::PICKUP_CARRY_V, Play::Once),
            1 => (anim::STAND_CARRY_V, Play::Loop),
            2 if self.fig.anim() == anim::START_WALK_CARRY => (anim::START_WALK_CARRY_V, Play::Once),
            2 => (anim::WALK_CARRY_V, Play::Loop),
            _ => (anim::PUTDOWN_CARRY_V, Play::Once),
        };
        Some((victim, at, self.fig.angle + std::f32::consts::PI, a, play))
    }

    /// Carrying someone.
    pub fn carrying(&self) -> Option<usize> {
        match self.state {
            State::Carry { victim, .. } => Some(victim),
            _ => None,
        }
    }
}
