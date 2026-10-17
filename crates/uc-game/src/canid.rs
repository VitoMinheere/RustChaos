//! Dogs (#110): the original's canids (fallen/Source/canid.cpp), which it never shipped. The
//! model (`data\DOG2.all`) and the barks are in the install, but every load of a dog file is
//! commented out, `ANIMAL`'s allocators `ASSERT(0)`, and `CANID_fn_normal`'s whole body is
//! commented out. This brings them back as an optional extra, off unless asked for.
//!
//! A dog lives round its home square. It prowls four squares one way or another and back,
//! or sleeps a while (`CANID_start_doing_something`). Anyone who comes within 0xd0 of it, or
//! whom it sees in front of it within five times that, it chases; close up it stops and barks,
//! and chases again once they're off (`CANID_process_chase`, `CANID_process_bark`). It walks
//! when its goal is near and runs when it's far (`CANID_Homing`, canid.cpp:127).
//!
//! What's new here:
//! - **Sounds:** the original's dogs make none. Here a dog growls as it starts a chase and
//!   barks while it stands barking (`General\AttackDog.wav`, `Dog.wav`, `Dog2.wav`), and
//!   now and then gives a far-off bark while prowling (`f_DOG1-3.wav`).
//! - **No navigation map:** the original finds its way with `MAV_do`, which this engine
//!   doesn't have. As the Balrog does here, a dog heads straight for its goal and slides
//!   along whatever is in the way. Chasing someone it can still smell or see, it makes for
//!   them, not the middle of their square; and it gives up on a goal it hasn't reached in
//!   ten seconds, so a wall can't hold it for ever.
//! - **The model:** `DOG2.all` says each keyframe has 28 elements, one per mesh, but they
//!   hold 15: the legs, the hips, the body and the head. Read as 28, each pose runs into the
//!   next and the dog comes out in pieces, so it's read as 15 and the other 13 meshes,
//!   which nothing animates, are left out. Its feet are at its origin, so it stands on the
//!   ground rather than the 55 up the original puts it (canid.cpp:180).
//! - **The colour:** the model has no textures. Its faces carry a palette index (`Col2`)
//!   into `tex01.pal`, which didn't ship either, so the dog is drawn in one brown.
//!
//! Like the creatures, a dog runs on the original's 20 ticks a second.

use crate::creature::{Around, TICK};
use crate::rand;
use glam::{Vec2, Vec3};
use std::f32::consts::TAU;
use uc_formats::units::radians;
use uc_world::Figure;

/// The model and its two animations.
pub const CHUNK: &str = "data\\DOG2.all";
/// Anim 1, an 8-frame loop: the walk (`ANIMAL_set_anim(canid, 1)`, canid.cpp:131).
pub const WALK: usize = 1;
/// Anim 2, an 8-frame loop: the run (canid.cpp:128).
pub const RUN: usize = 2;

/// The dog's brown, for the faces whose palette didn't ship.
pub const COAT: [u8; 3] = [150, 105, 62];

/// The sounds (`sound_list`).
pub mod sound {
    /// `f_DOG1.wav` to `f_DOG3.wav`: far-off barks.
    pub const FAR: usize = 91;
    pub const ATTACK: usize = 273;
    pub const ANGRY: usize = 274;
    /// `General\Dog.wav`, and `Dog2.wav` after it.
    pub const BARK: usize = 275;
}

/// `CANID_SENSE_RADIUS`: anyone this close is noticed, whichever way the dog faces.
pub const SENSE: f32 = 0xd0 as f32;
/// How far it looks (`CANID_LOS`): five times that, in front of it.
pub const SIGHT: f32 = SENSE * 5.0;
/// Squared distances, as the original keeps them: run beyond this (canid.cpp:127)...
const RUN_BEYOND: f32 = 48000.0;
/// ...stop and bark within this (canid.cpp:525)...
const BARK_WITHIN: f32 = 4096.0;
/// ...and chase again beyond this (canid.cpp:474).
const CHASE_BEYOND: f32 = 14000.0;
/// How high its origin is above the ground: none (see the module's notes on canid.cpp:180).
const HEIGHT: f32 = 0.0;
/// Not in the original: ticks to give up on a goal not reached (ten seconds).
const GIVE_UP: u32 = 200;
/// Ticks between barks.
const BARK_EVERY: u32 = 16;

/// `CANID_SUBSTATE_*`. Fleeing was never written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Sleep,
    Prowl,
    Chase,
    Bark,
}

/// What a dog does to the world in a tick.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Sound { id: usize, at: Vec3 },
}

#[derive(Clone)]
pub struct Canid {
    pub fig: Figure,
    pub state: State,
    /// Where it lives, in map squares (`home_x`, `home_z`).
    pub home: (i32, i32),
    /// Where it's going, in map squares (`dest_x`, `dest_z`).
    pub dest: (i32, i32),
    /// Who it's after (`animal->target`).
    pub target: Option<usize>,
    /// Ticks left asleep, or since it set off for `dest` (`counter`).
    counter: u32,
    /// 2048ths of a turn, the way it faces (`dt->Angle`).
    yaw: f32,
    /// Units a tick (`animal->extra`: 4 runs, 6 walks, as a shift of `SIN`).
    pub speed: f32,
}

/// `(SIN(a), COS(a))` as unit numbers, `a` in 2048ths of a turn.
fn sin_cos(a: f32) -> Vec2 {
    let r = radians(a);
    Vec2::new(r.sin(), r.cos())
}

/// The angle (0..2048) whose `(SIN, COS)` points along `d`.
fn calc_angle(d: Vec2) -> f32 {
    d.x.atan2(d.y).rem_euclid(TAU) * 2048.0 / TAU
}

fn wrap(a: f32) -> f32 {
    (a + 1024.0).rem_euclid(2048.0) - 1024.0
}

fn square(p: Vec3) -> (i32, i32) {
    ((p.x as i32) >> 8, (p.z as i32) >> 8)
}

/// The middle of map square `s`, which the original heads for (`(dest_x << 16) + (128 << 8)`).
fn centre(s: (i32, i32)) -> Vec2 {
    Vec2::new(((s.0 << 8) + 128) as f32, ((s.1 << 8) + 128) as f32)
}

impl Canid {
    /// A dog at `pos` facing `angle` (radians), with `fig` its model. `ground` is the height
    /// under it. `CANID_fn_init`: its home is where it starts, and it decides what to do.
    pub fn new(mut fig: Figure, pos: Vec3, angle: f32, ground: f32, rng: &mut u32) -> Canid {
        fig.pos = Vec3::new(pos.x, ground + HEIGHT, pos.z);
        fig.plain = Some(COAT);
        let home = square(pos);
        let mut c = Canid { fig, state: State::Sleep, home, dest: home, target: None, counter: 0, yaw: angle * 2048.0 / TAU, speed: 0.0 };
        c.sync();
        c.start_doing_something(rng);
        c
    }

    fn sync(&mut self) {
        self.fig.angle = radians(self.yaw);
    }

    /// The animation playing: [`WALK`] or [`RUN`].
    pub fn anim(&self) -> usize {
        self.fig.anim()
    }

    /// `ANIMAL_set_anim`.
    fn set_anim(&mut self, id: usize) {
        if self.fig.anim() != id {
            self.fig.restart_now(id, uc_world::Play::Loop);
        }
    }

    /// `ANIMAL_animate`: the tween goes `TweenStep * 2` a tick at 20 a second, as a
    /// creature's does; the figure counts at 15.
    fn animate(&mut self) {
        self.fig.advance_frames(TICK * 20.0 / 15.0);
    }

    /// `CANID_start_doing_something`: three times in four it prowls, else it sleeps.
    fn start_doing_something(&mut self, rng: &mut u32) {
        if rand(rng) & 3 == 3 {
            self.init_sleep(rng);
        } else {
            self.init_prowl(rng);
        }
    }

    /// `CANID_init_sleep`: for 16 to 271 ticks.
    fn init_sleep(&mut self, rng: &mut u32) {
        self.nap((rand(rng) & 0xff) + 0x10);
    }

    /// `CANID_init_prowl`: to four squares from home, one of the four ways.
    fn init_prowl(&mut self, rng: &mut u32) {
        let (i, j) = match rand(rng) & 3 {
            0 => (0, 4),
            1 => (4, 0),
            2 => (0, -4),
            _ => (-4, 0),
        };
        self.prowl_to((self.home.0 + i, self.home.1 + j));
    }

    /// Sets it prowling to square `dest`, as `CANID_init_prowl` does to one it picks.
    pub fn prowl_to(&mut self, dest: (i32, i32)) {
        self.state = State::Prowl;
        self.dest = dest;
        self.counter = 0;
    }

    /// Sends it to sleep for `ticks`, as `CANID_init_sleep` does for a random while.
    pub fn nap(&mut self, ticks: u32) {
        self.state = State::Sleep;
        self.counter = ticks;
    }

    /// `CANID_init_chase`: after `who`, to the square they're on.
    fn init_chase(&mut self, who: usize, w: &Around, rng: &mut u32, acts: &mut Vec<Act>) {
        if self.state != State::Chase && self.target != Some(who) {
            let id = if rand(rng) & 1 == 0 { sound::ATTACK } else { sound::ANGRY };
            acts.push(Act::Sound { id, at: self.fig.pos });
        }
        if self.state != State::Chase {
            self.counter = 0;
        }
        self.state = State::Chase;
        self.dest = square(w.people[who].pos);
        self.target = Some(who);
    }

    /// `CANID_init_bark`.
    fn init_bark(&mut self) {
        self.state = State::Bark;
        self.counter = 0;
    }

    /// Who is about that a dog could go for: on their feet, and nearest first (the
    /// original takes them in `THING_find_sphere`'s order).
    fn about(&self, w: &Around, radius: f32) -> Option<usize> {
        let me = self.fig.pos;
        w.people.iter().enumerate()
            .filter(|(_, p)| p.alive && (p.pos - me).length() < radius)
            .min_by(|a, b| (a.1.pos - me).length().total_cmp(&(b.1.pos - me).length()))
            .map(|(i, _)| i)
    }

    /// `CANID_6sense`: anyone within 0xd0 is chased. Whether there was someone.
    fn sense(&mut self, w: &Around, rng: &mut u32, acts: &mut Vec<Act>) -> bool {
        match self.about(w, SENSE) {
            Some(who) => {
                self.init_chase(who, w, rng, acts);
                true
            }
            None => false,
        }
    }

    /// `CANID_can_see`: within an eighth of a turn either side of straight ahead, and
    /// nothing in the way.
    pub fn can_see(&self, at: Vec3, w: &Around) -> bool {
        let d = at - self.fig.pos;
        let off = wrap(calc_angle(-Vec2::new(d.x, d.z)) - self.yaw);
        off.abs() < 256.0 && (w.clear)(self.fig.pos + Vec3::Y * 16.0, at + Vec3::Y * 64.0)
    }

    /// `CANID_LOS`: the nearest person within five times 0xd0 is chased if it can see them.
    /// Whether it saw someone.
    fn look(&mut self, w: &Around, rng: &mut u32, acts: &mut Vec<Act>) -> bool {
        match self.about(w, SIGHT) {
            Some(who) if self.can_see(w.people[who].pos, w) => {
                self.init_chase(who, w, rng, acts);
                true
            }
            _ => false,
        }
    }

    /// One tick (`CANID_fn_normal`).
    pub fn tick(&mut self, w: &Around, rng: &mut u32, acts: &mut Vec<Act>) {
        match self.state {
            // `CANID_process_sleep`.
            State::Sleep => {
                if self.sense(w, rng, acts) {
                    return;
                }
                if self.counter == 0 {
                    self.start_doing_something(rng);
                } else {
                    self.counter -= 1;
                }
            }
            // `CANID_process_prowl`.
            State::Prowl => {
                if !self.sense(w, rng, acts) {
                    self.look(w, rng, acts);
                }
                if self.state != State::Prowl {
                    return;
                }
                self.counter += 1;
                if square(self.fig.pos) == self.dest || self.counter > GIVE_UP {
                    self.start_doing_something(rng);
                    return;
                }
                if rand(rng) & 0xff == 0 {
                    acts.push(Act::Sound { id: sound::FAR + (rand(rng) % 3) as usize, at: self.fig.pos });
                }
                self.homing(centre(self.dest), w);
            }
            // `CANID_process_chase`.
            State::Chase => {
                let found = self.sense(w, rng, acts) || self.look(w, rng, acts);
                let Some(t) = self.target.and_then(|t| w.people.get(t)) else {
                    self.init_prowl(rng);
                    return;
                };
                let d = t.pos - self.fig.pos;
                if d.x * d.x + d.z * d.z < BARK_WITHIN {
                    self.init_bark();
                    return;
                }
                // Not in the original: with them in sight it makes straight for them, as
                // there's no route to follow; lost, it goes to where it last had them.
                if found {
                    self.counter = 0;
                    self.homing(Vec2::new(t.pos.x, t.pos.z), w);
                    return;
                }
                self.counter += 1;
                self.homing(centre(self.dest), w);
                if square(self.fig.pos) == self.dest || self.counter > GIVE_UP {
                    self.init_prowl(rng);
                }
            }
            // `CANID_process_bark`.
            State::Bark => {
                let Some(t) = self.target.and_then(|t| w.people.get(t)) else {
                    self.init_prowl(rng);
                    return;
                };
                if !t.alive {
                    self.target = None;
                    self.start_doing_something(rng);
                    return;
                }
                let d = t.pos - self.fig.pos;
                if d.x * d.x + d.z * d.z > CHASE_BEYOND {
                    let who = self.target.unwrap();
                    self.init_chase(who, w, rng, acts);
                    return;
                }
                if self.counter == 0 {
                    let id = sound::BARK + (rand(rng) & 1) as usize;
                    acts.push(Act::Sound { id, at: self.fig.pos });
                    self.counter = BARK_EVERY + (rand(rng) & 15);
                } else {
                    self.counter -= 1;
                }
            }
        }
    }

    /// `CANID_Homing`: towards `to`, running if it's far and walking if it's near, turning an
    /// eighth of the way a tick.
    fn homing(&mut self, to: Vec2, w: &Around) {
        let at = Vec2::new(self.fig.pos.x, self.fig.pos.z);
        let d = to - at;
        if d.length_squared() > RUN_BEYOND {
            self.set_anim(RUN);
            self.speed = 256.0 / 16.0;
        } else {
            self.set_anim(WALK);
            self.speed = 256.0 / 64.0;
        }
        let want = calc_angle(-d);
        self.yaw = (self.yaw + wrap(want - self.yaw) / 8.0).rem_euclid(2048.0);
        self.sync();
        let step = -sin_cos(self.yaw) * self.speed;
        self.animate();
        let ground = (w.terrain)(at.x, at.y);
        let to = (w.slide)(at, ground, step);
        let ground = (w.terrain)(to.x, to.y);
        self.fig.pos = Vec3::new(to.x, ground + HEIGHT, to.y);
    }
}
