//! Wet roads (#522): how wet the ground is, for the renderer's sheen and streaks of light.
//! Not in the original, which draws only its puddles (puddle.cpp).
//!
//! It rains on every night mission and on no day one (`GF_RAINING`, set in
//! `ELEV_game_init`, elev.cpp:2404, when the lighting has no `NIGHT_FLAG_DAYTIME`), and it
//! never starts or stops during a mission: the debug key that switched it (aeng.cpp:10937)
//! is commented out. So a rainy level starts soaked and stays so. The ramp is there for rain
//! that comes and goes: the ground wets in [`WET_SECS`] once it rains and dries in
//! [`DRY_SECS`] once it stops.

use crate::*;

/// Seconds from dry to soaked once it rains.
pub const WET_SECS: f32 = 20.0;
/// Seconds from soaked to dry once it stops: drying is slower than wetting.
pub const DRY_SECS: f32 = 120.0;

/// How wet the ground is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wetness {
    /// From 0, dry, to 1, soaked.
    pub amount: f32,
}

impl Wetness {
    /// At the start of a level: soaked if it's raining, as it's been raining a while.
    pub fn new(raining: bool) -> Wetness {
        Wetness { amount: if raining { 1.0 } else { 0.0 } }
    }

    /// `dt` seconds on: wetter while it rains, drier while it doesn't.
    pub fn update(&mut self, dt: f32, raining: bool) {
        self.amount = if raining {
            (self.amount + dt / WET_SECS).min(1.0)
        } else {
            (self.amount - dt / DRY_SECS).max(0.0)
        };
    }
}

/// A bright light whose streak shows on wet ground: where it is (world units), its colour
/// (0 to 1), and the way it shines (zero for every way).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glint {
    pub pos: Vec3,
    pub rgb: [f32; 3],
    pub facing: Vec3,
}

impl Game {
    /// The headlights of the cars being driven at night, the ones that glow
    /// (vehicle.cpp:1236, `0x606040`), shining ahead of them.
    pub fn headlight_glints(&self, level: &Level) -> Vec<Glint> {
        if !self.night {
            return Vec::new();
        }
        let rgb = [0x60, 0x60, 0x40].map(|c| c as f32 / 255.0);
        self.vehicles.iter()
            .filter(|v| v.driver.is_some() && !v.wrecked())
            .flat_map(|v| {
                let (lights, ahead) = v.headlights(v.lift(&level.item_prims));
                lights.map(|pos| Glint { pos, rgb, facing: ahead })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rainy_level_starts_soaked_and_a_dry_one_dry() {
        assert_eq!(Wetness::new(true).amount, 1.0);
        assert_eq!(Wetness::new(false).amount, 0.0);
    }

    #[test]
    fn it_wets_in_twenty_seconds_and_dries_in_two_minutes() {
        let mut w = Wetness::new(false);
        w.update(WET_SECS / 2.0, true);
        assert!((w.amount - 0.5).abs() < 1e-5);
        w.update(WET_SECS, true);
        assert_eq!(w.amount, 1.0, "no wetter than soaked");
        // A second's rain is a twentieth; a second's dry spell a hundred and twentieth.
        w.update(1.0, false);
        assert!((w.amount - (1.0 - 1.0 / DRY_SECS)).abs() < 1e-5);
        for _ in 0..(DRY_SECS as usize * 30) {
            w.update(1.0 / 30.0, false);
        }
        assert_eq!(w.amount, 0.0, "no drier than dry");
    }

    #[test]
    fn steady_rain_keeps_it_soaked() {
        let mut w = Wetness::new(true);
        for _ in 0..300 {
            w.update(1.0 / 30.0, true);
        }
        assert_eq!(w.amount, 1.0);
    }
}
