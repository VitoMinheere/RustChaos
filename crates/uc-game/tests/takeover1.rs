//! Takeover1 (Bank Sector Takeover, unreleased), played through with the controls: Darci
//! drops off the roof she starts on, goes down the street to the sector gate and works its
//! switch, and is in; up the lift to the second hostage and back down with him, up the
//! stairs and across the moving platform to the first, and out with both through the keycard
//! gate and the front gates to the extraction point, which wins. The gates are fences of the
//! map made doors (`uc_world::level::open_the_bank_sector`, #431), and the hostages find their
//! way through them once they're open (#439). Nobody else is left about to start a fight.
//! Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::{person, Controls, Outcome};
use uc_world::mav::{self, CAPS_DARCI};

mod common;
use common::{Run, DT};

/// A map square's middle.
fn mid(x: f32, z: f32) -> Vec2 {
    vec2(x, z) * 256.0
}

/// The hostages' points.
const WORKERS: [usize; 2] = [62, 63];

impl Run {
    fn at(&self) -> Vec2 {
        let p = self.g.people[self.player()].fig.pos;
        vec2(p.x, p.z)
    }

    /// Darci heads for `to` as a player would, a frame at a time, running: straight at it
    /// if she can walk there, else along the way the navigation map has (off edges too).
    /// Fails the test if she doesn't get within `near` in `secs`.
    fn go(&mut self, to: Vec2, near: f32, secs: f32) {
        let mut seed = 1u32;
        for _ in 0..(secs / DT) as usize {
            let at = self.at();
            if at.distance(to) < near {
                return;
            }
            let sq = |v: Vec2| ((v.x / 256.0).clamp(0.0, 127.0) as u8, (v.y / 256.0).clamp(0.0, 127.0) as u8);
            let middle = |s: (u8, u8)| vec2(s.0 as f32 + 0.5, s.1 as f32 + 0.5) * 256.0;
            let (a, b) = (sq(at), sq(to));
            let (mut aim, mut jump) = (to, false);
            if a != b && !self.g.nav().can_walk(a, b) {
                let (act, _) = self.g.nav().route(a, b, CAPS_DARCI, &mut || uc_game::rand(&mut seed));
                let (dx, dz) = mav::DIRS[act.dir as usize & 3];
                let way = vec2(dx as f32, dz as f32);
                aim = match act.action {
                    mav::GOTO if act.dest != a => middle(act.dest),
                    mav::GOTO => middle(act.first),
                    // Up onto a ledge or over a gap: from the edge, jumping that way.
                    mav::PULLUP | mav::JUMP | mav::JUMPPULL | mav::JUMPPULL2 => {
                        let edge = middle(a) + way * 100.0;
                        jump = at.distance(edge) < 40.0;
                        if jump { at + way * 100.0 } else { edge }
                    }
                    _ => middle(((a.0 as i32 + dx) as u8, (a.1 as i32 + dz) as u8)),
                };
            }
            self.play(DT, Controls { wish: (aim - at).normalize_or_zero(), run: !jump, jump, ..Default::default() });
        }
        let p = self.g.people[self.player()].fig.pos;
        let s = ((p.x / 256.0) as u8, (p.z / 256.0) as u8);
        let act = self.g.nav().route(s, ((to.x / 256.0) as u8, (to.y / 256.0) as u8), CAPS_DARCI, &mut || uc_game::rand(&mut seed));
        panic!("Darci got stuck at {p} on the way to {to} ({act:?}, {:?})", self.g.people[self.player()].state);
    }

    /// Walks straight at each of `pts` (squares) in turn.
    fn walk_by(&mut self, pts: &[(f32, f32)]) {
        for &(x, z) in pts {
            let to = mid(x, z);
            let mut done = false;
            for _ in 0..(8.0 / DT) as usize {
                let d = to - self.at();
                if d.length() < 40.0 || self.g.outcome.is_some() {
                    done = true;
                    break;
                }
                self.play(DT, Controls { wish: d.normalize(), ..Default::default() });
            }
            assert!(done, "Darci got stuck at {} on the way to ({x}, {z}) ({:?}; {:?})",
                    self.g.people[self.player()].fig.pos, self.g.people[self.player()].state, self.g.outcome);
        }
    }

    /// Presses action where she stands, then waits a moment.
    fn work(&mut self) {
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(1.0, Controls::default());
    }

    /// Waits for the moving platform `way` to stop at the bottom (or the top) of its run.
    fn wait_for_lift(&mut self, way: usize, top: bool) -> Vec2 {
        let k = self.g.platforms.iter().position(|p| p.way == way).unwrap();
        let there = |r: &Run| if top { r.g.platforms[k].pos.y > 900.0 } else { r.g.platforms[k].pos.y < 60.0 };
        // It may be on its way there already: wait for it to leave, then to come.
        for want in [false, true] {
            for _ in 0..(40.0 / 0.1) as usize {
                if there(self) == want {
                    break;
                }
                self.play(0.1, Controls::default());
            }
        }
        assert!(there(self), "lift {way} never came");
        let p = self.g.platforms[k].pos;
        vec2(p.x, p.z)
    }

    /// Waits for whoever follows her to be near her: within `near` of her, on the same floor.
    fn wait_for(&mut self, who: &[usize], near: f32, secs: f32) {
        let me = self.player();
        let ok = |r: &Run| who.iter().all(|&w| {
            let d = r.g.people[w].fig.pos - r.g.people[me].fig.pos;
            vec2(d.x, d.z).length() < near && d.y.abs() < 64.0
        });
        assert!(self.until(secs, ok), "they never caught up: {:?}, she's at {}",
                who.iter().map(|&w| self.g.people[w].fig.pos).collect::<Vec<_>>(), self.g.people[me].fig.pos);
    }

    /// Goes up to hostage `w` from his side (he's to see her, close by), until he follows.
    fn pick_up(&mut self, w: usize) {
        let q = self.g.people[w].fig.pos;
        let f = self.g.people[w].facing_dir();
        let side = vec2(-f.y, f.x) * 160.0;
        let spot = [vec2(q.x, q.z) + side, vec2(q.x, q.z) - side]
            .into_iter()
            .find(|s| (self.level.ground.surface(&self.level.map, s.x, s.y, q.y + 64.0) - q.y).abs() < 16.0)
            .expect("nowhere to stand by him");
        for _ in 0..(6.0 / DT) as usize {
            let d = spot - self.at();
            if d.length() < 30.0 {
                break;
            }
            self.play(DT, Controls { wish: d.normalize(), ..Default::default() });
        }
        assert!(self.until(3.0, |r| r.g.people[w].mv == person::MOVE_FOLLOW), "{w} never followed; she's at {}",
                self.g.people[self.player()].fig.pos);
    }

    fn beacons(&self) -> Vec<String> {
        self.g.beacons().into_iter().map(|(l, _)| l).collect()
    }
}

fn start() -> Option<Run> {
    let mut r = common::start("Takeover1")?;
    r.play(1.0, Controls::default());
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| p.hostile()) {
            p.removed = true;
        }
    });
    Some(r)
}

/// The sector gate's switch: in front of it on the street, facing it.
const SWITCH: (f32, f32) = (76.3, 47.5);

/// From her start to the sector gate, shut and then opened.
fn into_the_sector(r: &mut Run) {
    assert_eq!(r.beacons(), ["Worker 1", "Worker 2", "Extraction point", "Sector gate"]);
    // Off the west side of the roof onto the street, and down it.
    r.go(mid(69.5, 17.0), 64.0, 30.0);
    assert!(r.g.people[r.player()].fig.pos.y < 32.0, "still up on the roof");
    r.go(mid(74.5, 47.5), 64.0, 60.0);
    // The gate's shut: she walks into it and stops.
    for _ in 0..(3.0 / DT) as usize {
        r.play(DT, Controls { wish: vec2(1.0, 0.0), ..Default::default() });
    }
    assert!(r.at().x < 77.0 * 256.0, "through the gate before it's open");
    r.walk_by(&[SWITCH]);
    r.work();
    assert!(r.until(5.0, |r| !r.beacons().iter().any(|l| l == "Sector gate")), "{:?}", r.beacons());
    r.walk_by(&[(79.5, 47.5)]);
}

/// Darci walks into the bank sector, through the gate she opens.
#[test]
fn darci_gets_into_the_sector_through_its_gate() {
    let Some(mut r) = start() else { return };
    into_the_sector(&mut r);
    assert!(r.at().x > 79.0 * 256.0);
}

/// The whole of it, won.
#[test]
fn darci_brings_both_hostages_out_and_wins() {
    let Some(mut r) = start() else { return };
    into_the_sector(&mut r);
    let [one, two] = WORKERS.map(|w| r.who(w));

    // The second hostage, up the lift beside his building (point 60).
    r.go(mid(86.5, 80.8), 64.0, 60.0);
    r.walk_by(&[(89.6, 81.0)]);
    let lift = r.wait_for_lift(60, false);
    r.walk_by(&[(lift.x / 256.0, lift.y / 256.0)]);
    r.wait_for_lift(60, true);
    r.walk_by(&[(90.5, 78.8)]);
    assert!((r.g.people[r.player()].fig.pos.y - 1024.0).abs() < 16.0, "not up on his roof");
    r.pick_up(two);
    // And down again: the lift for her, and he finds his own way.
    r.walk_by(&[(90.5, 79.6)]);
    let lift = r.wait_for_lift(60, true);
    r.walk_by(&[(lift.x / 256.0, lift.y / 256.0)]);
    r.wait_for_lift(60, false);
    r.walk_by(&[(88.5, 81.0), (86.5, 78.0)]);
    r.wait_for(&[two], 900.0, 30.0);

    // The first, up the stairs at the back of the block west of his (building 57), and
    // across the moving platform (point 56) by jumping on and off it at its north end.
    r.go(mid(50.5, 104.5), 64.0, 120.0);
    r.walk_by(&[(50.5, 110.0), (51.5, 109.6), (56.5, 109.6), (56.5, 112.5), (58.5, 112.5), (58.5, 107.0), (62.0, 107.0)]);
    assert!((r.g.people[r.player()].fig.pos.y - 512.0).abs() < 16.0, "not up the stairs");
    let k = r.g.platforms.iter().position(|p| p.way == 56).unwrap();
    // It pauses at that end: wait for it to come back there.
    let north = |r: &Run| r.g.platforms[k].pos.z < 27400.0;
    assert!(r.until(30.0, |r| !north(r)) && r.until(30.0, north), "the platform never came");
    let (mut on, mut off) = (false, false);
    for _ in 0..(5.0 / DT) as usize {
        let x = r.at().x / 256.0;
        let mut c = Controls { wish: vec2(1.0, 0.0), run: true, ..Default::default() };
        if !on && x > 62.8 {
            (c.jump, on) = (true, true);
        } else if on && !off && x > 65.6 {
            (c.jump, off) = (true, true);
        }
        r.play(DT, c);
    }
    let me = r.player();
    assert!(r.at().x > 67.0 * 256.0 && (r.g.people[me].fig.pos.y - 512.0).abs() < 16.0,
            "didn't get across: she's at {}", r.g.people[me].fig.pos);
    // Along the roof, and up the steps at its east end (building 61) to his.
    r.go(mid(76.5, 110.5), 48.0, 30.0);
    r.walk_by(&[(77.5, 111.5), (78.5, 111.5), (78.5, 117.3), (79.8, 117.4)]);
    assert!((r.g.people[me].fig.pos.y - 1024.0).abs() < 16.0, "not up on his roof");
    r.pick_up(one);

    // Out with both of them: down off the roofs, to the keycard gate.
    r.go(mid(43.8, 60.4), 48.0, 180.0);
    r.wait_for(&[one, two], 900.0, 60.0);
    r.walk_by(&[(43.8, 60.05)]);
    r.work();
    // Through it into the yard behind the front gates, and along to the front gates' switch.
    r.walk_by(&[(40.5, 59.5), (40.5, 58.5), (42.5, 58.5), (42.5, 56.5)]);
    r.wait_for(&[one, two], 900.0, 60.0);
    r.go(mid(39.8, 41.5), 48.0, 60.0);
    r.walk_by(&[(39.8, 41.07)]);
    r.work();
    r.wait_for(&[one, two], 900.0, 60.0);
    // Out through them, to the extraction point just outside, with the hostages behind.
    r.walk_by(&[(37.0, 40.5), (37.0, 39.0), (36.8, 37.6)]);
    assert!(r.until(30.0, |r| r.said("Worker 1 rescued") && r.said("Worker 2 rescued")), "{:?}; hostages at {:?}",
            r.seen, [one, two].map(|w| r.g.people[w].fig.pos));
    r.play(0.2, Controls::default());
    assert!(r.until(5.0, |r| r.g.outcome.is_some()), "not over");
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// A hostage killed, the mission can't be won: it's lost.
#[test]
fn a_hostage_killed_loses_it() {
    let Some(mut r) = start() else { return };
    let w = r.who(WORKERS[1]);
    let at = r.g.people[w].fig.pos;
    r.g.people[w].take_hit(at + vec3(50.0, 0.0, 0.0), 10_000, person::ko::FINISHERS, false, false, uc_game::Weapon::None);
    assert!(r.until(10.0, |r| r.g.outcome.is_some()), "not over");
    assert!(r.said("A hostage has been killed"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}
