//! Balloons (#108, balloon.cpp): the civilians Gun Hunt and Grim Gardens give one to hold it
//! on a string above their hand as they go, let go of it when knocked down, and it floats
//! away out of sight. Darci can jump and catch a loose one. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::game::balloons::{POINTS, POINT_DIST};
use uc_game::person::ko;
use uc_game::{Controls, Weapon};

mod common;
use common::Run;

/// The string's full length, world units.
const STRING: f32 = (POINTS - 1) as f32 * POINT_DIST as f32 / 256.0;

impl Run {
    /// The balloon person `i` holds.
    fn balloon_of(&self, i: usize) -> Option<usize> {
        self.g.balloons.iter().position(|b| b.holder == Some(i))
    }

    /// Plays until event point `way` has made its person, and gives their index.
    fn made(&mut self, way: usize) -> usize {
        assert!(self.until(120.0, |r| r.g.people.iter().any(|p| p.way == way)), "point {way} made nobody");
        self.who(way)
    }

    /// Person `i`'s balloon: how far it is above their hand, and how far across from it.
    fn over_hand(&self, i: usize) -> (f32, f32) {
        let b = self.balloon_of(i).expect("let go of the balloon");
        let pts = self.g.balloons[b].points();
        let d = pts[POINTS - 1] - pts[0];
        (d.y, d.with_y(0.0).length())
    }

    /// Plays `secs`, checking every turn that person `i`'s balloon is tied to their hand, is
    /// above it and no further from it than its string. How far they went.
    fn follows(&mut self, i: usize, secs: f32) -> f32 {
        let start = self.g.people[i].fig.pos;
        let mut went = 0.0f32;
        for _ in 0..(secs / common::DT).max(1.0) as usize {
            self.play(common::DT, Controls::default());
            let b = self.balloon_of(i).expect("let go of the balloon");
            let pts = self.g.balloons[b].points();
            let (hand, top) = (pts[0], pts[POINTS - 1]);
            let p = &self.g.people[i];
            assert!((hand - p.fig.pos).length() < 200.0, "the string isn't at the hand: {hand} for {}", p.fig.pos);
            // (The string's kept to length by `QDIST3`, which comes out short on a slant.)
            assert!((top - hand).length() < STRING * 1.2, "the balloon got away: {top}, hand {hand}");
            assert!(top.y > hand.y, "the balloon is below the hand: {top}, hand {hand}");
            went = went.max((p.fig.pos - start).length());
        }
        went
    }
}

/// Every turn the balloon stays near and above `i`'s hand as they go about, trailing a
/// little when they walk (it only rises at 80 units a second), and when they stand still it
/// floats straight up over their hand.
fn holds_and_follows(r: &mut Run, i: usize, way: usize) {
    assert!(r.balloon_of(i).is_some(), "no balloon");
    r.follows(i, 3.0);
    // Pulled back the way they came, quickly, and it comes along. (Somewhere open: up
    // against a wall, the balloon's pushed off it, and leans.)
    let at = r.g.people[i].fig.pos;
    let back = (r.point(way) - at).with_y(0.0);
    for k in 0..60 {
        let to = at + back.normalize_or_zero() * (k as f32 * 8.0).min(back.length());
        let y = r.level.ground.surface(&r.level.map, to.x, to.z, to.y + 64.0);
        r.g.people[i].fig.pos = vec3(to.x, y, to.z);
        r.follows(i, common::DT);
    }
    let went = r.follows(i, 4.0);
    println!("went {went:.0} on their own; balloon (up, across) from the hand {:?}", r.over_hand(i));
    // Standing still, it rises over the hand.
    r.g.cheats.freeze = true;
    r.follows(i, 4.0);
    let (up, across) = r.over_hand(i);
    println!("standing: (up, across) {up:.0} {across:.0}");
    assert!(up > STRING * 0.9 && across < STRING * 0.3, "not straight up: {up} up, {across} across");
    r.g.cheats.freeze = false;
}

#[test]
fn gun_hunts_balloon_follows_its_holder_and_floats_off_when_he_is_knocked_down() {
    let Some(mut r) = common::start("police3") else { return };
    let i = r.made(152);
    holds_and_follows(&mut r, i, 152);

    // Knocked down: he lets go and it floats away up, string and all.
    let b = r.balloon_of(i).unwrap();
    let from = r.g.people[i].fig.pos + vec3(0.0, 0.0, 100.0);
    r.g.people[i].take_hit(from, 1000, ko::KNOCKDOWN, false, false, Weapon::None);
    r.play(0.2, Controls::default());
    assert!(r.g.balloons[b].holder.is_none(), "still held");
    let low = r.g.balloons[b].points()[POINTS - 1].y;
    r.play(5.0, Controls::default());
    let pts = r.g.balloons[b].points();
    assert!(pts[POINTS - 1].y > low + 200.0, "not floating up: {} from {low}", pts[POINTS - 1].y);
    assert!(pts[0].y < pts[POINTS - 1].y, "the string isn't hanging below it");
    // Up out of sight, it's gone.
    assert!(r.until(90.0, |r| r.g.balloons[b].gone), "never gone: {:?}", r.g.balloons[b].points());
}

#[test]
fn darci_jumps_and_catches_a_loose_balloon() {
    let Some(mut r) = common::start("police3") else { return };
    let i = r.made(152);
    r.play(2.0, Controls::default());
    let b = r.balloon_of(i).unwrap();
    // Let go: Darci's under it at once, and jumps.
    r.g.people[i].removed = true;
    r.play(common::DT * 3.0, Controls::default());
    assert!(r.g.balloons[b].holder.is_none());
    let end = r.g.balloons[b].points()[0];
    r.stand(Vec3::new(end.x + 20.0, end.y - 150.0, end.z));
    r.play(common::DT, Controls { jump: true, ..Default::default() });
    let me = r.player();
    assert!(r.until(2.0, |r| r.g.balloons[b].holder == Some(me)), "she didn't catch it");
    // And it's hers to hold, once she's landed.
    r.play(1.5, Controls::default());
    r.follows(me, 4.0);
}

#[test]
fn grim_gardens_balloon_follows_its_holder() {
    let Some(mut r) = common::start("botanicc") else { return };
    let i = r.made(299);
    holds_and_follows(&mut r, i, 299);
}
