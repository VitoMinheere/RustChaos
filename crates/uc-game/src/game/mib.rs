//! The Men in Black (`PersonIsMIB`, fallen/Source/Person.cpp): their rounds never run out,
//! and once one is down for good the body rises off the ground, crackling with lightning
//! (`DRAWXTRA_MIB_destruct`), until it goes up in a lightning strike (Person.cpp:17536).

use crate::*;

/// `Timer1` reaching `20 * 20 * 5` at 16 a tick, 20 ticks a second.
const GOES_UP: f32 = (20 * 20 * 5) as f32 / (16.0 * 20.0);

impl Game {
    pub(crate) fn men_in_black(&mut self, dt: f32, level: &Level) {
        for i in 0..self.people.len() {
            let p = &mut self.people[i];
            if !p.is_mib() || p.removed {
                continue;
            }
            // "Men In Black all have AK47s" (Person.cpp:5914), with no end of rounds.
            if !p.has(Weapon::Ak47) && !p.knocked_out {
                p.give(Weapon::Ak47);
                p.weapon = Weapon::Ak47;
            }
            if let Some(slot) = Weapon::Ak47.slot() {
                p.ammo[slot] = p.ammo[slot].max(30);
            }
            if p.knocked_out && p.down_timer >= GOES_UP {
                self.mib_goes_up(i, level);
            } else if p.knocked_out {
                self.mib_levitates(i, dt, level);
            }
        }
    }

    /// Down for good, the body rises (`WorldPos.Y += SIN(Timer1 >> 2) >> 7` a frame, about a
    /// storey by the time it goes up), and lightning crackles from the ground round it to its
    /// hips every other tick, from further off as time goes on.
    fn mib_levitates(&mut self, i: usize, dt: f32, level: &Level) {
        let t = self.people[i].down_timer;
        // `Timer1` counts 16 a tick: `Timer1 >> 2` is 80 a second, of 2048 to the circle;
        // `>> 7` of a 16.16 sine is up to 2 units a frame, at 30 frames a second.
        let a = 80.0 * t * std::f32::consts::TAU / 2048.0;
        self.people[i].fig.pos.y += 60.0 * a.sin() * dt;
        self.people[i].crackle -= dt;
        if self.people[i].crackle > 0.0 {
            return;
        }
        self.people[i].crackle = 0.1;
        let hips = self.people[i].body_pos() + Vec3::new(0.0, 30.0, 0.0);
        let ctr = t * 320.0;
        let spread = if ctr < 400.0 { 127.0 } else if ctr < 800.0 { 255.0 } else { 511.0 };
        let mut off = || (self.rand() % 1000) as f32 / 1000.0 * 2.0 * spread - spread;
        let (x, z) = (hips.x + off(), hips.z + off());
        let ground = level.ground.map_height(&level.map, x, z);
        self.sparks.push(Spark { from: Vec3::new(x, ground, z), to: hips, left: 25.0 / 20.0 });
    }

    /// The body goes up (Person.cpp:17546): a white bloom, pale clouds climbing, ten bolts of lightning from
    /// a thousand up down to the ground around it, `S_MIB_EXPLODE`, a shockwave of 0x300 for
    /// 200, a gust through the litter, and it's gone.
    fn mib_goes_up(&mut self, i: usize, level: &Level) {
        let at = self.people[i].body_pos() + Vec3::new(0.0, 40.0, 0.0);
        self.pyro.lightning_strike(at);
        self.pyro.burst(at, pyro::DUST, 400.0);
        for _ in 0..10 {
            let mut off = || (self.rand() & 0x3f) as f32 - 0x1f as f32;
            let (x, z) = (at.x + off(), at.z + off());
            let ground = level.ground.map_height(&level.map, x, z);
            let left = (25 + (self.rand() & 0xf)) as f32 / 20.0;
            self.sparks.push(Spark { from: Vec3::new(x, at.y + 1000.0, z), to: Vec3::new(x, ground, z), left });
        }
        self.sounds.push((Sfx::MibExplode, at));
        self.shockwave(level, at, 0x300 as f32, 200, Blame::Nobody);
        self.dirt.gust(at.x as i32, at.z as i32, at.x as i32 + 400, at.z as i32);
        self.people[i].removed = true;
        self.people[i].blown_up = true;
    }
}
