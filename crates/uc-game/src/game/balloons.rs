//! Balloons (fallen/Source/balloon.cpp, #108). A balloon is a string of four points, the
//! first tied to the holder's left hand and the last the balloon itself. Each point floats
//! up and is pulled back to the string's length from the one before (`BALLOON_process`), so
//! the balloon floats over whoever holds it, trailing behind as they walk (it only rises at
//! 80 units a second), and is pushed back off walls. Gun Hunt and
//! Grim Gardens each give one to a civilian (`ET_CIV_BALLOON`).
//!
//! Knocked down or killed, or running scared for half a second, the holder lets go
//! (`BALLOON_release`). The balloon then floats away, its string pulled up after it, and is
//! gone once it's high enough. Someone jumping with a hand near a loose balloon's string
//! catches it (`BALLOON_find_grab`, from `grab_ledge`), so Darci can jump for one.
//!
//! It runs a turn (a twentieth of a second) at a time in the original's units: positions in
//! 256ths of a world unit.

// The index loops follow the original's, over x, y and z.
#![allow(clippy::needless_range_loop)]

use crate::barrel::qdist3;
use crate::*;

/// `BALLOON_POINTS_PER_BALLOON`.
pub const POINTS: usize = 4;
/// `BALLOON_POINT_DIST`: the string between two points, in 256ths.
pub const POINT_DIST: i64 = 0x2000;
/// `BALLOON_MAX_BALLOONS`, less slot 0, which the original never uses.
const MAX: usize = 31;
/// `BALLOON_RADIUS` and `BALLOON_BOUNCE`: how far off a wall the balloon feels it, and the
/// push it gets from it.
const RADIUS: i64 = 0x30;
const BOUNCE: i64 = 0x100;
/// A balloon let go is gone above this (`0x100000`, 4096 world units up).
const GONE_ABOVE: i64 = 0x100000;
/// `BALLOON_find_grab`: a hand this near the end of a loose string catches it.
const GRAB: i64 = 0x10000;
/// `PCOM_get_duration(5)`: running scared this long, a holder lets go (pcom.cpp:7678).
const SCARED: f32 = 0.5;
pub const TICK: f32 = 1.0 / 20.0;

/// `BALLOON_Balloon`.
#[derive(Debug, Clone)]
pub struct Balloon {
    /// Who holds it, as an index into the people; none once let go.
    pub holder: Option<usize>,
    /// From the hand (first) to the balloon (last), and how each is moving.
    p: [[i64; 3]; POINTS],
    d: [[i64; 3]; POINTS],
    /// Floated out of sight (`BALLOON_TYPE_UNUSED`).
    pub gone: bool,
}

impl Balloon {
    /// The string's points in world units, from the hand to the balloon.
    pub fn points(&self) -> [Vec3; POINTS] {
        self.p.map(|p| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 256.0)
    }
}

fn to_fixed(v: Vec3) -> [i64; 3] {
    [(v.x * 256.0) as i64, (v.y * 256.0) as i64, (v.z * 256.0) as i64]
}

/// `MAV_inside` (mav.cpp:2403): under the top of the square there.
fn inside(level: &Level, p: [i64; 3]) -> bool {
    level.ground.map_height(&level.map, p[0] as f32, p[2] as f32) > p[1] as f32
}

impl Game {
    /// `BALLOON_create`: a balloon for person `i`, its string straight up from their hand.
    /// Nothing if they have one, or if all 31 have been made.
    pub(crate) fn give_balloon(&mut self, i: usize) {
        if self.balloons.len() >= MAX || self.balloons.iter().any(|b| b.holder == Some(i)) {
            return;
        }
        let a = to_fixed(game::hook::hand(&self.people[i]));
        let p = std::array::from_fn(|k| [a[0], a[1] + k as i64 * POINT_DIST, a[2]]);
        self.balloons.push(Balloon { holder: Some(i), p, d: [[0; 3]; POINTS], gone: false });
    }

    /// Each turn: holders who are down or running let go, someone jumping catches a loose
    /// string, and every balloon moves (`BALLOON_process`, at Game.cpp:2555).
    pub(crate) fn run_balloons(&mut self, dt: f32, level: &Level) {
        if self.balloons.is_empty() {
            return;
        }
        self.clocks.balloon += dt;
        while self.clocks.balloon >= TICK {
            self.clocks.balloon -= TICK;
            for b in &mut self.balloons {
                // `set_person_dying` (Person.cpp:1120) and `PCOM_AI_SUBSTATE_LEGIT`.
                let Some(h) = b.holder else { continue };
                let p = &self.people[h];
                if p.removed || p.is_down() || p.is_dead() || matches!(p.state, State::Injured)
                    || p.scared.is_some_and(|s| s.time > SCARED)
                {
                    b.holder = None;
                }
            }
            self.grab_balloons();
            for k in 0..self.balloons.len() {
                if self.balloons[k].gone {
                    continue;
                }
                match self.balloons[k].holder {
                    Some(h) => {
                        let hand = to_fixed(game::hook::hand(&self.people[h]));
                        held(&mut self.balloons[k], hand, level);
                    }
                    None => loose(&mut self.balloons[k]),
                }
            }
        }
    }

    /// `BALLOON_find_grab`: anyone in the air without a balloon catches the first loose one
    /// whose string ends within a square of their hand.
    fn grab_balloons(&mut self) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.removed || !matches!(p.state, State::Fall { .. }) || self.balloons.iter().any(|b| b.holder == Some(i)) {
                continue;
            }
            let a = to_fixed(game::hook::hand(p));
            let near = |b: &Balloon| qdist3(b.p[0][0] - a[0], b.p[0][1] - a[1], b.p[0][2] - a[2]) < GRAB;
            if let Some(b) = self.balloons.iter_mut().find(|b| !b.gone && b.holder.is_none() && near(b)) {
                b.holder = Some(i);
            }
        }
    }

    /// The balloons and their strings, for [`Game::batches`]. A held string starts at the
    /// hand as it's drawn. The shipped game draws only loose balloons (`AENG_draw_released_balloons`;
    /// its held ones are under `#if NO_MORE_BALLOONS`), but held ones are what the missions
    /// mean to show.
    pub(crate) fn draw_balloons(&self, out: &mut BTreeMap<u32, Batch>, level: &Level, eye: Vec3) {
        let lighter = level.lighter();
        for (k, b) in self.balloons.iter().enumerate().filter(|(_, b)| !b.gone) {
            let mut pts = b.points();
            if let Some(h) = b.holder {
                pts[0] = game::hook::hand(&self.people[h]);
            }
            // `SHAPE_draw_balloon` (shape.cpp:2140): the string in `0x558845`, 4 wide.
            for w in pts.windows(2) {
                let (a, c) = (w[0], w[1]);
                let across = (a - eye).cross(c - a).normalize_or(Vec3::X) * 1.0;
                let corner = |p: Vec3, s: f32| (p + across * s) / uc_world::UNIT;
                out.entry(uc_world::UNTEXTURED).or_default().quad(
                    [corner(a, -1.0), corner(a, 1.0), corner(c, -1.0), corner(c, 1.0)],
                    [[0.0, 0.0]; 4],
                    [[0x55, 0x88, 0x45, 255]; 4],
                );
            }
            // The balloon, along the string's last piece. Its colour is the slot's
            // (`balloon_colours[balloon & 3]`), so the first made is yellow. The original's
            // mesh (`PRIM_OBJ_BALLOON`) isn't in the shipped files, where prim 132 is the
            // transmitter, so it's an egg shape.
            let rgb = [[255, 0, 0], [255, 255, 0], [0, 255, 0], [0, 0, 255]][(k + 1) & 3];
            let up = (pts[POINTS - 1] - pts[POINTS - 2]).normalize_or(Vec3::Y);
            let turn = glam::Quat::from_rotation_arc(Vec3::Y, up);
            let shade = |at: Vec3, n: Vec3| -> [u8; 4] {
                let l = lighter.as_ref().map_or([0.55 + 0.45 * n.dot(Vec3::new(0.3, 0.9, 0.3).normalize()).max(0.0); 3], |l| l.light(at.into(), n.into()));
                let c = |i: usize| (rgb[i] as f32 * l[i].min(1.0)) as u8;
                [c(0), c(1), c(2), 255]
            };
            egg(out, pts[POINTS - 1], turn, shade);
        }
    }
}

/// `BALLOON_process` for a held balloon: the first point at the hand, and each after it
/// moving, pulled back to the string's length from the one before, floating up and slowing.
/// The balloon itself is pushed off any wall it's up against.
fn held(b: &mut Balloon, hand: [i64; 3], level: &Level) {
    b.p[0] = hand;
    for j in 1..POINTS {
        let p1 = b.p[j - 1];
        let (p2, d2) = (&mut b.p[j], &mut b.d[j]);
        for k in 0..3 {
            p2[k] += d2[k];
        }
        // "Avoid overflows."
        let mut d = [0; 3];
        for k in 0..3 {
            d[k] = (p2[k] - p1[k]).clamp(-0x7000, 0x7000);
        }
        let dist = qdist3(d[0], d[1], d[2]) + 1;
        if dist > POINT_DIST {
            let ddist = dist - POINT_DIST;
            for k in 0..3 {
                let pull = d[k] * ddist / dist;
                p2[k] -= pull;
                d2[k] -= pull / 0x10;
            }
        }
        d2[1] += 0x40;
        for v in d2.iter_mut() {
            *v -= *v / 0x10;
        }
    }
    // "Stop the balloon going through walls."
    let top = b.p[POINTS - 1];
    let at = |dx: i64, dz: i64| inside(level, [(top[0] >> 8) + dx, top[1] >> 8, (top[2] >> 8) + dz]);
    let d = &mut b.d[POINTS - 1];
    if at(RADIUS, 0) {
        d[0] -= BOUNCE;
    }
    if at(-RADIUS, 0) {
        d[0] += BOUNCE;
    }
    if at(0, RADIUS) {
        d[2] -= BOUNCE;
    }
    if at(0, -RADIUS) {
        d[2] += BOUNCE;
    }
}

/// `BALLOON_process` for one let go: the balloon drifts up and away, gone once high
/// enough, pulling its string up after it.
fn loose(b: &mut Balloon) {
    let (p, d) = (&mut b.p[POINTS - 1], &mut b.d[POINTS - 1]);
    d[1] += 0x40;
    for k in 0..3 {
        d[k] -= d[k] / 0x10;
        p[k] += d[k];
    }
    if p[1] > GONE_ABOVE {
        b.gone = true;
        return;
    }
    for j in (0..POINTS - 1).rev() {
        let (p1, d1) = (b.p[j + 1], b.d[j + 1]);
        let (p2, d2) = (&mut b.p[j], &mut b.d[j]);
        for k in 0..3 {
            p2[k] += d2[k];
        }
        let mut d = [0; 3];
        for k in 0..3 {
            d[k] = (p2[k] - p1[k]).clamp(-0x4000, 0x4000);
        }
        let dist = qdist3(d[0], d[1], d[2]) + 1;
        if dist > POINT_DIST {
            let ddist = dist - POINT_DIST;
            for k in 0..3 {
                p2[k] -= d[k] * ddist / dist;
            }
            *d2 = d1;
        }
        d2[1] -= 0x100;
        for v in d2.iter_mut() {
            *v -= *v / 0x10;
        }
    }
}

/// A balloon 44 across and 56 high, its knot at `knot` and `turn` taking straight up to the
/// way it points, lit by `shade` (for a point and its normal).
fn egg(out: &mut BTreeMap<u32, Batch>, knot: Vec3, turn: glam::Quat, shade: impl Fn(Vec3, Vec3) -> [u8; 4]) {
    const RINGS: usize = 6;
    const SIDES: usize = 10;
    let (across, high) = (22.0, 28.0);
    let at = |ring: usize, side: usize| {
        let lat = std::f32::consts::PI * ring as f32 / RINGS as f32;
        let lon = std::f32::consts::TAU * side as f32 / SIDES as f32;
        let n = Vec3::new(lat.sin() * lon.cos(), -lat.cos(), lat.sin() * lon.sin());
        // Narrower towards the knot.
        let squeeze = if n.y < 0.0 { 1.0 + 0.25 * n.y } else { 1.0 };
        let local = Vec3::new(n.x * across * squeeze, high + n.y * high, n.z * across * squeeze);
        let world = knot + turn * local;
        (world, turn * n)
    };
    let batch = out.entry(uc_world::UNTEXTURED).or_default();
    for ring in 0..RINGS {
        for side in 0..SIDES {
            let q = [at(ring, side), at(ring, side + 1), at(ring + 1, side), at(ring + 1, side + 1)];
            batch.quad(q.map(|(p, _)| p / uc_world::UNIT), [[0.0, 0.0]; 4], q.map(|(p, n)| shade(p, n)));
        }
    }
}
