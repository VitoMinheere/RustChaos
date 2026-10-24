//! Dynamic lights (`NIGHT_dlight_*`, fallen/Source/night.cpp; #240): what lights the scene
//! up as it goes, over the level's baked lighting.
//!
//! A dynamic light adds its colour to the light at each point within its radius, falling
//! off evenly to nothing at the edge (`bright = 256 - dist * 256 / radius`, night.cpp:1730).
//! The original's radius is a quarter of the world units (`lradius = radius << 2`) and its
//! colours are added to the light before `NIGHT_get_d3d_colour` multiplies by 4, so here
//! each light's colour is the original's times 4 / 255, on the 0 to 1 scale of the vertex
//! colours.
//!
//! - An explosion's fireball (`PYRO_EXPLODE2`, pyro.cpp:543): out to half its scale, a white
//!   that goes yellow as it cools (`COS(counter << 2) >> 10` for red and green, half that for
//!   blue), gone in about a second.
//! - A fire burning (`PYRO_BONFIRE`, pyro.cpp:481 and 745): out to 400, a flickering orange.
//! - A gun going off (Person.cpp:6229): for one frame, a little ahead of the shooter and 0x60
//!   up, out to 400.
//! - A car at night (vehicle.cpp:982 and 4489): 512 ahead of it and 32 up, out to 800: its
//!   headlights on the road.
//! - The Balrog, on fire (bat.cpp:2507): out to 800, at its middle.
//! - The end of the world (`PYRO_GAMEOVER`, pyro.cpp:1084): eight lights round the player,
//!   out to 800, from dark to a flickering fiery red.

use crate::*;

/// One light: where it is and how far it reaches (world units), and what it adds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub pos: Vec3,
    pub radius: f32,
    pub rgb: [f32; 3],
}

/// The original's 0..255 light units to the vertex colours' 0 to 1.
fn units(r: f32, g: f32, b: f32) -> [f32; 3] {
    [r, g, b].map(|c| c * 4.0 / 255.0)
}

impl Game {
    /// Every dynamic light this frame.
    pub fn lights(&self) -> Vec<Light> {
        let mut out = Vec::new();
        // A flicker that comes from when it is and which light it is, not from the game's
        // chance (0 to 1).
        let flicker = |k: f32| ((self.time * 23.0 + k * 7.3).sin() * 0.5 + 0.5) * ((self.time * 37.0 + k * 3.1).sin() * 0.25 + 0.75);
        for (pos, radius, age) in self.pyro.flashes() {
            // The counter climbs 7 a tick; COS of four times it, over a quarter turn.
            let counter = age * 20.0 * 7.0;
            let a = (counter * 4.0).min(512.0) / 2048.0 * std::f32::consts::TAU;
            let rgb = a.cos() * 64.0;
            if rgb > 0.0 {
                out.push(Light { pos, radius, rgb: units(rgb, rgb, rgb / 2.0) });
            }
        }
        for (k, (pos, _)) in self.pyro.fires().enumerate() {
            let max = 0x90 as f32 + 0x50 as f32 * flicker(k as f32);
            out.push(Light { pos: pos + Vec3::new(0.0, 32.0, 0.0), radius: 400.0, rgb: units(max, max - 15.0 * flicker(k as f32 + 0.5), 0.0) });
        }
        for p in self.people.iter().filter(|p| !p.removed && p.flash > 0.0 && p.weapon.is_gun()) {
            let f = p.facing_dir();
            let at = p.fig.pos + Vec3::new(f.x * 128.0, 0x60 as f32, f.y * 128.0);
            out.push(Light { pos: at, radius: 400.0, rgb: units(30.0, 25.0, 5.0) });
        }
        if self.night {
            for v in self.vehicles.iter().filter(|v| !v.wrecked()) {
                let ahead = v.to_world(0.0, 512.0);
                out.push(Light { pos: Vec3::new(ahead.x, v.pos.y + 32.0, ahead.y), radius: 800.0, rgb: units(35.0, 32.0, 10.0) });
            }
        }
        for c in self.creatures.iter().filter(|c| c.kind == creature::BALROG && !c.dead) {
            out.push(Light { pos: c.fig.pos + Vec3::new(0.0, 64.0, 0.0), radius: 800.0, rgb: units(32.0, 28.0, 10.0) });
        }
        if let (Some(t), Some(p)) = (self.end_of_world, self.player()) {
            // The counter climbs 3 a tick; the lights' red is an eighth of it.
            let col = ((t * 20.0 * 3.0).min(253.0) / 8.0).floor();
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let at = p.fig.pos + Vec3::new(a.sin() * 1024.0, 10.0, a.cos() * 1024.0);
                let col2 = (col / 2.0 * flicker(i as f32)).floor();
                out.push(Light { pos: at, radius: 800.0, rgb: units(col, col2, 0.0) });
            }
        }
        out
    }
}
