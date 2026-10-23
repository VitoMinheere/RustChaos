//! Back to a wall (#205; `STATE_HUG_WALL`, `fn_person_hug_wall`, fallen/Source/Person.cpp:
//! 21468): action standing still in front of a building's wall turns her round flat against
//! it; left and right edge her along it, and at its end she looks round the corner, from where
//! action leaps out.

use super::*;

/// What she's doing against the wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hug {
    /// Turning round to put her back to it (`SUB_STATE_HUG_WALL_TURN`).
    Turn,
    Stand,
    /// Edging along it: `dir` is +1 towards the wall's second end, -1 towards its first.
    Sidle { dir: f32 },
    /// At its end, looking round the corner the way she was going (`SUB_STATE_HUG_WALL_LOOK_L`,
    /// `_R`), for `ticks` (up to 16, `InsideRoom`, which leads the camera round).
    Look { dir: f32, ticks: f32 },
    /// Leaping out round the corner (`SUB_STATE_HUG_WALL_LEAP_OUT`).
    LeapOut,
}

/// How far from the wall's line she stands: `fn_person_hug_wall` keeps her 32 off it.
const GAP: f32 = 32.0;

/// Edging speed: 5 a tick (Person.cpp:21497).
const SIDLE_SPEED: f32 = 5.0 * 20.0;

/// The animations, unarmed, with a two-handed gun and with the pistol (`ANIM_PRESS_WALL_*`,
/// fallen/Headers/animate.h:533). The left and right ones are named as seen from in front of
/// her: edging to her own left is the `_R` set.
const TURN: [usize; 3] = [322, 326, 341];
const STAND: [usize; 3] = [323, 327, 342];
const SIDLE_L: [usize; 3] = [324, 328, 343];
const SIDLE_R: [usize; 3] = [325, 329, 344];
const LOOK_L: [usize; 3] = [349, 347, 345];
const LOOK_R: [usize; 3] = [350, 348, 346];
const FREEZE_L: [usize; 3] = [355, 353, 351];
const FREEZE_R: [usize; 3] = [356, 354, 352];

impl Person {
    /// Which of each animation's three: unarmed, two-handed or the pistol (`gun`, Person.cpp:
    /// 7360).
    fn hug_set(&self) -> usize {
        match self.weapon {
            Weapon::Pistol => 2,
            Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper => 1,
            _ => 0,
        }
    }

    /// Puts her back to the wall from `a` to `b`, `out` its normal towards her
    /// (`set_person_turn_to_hug_wall`): she faces it, and turns round.
    pub(super) fn start_hug(&mut self, a: Vec2, b: Vec2, out: Vec2) -> bool {
        self.fig.angle = uc_world::character::angle_of(-out);
        if !self.fig.restart(TURN[self.hug_set()], Play::Once) {
            return false;
        }
        self.state = State::HugWall { a, b, out, hug: Hug::Turn };
        true
    }

    /// Whether she's moving towards her own left going `dir` along the wall from `a` to `b`.
    fn to_her_left(&self, a: Vec2, b: Vec2, dir: f32) -> bool {
        let f = self.fig.facing();
        let right = vec2(-f.y, f.x);
        (b - a).dot(right) * dir < 0.0
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_hug(&mut self, a: Vec2, b: Vec2, out: Vec2, hug: Hug, c: &Controls, ground: &Ground, map: &Map, dt: f32) {
        let set = self.hug_set();
        let along = (b - a).normalize_or_zero();
        let p = vec2(self.fig.pos.x, self.fig.pos.z);
        // Where she stands against it: on her line out from the wall, `GAP` off it.
        let on_wall = |p: Vec2| a + along * (p - a).dot(along) + out * GAP;
        let away = c.wish.dot(out) > 0.5;
        let side = c.wish.dot(along);
        let hugging = |hug| State::HugWall { a, b, out, hug };
        match hug {
            Hug::Turn => {
                // Up against it as she turns.
                let to = on_wall(p);
                let step = (to - p).clamp_length_max(SIDLE_SPEED * dt);
                self.fig.pos += vec3(step.x, 0.0, step.y);
                if self.fig.finished() {
                    self.fig.angle = uc_world::character::angle_of(out);
                    self.fig.restart_locked(STAND[set], Play::Loop);
                    self.fig.lift = 0.0;
                    self.state = hugging(Hug::Stand);
                }
            }
            // Walking away from it, she's off.
            _ if away && hug != Hug::LeapOut => {
                self.state = State::Move;
                self.idle(false);
            }
            Hug::Stand => {
                // Flat against it, wherever the turn's animation left her.
                let to = on_wall(p);
                (self.fig.pos.x, self.fig.pos.z) = (to.x, to.y);
                if side.abs() > 0.5 {
                    self.state = hugging(Hug::Sidle { dir: side.signum() });
                }
            }
            Hug::Sidle { dir } => {
                // Only while the way is held (`continue_dir`).
                if side * dir < 0.5 {
                    self.fig.restart(STAND[set], Play::Loop);
                    self.state = hugging(Hug::Stand);
                    return;
                }
                let anim = if self.to_her_left(a, b, dir) { SIDLE_R[set] } else { SIDLE_L[set] };
                self.fig.play(anim, Play::Loop);
                let to = on_wall(p + along * dir * SIDLE_SPEED * dt);
                // The wall runs out just ahead: look round its end (`check_near_facet` 40 on).
                let t = (to + along * dir * 40.0 - a).dot(along) / (b - a).length().max(1.0);
                if !(0.0..=1.0).contains(&t) {
                    let look = if self.to_her_left(a, b, dir) { LOOK_R[set] } else { LOOK_L[set] };
                    self.fig.restart(look, Play::Once);
                    self.state = hugging(Hug::Look { dir, ticks: 0.0 });
                    return;
                }
                // A step up or down, a wall across the way: stop (`move_ok`).
                let y = ground.surface(map, to.x, to.y, self.fig.pos.y + STEP_HEIGHT);
                let mid = vec3(0.0, 64.0, 0.0);
                let ahead = vec3(to.x, self.fig.pos.y, to.y) + vec3(along.x, 0.0, along.y) * dir * RADIUS;
                if (y - self.fig.pos.y).abs() > 60.0 || !ground.line_of_sight(self.fig.pos + mid, ahead + mid) {
                    self.fig.restart(STAND[set], Play::Loop);
                    self.state = hugging(Hug::Stand);
                    return;
                }
                self.fig.pos = vec3(to.x, y, to.y);
            }
            Hug::Look { dir, ticks } => {
                if c.action {
                    // Out round the corner (`set_person_hug_wall_leap_out`).
                    let leap = if self.to_her_left(a, b, dir) { FREEZE_R[set] } else { FREEZE_L[set] };
                    if self.fig.restart(leap, Play::Once) {
                        self.state = hugging(Hug::LeapOut);
                        return;
                    }
                }
                if side * dir < -0.5 {
                    self.state = hugging(Hug::Sidle { dir: -dir });
                    return;
                }
                self.state = hugging(Hug::Look { dir, ticks: (ticks + dt * 20.0).min(16.0) });
            }
            Hug::LeapOut => {
                // The leap ends facing back the way she came: turn her to match, keep her
                // hips where the animation left them, and stand ready (Person.cpp:21539).
                if self.fig.finished() {
                    let hips = self.fig.hips();
                    self.fig.angle += std::f32::consts::PI;
                    self.fig.restart_now(anim::STAND_READY, Play::Loop);
                    let d = hips - self.fig.hips();
                    self.fig.pos += vec3(d.x, 0.0, d.z);
                    self.fig.lift = 0.0;
                    let floor = ground.surface(map, self.fig.pos.x, self.fig.pos.z, self.fig.pos.y + STEP_HEIGHT);
                    self.fig.pos.y = floor;
                    self.state = State::Move;
                }
            }
        }
    }

    /// Against a wall looking round its end: how far the camera leads round the corner, in
    /// world units (fc.cpp:703: `InsideRoom` sixteenths of a square).
    pub fn peek(&self) -> Option<Vec2> {
        match self.state {
            State::HugWall { a, b, hug: Hug::Look { dir, ticks }, .. } => Some((b - a).normalize_or_zero() * dir * ticks * 16.0),
            _ => None,
        }
    }

    /// Back to a wall.
    pub fn hugging(&self) -> bool {
        matches!(self.state, State::HugWall { .. })
    }
}
