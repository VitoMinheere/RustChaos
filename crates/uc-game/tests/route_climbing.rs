//! People on routes jump, pull up and climb ladders (`MAV_ACTION_JUMP`, `PULLUP`,
//! `LADDER_UP`, #88): someone following Darci goes up after her onto a roof. Skipped
//! without an install.

use glam::{vec3, Vec3};
use uc_game::Controls;
use uc_world::mav::{caps, Mav, DIRS, GOTO, JUMPPULL, LADDER_UP, PULLUP};

mod common;
use common::Run;

impl Run {
    /// Someone else to follow her: the first person on foot who isn't hostile, made a
    /// follower of the player (`PCOM_MOVE_FOLLOW`) and put at `at`.
    fn follower(&mut self, at: Vec3) -> usize {
        let i = self.g.people.iter().position(|p| !p.player && !p.removed && !p.hostile() && !p.drives() && !p.is_down())
            .expect("someone to follow her");
        let p = &mut self.g.people[i];
        (p.mv, p.follow, p.bent) = (5, 0, 0);
        self.g.people[i].respawn(at);
        let y = self.level.ground.surface(&self.level.map, at.x, at.z, at.y + 64.0);
        self.g.people[i].fig.pos.y = y;
        i
    }

    /// Puts the player at `at` and plays `secs` with her standing still, returning how high
    /// person `i` got.
    fn wait_on(&mut self, at: Vec3, i: usize, secs: f32) -> f32 {
        let mut best = f32::MIN;
        for _ in 0..(secs / 0.5) as usize {
            let p = self.player();
            self.g.people[p].fig.pos = at;
            self.play(0.5, Controls::default());
            best = best.max(self.g.people[i].fig.pos.y);
        }
        best
    }
}

/// The middle of square `s` on the ground or roof there.
fn middle(r: &Run, s: (i32, i32), y: f32) -> Vec3 {
    let (x, z) = (s.0 as f32 * 256.0 + 128.0, s.1 as f32 * 256.0 + 128.0);
    vec3(x, r.level.ground.surface(&r.level.map, x, z, y), z)
}

/// The first edge of the navigation map that only `action` gets over, going up at least
/// `rise` (world units): (from square, to square).
fn edge(r: &Run, mav: &Mav, action: u8, rise: f32) -> Option<((i32, i32), (i32, i32))> {
    (0..128 * 128).find_map(|k| {
        let a = (k / 128, k % 128);
        (0..4).find_map(|d| {
            let o = mav.opt(a.0, a.1, d);
            let b = (a.0 + DIRS[d].0, a.1 + DIRS[d].1);
            let ok = o & caps(action) != 0 && o & caps(GOTO) == 0 && (0..128).contains(&b.0) && (0..128).contains(&b.1);
            let up = ok && middle(r, b, 1.0e5).y - middle(r, a, 1.0e5).y > rise;
            // The higher level goes on three squares, for her to stand on out of the
            // follower's stopping distance.
            let into = (b.0 - a.0, b.1 - a.1);
            let on = |k: i32| {
                let s = (b.0 + k * into.0, b.1 + k * into.1);
                (0..128).contains(&s.0) && (0..128).contains(&s.1) && (middle(r, s, 1.0e5).y - middle(r, b, 1.0e5).y).abs() < 16.0
            };
            (up && (1..=3).all(on)).then_some((a, b))
        })
    })
}

fn follows_up(mission: &str, action: u8) {
    assert!(follows_up_by(mission, action, 200.0));
}

/// Whether there was such an edge in `mission` to check.
fn follows_up_by(mission: &str, action: u8, rise: f32) -> bool {
    let Some(mut r) = common::start(mission) else { return true };
    r.play(2.0, Controls::default());
    let mav = Mav::new(&r.level);
    let Some((a, b)) = edge(&r, &mav, action, rise) else {
        println!("{mission}: no such edge");
        return false;
    };
    // Her on the higher roof, three squares further in (followers stop two short); him on the square below the edge, at
    // its top (the navigation map's height for it), a square back from the edge.
    let into = (b.0 - a.0, b.1 - a.1);
    let her = middle(&r, (b.0 + 3 * into.0, b.1 + 3 * into.1), 1.0e5);
    let start = middle(&r, a, 1.0e5) - vec3(into.0 as f32, 0.0, into.1 as f32) * 100.0;
    let i = r.follower(start);
    let street = r.g.people[i].fig.pos.y;
    let best = r.wait_on(her, i, 30.0);
    println!("{mission}: edge {a:?} -> {b:?}, from {street} up to {}, he got to {best}", middle(&r, b, 1.0e5).y);
    let top = middle(&r, b, 1.0e5).y;
    assert!(best > top - 40.0, "{mission}: he never got up onto the roof ({best} of {top})");
    true
}

/// The first of a few missions with an edge only `action` gets up.
fn somewhere(action: u8, rise: f32) {
    let missions = ["police1", "police2", "police3", "Gangorder1", "park2", "Bankbomb1", "estate2", "bball2"];
    assert!(missions.iter().any(|m| follows_up_by(m, action, rise)), "no mission has such an edge");
}

#[test]
fn a_follower_climbs_a_ladder_after_her() {
    follows_up("police1", LADDER_UP);
}

#[test]
fn a_follower_pulls_himself_up_after_her() {
    somewhere(PULLUP, 100.0);
}

#[test]
fn a_follower_jumps_up_after_her() {
    somewhere(JUMPPULL, 100.0);
}
