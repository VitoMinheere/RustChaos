//! The sniper rifle's scope: the original's sniper mode (fallen/Source/snipe.cpp), which the
//! shipped game never turns on (its only way in is a debug key in a commented-out block,
//! Controls.cpp:4769). Here the scope key turns it on with the sniper rifle in hand: the
//! view goes to Darci's head, facing her way, and turns with a little momentum
//! (`SNIPE_turn`, `SNIPE_process`), springing back past 300 up or down, while the lens eases
//! in from 20 to 30. Firing turns her to the scope's heading and hits the best target
//! there (`SNIPE_shoot`, `find_snipe_target`) for 260.

use crate::*;

/// `SNIPE_LENS_START`, `_END` (16.16), and the original's ordinary lens (`AENG_lens`, 4).
const LENS_START: i64 = 20 << 16;
const LENS_END: i64 = 30 << 16;
const LENS_NORMAL: f32 = 4.0;
/// `SNIPE_TURN_SPEED_YAW`, `_PITCH`, and `SNIPE_PITCH_MAX` (16.16, 2048ths of a turn).
const TURN: i64 = 0x6000;
const PITCH_MAX: i64 = 300 << 16;
/// The game's turn.
const TICK: f32 = 1.0 / 20.0;

/// The scope (`SNIPE_*`).
#[derive(Debug, Clone)]
pub struct Snipe {
    yaw: i64,
    pitch: i64,
    dyaw: i64,
    dpitch: i64,
    lens: i64,
    dlens: i64,
    /// Where it looks from: her head.
    pub eye: Vec3,
    clock: f32,
}

impl Snipe {
    /// The way it looks (a unit vector), and how far it's zoomed in (the lens over the
    /// ordinary one).
    pub fn view(&self) -> (Vec3, f32) {
        let yaw = (self.yaw >> 16) as f32 * std::f32::consts::TAU / 2048.0;
        let pitch = (self.pitch >> 16) as f32 * std::f32::consts::TAU / 2048.0;
        let (sp, cp) = pitch.sin_cos();
        (Vec3::new(yaw.sin() * cp, sp, yaw.cos() * cp), (self.lens as f32 / 65536.0) / LENS_NORMAL)
    }

    /// The angle a person faces looking the scope's way (`SNIPE_yaw >> 16` + 1024).
    fn facing(&self) -> f32 {
        (((self.yaw >> 16) + 1024) & 2047) as f32 * std::f32::consts::TAU / 2048.0
    }

    /// `SNIPE_process`, one turn.
    fn process(&mut self) {
        if self.pitch > PITCH_MAX {
            self.dpitch -= (self.pitch - PITCH_MAX) >> 4;
        }
        if self.pitch < -PITCH_MAX {
            self.dpitch -= (self.pitch + PITCH_MAX) >> 4;
        }
        self.pitch += self.dpitch;
        self.yaw += self.dyaw;
        self.dyaw -= self.dyaw >> 3;
        self.dpitch -= self.dpitch >> 3;
        if self.lens < LENS_END {
            self.dlens += 0x100;
        }
        self.dlens = self.dlens.min(0x1000);
        self.lens += self.dlens;
        self.dlens -= self.dlens >> 3;
        for d in [&mut self.dpitch, &mut self.dyaw, &mut self.dlens] {
            if (-20..=20).contains(d) {
                *d = 0;
            }
        }
    }
}

impl Game {
    /// Looking through the scope.
    pub fn sniping(&self) -> Option<&Snipe> {
        self.snipe.as_ref()
    }

    /// The scope a frame: the scope key in and out of it (only standing, with the rifle in
    /// hand), the direction keys to turn it, and the trigger to shoot. Returns whether it
    /// had the controls (the player stands still meanwhile).
    pub(crate) fn scope(&mut self, dt: f32, input: &Controls) -> bool {
        let Some(pi) = self.player else { return false };
        let p = &self.people[pi];
        let able = p.weapon == Weapon::Sniper && matches!(p.state, State::Move) && self.outcome.is_none();
        if !able {
            self.snipe = None;
            return false;
        }
        if input.scope {
            self.snipe = match self.snipe {
                Some(_) => None,
                // `SNIPE_mode_on` at her head (`SUB_OBJECT_HEAD`), facing her way.
                None => {
                    let eye = p.fig.part_world(11).map_or(p.fig.pos + Vec3::new(0.0, 0xc0 as f32, 0.0), |(_, at)| at);
                    let angle = (p.fig.angle * 2048.0 / std::f32::consts::TAU) as i64;
                    let yaw = ((angle + 1024) & 2047) << 16;
                    Some(Snipe { yaw, pitch: 0, dyaw: 0, dpitch: 0, lens: LENS_START, dlens: 0, eye, clock: 0.0 })
                }
            };
        }
        let Some(s) = self.snipe.as_mut() else { return false };
        // `SNIPE_turn` while a key's held, a turn at a time: left, right, up and down.
        s.clock += dt;
        while s.clock >= TICK {
            s.clock -= TICK;
            let d = input.drive;
            if d.steer < -0.3 {
                s.dyaw += TURN;
            }
            if d.steer > 0.3 {
                s.dyaw -= TURN;
            }
            if d.throttle > 0.3 {
                s.dpitch += TURN;
            }
            if d.throttle < -0.3 {
                s.dpitch -= TURN;
            }
            s.process();
        }
        if input.punch {
            self.snipe_shoot(pi);
        }
        true
    }

    /// `SNIPE_shoot`: she turns to the scope's heading and the best target in its sights
    /// (`find_snipe_target`) is hit for 260.
    fn snipe_shoot(&mut self, pi: usize) {
        let Some(s) = &self.snipe else { return };
        let facing = s.facing();
        let slot = Weapon::Sniper.slot().unwrap();
        let p = &mut self.people[pi];
        if p.ammo[slot] == 0 {
            p.cues.push(Sfx::Click);
            return;
        }
        p.ammo[slot] -= 1;
        p.fig.angle = facing;
        p.cues.push(Sfx::Shot(Weapon::Sniper));
        p.flash = 0.07;
        self.player_fired = true;
        self.gunshot(pi);
        let me = self.people[pi].fig.pos;
        let angle = (facing * 2048.0 / std::f32::consts::TAU) as i64 & 2047;
        let best = (0..self.people.len())
            .filter(|&i| i != pi && !self.people[i].removed && !self.people[i].is_dead())
            .filter_map(|i| snipe_score(me, angle, self.people[i].fig.pos).map(|s| (i, s)))
            .max_by_key(|&(_, s)| s)
            .map(|(i, _)| i);
        if let Some(t) = best {
            let was_down = self.people[t].is_down();
            self.people[t].take_hit(me, SNIPER_DAMAGE, 0, true, false, Weapon::Sniper);
            if !was_down && self.people[t].is_down() {
                let kind = self.people[pi].kind;
                self.people[pi].cues.push(Sfx::Taunt { kind });
            }
            self.drop_weapon(t);
        }
    }
}

/// `calc_snipe_target_score` (guns.cpp:721): no one more than 19 squares away along x or z,
/// 13 up or down, or 20 by 10 squares' worth all told; and only within 56 (of 2048) either
/// side of where she faces, the nearer the middle the better.
fn snipe_score(me: Vec3, angle: i64, at: Vec3) -> Option<i64> {
    let (dx, dy, dz) = ((me.x - at.x) as i64, (me.y - at.y) as i64, (me.z - at.z) as i64);
    if dx.abs() > 19 * 256 || dy.abs() > 13 * 256 || dz.abs() > 19 * 256 {
        return None;
    }
    if dx * dx + dy * dy + dz * dz > 20 * 256 * 10 * 256 {
        return None;
    }
    // `Arctan(dx, -dz)`: the angle someone faces to look from her to them (people face
    // (-sin, -cos) of their angle).
    let toward = ((dx as f64).atan2(dz as f64) * 2048.0 / std::f64::consts::TAU).round() as i64 & 2047;
    let diff = (toward - angle + 2048) & 2047;
    if (56..=2048 - 56).contains(&diff) {
        return None;
    }
    Some(if diff < 256 { 256 - diff } else { diff - (2048 - 256) })
}
