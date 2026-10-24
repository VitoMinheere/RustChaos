//! The free-flying and orbiting cameras.

use crate::*;

/// Free camera in square units; yaw 0 looks along +z.
#[derive(Clone, Copy)]
pub(crate) struct FlyCam {
    pub(crate) pos: Vec3,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

impl FlyCam {
    pub(crate) fn from(s: [f32; 5]) -> Self {
        FlyCam { pos: vec3(s[0], s[1], s[2]), yaw: s[3].to_radians(), pitch: s[4].to_radians() }
    }
    pub(crate) fn forward(&self) -> Vec3 {
        vec3(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos())
    }
    pub(crate) fn camera(&self) -> Camera {
        Camera { pos: self.pos, target: self.pos + self.forward(), fov_y: 60f32.to_radians(), ortho: None }
    }
}

/// Third-person camera orbiting Darci.
#[derive(Clone, Copy)]
pub(crate) struct Orbit {
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) dist: f32,
    /// What's left of a swing round her (`fc->rotate`, in 2048ths of a turn), from the
    /// rotate keys.
    pub(crate) swing: f32,
}

/// The original's four camera types (`FC_change_camera_type`, fallen/Source/fc.cpp:211): the
/// distance and height for F5 to F8, in world units (the first is `0x280 * CAM_MORE_IN`).
const CAMERA_TYPES: [(f32, f32); 4] = [(480.0, 352.0), (640.0, 512.0), (896.0, 592.0), (768.0, 128.0)];

/// How far above her feet looking round looks from: `0xb000`, in squares.
pub(crate) const LOOK_ABOVE: f32 = 176.0 / 256.0;

/// The default orbit, for the first type.
const DIST: f32 = 2.4;
const PITCH: f32 = -0.30;

impl Orbit {
    /// Behind a character facing (-sin a, -cos a).
    pub(crate) fn behind(angle: f32) -> Self {
        Orbit { yaw: angle + std::f32::consts::PI, pitch: PITCH, dist: DIST, swing: 0.0 }
    }

    /// Looking round from where she stands (#203, the manual's "look around"; `fpm`,
    /// fallen/Source/interfac.cpp:7537): close behind her head, three quarters of a square off
    /// (`FC_position_for_lookaround`, fc.cpp:2058), level to start with. Its target is 176
    /// above her feet ([`LOOK_ABOVE`]).
    pub(crate) fn look(angle: f32) -> Self {
        Orbit { yaw: angle + std::f32::consts::PI, pitch: 0.0, dist: 0.75, swing: 0.0 }
    }

    /// Looking round: up or down by `up` (positive looks up) and round by `round` (radians),
    /// within the original's limits (`look_pitch` from -198 to 256 of 2048: 35 degrees up, 45
    /// down).
    pub(crate) fn look_by(&mut self, up: f32, round: f32) {
        self.pitch = (self.pitch + up).clamp(-45f32.to_radians(), 35f32.to_radians());
        self.yaw += round;
    }

    /// Camera type `t` (0 to 3, F5 to F8): the original's distances and heights, as a
    /// distance and a pitch scaled from this orbit's own for the first type, so F5 is the
    /// default view. F6 and F7 put it higher, close and further off; F8 low and further off.
    pub(crate) fn set_type(&mut self, t: usize) {
        let ((d0, h0), (d, h)) = (CAMERA_TYPES[0], CAMERA_TYPES[t]);
        self.dist = DIST * d / d0;
        self.pitch = PITCH * (h / d).atan() / (h0 / d0).atan();
    }

    /// Starts a swing round her (`FC_rotate_left`, `FC_rotate_right`, fc.cpp:1047): to the
    /// camera's left (`-1`, Delete) or right (`+1`, Page Down).
    pub(crate) fn rotate(&mut self, dir: f32) {
        self.swing = 0x600 as f32 * dir;
    }

    /// The swing for `dt` seconds (fc.cpp:1233): each tick the camera goes sideways by
    /// `rotate / 4096` of the way to her, and `rotate` drops by 0x80, until it's under 0x100.
    /// A tap swings it about 130 degrees round her in half a second.
    pub(crate) fn turn(&mut self, dt: f32) {
        if self.swing == 0.0 {
            return;
        }
        let ticks = dt * 20.0;
        self.yaw += (self.swing / 4096.0).atan() * ticks;
        self.swing -= self.swing.signum() * 0x80 as f32 * ticks;
        if self.swing.abs() < 0x100 as f32 {
            self.swing = 0.0;
        }
    }
    pub(crate) fn cam(&self, target: Vec3) -> FlyCam {
        let mut c = FlyCam { pos: Vec3::ZERO, yaw: self.yaw, pitch: self.pitch };
        c.pos = target - c.forward() * self.dist;
        c
    }
}


impl World {
    /// The original's camera keys beside the mouse (#203; the keyboard's, interfac.cpp:7118):
    /// F5 to F8 the camera types, Delete and Page Down swing the camera round her, End puts
    /// it behind her (`INPUT_MASK_CAM_BEHIND`), and F held looks round (the manual's A); all
    /// but the F keys as the bindings have them (#202).
    pub(crate) fn classic_camera(&mut self, inp: &Input, delta: Vec2, dt: f32) {
        // The editor has its own Delete.
        if self.editor.is_some() {
            return;
        }
        for (t, k) in [KeyCode::F5, KeyCode::F6, KeyCode::F7, KeyCode::F8].into_iter().enumerate() {
            if inp.own(k) {
                self.orbit.set_type(t);
            }
        }
        if inp.tap(Act::CamLeft) {
            self.orbit.rotate(-1.0);
        }
        if inp.tap(Act::CamRight) {
            self.orbit.rotate(1.0);
        }
        self.orbit.turn(dt);
        let angle = self.game.player().map(|p| p.fig.angle);
        let behind = |o: &mut Orbit| {
            if let Some(a) = angle {
                (o.yaw, o.swing) = (Orbit::behind(a).yaw, 0.0);
            }
        };
        if inp.tap(Act::CamBehind) {
            behind(&mut self.orbit);
        }
        // Looking round only on foot, as the original only lets her standing, aiming or
        // reeling (interfac.cpp:7493); the camera goes back behind her after. Not in a
        // replay, where F and the arrows are its own.
        let on_foot = self.game.player_vehicle().is_none() && self.game.riding().is_none() && self.game.sniping().is_none()
            && self.replay.is_none();
        match (inp.on(Act::Look) && on_foot, &mut self.look) {
            (true, None) => self.look = angle.map(Orbit::look),
            (true, Some(look)) => {
                let key = |k| if inp.held(k) { 1.0 } else { 0.0 };
                // Up looks down and Down up, as the manual has it; 13 of 2048 a tick for
                // `look_pitch`, and she turns 32 a tick (interfac.cpp:7571).
                let up = (key(KeyCode::ArrowDown) + inp.axis(Act::Back) - key(KeyCode::ArrowUp) - inp.axis(Act::Forward)).clamp(-1.0, 1.0);
                let round = (key(KeyCode::ArrowLeft) + inp.axis(Act::Left) - key(KeyCode::ArrowRight) - inp.axis(Act::Right)).clamp(-1.0, 1.0);
                let tick = 20.0 * dt * std::f32::consts::TAU / 2048.0;
                look.look_by(up * 13.0 * tick, round * 32.0 * tick);
                if inp.rmb || inp.mouse_look {
                    look.look_by(-delta.y * 0.005, -delta.x * 0.005);
                }
            }
            (false, Some(_)) => {
                self.look = None;
                behind(&mut self.orbit);
            }
            (false, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f5_is_the_default_view_and_the_others_differ() {
        let mut o = Orbit::behind(0.0);
        o.set_type(0);
        assert!((o.dist - DIST).abs() < 1e-5 && (o.pitch - PITCH).abs() < 1e-5);
        o.set_type(2);
        assert!(o.dist > DIST * 1.5, "F7 is further off: {}", o.dist);
        o.set_type(3);
        assert!(o.pitch > PITCH / 2.0, "F8 is low: {}", o.pitch);
    }

    #[test]
    fn a_tap_swings_the_camera_most_of_the_way_round_in_half_a_second() {
        let mut o = Orbit::behind(0.0);
        let start = o.yaw;
        o.rotate(-1.0);
        let mut t = 0.0;
        while o.swing != 0.0 {
            o.turn(1.0 / 60.0);
            t += 1.0 / 60.0;
        }
        let swung = (o.yaw - start).to_degrees();
        assert!((-140.0..-120.0).contains(&swung), "swung {swung}");
        assert!((0.4..0.65).contains(&t), "took {t}");
    }

    #[test]
    fn looking_round_stops_at_the_originals_limits() {
        let mut o = Orbit::look(0.0);
        o.look_by(10.0, 0.0);
        assert!((o.pitch.to_degrees() - 35.0).abs() < 1e-3);
        o.look_by(-10.0, 1.0);
        assert!((o.pitch.to_degrees() + 45.0).abs() < 1e-3);
        assert!((o.yaw - (std::f32::consts::PI + 1.0)).abs() < 1e-5);
    }
}
