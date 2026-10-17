//! POW, "yet another explosion system" (fallen/Source/pow.cpp): a bang is a ball of sprites
//! flung out from a point, each playing the sixteen frames of `explode1.tga` and slowing as
//! it goes. The bigger bangs set off smaller ones round them as their timer runs down, and
//! those set off smaller ones again.
//!
//! Only the PlayStation build makes them. Every `POW_create` in the source is under
//! `#ifdef PSX` (barrel.cpp:1891, ob.cpp:1591, vehicle.cpp:2974, Special.cpp:768, 897, 974
//! and 1014) or commented out (eway.cpp:4992, Person.cpp:17566, psystem.cpp:215,
//! Controls.cpp:2151); the PC build makes a `PYRO_FIREBOMB` there instead. The PC game
//! still runs `POW_process` every turn (Game.cpp:2559) and draws what there is
//! (`AENG_draw_pows`, aeng.cpp:4302), but there's never anything. So nothing in a mission
//! sets one off here either: the console's `pow` does, as the PC's own test key did.

use crate::rand;
use glam::{vec3, Vec3};
use uc_formats::units::{radians, TURNS_PER_SECOND};

/// `POW_TICKS_PER_SECOND`: the timers count 2000 a second.
pub const TICKS_PER_SECOND: i32 = 2000;
/// The ticks a turn (`POW_process`, pow.cpp:659, at the game's 20 turns a second).
const TICKS: i32 = TICKS_PER_SECOND / 20;
/// `POW_MAX_FRAME_SPEED`: the counter a frame lasts.
const MAX_FRAME_SPEED: i32 = TICKS_PER_SECOND / 25;
/// A sprite is gone once it's been through all sixteen frames (pow.cpp:738).
pub const FRAMES: u8 = 16;
/// `POW_DELTA_SHIFT`: velocities are kept in 16ths.
const DELTA_SHIFT: i32 = 4;
/// `POW_MAX_SPRITES` and `POW_MAX_POWS` on the PC, less the unused slot 0 of each.
pub const MAX_SPRITES: usize = 255;
pub const MAX_POWS: usize = 31;

/// `POW_TYPE_*`.
pub const BASIC_SPHERE_LARGE: u8 = 1;
pub const BASIC_SPHERE_MEDIUM: u8 = 2;
pub const BASIC_SPHERE_SMALL: u8 = 3;
pub const SPAWN_SPHERE_LARGE: u8 = 4;
pub const SPAWN_SPHERE_MEDIUM: u8 = 5;
pub const MULTISPAWN: u8 = 6;
pub const MULTISPAWN_SEMI: u8 = 7;

/// `POW_create`'s sizes (pow.h): what each makes.
pub const CREATE_LARGE_SEMI: u8 = MULTISPAWN_SEMI;
pub const CREATE_MEDIUM: u8 = SPAWN_SPHERE_MEDIUM;
pub const CREATE_LARGE: u8 = MULTISPAWN;

/// `POW_SPAWN_FLAG_AWAY`: the child moves away from its parent's middle.
const AWAY: u8 = 2;
/// `POW_SPAWN_FLAG_FAR_OFF`: the child starts further out.
const FAR_OFF: u8 = 8;

/// `POW_Type`: whether it's a semi-sphere (else a sphere), its speed, density, frame speed
/// and damping (each 0-3), its life in ticks, and the children it spawns: (when its timer
/// falls below this, what type, `POW_SPAWN_FLAG_*`).
struct Type {
    semi: bool,
    speed: i32,
    density: i32,
    framespeed: i32,
    damp: i32,
    life: i32,
    spawn: [(i32, u8, u8); 3],
}

/// `POW_type` (pow.cpp:99). The three children come at 3/8, 2/8 and 1/8 of a second left.
static TYPES: [Type; 8] = {
    const T: i32 = TICKS_PER_SECOND;
    const NONE: [(i32, u8, u8); 3] = [(0, 0, 0); 3];
    const fn kids(kind: u8, flag: u8) -> [(i32, u8, u8); 3] {
        [((T * 3) >> 3, kind, flag), ((T * 2) >> 3, kind, flag), (T >> 3, kind, flag)]
    }
    const fn t(semi: bool, speed: i32, density: i32, framespeed: i32, damp: i32, life: i32, spawn: [(i32, u8, u8); 3]) -> Type {
        Type { semi, speed, density, framespeed, damp, life, spawn }
    }
    [
        // Unused (`POW_ARRANGE_NOTHING`).
        t(false, 0, 0, 0, 0, 0, NONE),
        t(false, 1, 3, 0, 0, 1, NONE),
        t(false, 2, 1, 1, 0, 1, NONE),
        t(false, 3, 0, 0, 1, 1, NONE),
        // A large bang with three medium ones.
        t(false, 1, 3, 0, 0, (T * 4) >> 3, kids(BASIC_SPHERE_MEDIUM, AWAY | FAR_OFF)),
        // A medium bang with three small ones.
        t(false, 2, 1, 1, 0, (T * 4) >> 3, kids(BASIC_SPHERE_SMALL, AWAY)),
        // A large bang with three medium ones that spawn too.
        t(false, 1, 3, 0, 0, (T * 4) >> 3, kids(SPAWN_SPHERE_MEDIUM, AWAY | FAR_OFF)),
        // The same as a semi-sphere, sitting on the ground.
        t(true, 1, 3, 0, 0, (T * 4) >> 3, kids(SPAWN_SPHERE_MEDIUM, AWAY | FAR_OFF)),
    ]
};

/// `SIN` and `COS` (16.16, 2048 to a turn).
fn sin(a: i32) -> i32 {
    (radians((a & 2047) as f32).sin() * 65536.0) as i32
}

fn cos(a: i32) -> i32 {
    (radians((a & 2047) as f32).cos() * 65536.0) as i32
}

/// `FMATRIX_vector` (fmatrix.cpp:134): the unit vector (16.16) at `yaw` and `pitch`.
fn vector(yaw: i32, pitch: i32) -> [i32; 3] {
    let cp = cos(pitch) as i64;
    [((sin(yaw) as i64 * cp) >> 16) as i32, sin(pitch), ((cos(yaw) as i64 * cp) >> 16) as i32]
}

/// `POW_Sprite`: one frame of fire. Positions are 256ths of a world unit.
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub frame: u8,
    frame_speed: i32,
    damp: i32,
    pos: [i32; 3],
    /// In 16ths of a position step (`POW_DELTA_SHIFT`).
    vel: [i32; 3],
    frame_counter: i32,
}

impl Sprite {
    /// Where it is, in world units.
    pub fn pos(&self) -> Vec3 {
        vec3(self.pos[0] as f32, self.pos[1] as f32, self.pos[2] as f32) / 256.0
    }
}

/// `POW_Pow`: one bang and its sprites.
#[derive(Debug, Clone)]
pub struct Bang {
    pub kind: u8,
    pos: [i32; 3],
    /// Ticks left: its children come as it runs down.
    pub timer: i32,
    /// Which of its children it has spawned (`POW_FLAG_SPAWNED_*`).
    flag: u8,
    pub sprites: Vec<Sprite>,
}

/// Every bang going.
#[derive(Debug, Clone)]
pub struct Pow {
    pub bangs: Vec<Bang>,
    /// Seconds not yet played: it runs in whole turns.
    clock: f32,
    /// Its own chance, like [`crate::Pyro`]'s.
    rng: u32,
    /// The size the console's next `pow` makes (Controls.cpp:2153).
    next_size: u8,
}

impl Default for Pow {
    fn default() -> Pow {
        Pow { bangs: Vec::new(), clock: 0.0, rng: 0x9077, next_size: CREATE_MEDIUM }
    }
}

impl Pow {
    fn random(&mut self) -> i32 {
        rand(&mut self.rng) as i32
    }

    /// How many sprites there are.
    pub fn sprite_count(&self) -> usize {
        self.bangs.iter().map(|b| b.sprites.len()).sum()
    }

    /// Every sprite: where it is (world units) and its frame (0-15).
    pub fn sprites(&self) -> impl Iterator<Item = (Vec3, u8)> + '_ {
        self.bangs.iter().flat_map(|b| b.sprites.iter().map(|s| (s.pos(), s.frame)))
    }

    /// `POW_new` (pow.cpp:459): a bang of `kind` at `pos` (world units), moving at `vel`
    /// (world units a turn), which its sprites take on. Nothing when all the bangs are in use.
    pub fn bang(&mut self, kind: u8, pos: Vec3, vel: Vec3) {
        let fixed = |v: Vec3| [(v.x * 256.0) as i32, (v.y * 256.0) as i32, (v.z * 256.0) as i32];
        self.make(kind, fixed(pos), fixed(vel));
    }

    fn make(&mut self, kind: u8, pos: [i32; 3], d: [i32; 3]) {
        if self.bangs.len() >= MAX_POWS {
            return;
        }
        let t = &TYPES[kind as usize];
        let mut b = Bang { kind, pos, timer: t.life, flag: 0, sprites: Vec::new() };
        // The bang itself never moves (its own `dx` is kept but not used).
        let around = 5 + t.density * 3;
        let upndown = 4 + t.density;
        let framespeed = 96 + t.framespeed * 32;
        let (dpitch, mut pitch) = if t.semi {
            let d = 512 / ((upndown + 1) / 2);
            (d, d / 2)
        } else {
            let d = 1024 / upndown;
            (d, -512 + d / 2)
        };
        let mut free = MAX_SPRITES - self.sprite_count();
        while pitch < 512 {
            // How many in this ring.
            let ring = ((cos(pitch) as i64 * around as i64) >> 16) as i32;
            let dyaw = if ring > 0 { 2048 / ring } else { 0 };
            for i in 0..ring {
                let v = vector(i * dyaw, pitch);
                let mut vel = [0; 3];
                for k in 0..3 {
                    vel[k] = ((v[k] + (self.random() & 0x3fff)) >> (t.speed + 1)) + d[k];
                }
                let frame_speed = framespeed + (self.random() & 0x3f);
                // `POW_insert_sprite`: none once they're all in use.
                if free > 0 {
                    free -= 1;
                    b.sprites.push(Sprite {
                        frame: 0,
                        frame_speed,
                        damp: t.damp + 1,
                        pos,
                        vel: vel.map(|v| v >> DELTA_SHIFT),
                        frame_counter: MAX_FRAME_SPEED,
                    });
                }
            }
            pitch += dpitch;
        }
        self.bangs.push(b);
    }

    /// `POW_create` at `pos` (world units), standing still: `size` is one of `CREATE_*`.
    pub fn create(&mut self, size: u8, pos: Vec3) {
        self.bang(size, pos, Vec3::ZERO);
    }

    /// The console's `pow`: the PC's test key (Controls.cpp:2153, commented out), a bang a
    /// square along `ahead` from `at`, each one the next size round: large, large
    /// semi-sphere, medium.
    pub fn test_key(&mut self, at: Vec3, ahead: Vec3) {
        self.next_size = match self.next_size {
            CREATE_MEDIUM => CREATE_LARGE,
            CREATE_LARGE => CREATE_LARGE_SEMI,
            _ => CREATE_MEDIUM,
        };
        self.create(self.next_size, at + ahead.normalize_or_zero() * 256.0);
    }

    /// Plays `dt` seconds, a turn at a time.
    pub fn update(&mut self, dt: f32) {
        if self.bangs.is_empty() {
            self.clock = 0.0;
            return;
        }
        self.clock += dt;
        while self.clock >= 1.0 / TURNS_PER_SECOND {
            self.clock -= 1.0 / TURNS_PER_SECOND;
            self.turn();
        }
    }

    /// `POW_process` (pow.cpp:628): one turn.
    pub fn turn(&mut self) {
        for b in &mut self.bangs {
            for s in &mut b.sprites {
                // Animation.
                s.frame_counter -= (TICKS * s.frame_speed) >> 8;
                while s.frame_counter <= 0 {
                    s.frame_counter += MAX_FRAME_SPEED;
                    s.frame = s.frame.saturating_add(1);
                }
                // Movement, then damping.
                for k in 0..3 {
                    s.pos[k] += s.vel[k] << DELTA_SHIFT;
                    s.vel[k] -= s.vel[k] >> s.damp;
                }
            }
            b.sprites.retain(|s| s.frame < FRAMES);
            b.timer = (b.timer - TICKS).max(0);
        }
        self.bangs.retain(|b| !b.sprites.is_empty() || b.timer > 0);
        // Bangs spawn their children.
        for i in 0..self.bangs.len() {
            let t = &TYPES[self.bangs[i].kind as usize];
            for j in 0..3 {
                let (when, kind, flag) = t.spawn[j];
                if when == 0 || when <= self.bangs[i].timer || self.bangs[i].flag & 1 << j != 0 {
                    continue;
                }
                self.bangs[i].flag |= 1 << j;
                let mut pos = self.bangs[i].pos;
                let yaw = self.random() & 2047;
                let mut pitch = (self.random() & 255) - 128;
                if t.semi {
                    pitch = pitch.abs() + 128;
                }
                let mut d = vector(yaw, pitch).map(|v| v >> 2);
                if flag & FAR_OFF != 0 {
                    d = d.map(|v| v + (v >> 1));
                }
                for k in 0..3 {
                    pos[k] += d[k];
                }
                d = if flag & AWAY != 0 { d.map(|v| v >> 2) } else { [0; 3] };
                for v in &mut d {
                    *v += (self.random() & 0xfff) - 0x7ff;
                }
                self.make(kind, pos, d);
            }
        }
    }
}
