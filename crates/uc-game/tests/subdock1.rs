//! subdock1 (Dock Extraction, unreleased), played through with the controls: Darci goes
//! from her start across the docks, climbs the fence into the corridor below the dock
//! operator's block, goes up its steps, onto the annex and up the ladder to his roof, and
//! takes him down there. The police chopper (point 41) comes in over building 66 at the south
//! end; she climbs the fence into the yard beside it, goes up its ladder to the roof, and
//! getting there (point 42) wins. One fence of each way up was unclimbable, so neither roof
//! could be got to (`uc_world::level::climb_into_the_docks`, #432). The beacons, on the
//! operator and then on the chopper, were added too. Nobody but the operator is left about
//! to start a fight. Skipped without an install.

use glam::{vec2, Vec2};
use uc_game::{Controls, Outcome, State};
use uc_world::mav;

mod common;
use common::{Run, DT};

/// A map square's middle.
fn mid(x: f32, z: f32) -> Vec2 {
    vec2(x, z) * 256.0
}

/// The dock operator's point.
const OPERATOR: usize = 37;

impl Run {
    fn at(&self) -> Vec2 {
        let p = self.g.people[self.player()].fig.pos;
        vec2(p.x, p.z)
    }

    fn y(&self) -> f32 {
        self.g.people[self.player()].fig.pos.y
    }

    /// Darci heads for `to` as a player would, a frame at a time, running: along the
    /// shortest way the navigation map has on foot (walking, and dropping off edges), cutting
    /// across where it can be walked straight. Fails the test if she doesn't get within
    /// `near` in `secs`.
    fn go(&mut self, to: Vec2, near: f32, secs: f32) {
        let nav = mav::Mav::new(&self.level);
        let sq = |v: Vec2| ((v.x / 256.0).clamp(0.0, 127.0) as i32, (v.y / 256.0).clamp(0.0, 127.0) as i32);
        let middle = |s: (i32, i32)| vec2(s.0 as f32 + 0.5, s.1 as f32 + 0.5) * 256.0;
        // Breadth first from where she is.
        let (from, goal) = (sq(self.at()), sq(to));
        let mut came = std::collections::HashMap::from([(from, from)]);
        let mut queue = std::collections::VecDeque::from([from]);
        while let Some(s) = queue.pop_front() {
            if s == goal {
                break;
            }
            for (i, &(dx, dz)) in mav::DIRS.iter().enumerate() {
                let t = (s.0 + dx, s.1 + dz);
                let ok = nav.opt(s.0, s.1, i) & (mav::caps(mav::GOTO) | mav::caps(mav::FALL_OFF)) != 0;
                if ok && (0..128).contains(&t.0) && (0..128).contains(&t.1) && !came.contains_key(&t) {
                    came.insert(t, s);
                    queue.push_back(t);
                }
            }
        }
        assert!(came.contains_key(&goal), "no way on foot from {from:?} to {goal:?}");
        let mut path = vec![goal];
        while *path.last().unwrap() != from {
            path.push(came[path.last().unwrap()]);
        }
        path.reverse();
        let mut k = 0;
        for _ in 0..(secs / DT) as usize {
            let at = self.at();
            if at.distance(to) < near {
                return;
            }
            let here = sq(at);
            if let Some(j) = path.iter().position(|&s| s == here) {
                k = k.max(j);
            }
            // The furthest square along it she can walk to straight.
            let ahead = (k + 1..path.len()).rev()
                .find(|&j| nav.can_walk((here.0 as u8, here.1 as u8), (path[j].0 as u8, path[j].1 as u8)))
                .unwrap_or((k + 1).min(path.len() - 1));
            let aim = if ahead == path.len() - 1 { to } else { middle(path[ahead]) };
            self.play(DT, Controls { wish: (aim - at).normalize_or_zero(), run: true, ..Default::default() });
        }
        panic!("Darci got stuck at {} on the way to {to}", self.g.people[self.player()].fig.pos);
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
            assert!(done, "Darci got stuck at {} on the way to ({x}, {z}) ({:?})",
                    self.g.people[self.player()].fig.pos, self.g.people[self.player()].state);
        }
    }

    /// Holds the way `dir` for `secs`, jumping first if `jump`.
    fn push(&mut self, dir: Vec2, jump: bool, secs: f32) {
        if jump {
            self.play(DT, Controls { wish: dir, jump: true, ..Default::default() });
        }
        self.play(secs, Controls { wish: dir, ..Default::default() });
    }

    /// Walks into the ladder ahead (the way `dir`) and up it, till she stands at `top`.
    fn climb(&mut self, dir: Vec2, top: f32) {
        for _ in 0..(15.0 / DT) as usize {
            if (self.y() - top).abs() < 16.0 && matches!(self.g.people[self.player()].state, State::Move) {
                return;
            }
            self.play(DT, Controls { wish: dir, ..Default::default() });
        }
        panic!("not up the ladder to {top}: at {} ({:?})", self.g.people[self.player()].fig.pos, self.g.people[self.player()].state);
    }

    fn beacons(&self) -> Vec<String> {
        self.g.beacons().into_iter().map(|(l, _)| l).collect()
    }
}

fn start() -> Option<Run> {
    let mut r = common::start("subdock1")?;
    r.play(1.0, Controls::default());
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| p.hostile() && p.way != OPERATOR) {
            p.removed = true;
        }
    });
    Some(r)
}

/// From her start up to the operator's roof.
fn up_to_the_operator(r: &mut Run) {
    assert!(r.said("Terminate dock operator"), "{:?}", r.seen);
    assert_eq!(r.beacons(), ["Dock operator"]);
    // Out of the fenced lane she starts in by the ladder at its end, onto the roof there
    // and off its far side.
    r.walk_by(&[(23.5, 33.4)]);
    r.climb(vec2(0.0, 1.0), 512.0);
    r.walk_by(&[(24.5, 35.5), (25.5, 40.0), (25.5, 44.3)]);
    r.push(vec2(0.0, 1.0), false, 2.0);
    r.play(2.0, Controls::default());
    // Across the docks to the street west of the corridor below his block.
    assert!(r.y() < 500.0, "still up on the roof: {}", r.g.people[r.player()].fig.pos);
    r.go(mid(104.5, 77.5), 48.0, 120.0);
    // Over its fence.
    r.push(vec2(1.0, 0.0), false, 8.0);
    assert!(r.at().x > 106.2 * 256.0, "didn't get over the fence: she's at {}", r.g.people[r.player()].fig.pos);
    // Up the steps at its north end, to the well in the annex.
    r.walk_by(&[(108.5, 76.5), (108.5, 71.5), (109.4, 69.5)]);
    assert!((r.y() - 256.0).abs() < 16.0, "not up the steps: at {}", r.y());
    // Up onto the annex.
    r.push(vec2(1.0, 0.0), true, 2.5);
    assert!((r.y() - 512.0).abs() < 16.0, "not up on the annex: at {}", r.g.people[r.player()].fig.pos);
    // Along to the ladder, and up it.
    r.walk_by(&[(113.5, 69.6)]);
    r.climb(vec2(0.0, -1.0), 1024.0);
    r.walk_by(&[(113.5, 66.5)]);
}

/// Darci gets up to the dock operator's roof.
#[test]
fn darci_gets_up_to_the_dock_operator() {
    let Some(mut r) = start() else { return };
    up_to_the_operator(&mut r);
}

/// The whole of it: the operator taken down, and Darci at the chopper, won.
#[test]
fn the_operator_down_and_darci_at_the_chopper_wins() {
    let Some(mut r) = start() else { return };
    up_to_the_operator(&mut r);
    assert!(!r.g.point_active(41), "chopper too soon");
    // Fists and feet till he's down.
    let op = r.who(OPERATOR);
    for k in 0..(60.0 / DT) as usize {
        if r.g.point_active(41) {
            break;
        }
        let q = r.g.people[op].fig.pos;
        let d = vec2(q.x, q.z) - r.at();
        let c = if d.length() > 90.0 {
            Controls { wish: d.normalize(), ..Default::default() }
        } else {
            Controls { wish: d.normalize() * 0.01, punch: k % 12 == 0, kick: k % 12 == 6, ..Default::default() }
        };
        r.play(DT, c);
    }
    assert!(r.until(10.0, |r| r.g.point_active(41)), "no chopper; the operator is {:?}", r.g.people[op].state);
    assert!(r.until(5.0, |r| r.said("Choppers comin in")), "{:?}", r.seen);
    assert_eq!(r.beacons(), ["Chopper"]);
    // Down off his roof the way she came: the ladder, the annex, the steps and the fence.
    r.walk_by(&[(109.5, 67.0), (113.5, 67.0), (113.5, 68.3)]);
    for _ in 0..(10.0 / DT) as usize {
        if r.y() < 520.0 && matches!(r.g.people[r.player()].state, State::Move) {
            break;
        }
        r.play(DT, Controls { wish: vec2(0.0, 1.0), ..Default::default() });
    }
    assert!((r.y() - 512.0).abs() < 16.0, "not down the ladder: {} {:?}", r.g.people[r.player()].fig.pos, r.g.people[r.player()].state);
    r.walk_by(&[(110.5, 69.5), (108.5, 69.8), (108.5, 75.6)]);
    r.push(vec2(-1.0, 0.0), false, 8.0);
    assert!(r.at().x < 105.8 * 256.0, "didn't get back over the fence: she's at {}", r.g.people[r.player()].fig.pos);
    // South to the yard by building 66.
    r.go(mid(80.0, 109.6), 48.0, 120.0);
    // Over its fence, to the ladder, and up it.
    r.push(vec2(1.0, 0.0), false, 8.0);
    assert!(r.at().x > 81.2 * 256.0, "didn't get over the fence: she's at {}", r.g.people[r.player()].fig.pos);
    r.walk_by(&[(83.4, 110.5)]);
    for _ in 0..(15.0 / DT) as usize {
        if r.y() > 1000.0 && matches!(r.g.people[r.player()].state, State::Move) || r.g.outcome.is_some() {
            break;
        }
        r.play(DT, Controls { wish: vec2(1.0, 0.0), ..Default::default() });
    }
    assert!(r.y() > 1000.0, "not up the ladder: {}", r.g.people[r.player()].fig.pos);
    assert_eq!(r.g.outcome, None, "won before she got to the rendezvous");
    r.walk_by(&[(84.5, 112.5), (81.5, 116.0)]);
    assert!(r.until(5.0, |r| r.g.outcome.is_some()), "not over: she's at {}", r.g.people[r.player()].fig.pos);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
