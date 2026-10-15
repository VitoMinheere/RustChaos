//! The Men in Black (`PersonIsMIB`, fallen/Source/Person.cpp): their rounds never run out,
//! and a while after one goes down the body goes up in lightning (Person.cpp:17536).

use crate::*;

/// `Timer1` reaching `20 * 20 * 5` at 16 a tick, 20 ticks a second.
const GOES_UP: f32 = (20 * 20 * 5) as f32 / (16.0 * 20.0);

impl Game {
    pub(crate) fn men_in_black(&mut self, level: &Level) {
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
            }
        }
    }

    /// The body goes up (Person.cpp:17546): a white flash, smoke, ten bolts of lightning from
    /// a thousand up down to the ground around it, `S_MIB_EXPLODE`, a shockwave of 0x300 for
    /// 200, a gust through the litter, and it's gone.
    fn mib_goes_up(&mut self, i: usize, level: &Level) {
        let at = self.people[i].body_pos() + Vec3::new(0.0, 40.0, 0.0);
        self.pyro.burst(at, pyro::FLASH | pyro::DUST, 120.0);
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
