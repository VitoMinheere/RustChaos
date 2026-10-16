//! Creatures: what a `create_creature` point makes (`EWAY_create_animal`, fallen/Source/eway.cpp),
//! run the way `BAT_normal` and `BAT_change_state` run them (fallen/Source/bat.cpp).
//!
//! The original files all of them under "bats". There are four kinds, but the bat itself is
//! compiled out of the shipped game (`#ifdef UNUSED` round its `BAT_create`), so a point that
//! asks for one makes nothing at all. Psycho Park and Stern Revenge ask for nothing else.
//! What's left:
//! - the **gargoyle** sits until the player comes near, flies up, then circles its home,
//!   swooping on the player and spitting fireballs;
//! - the **Balrog** stomps about on the ground on fire, picks on whoever is nearest (the player
//!   above all), and swipes, stomps and throws fireballs. Fists don't touch it: only bullets and
//!   blasts do (`BAT_apply_hit`'s callers), and it takes an eighth of what a gargoyle does;
//! - **Bane** hovers, draws the lifeless bodies round him up into the air, and every so often
//!   lets go a blast and a bolt at the player. Nothing hurts him.
//!
//! Each of them is an anim prim (`anim002.all` to `anim004.all`): meshes and keyframes in one
//! file, driven like a person's. They run on the original's 20 ticks a second, with its
//! speeds in units a tick and its timers in 160ths of a second (`BAT_TICKS_PER_SECOND`).
//!
//! The original finds its way about with the navigation map (`MAV_do`), which this engine
//! doesn't have. Where it asks it, the Balrog here heads straight for where it's going and
//! slides along whatever is in the way, and it only picks on people it could walk to: those on
//! the ground, not up on a roof.

use uc_formats::units::radians;
use crate::rand;
use glam::{vec3, Vec2, Vec3};
use std::f32::consts::TAU;
use uc_world::Figure;

pub const BAT: u8 = 1;
pub const GARGOYLE: u8 = 2;
pub const BALROG: u8 = 3;
pub const BANE: u8 = 4;

/// The original's game tick, and how much a creature's timer drops in one.
pub const TICK: f32 = 1.0 / 20.0;
const SECOND: f32 = 160.0;
const TIMER_TICK: f32 = SECOND / 20.0;

/// `BAT_ANIM_*`.
mod anim {
    pub const GARGOYLE_WAIT: usize = 1;
    pub const GARGOYLE_FLY: usize = 2;
    pub const GARGOYLE_FLY_UP: usize = 3;
    pub const GARGOYLE_ATTACK: usize = 4;
    pub const GARGOYLE_TAKE_HIT: usize = 14;
    pub const GARGOYLE_WAKE_UP: usize = 15;
    pub const GARGOYLE_START_FALL: usize = 16;
    pub const GARGOYLE_FALL_LOOP: usize = 17;
    pub const GARGOYLE_HIT_GROUND: usize = 18;
    pub const BALROG_YOMP: usize = 2;
    pub const BALROG_IDLE: usize = 3;
    pub const BALROG_SWIPE: usize = 4;
    pub const BALROG_YOMP_START: usize = 5;
    pub const BALROG_YOMP_END: usize = 6;
    pub const BALROG_TURN: usize = 8;
    pub const BALROG_ROAR: usize = 9;
    pub const BALROG_STOMP: usize = 10;
    pub const BALROG_DIE: usize = 12;
    pub const BANE_IDLE: usize = 2;
    pub const BANE_ATTACK: usize = 3;
}

/// The game's sounds by id (`sound_list`): `S_BALROG_GROWL_START` and the rest.
pub mod sound {
    pub const BALROG_GROWL: usize = 364;
    pub const BALROG_STEP: usize = 367;
    pub const BALROG_DEATH: usize = 369;
    pub const BALROG_ROAR: usize = 370;
    pub const BALROG_FIREBALL: usize = 371;
    pub const RECKONING_LOOP: usize = 506;
}

/// `BAT_STATE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Goto,
    Circle,
    Attack,
    Dying,
    Dead,
    Ground,
    Recoil,
    Wander,
    Roar,
    Follow,
    Charge,
    Swipe,
    Stomp,
    Fireball,
    BalrogIdle,
    BaneIdle,
    BaneAttack,
    BaneStart,
}

/// `BAT_SUBSTATE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sub {
    None,
    CircleHome,
    CircleTarget,
    GroundWait,
    WakeUp,
    FlyUp,
    DeadInitial,
    DeadLoop,
    DeadFinal,
    YompStart,
    YompMiddle,
    YompEnd,
    FireballTurn,
    FireballFire,
}

/// What a creature needs to know about someone.
#[derive(Debug, Clone, Copy)]
pub struct Seen {
    pub pos: Vec3,
    /// The top of the head (`SUB_OBJECT_HEAD`).
    pub head: Vec3,
    /// Not dead, not knocked out, still in the game.
    pub alive: bool,
    /// Dead for good (`is_person_dead`).
    pub dead: bool,
    pub player: bool,
    /// `PCOM_AI_*`: Bane summons those with none (or who are bent on killing themselves).
    pub ai: u8,
    /// Standing on a roof rather than the ground.
    pub up_high: bool,
}

/// What a creature does to the world in a tick, for the game to carry out.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    /// `create_shockwave`, set off by creature `from` (which it doesn't hurt).
    Shockwave { at: Vec3, radius: f32, damage: i32, from: usize },
    Fireball { at: Vec3, vel: Vec3 },
    /// Damage straight off someone's health (a gargoyle's swoop, Bane's bolt), maybe making
    /// them reel from `from`.
    Hurt { person: usize, damage: i32, recoil: bool, from: Vec3 },
    Sound { id: usize, at: Vec3 },
    /// Bane draws a body up into the air (`set_person_float_up`).
    Float { person: usize },
    /// A bolt of lightning from the creature to someone: to their head, or else their pelvis.
    Spark { person: usize, head: bool },
    Dust { at: Vec3 },
    /// The person the Balrog was after has died: counter 8 goes up, as the original assumes it
    /// was him (`EWAY_counter[8]`).
    BalrogKill,
}

/// The surroundings: who is about, and how high the ground is.
pub struct Around<'a> {
    pub people: &'a [Seen],
    pub player: Option<usize>,
    /// The ground under (x, z), roofs included (`PAP_calc_map_height_at`).
    pub floor: &'a dyn Fn(f32, f32) -> f32,
    /// The ground under (x, z), roofs not included (`PAP_calc_height_at`).
    pub terrain: &'a dyn Fn(f32, f32) -> f32,
    /// Whether anything stands between two points.
    pub clear: &'a dyn Fn(Vec3, Vec3) -> bool,
    /// Where something at `from` trying to move by `by` ends up, walls in the way.
    pub slide: &'a dyn Fn(Vec2, f32, Vec2) -> Vec2,
    /// A fly-by is holding the player still: no swooping on them then.
    pub frozen: bool,
}

/// `calc_angle(dx, dz)`: the angle (0..2048) whose `(SIN, COS)` points along `d`.
fn calc_angle(d: Vec2) -> f32 {
    d.x.atan2(d.y).rem_euclid(TAU) * 2048.0 / TAU
}

/// An angle difference wrapped into -1024..1024.
fn wrap(a: f32) -> f32 {
    (a + 1024.0).rem_euclid(2048.0) - 1024.0
}

/// `(SIN(a), COS(a))` as unit numbers, `a` in 2048ths of a turn.
fn sin_cos(a: f32) -> Vec2 {
    let r = radians(a);
    Vec2::new(r.sin(), r.cos())
}

/// `QDIST2`: the larger of the two plus half the smaller.
pub(crate) fn qdist2(d: Vec2) -> f32 {
    let d = d.abs();
    d.max_element() + d.min_element() / 2.0
}

/// `QDIST3`: the largest plus a quarter of the other two.
pub(crate) fn qdist3(d: Vec3) -> f32 {
    let d = d.abs();
    let m = d.max_element();
    m + (d.x + d.y + d.z - m) / 4.0
}

#[derive(Clone)]
pub struct Creature {
    /// The event point that made it.
    pub way: usize,
    /// [`GARGOYLE`], [`BALROG`] or [`BANE`].
    pub kind: u8,
    pub fig: Figure,
    pub state: State,
    sub: Sub,
    pub health: i32,
    /// Where it was made, in map squares (`home_x`, `home_z`).
    home: (i32, i32),
    want: Vec3,
    /// Units a tick.
    vel: Vec3,
    /// 2048ths of a turn, as the original keeps it.
    yaw: f32,
    /// 160ths of a second.
    timer: f32,
    /// Who it is after.
    pub target: Option<usize>,
    /// Its blow has landed this time round (`BAT_FLAG_ATTACKED`).
    attacked: bool,
    /// Which footfall of the stride has sounded (`BAT_FLAG_SYNC_FX`, `_FX2`).
    steps: [bool; 2],
    /// Keyframes passed since the animation was set (`FrameIndex`).
    frame: u32,
    anim: usize,
    /// `STATE_DEAD`, which a `killed` trigger watches.
    pub dead: bool,
    /// The bodies Bane is drawing up (`BAT_summon`).
    pub summon: [Option<usize>; 4],
    /// Ticks to the next bolt to the summoned (`last` in `BAT_process_bane_sparks`).
    sparks: f32,
}

impl Creature {
    /// `BAT_create`. `fig` is the creature's anim prim; `floor` is the height under it.
    pub fn new(way: usize, kind: u8, mut fig: Figure, pos: Vec3, angle: f32, floor: f32) -> Creature {
        let yaw = angle * 2048.0 / TAU;
        fig.pos = Vec3::new(pos.x, floor, pos.z);
        let mut c = Creature {
            way, kind, fig, state: State::Ground, sub: Sub::None, health: 100,
            home: ((pos.x as i32) >> 8, (pos.z as i32) >> 8), want: Vec3::ZERO, vel: Vec3::ZERO, yaw,
            timer: 0.0, target: None, attacked: false, steps: [false; 2], frame: 0, anim: 1, dead: false,
            summon: [None; 4], sparks: 0.0,
        };
        match kind {
            GARGOYLE => {
                c.sub = Sub::GroundWait;
                c.anim = anim::GARGOYLE_WAIT;
            }
            BALROG => {
                c.state = State::Roar;
                c.health = 255;
                c.set_anim(anim::BALROG_ROAR);
            }
            _ => {
                c.state = State::BaneIdle;
                c.fig.pos.y += 0x60 as f32;
                c.set_anim(anim::BANE_IDLE);
            }
        }
        c.sync();
        c
    }

    /// `BAT_set_anim`: straight to the first frame, no blend.
    fn set_anim(&mut self, id: usize) {
        if self.anim == id {
            return;
        }
        self.anim = id;
        self.frame = 0;
        let play = if self.fig.loops_round(id) { uc_world::Play::Loop } else { uc_world::Play::Once };
        self.fig.restart_now(id, play);
        self.steps = [false; 2];
    }

    /// `BAT_animate`: one tick of the animation; whether it's over.
    fn animate(&mut self) -> bool {
        // The original's tween goes `TweenStep * 2` a tick at 20 a second; the figure counts at 15.
        let (passed, ended) = self.fig.advance_frames(TICK * 20.0 / 15.0);
        self.frame += passed;
        ended
    }

    fn sync(&mut self) {
        self.fig.angle = radians(self.yaw);
    }

    fn pos(&self) -> Vec3 {
        self.fig.pos
    }

    fn rand(rng: &mut u32) -> u32 {
        rand(rng)
    }

    /// `BAT_turn_to_place`: a sixteenth of the way round a tick, at most 20. The Balrog is
    /// modelled back to front, so he turns to face the other way. Returns how far off it was.
    fn turn_to(&mut self, at: Vec3) -> f32 {
        let d = Vec2::new(at.x - self.pos().x, at.z - self.pos().z);
        let mut want = calc_angle(d);
        if self.kind == BALROG {
            want += 1024.0;
        }
        let off = wrap(want - self.yaw);
        self.yaw = (self.yaw + (off / 16.0).clamp(-20.0, 20.0)).rem_euclid(2048.0);
        off
    }

    fn turn_to_target(&mut self, w: &Around) -> f32 {
        match self.target.and_then(|t| w.people.get(t)) {
            Some(t) => self.turn_to(t.pos),
            None => 0.0,
        }
    }

    /// `BAT_emit_fireball`: from a little above it, at the target's middle, with a lob.
    fn fireball(&self, w: &Around, rng: &mut u32, acts: &mut Vec<Act>) {
        let Some(t) = self.target.and_then(|t| w.people.get(t)) else { return };
        let d = t.pos + vec3(0.0, 0x30 as f32, 0.0) - self.pos();
        let dist = qdist3(d) + 1.0;
        let mut vel = d / dist * 0x40 as f32;
        vel.y += (Self::rand(rng) & 0x3ff) as f32 / 256.0;
        acts.push(Act::Fireball { at: self.pos() + vec3(0.0, 0x50 as f32, 0.0), vel });
        acts.push(Act::Sound { id: sound::BALROG_FIREBALL, at: self.pos() });
    }

    fn growl(&self, rng: &mut u32, acts: &mut Vec<Act>) {
        acts.push(Act::Sound { id: sound::BALROG_GROWL + (Self::rand(rng) % 3) as usize, at: self.pos() });
    }

    /// `BAT_change_state`: what to do next.
    fn change_state(&mut self, w: &Around, rng: &mut u32, acts: &mut Vec<Act>) {
        let old = self.target;
        let mut dangle = 0.0;
        let new = match self.kind {
            BALROG => {
                if self.target.and_then(|t| w.people.get(t)).is_some_and(|t| t.dead) {
                    acts.push(Act::BalrogKill);
                }
                // The best person to bully: lower scores are better, height counts double,
                // and he likes to keep to the same one and to go for the player.
                let me = self.pos();
                let mut best: Option<(usize, f32, f32)> = None;
                for (i, p) in w.people.iter().enumerate() {
                    if !p.alive || p.up_high || (p.pos - me).length() >= 0xa00 as f32 {
                        continue;
                    }
                    let d = (p.pos - me).abs();
                    let dist = d.x + d.y + d.y + d.z;
                    let mut score = dist;
                    if self.target == Some(i) {
                        score /= 2.0;
                    }
                    if p.player {
                        score /= 2.0;
                    }
                    if best.is_none_or(|b| b.1 > score) {
                        best = Some((i, score, dist));
                    }
                }
                self.target = best.map(|b| b.0);
                let r = |rng: &mut u32| Self::rand(rng);
                match best {
                    Some((_, _, dist)) => {
                        dangle = self.turn_to_target(w).abs();
                        if self.target != old {
                            State::Roar
                        } else if dist < 0x200 as f32 && dangle < 256.0 {
                            State::Swipe
                        } else if dist < 0x400 as f32 {
                            if r(rng) & 0x40 != 0 { State::Stomp } else { State::Follow }
                        } else if r(rng) & 0x20 != 0 {
                            if r(rng) & 3 == 0 { State::Charge } else { State::Fireball }
                        } else {
                            State::Follow
                        }
                    }
                    None if matches!(self.state, State::Fireball | State::Charge | State::Swipe | State::Stomp) => State::Roar,
                    None if r(rng) & 3 == 0 => {
                        if r(rng) & 2 != 0 { State::Roar } else { State::BalrogIdle }
                    }
                    None => State::Wander,
                }
            }
            BANE => {
                self.target = None;
                if self.summon[0].is_none() {
                    State::BaneStart
                } else if Self::rand(rng) & 3 == 0 {
                    State::BaneAttack
                } else {
                    State::BaneIdle
                }
            }
            _ => {
                let me = self.pos();
                self.target = w.player.filter(|&p| (w.people[p].pos - me).length() < 0x600 as f32);
                let off = Vec2::new(me.x - ((self.home.0 << 8) + 0x80) as f32, me.z - ((self.home.1 << 8) + 0x80) as f32);
                if off.x.abs() > 0x400 as f32 || off.y.abs() > 0x400 as f32 {
                    State::Goto
                } else {
                    match Self::rand(rng) % 3 {
                        0 => State::Idle,
                        1 => State::Goto,
                        _ if self.target.is_some() => State::Attack,
                        _ => State::Circle,
                    }
                }
            }
        };
        self.state = new;
        self.attacked = false;
        let r = |rng: &mut u32| Self::rand(rng);
        match new {
            State::Idle => {
                self.timer = SECOND * (1 + (r(rng) & 1)) as f32;
                self.set_anim(anim::GARGOYLE_FLY);
            }
            State::Goto => {
                let x = ((self.home.0 << 8) + (r(rng) & 0x3ff) as i32 - 0x1ff) as f32;
                let z = ((self.home.1 << 8) + (r(rng) & 0x3ff) as i32 - 0x1ff) as f32;
                let y = (w.floor)(x, z) + 0x80 as f32 + (r(rng) & 0x7f) as f32;
                self.want = vec3(x, y, z);
                self.timer = SECOND * (2 + (r(rng) & 1)) as f32;
                self.set_anim(anim::GARGOYLE_FLY);
            }
            State::Circle => {
                self.sub = if self.target.is_some() { Sub::CircleTarget } else { Sub::CircleHome };
                self.timer = SECOND * (3 + (r(rng) & 3)) as f32;
                self.set_anim(anim::GARGOYLE_FLY);
            }
            State::Attack => self.set_anim(anim::GARGOYLE_ATTACK),
            State::Wander => {
                // `WAND_get_next_place` and the navigation map choose somewhere to walk to; here
                // it's somewhere within four squares.
                let me = self.pos();
                let dx = (r(rng) & 0x7ff) as f32 - 0x400 as f32;
                let dz = (r(rng) & 0x7ff) as f32 - 0x400 as f32;
                self.want = vec3(me.x + dx, me.y, me.z + dz);
                self.timer = SECOND * (2 + (r(rng) & 1)) as f32;
                self.set_anim(anim::BALROG_YOMP_START);
                self.sub = Sub::YompStart;
            }
            State::Roar => {
                self.set_anim(anim::BALROG_ROAR);
                acts.push(Act::Sound { id: sound::BALROG_ROAR, at: self.pos() });
                self.vel.x = 0.0;
                self.vel.z = 0.0;
            }
            State::Charge | State::Follow => {
                self.set_anim(anim::BALROG_YOMP_START);
                self.timer = SECOND * (2 + (r(rng) & 1)) as f32;
                self.sub = Sub::YompStart;
            }
            State::Swipe | State::Stomp => {
                self.set_anim(if new == State::Swipe { anim::BALROG_SWIPE } else { anim::BALROG_STOMP });
                self.growl(rng, acts);
                self.vel.x = 0.0;
                self.vel.z = 0.0;
            }
            State::Fireball => {
                if dangle < 256.0 {
                    self.set_anim(anim::BALROG_SWIPE);
                    self.sub = Sub::FireballFire;
                    self.growl(rng, acts);
                } else {
                    self.set_anim(anim::BALROG_TURN);
                    self.sub = Sub::FireballTurn;
                }
                self.vel.x = 0.0;
                self.vel.z = 0.0;
            }
            State::BalrogIdle => {
                self.set_anim(anim::BALROG_IDLE);
                self.timer = SECOND * (2 + (r(rng) & 3)) as f32;
                self.vel.x = 0.0;
                self.vel.z = 0.0;
            }
            State::BaneIdle => {
                self.set_anim(anim::BANE_IDLE);
                self.timer = SECOND * (3 + (r(rng) & 3)) as f32;
                self.vel.x = 0.0;
                self.vel.z = 0.0;
            }
            State::BaneAttack => {
                self.set_anim(anim::BANE_ATTACK);
                self.vel.x = 0.0;
                self.vel.z = 0.0;
            }
            State::BaneStart => {
                // The bodies round him, four at most: people with nothing to do.
                let me = self.pos();
                self.summon = [None; 4];
                let mut found = w.people.iter().enumerate()
                    .filter(|(_, p)| !p.player && matches!(p.ai, 0 | 15) && (p.pos - me).length() < 0x800 as f32)
                    .map(|(i, _)| i);
                for s in &mut self.summon {
                    *s = found.next();
                    if let Some(person) = *s {
                        acts.push(Act::Float { person });
                    }
                }
                self.set_anim(anim::BANE_IDLE);
                acts.push(Act::Sound { id: sound::RECKONING_LOOP, at: me });
            }
            _ => {}
        }
    }

    /// The Balrog walking: turn towards where he's going, and only get going once he faces
    /// it. His stride starts, goes on until his timer runs out, then ends.
    fn yomp(&mut self, to: Vec3, damp: f32) -> bool {
        let off = self.turn_to(to);
        if off.abs() > 256.0 || self.sub == Sub::YompEnd {
            self.vel.x -= self.vel.x * damp;
            self.vel.z -= self.vel.z * damp;
        } else {
            // `-SIN(angle) >> 4`, half as much again, less a thirty-second: 23 units a tick.
            let f = -sin_cos(self.yaw) * (4096.0 * 1.5 * (31.0 / 32.0) / 256.0);
            self.vel.x += (f.x - self.vel.x) / 8.0;
            self.vel.z += (f.y - self.vel.z) / 8.0;
        }
        let mut end = self.animate();
        if end {
            match self.sub {
                Sub::YompStart => {
                    end = false;
                    self.set_anim(anim::BALROG_YOMP);
                    self.sub = Sub::YompMiddle;
                }
                Sub::YompMiddle => {
                    end = false;
                    if self.timer == 0.0 {
                        self.set_anim(anim::BALROG_YOMP_END);
                        self.sub = Sub::YompEnd;
                    }
                }
                _ => {}
            }
        }
        end
    }

    /// `BAT_normal`: one tick.
    pub fn tick(&mut self, me: usize, w: &Around, rng: &mut u32, acts: &mut Vec<Act>) {
        let end = match self.state {
            State::Dead => return,
            State::Idle => {
                self.vel -= self.vel / 16.0;
                if self.target.is_some() {
                    self.turn_to_target(w);
                }
                self.animate()
            }
            State::Goto => {
                let d = (self.want - self.pos()) * 32.0 / 256.0;
                let want = vec3(d.x.clamp(-32.0, 32.0), d.y.clamp(-8.0, 8.0), d.z.clamp(-32.0, 32.0));
                self.vel += (want - self.vel) / 16.0;
                self.vel -= self.vel / 16.0;
                if self.target.is_some() {
                    self.turn_to_target(w);
                }
                self.animate()
            }
            State::Circle => self.circle(w, acts),
            State::Attack => {
                if self.target.is_some() && !self.attacked && self.frame > 6 {
                    self.fireball(w, rng, acts);
                    self.attacked = true;
                }
                self.turn_to_target(w);
                let end = self.animate();
                self.timer = 0.0;
                end
            }
            State::Dying => {
                match self.sub {
                    Sub::DeadInitial => {
                        if self.animate() {
                            self.sub = Sub::DeadLoop;
                            self.set_anim(anim::GARGOYLE_FALL_LOOP);
                        }
                    }
                    Sub::DeadLoop => {
                        self.animate();
                        // Out of the sky.
                        self.vel.y -= 0x180 as f32 / 256.0;
                        self.vel.x -= self.vel.x / 32.0;
                        self.vel.z -= self.vel.z / 32.0;
                        let p = self.pos();
                        if p.y <= (w.floor)(p.x, p.z) + 0x40 as f32 {
                            self.vel = Vec3::ZERO;
                            self.sub = Sub::DeadFinal;
                            self.set_anim(anim::GARGOYLE_HIT_GROUND);
                        }
                    }
                    _ => {
                        if self.animate() {
                            self.state = State::Dead;
                            self.dead = true;
                        }
                    }
                }
                false
            }
            State::Ground => {
                match self.sub {
                    Sub::GroundWait => {
                        // Wakes when the player comes within ten squares.
                        let near = w.player.is_some_and(|p| {
                            let d = w.people[p].pos - self.pos();
                            qdist2(Vec2::new(d.x, d.z)) < 0xa00 as f32
                        });
                        if near {
                            self.set_anim(anim::GARGOYLE_WAKE_UP);
                            self.sub = Sub::WakeUp;
                        }
                    }
                    Sub::WakeUp => {
                        if self.animate() {
                            self.set_anim(anim::GARGOYLE_FLY_UP);
                            self.sub = Sub::FlyUp;
                        }
                    }
                    _ => {
                        if self.animate() {
                            self.change_state(w, rng, acts);
                        }
                    }
                }
                self.timer = 0.0;
                false
            }
            State::Recoil => {
                let end = self.animate();
                self.timer = 0.0;
                end
            }
            State::Wander => self.yomp(self.want, 0.25),
            State::Roar => {
                let end = self.animate();
                self.timer = 0.0;
                end
            }
            State::Follow | State::Charge => match self.target.and_then(|t| w.people.get(t)) {
                // Straight at them when he can see them. When he can't, the original asks its
                // navigation map the way (`MAV_do`); here he makes for whichever of eight
                // places two squares off he can get to that brings him nearest them.
                Some(t) => {
                    let (me, eye) = (self.pos(), vec3(0.0, 0x40 as f32, 0.0));
                    let to = if (w.clear)(me + eye, t.pos + eye) {
                        t.pos
                    } else {
                        (0..8)
                            .map(|k| {
                                let a = k as f32 * TAU / 8.0;
                                me + vec3(a.sin(), 0.0, a.cos()) * 512.0
                            })
                            .filter(|&p| (w.clear)(me + eye, p + eye))
                            .min_by(|a, b| (*a - t.pos).length().total_cmp(&(*b - t.pos).length()))
                            .unwrap_or(t.pos)
                    };
                    self.want = to;
                    self.yomp(to, 1.0 / 8.0)
                }
                None => {
                    self.timer = 0.0;
                    true
                }
            },
            State::Swipe | State::Stomp => {
                let (frame, radius, damage) = if self.state == State::Swipe { (4, 0x200, 100) } else { (12, 0x300, 150) };
                if self.target.is_some() && !self.attacked && self.frame > frame {
                    acts.push(Act::Shockwave { at: self.pos(), radius: radius as f32, damage, from: me });
                    if self.state == State::Stomp {
                        acts.push(Act::Dust { at: self.pos() });
                    }
                    self.attacked = true;
                }
                let end = self.animate();
                self.timer = 0.0;
                end
            }
            State::Fireball => match self.sub {
                Sub::FireballTurn => {
                    let end = self.animate();
                    if self.turn_to_target(w).abs() < 128.0 && end {
                        self.sub = Sub::FireballFire;
                        self.set_anim(anim::BALROG_SWIPE);
                    }
                    end
                }
                _ => {
                    if self.target.is_some() && !self.attacked && self.frame > 4 {
                        self.fireball(w, rng, acts);
                        self.attacked = true;
                    }
                    let end = self.animate();
                    self.timer = 0.0;
                    end
                }
            },
            State::BalrogIdle => self.animate(),
            State::BaneIdle => {
                let end = self.animate();
                // Every five seconds, a bolt to each of the bodies he is drawing up.
                self.sparks += 16.0;
                if self.sparks > (16 * 20 * 5) as f32 {
                    self.sparks = 0.0;
                    for person in self.summon.iter().flatten() {
                        acts.push(Act::Spark { person: *person, head: false });
                    }
                }
                end
            }
            State::BaneAttack => {
                if !self.attacked && self.frame > 12 {
                    acts.push(Act::Shockwave { at: self.pos(), radius: 0x300 as f32, damage: 150, from: me });
                    acts.push(Act::Dust { at: self.pos() });
                    self.attacked = true;
                    // And a bolt at the player, if he can see them within six squares.
                    if let Some(p) = w.player.filter(|&p| !w.people[p].dead) {
                        let d = w.people[p].pos;
                        let near = (d - self.pos()).length() < 0x600 as f32;
                        if near && (w.clear)(self.pos() + vec3(0.0, 0xc0 as f32, 0.0), d + vec3(0.0, 0x60 as f32, 0.0)) {
                            acts.push(Act::Spark { person: p, head: true });
                            acts.push(Act::Hurt { person: p, damage: 50, recoil: true, from: self.pos() });
                        }
                    }
                }
                let end = self.animate();
                self.timer = 0.0;
                end
            }
            State::BaneStart => {
                let end = self.animate();
                self.timer = 0.0;
                end
            }
        };
        self.travel(w, acts);

        // Too far from home, and it gets restless twice as fast.
        let p = self.pos();
        let off = Vec2::new(p.x - ((self.home.0 << 8) + 0x80) as f32, p.z - ((self.home.1 << 8) + 0x80) as f32);
        if off.x.abs() > 0x500 as f32 || off.y.abs() > 0x500 as f32 {
            self.timer = (self.timer / 2.0).floor();
        }
        if self.timer <= TIMER_TICK {
            self.timer = 0.0;
            if end {
                self.change_state(w, rng, acts);
            }
        } else {
            self.timer -= TIMER_TICK;
        }
        self.sync();
    }

    /// `BAT_STATE_CIRCLE`: round and round the place it wants, swooping on its target.
    fn circle(&mut self, w: &Around, acts: &mut Vec<Act>) -> bool {
        let me = self.pos();
        let target = self.target.and_then(|t| w.people.get(t).map(|p| (t, p)));
        let mut want = match (self.sub, target) {
            (Sub::CircleTarget, Some((_, p))) => p.head,
            _ => {
                let x = ((self.home.0 << 8) + 0x80) as f32;
                let z = ((self.home.1 << 8) + 0x80) as f32;
                vec3(x, (w.floor)(x, z) + 0x100 as f32, z)
            }
        };
        let d = Vec2::new(want.x - me.x, want.z - me.z);
        let mut dangle = wrap(calc_angle(d) - self.yaw);
        if let (Sub::CircleTarget, Some((t, p))) = (self.sub, target) {
            if dangle.abs() < 300.0 {
                // The swoop: it doesn't stop until it's through.
                self.timer += TIMER_TICK;
                if qdist3(want - me) < 0x40 as f32 {
                    if !(p.player && w.frozen) && !self.attacked {
                        acts.push(Act::Hurt { person: t, damage: 60, recoil: false, from: me });
                        self.attacked = true;
                    }
                } else {
                    self.attacked = false;
                }
            } else {
                // Back to whatever height it last had in mind, which is nothing at all if it
                // has never had one (the original's `want_y` starts at 0).
                want.y = self.want.y;
            }
        }
        dangle /= 4.0;
        let dist = (qdist2(d) / 16.0).max(10.0);
        self.yaw = (self.yaw + dangle.clamp(-dist, dist)).rem_euclid(2048.0);
        // Flies the way it faces: `SIN(angle) >> 2`, 64 units a tick.
        let f = sin_cos(self.yaw) * 64.0;
        self.vel.x += ((f.x - self.vel.x) / 16.0).clamp(-8.0, 8.0);
        self.vel.z += ((f.y - self.vel.z) / 16.0).clamp(-8.0, 8.0);
        self.vel.x -= self.vel.x / 32.0;
        self.vel.y -= self.vel.y / 4.0;
        self.vel.z -= self.vel.z / 32.0;
        self.vel.y += (want.y - me.y) * 4.0 / 256.0;
        self.animate()
    }

    /// Moves it on by its speed. The Balrog stays on the ground and slides along walls; he
    /// also stamps, a footfall to each half of his stride.
    fn travel(&mut self, w: &Around, acts: &mut Vec<Act>) {
        let p = self.pos();
        let mut to = p + self.vel;
        if self.kind == BALROG {
            let flat = (w.slide)(Vec2::new(p.x, p.z), p.y, Vec2::new(self.vel.x, self.vel.z));
            to = vec3(flat.x, (w.terrain)(p.x, p.z) + 0x40 as f32, flat.y);
            let f = self.frame;
            let (first, second) = match self.anim {
                anim::BALROG_YOMP => ((1..6).contains(&f) || (11..16).contains(&f), (6..=11).contains(&f)),
                anim::BALROG_YOMP_END => ((1..5).contains(&f) || (11..16).contains(&f), (5..=11).contains(&f)),
                _ => (false, false),
            };
            if first && !self.steps[0] {
                acts.push(Act::Sound { id: sound::BALROG_STEP + (f as usize & 1), at: p });
                self.steps = [true, false];
            }
            if second && !self.steps[1] {
                acts.push(Act::Sound { id: sound::BALROG_STEP + (f as usize & 1), at: p });
                self.steps = [false, true];
            }
        }
        self.fig.pos = to;
    }

    /// `BAT_apply_hit`: shot or caught in a blast for `damage`, by someone at `from`.
    pub fn hit(&mut self, from: Option<(Vec3, Option<usize>)>, damage: i32, acts: &mut Vec<Act>) {
        if self.kind == BANE || matches!(self.state, State::Dying | State::Dead) {
            return;
        }
        if let Some((at, _)) = from {
            if self.kind != BALROG {
                // Knocked back by it.
                let d = at - self.pos();
                let dist = qdist3(d) + 1.0;
                self.vel -= d / dist * 16.0;
            }
        }
        // "Bats are nails!", and the Balrog more so.
        let mut damage = damage.max(0) >> 1;
        if self.kind == BALROG {
            damage >>= 3;
        }
        damage += 1;
        if self.health <= damage {
            self.health = 0;
            self.state = State::Dying;
            if self.kind == GARGOYLE {
                self.set_anim(anim::GARGOYLE_START_FALL);
                self.sub = Sub::DeadInitial;
                self.dead = true;
            } else {
                self.sub = Sub::DeadFinal;
                self.vel = Vec3::ZERO;
                self.set_anim(anim::BALROG_DIE);
                acts.push(Act::Sound { id: sound::BALROG_DEATH, at: self.pos() });
            }
        } else {
            self.health -= damage;
            if self.kind == BALROG {
                // Whoever hurt him is who he's after now.
                if let Some((_, Some(person))) = from {
                    self.target = Some(person);
                    if matches!(self.state, State::BalrogIdle | State::Wander) {
                        self.timer = 0.0;
                    }
                }
            } else {
                self.set_anim(anim::GARGOYLE_TAKE_HIT);
                self.state = State::Recoil;
            }
        }
    }

    /// Who can be aimed at: not Bane, nobody dying.
    pub fn targetable(&self) -> bool {
        self.kind != BANE && !matches!(self.state, State::Dying | State::Dead)
    }

    /// How high its middle is above its position, for aiming and blasts.
    pub fn middle(&self) -> Vec3 {
        self.pos() + vec3(0.0, if self.kind == BALROG { 0x80 as f32 } else { 0x30 as f32 }, 0.0)
    }

    /// What to call it on screen.
    pub fn name(&self) -> &'static str {
        match self.kind {
            GARGOYLE => "GARGOYLE",
            BALROG => "BALROG",
            _ => "BANE",
        }
    }

    pub fn max_health(&self) -> i32 {
        if self.kind == BALROG { 255 } else { 100 }
    }
}

/// A fireball in flight (a `PFLAG_EXPLODE_ON_IMPACT | PFLAG_GRAVITY` particle): falls about
/// a unit a tick faster each tick, and bursts when it meets the ground or a roof.
#[derive(Debug, Clone, Copy)]
pub struct Fireball {
    pub pos: Vec3,
    /// Units a tick.
    pub vel: Vec3,
    /// Seconds in the air: it gives out after 100 particle ticks at 15 a second.
    pub age: f32,
}

impl Fireball {
    pub const LIFE: f32 = 100.0 / 15.0;
    /// `0xff` (in 256ths) a tick off its rise.
    pub const GRAVITY: f32 = 255.0 / 256.0;
}
