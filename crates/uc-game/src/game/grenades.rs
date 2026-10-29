//! Grenades (fallen/Source/grenade.cpp, `SPECIAL_GRENADE` in Special.cpp): primed with
//! attack (or action), thrown with it again (`player_activate_in_hand`), leaving her hand on
//! the throw's fourth frame, bouncing off the ground and walls until the fuse runs out.

use crate::*;

/// `IT_GRENADE`: the `create_item` that puts down grenades.
pub const IT_GRENADE: u8 = 22;
/// `PRIM_OBJ_ITEM_GRENADE`.
pub const GRENADE_PRIM: u8 = 168;
/// `SPECIAL_prime_grenade`: `16 * 20 * 6` counted down by 16 a tick at 20 ticks a second.
pub const FUSE: f32 = 6.0;
/// `ANIM_CAN_RELEASE`: the throw.
pub const ANIM_CAN_RELEASE: usize = 65;

// grenade.cpp works in 1/256 units a tick at 20 ticks a second: these are a second's worth.
/// The throw: `181 << 11` along her facing (of a 16.16 unit vector), `181 << 6` up.
const THROW_ACROSS: f32 = 65536.0 * 181.0 / 2048.0 / 256.0 * 20.0;
const THROW_UP: f32 = (181 << 6) as f32 / 256.0 * 20.0;
/// `dy -= TICK_RATIO * 2` a tick, falling no faster than `0x2000` a tick.
const GRAVITY: f32 = 2.0 * 20.0 * 20.0;
const FASTEST_FALL: f32 = 0x2000 as f32 / 256.0 * 20.0;
/// A bounce slower than `0x400` a tick stops it dead.
const SETTLE: f32 = 0x400 as f32 / 256.0 * 20.0;

/// A grenade in the air or rolling (`Grenade`).
#[derive(Debug, Clone)]
pub struct Grenade {
    /// Who threw it: the blast is theirs.
    pub owner: usize,
    pub pos: Vec3,
    pub vel: Vec3,
    /// Seconds to going off.
    pub fuse: f32,
    pub pitch: f32,
    /// Seconds to the next warning to people near (`PCOM_SOUND_GRENADE_FLY`, every 16 ticks).
    warn: f32,
}

impl Game {
    /// Attack or action with a grenade in hand: the first press primes it
    /// (`SPECIAL_prime_grenade`), the next starts the throw (`set_person_can_release`:
    /// `ANIM_CAN_RELEASE` in `STATE_CANNING`), which lets it go a few frames in. Not inside a
    /// warehouse.
    pub(crate) fn use_grenade(&mut self, pi: usize) {
        if self.indoors.is_some() || !matches!(self.people[pi].state, State::Move) {
            return;
        }
        if self.primed.is_none() {
            self.primed = Some(FUSE);
            self.messages.push(Message { text: "Grenade primed".into(), left: 2.0, face: None, filmed: false });
            return;
        }
        let p = &mut self.people[pi];
        if p.fig.restart(ANIM_CAN_RELEASE, uc_world::Play::Once) {
            p.state = State::Throw { released: false };
        } else {
            self.release_grenade(pi);
        }
    }

    /// The throw reaches its release (`SUB_STATE_CANNING_RELEASE`, frame 3): the primed
    /// grenade leaves her left hand (`SPECIAL_throw_grenade`, `CreateGrenadeFromPerson`).
    ///
    /// The AI primes theirs as they start the throw (`shoot_get_ammo_sound_anim_time`,
    /// Person.cpp:6060), and lobs it half as far at someone within 0x500 they're out to get
    /// (grenade.cpp:118). Their last gone, they get out a pistol if they've one.
    pub(crate) fn release_grenade(&mut self, pi: usize) {
        let player = Some(pi) == self.player;
        let Some(fuse) = (if player { self.primed } else { Some(FUSE) }) else { return };
        let near = self.people[pi].foe.filter(|_| !player && self.people[pi].aggro())
            .is_some_and(|t| Vec2::new(self.people[t].fig.pos.x - self.people[pi].fig.pos.x, self.people[t].fig.pos.z - self.people[pi].fig.pos.z).length() < 0x500 as f32);
        let p = &mut self.people[pi];
        if p.grenades == 0 {
            return;
        }
        let hand = p.fig.part_world(7).map_or(p.fig.pos + Vec3::new(0.0, 120.0, 0.0), |(_, at)| at);
        let f = p.fig.facing();
        let mut vel = Vec3::new(f.x * THROW_ACROSS, THROW_UP, f.y * THROW_ACROSS);
        if near {
            vel /= 2.0;
        }
        p.grenades -= 1;
        if p.grenades == 0 {
            p.weapon = if !player && p.has(Weapon::Pistol) { Weapon::Pistol } else { Weapon::None };
        }
        if player {
            self.primed = None;
        }
        self.grenades.push(Grenade { owner: pi, pos: hand, vel, fuse, pitch: 0.0, warn: 0.0 });
    }

    /// `ProcessGrenades`, for the player in focus: a primed grenade still in hand goes off
    /// there.
    pub(crate) fn run_primed(&mut self, dt: f32, level: &Level) {
        if let (Some(t), Some(pi)) = (self.primed, self.player) {
            let t = t - dt;
            if t > 0.0 {
                self.primed = Some(t);
            } else {
                self.primed = None;
                let p = &mut self.people[pi];
                p.grenades = p.grenades.saturating_sub(1);
                if p.grenades == 0 {
                    p.weapon = Weapon::None;
                }
                // Special.cpp:896: where she stands, `2256` up.
                let at = p.fig.pos + Vec3::new(0.0, 2256.0 / 256.0, 0.0);
                self.grenade_blast(at, pi, level);
            }
        }
    }

    /// `ProcessGrenades`: thrown grenades fall, bounce off the ground (losing half their speed
    /// upwards) and off buildings (losing half across, `under > 0x4000`), settle, and go off
    /// when their fuse runs out.
    pub(crate) fn run_grenades(&mut self, dt: f32, level: &Level) {
        let mut k = 0;
        while k < self.grenades.len() {
            let g = &mut self.grenades[k];
            g.fuse -= dt;
            if g.fuse <= 0.0 {
                let (at, owner) = (g.pos, g.owner);
                self.grenades.remove(k);
                self.grenade_blast(at, owner, level);
                continue;
            }
            g.warn -= dt;
            let warn = g.warn <= 0.0;
            if warn {
                g.warn = 16.0 / 20.0;
            }
            g.vel.y = (g.vel.y - GRAVITY * dt).max(-FASTEST_FALL);
            let old = g.pos;
            g.pos += g.vel * dt;
            g.pitch += 50.0 * dt;
            let floor = level.ground.map_height(&level.map, g.pos.x, g.pos.z) + 11.0;
            if g.pos.y < floor {
                if floor - g.pos.y > 64.0 || g.vel.y > 0.0 {
                    // A building: back out across whichever square edge it crossed.
                    if (old.x / 256.0).floor() != (g.pos.x / 256.0).floor() {
                        g.pos.x = old.x;
                        g.vel.x = -g.vel.x / 2.0;
                    }
                    if (old.z / 256.0).floor() != (g.pos.z / 256.0).floor() {
                        g.pos.z = old.z;
                        g.vel.z = -g.vel.z / 2.0;
                    }
                } else {
                    // Hitting the ground hard enough to hear (`dy` over 0x80000, grenade.cpp:259).
                    if g.vel.y.abs() > 8.0 * 20.0 {
                        let owner = g.owner;
                        self.noises.push((person::Noise::GrenadeHit, old, Some(owner)));
                    }
                    g.vel.y = g.vel.y.abs() / 2.0;
                    g.pos.y = old.y;
                    if g.vel.y <= SETTLE {
                        g.vel = Vec3::ZERO;
                        g.pos.y = floor;
                    }
                }
            }
            let (at, owner) = (g.pos, g.owner);
            if warn {
                self.bang(owner, at);
            }
            k += 1;
        }
    }

    /// `CreateGrenadeExplosion`: just above the ground, a shockwave of 0x300 doing up to 500
    /// that's the thrower's doing.
    fn grenade_blast(&mut self, at: Vec3, owner: usize, level: &Level) {
        let ground = level.ground.map_height(&level.map, at.x, at.z);
        let at = Vec3::new(at.x, at.y.max(ground + 1.0), at.z);
        self.pyro.burst(at, pyro::FIREBOMB | pyro::DUST | pyro::STREAMERS, 300.0);
        self.sounds.push((Sfx::Explosion, at));
        self.shockwave(level, at, 0x300 as f32, 500, Blame::Person(owner));
    }
}
