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
}

impl Orbit {
    /// Behind a character facing (-sin a, -cos a).
    pub(crate) fn behind(angle: f32) -> Self {
        Orbit { yaw: angle + std::f32::consts::PI, pitch: -0.30, dist: 2.4 }
    }
    pub(crate) fn cam(&self, target: Vec3) -> FlyCam {
        let mut c = FlyCam { pos: Vec3::ZERO, yaw: self.yaw, pitch: self.pitch };
        c.pos = target - c.forward() * self.dist;
        c
    }
}
