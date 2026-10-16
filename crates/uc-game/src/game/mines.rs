//! Mines (`SPECIAL_MINE`, fallen/Source/Special.cpp): set off underfoot, under a wheel, by
//! the Balrog, by a shot, or a moment after a blast nearby.

use crate::*;

/// `PRIM_OBJ_MINE` (Editor/Headers/Prim.h).
pub const MINE_PRIM: u8 = 188;
/// `IT_MINE`: the `create_item` that lays one.
pub const IT_MINE: u8 = 7;

/// A laid mine. Nobody can pick it up (`should_person_get_item` says no).
#[derive(Debug, Clone)]
pub struct Mine {
    pub way: usize,
    pub pos: Vec3,
    /// Seconds to going off, once a blast has caught it (`Genus.Special->ammo`).
    pub fuse: Option<f32>,
    pub gone: bool,
}

impl Game {
    /// What sets mines off (`special_process`, Special.cpp:1065): someone whose left foot is
    /// within 0x30 across and no more than 0x20 above it (jumping clears it; the player's
    /// bodyguards never do), the Balrog within 0x60, a car's wheel within 0x50; and a fuse
    /// running out.
    pub(crate) fn run_mines(&mut self, dt: f32, level: &Level) {
        for k in 0..self.mines.len() {
            let m = &mut self.mines[k];
            if m.gone {
                continue;
            }
            if let Some(t) = &mut m.fuse {
                *t -= dt;
                if *t <= 0.0 {
                    self.set_off_mine(k, level);
                }
                continue;
            }
            let at = m.pos;
            let flat = |p: Vec3| Vec2::new(p.x - at.x, p.z - at.z).length();
            let player = self.player;
            let stepped = self.people.iter().any(|p| {
                if p.removed || matches!(p.state, State::InCar { .. } | State::Carried { .. }) || flat(p.fig.pos) >= 0x30 as f32 {
                    return false;
                }
                let foot = p.fig.part_world(3).map_or(p.fig.pos.y, |(_, f)| f.y);
                let guarding_player = p.ai == person::AI_BODYGUARD
                    && (if p.other == 0 { player } else { self.by_way.get(&p.other).copied() }) == player
                    && player.is_some();
                foot <= at.y + 0x20 as f32 && !guarding_player
            });
            let balrog = self.creatures.iter().any(|c| c.kind == creature::BALROG && !c.dead && flat(c.fig.pos) < 0x60 as f32);
            let wheel = self.vehicles.iter().any(|v| {
                let (_, _, wx, fz, rz) = vehicle::TYPES[v.kind as usize];
                [(wx, -rz), (-wx, -rz), (wx, -fz), (-wx, -fz)].iter().any(|&(x, z)| (v.place(Vec3::new(x, 0.0, z)) - at).length() < 0x50 as f32)
            });
            if stepped || balrog || wheel {
                self.set_off_mine(k, level);
            }
        }
    }

    /// `special_activate_mine` (Special.cpp:746): a firebomb and a dust wave, a bang, and a
    /// shockwave of 0x200 doing up to 250 that's nobody's doing, which sets the fuses of
    /// other mines close by.
    pub fn set_off_mine(&mut self, k: usize, level: &Level) {
        let m = &mut self.mines[k];
        if m.gone {
            return;
        }
        m.gone = true;
        let at = m.pos;
        if self.aiming.mine == Some(k) {
            self.aiming.mine = None;
        }
        self.pyro.burst(at, pyro::FIREBOMB | pyro::DUST, 256.0);
        self.sounds.push((Sfx::Explosion, at));
        self.shockwave(level, at, 0x200 as f32, 250, Blame::Nobody);
    }

    /// A blast lights the fuse of every mine within 0x120 that isn't already burning: it goes
    /// off 5 to 12 ticks later (`create_shockwave`, collide.cpp).
    pub(crate) fn light_mines(&mut self, at: Vec3) {
        for k in 0..self.mines.len() {
            if self.mines[k].gone || self.mines[k].fuse.is_some() || (self.mines[k].pos - at).length() >= 0x120 as f32 {
                continue;
            }
            let ticks = (self.rand() & 7) + 5;
            self.mines[k].fuse = Some(ticks as f32 / 20.0);
        }
    }
}
