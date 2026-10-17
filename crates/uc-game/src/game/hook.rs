//! Darci's grappling hook (#109): fallen/Source/hook.cpp, built and then switched off in the
//! original (`HOOK_init` is commented out of elev.cpp, and only debug keys ever drove it).
//! With the hook in hand, attack spins it round her head and lets it go
//! (`ANIM_GRAPPLING_HOOK_WINDUP`, `_RELEASE`; neither is in the shipped `darci1.all`, so she
//! throws it as she throws a grenade, `ANIM_CAN_RELEASE`); it flies and bounces as `HOOK_process_flying`
//! has it, trailing its line (`HOOK_process_points`), drawn going from dark orange to white
//! as `AENG_draw` drew it. The original never got as far as climbing the line: here a hook
//! that comes down on a roof, or hits a wall just under its top, catches the edge, and attack
//! again at the foot of the wall climbs it like a ladder, off the top onto the roof. Anything
//! else, or walking off too far, and she reels it back in (`ANIM_GRAPPLING_HOOK_PICKUP`).

use crate::*;

/// `ANIM_GRAPPLING_HOOK_WINDUP`, `_RELEASE` and `_PICKUP` (fallen/Headers/animate.h). Only
/// the pick-up made it into the shipped animations; the others are used if a set has them.
pub const WINDUP: usize = 137;
pub const RELEASE: usize = 138;
pub const PICKUP: usize = 140;
/// `PRIM_OBJ_HOOK`.
pub const HOOK_PRIM: u8 = 86;
/// The throw, world units a second: harder up than a grenade, to clear a couple of storeys.
const THROW_ACROSS: f32 = 800.0;
const THROW_UP: f32 = 1150.0;
const GRAVITY: f32 = 800.0;
/// How much line there is: the hook goes no further from her hand.
pub const LINE: f32 = 12.0 * 256.0;
/// The line's points (`HOOK_NUM_POINTS` is 256; this many are plenty to draw).
const POINTS: usize = 24;
/// Reeling in, world units a second.
const REEL: f32 = 1200.0;
/// To catch, the edge has to be at least this far above her feet (no use for a kerb).
const CATCH_ABOVE: f32 = 256.0;
/// A wall hit this near its top catches on the edge.
const CATCH_UNDER: f32 = 96.0;
/// How near the foot of the wall she has to be to start climbing.
pub const CLIMB_FROM: f32 = 2.0 * 256.0;

/// What the hook's doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hold {
    Flying,
    /// Caught on the edge of a roof: `edge` on its top, `dir` the way into the building
    /// (along a square's side), `to` where she'll stand on the roof.
    Caught { edge: Vec3, dir: Vec2, to: Vec3 },
    /// Coming back to her hand.
    Reeling,
}

/// The hook out of her hand.
#[derive(Debug, Clone)]
pub struct Hook {
    pub hold: Hold,
    pub pos: Vec3,
    vel: Vec3,
    /// The line from her hand (first) to the hook (last), and where each point was a turn
    /// ago.
    pub line: Vec<Vec3>,
    old: Vec<Vec3>,
    /// The hook's tumble as it flies (`HOOK_grapple_pitch`).
    pub pitch: f32,
    pub yaw: f32,
}

impl Game {
    /// Attack with the hook in hand: thrown if it's in hand (the wind-up, then the release
    /// lets it go), or, caught on a roof with her at the foot of the wall, up she goes.
    pub(crate) fn use_hook(&mut self, pi: usize) {
        if self.indoors.is_some() || !matches!(self.people[pi].state, State::Move) {
            return;
        }
        match self.hook.as_ref().map(|h| h.hold) {
            None => {
                let p = &mut self.people[pi];
                if p.fig.restart(WINDUP, uc_world::Play::Once) {
                    p.state = State::Hook { released: false, windup: true };
                } else if p.fig.restart(game::grenades::ANIM_CAN_RELEASE, uc_world::Play::Once) {
                    p.state = State::Hook { released: false, windup: false };
                } else {
                    self.release_hook(pi);
                }
            }
            Some(Hold::Caught { edge, dir, to }) => {
                let p = &self.people[pi];
                let foot = Vec2::new(edge.x, edge.z) - dir * 40.0;
                let near = (Vec2::new(p.fig.pos.x, p.fig.pos.z) - foot).length() < CLIMB_FROM;
                if near && edge.y - p.fig.pos.y > CATCH_ABOVE * 0.5 {
                    self.people[pi].climb_rope(foot, dir, edge.y, to);
                } else {
                    self.messages.push(Message { text: "Get to the foot of the wall to climb".into(), left: 2.0, face: None, filmed: false });
                }
            }
            Some(_) => {}
        }
    }

    /// The release lets the hook go from her hand (`HOOK_release`), out along her facing
    /// and up.
    pub(crate) fn release_hook(&mut self, pi: usize) {
        if self.hook.is_some() {
            return;
        }
        let p = &self.people[pi];
        let hand = hand(p);
        let f = p.fig.facing();
        let vel = Vec3::new(f.x * THROW_ACROSS, THROW_UP, f.y * THROW_ACROSS);
        self.hook = Some(Hook {
            hold: Hold::Flying, pos: hand, vel, line: vec![hand; POINTS], old: vec![hand; POINTS], pitch: 0.0, yaw: f.x.atan2(f.y),
        });
    }

    /// Every turn: the hook flies, bounces and catches (`HOOK_process_flying`), stays caught
    /// while she's within the line of it, and comes back when reeled in; the line follows.
    pub(crate) fn run_hook(&mut self, dt: f32, level: &Level) {
        let Some(mut h) = self.hook.take() else { return };
        let Some(pi) = self.player else { return };
        let p = &self.people[pi];
        let (hand, feet) = (hand(p), p.fig.pos);
        // Not hers any more (dropped, or she's driving off): it's gone.
        if !p.has(Weapon::Hook) || matches!(p.state, State::InCar { .. } | State::Bike { .. }) {
            return;
        }
        match h.hold {
            Hold::Flying => {
                h.vel.y -= GRAVITY * dt;
                let old = h.pos;
                h.pos += h.vel * dt;
                h.pitch += 12.0 * dt;
                let floor = level.ground.map_height(&level.map, h.pos.x, h.pos.z);
                if h.pos.y < floor {
                    let crossed_x = (old.x / 256.0).floor() != (h.pos.x / 256.0).floor();
                    let crossed_z = (old.z / 256.0).floor() != (h.pos.z / 256.0).floor();
                    let wall = floor - h.pos.y > 64.0 || h.vel.y > 0.0;
                    if wall && floor - h.pos.y < CATCH_UNDER && floor - feet.y > CATCH_ABOVE && (crossed_x || crossed_z) {
                        // Just under the top: over the edge it goes, and catches.
                        let dir = if crossed_x { Vec2::new((h.pos.x - old.x).signum(), 0.0) } else { Vec2::new(0.0, (h.pos.z - old.z).signum()) };
                        h.hold = caught(level, old, dir, floor);
                        h.pos = Vec3::new(h.pos.x, floor, h.pos.z);
                    } else if wall {
                        // Off a wall, losing half its speed across.
                        if crossed_x {
                            h.pos.x = old.x;
                            h.vel.x = -h.vel.x / 2.0;
                        }
                        if crossed_z {
                            h.pos.z = old.z;
                            h.vel.z = -h.vel.z / 2.0;
                        }
                    } else if floor - feet.y > CATCH_ABOVE {
                        // Down on a roof: dragged back towards her, it catches the edge.
                        h.pos.y = floor;
                        h.hold = edge_towards(level, h.pos, feet).unwrap_or(Hold::Reeling);
                    } else {
                        // Down at her level: nothing to hold.
                        h.pos.y = floor;
                        h.hold = Hold::Reeling;
                    }
                }
                if (h.pos - hand).length() > LINE {
                    // Out of line: it pulls up short and comes back.
                    h.hold = Hold::Reeling;
                }
            }
            Hold::Caught { edge, to, .. } => {
                let on_rope = matches!(p.state, State::Rope { .. });
                if p.fig.pos.y >= to.y - 32.0 && !on_rope {
                    // Up on the roof: the hook's freed and wound in.
                    h.hold = Hold::Reeling;
                } else if !on_rope && (edge - feet).length() > LINE {
                    // Walked off with the line out: it's pulled free.
                    h.hold = Hold::Reeling;
                }
            }
            Hold::Reeling => {
                let to = hand - h.pos;
                if to.length() < REEL * dt + 32.0 {
                    // Back in her hand.
                    let p = &mut self.people[pi];
                    if matches!(p.state, State::Move) && p.fig.restart(PICKUP, uc_world::Play::Once) {
                        p.state = State::Hook { released: true, windup: false };
                    }
                    return;
                }
                h.pos += to.normalize() * REEL * dt;
                h.pitch += 8.0 * dt;
            }
        }
        // The line: verlet points pulled to their spacing, under gravity, kept out of the
        // ground, its ends at her hand and the hook.
        let n = h.line.len();
        let rest = ((h.pos - hand).length() * 1.1).min(LINE) / (n - 1) as f32;
        for k in 1..n - 1 {
            let (now, was) = (h.line[k], h.old[k]);
            h.old[k] = now;
            h.line[k] = now + (now - was) * 0.96 - Vec3::new(0.0, 1200.0 * dt * dt, 0.0);
        }
        h.line[0] = hand;
        h.line[n - 1] = h.pos;
        for _ in 0..8 {
            for k in 0..n - 1 {
                let d = h.line[k + 1] - h.line[k];
                let len = d.length();
                if len <= rest || len == 0.0 {
                    continue;
                }
                let fix = d * ((len - rest) / len);
                match (k, k + 1 == n - 1) {
                    (0, true) => {}
                    (0, false) => h.line[k + 1] -= fix,
                    (_, true) => h.line[k] += fix,
                    _ => {
                        h.line[k] += fix * 0.5;
                        h.line[k + 1] -= fix * 0.5;
                    }
                }
            }
        }
        for q in &mut h.line[1..n - 1] {
            let floor = level.ground.map_height(&level.map, q.x, q.z) + 2.0;
            if q.y < floor && floor - q.y < 64.0 {
                q.y = floor;
            }
        }
        self.hook = Some(h);
    }
}

/// Her left hand (`SUB_OBJECT_LEFT_HAND`), where the line is held, and a balloon's string.
pub(crate) fn hand(p: &Person) -> Vec3 {
    p.fig.part_world(7).map_or(p.fig.pos + Vec3::new(0.0, 120.0, 0.0), |(_, at)| at)
}

/// A hook come over the edge of a roof at `floor`, from `from` outside it going `dir` (along
/// a square's side): caught there, with where she'll stand on top.
fn caught(level: &Level, from: Vec3, dir: Vec2, floor: f32) -> Hold {
    // The edge is the side of the square it crossed into.
    let snap = |v: f32, d: f32| if d > 0.0 { ((v / 256.0).floor() + 1.0) * 256.0 } else if d < 0.0 { (v / 256.0).floor() * 256.0 } else { v };
    let edge = Vec2::new(snap(from.x, dir.x), snap(from.z, dir.y));
    let on = edge + dir * 90.0;
    let to = Vec3::new(on.x, level.ground.surface(&level.map, on.x, on.y, floor + 64.0), on.y);
    Hold::Caught { edge: Vec3::new(edge.x, floor, edge.y), dir, to }
}

/// A hook lying on a roof at `at`: dragged back towards her feet at `feet` until it reaches
/// the edge, and caught there. None if it never reaches one (the roof slopes away, or it's
/// further than the line).
fn edge_towards(level: &Level, at: Vec3, feet: Vec3) -> Option<Hold> {
    let back = Vec2::new(feet.x - at.x, feet.z - at.z);
    let len = back.length();
    let step = back.normalize_or_zero() * 16.0;
    let mut p = Vec2::new(at.x, at.z);
    for _ in 0..(len / 16.0) as usize {
        let next = p + step;
        if level.ground.map_height(&level.map, next.x, next.y) < at.y - 64.0 {
            // Over the edge between `p` and `next`: whichever square side it crossed.
            let dir = if (p.x / 256.0).floor() != (next.x / 256.0).floor() {
                Vec2::new(-step.x.signum(), 0.0)
            } else {
                Vec2::new(0.0, -step.y.signum())
            };
            let from = Vec3::new(next.x, at.y, next.y);
            return Some(caught(level, from, dir, at.y));
        }
        p = next;
    }
    None
}
