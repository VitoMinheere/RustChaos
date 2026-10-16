//! Estate of Emergency (`estate2`), played through to the arrest of Bane. The fighting and
//! the travelling are done by cheating: people are knocked out or moved where the script
//! wants them. Checks the mission logic, not the player's skill. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Outcome, State, Weapon};
use uc_world::character::person as ptype;

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    common::start("estate2")
}

impl Run {
    fn alive(&self, way: usize) -> Option<usize> {
        self.g.people.iter().position(|p| p.way == way && !p.removed)
    }

    /// Puts person `i` on the floor under `at` (indoors, not on the roof).
    fn put_person(&mut self, i: usize, at: Vec3) {
        let y = self.level.ground.surface(&self.level.map, at.x, at.z, at.y + 64.0);
        self.g.people[i].fig.pos = vec3(at.x, y, at.z);
    }

    fn put(&mut self, at: Vec3) {
        let p = self.player();
        self.put_person(p, at);
    }

    fn kill(&mut self, way: usize) {
        let i = self.alive(way).unwrap_or_else(|| panic!("nobody from point {way}"));
        let from = self.g.people[i].fig.pos + vec3(50.0, 0.0, 0.0);
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }

    /// Knocks person `i` down and has the player arrest them, from beside them.
    fn arrest(&mut self, i: usize) -> bool {
        let at = self.g.people[i].fig.pos;
        self.g.people[i].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, Weapon::None);
        self.play(0.5, Controls::default());
        let body = self.g.people[i].body_pos();
        let Some(beside) = (0..8)
            .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
            .find(|p| (self.level.ground.surface(&self.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
        else {
            return false;
        };
        self.put(beside);
        self.play(0.2, Controls { action: true, ..Default::default() });
        self.play(4.0, Controls::default());
        self.g.people[i].arrested || matches!(self.g.people[i].state, State::Cuffed)
    }

    fn playing_as(&self, kind: u8) -> bool {
        self.g.people[self.player()].kind == kind
    }
}

/// The way to Bane: the alarms off, his taunt, Roper saving Darci from his three Wildcats,
/// then Bane at his helicopter (points 172 and 173) with his pilot.
fn to_bane(r: &mut Run) {
    r.play(1.0, Controls::default());

    // 1. The override switch in the pool house (point 88) turns the security off.
    r.put(r.point(88));
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(1.0, Controls::default());
    assert!(r.said("Security systems off!"), "the switch did nothing: {:?}", r.seen);

    // 2. Inside the house, near Bane (within 512 of point 124): he taunts her, and his
    //    Wildcats turn up (points 47, 230 and 293).
    r.put(r.point(124));
    assert!(r.until(60.0, |r| r.alive(47).is_some()), "Bane never talked: {:?}", r.seen);
    assert!(r.said("almost a thousand years"), "{:?}", r.seen);

    // 3. Once Wildcat 47 is close (point 290), the player becomes Roper.
    if let Some(w) = r.alive(47) {
        let at = r.point(290);
        r.put_person(w, at);
    }
    assert!(r.until(10.0, |r| r.playing_as(ptype::ROPER)), "never handed over to Roper");

    // 4. Roper kills the three and stands in the room (cuboid 229): they talk, and the
    //    player is Darci again.
    for way in [47, 230, 293] {
        r.kill(way);
    }
    r.put(r.point(229));
    assert!(r.until(60.0, |r| r.said("MISSILES")), "Darci and Roper never talked: {:?}", r.seen);
    assert!(r.until(30.0, |r| r.playing_as(ptype::DARCI)), "never handed back to Darci");
    assert!(r.until(30.0, |r| r.alive(172).is_some()), "Bane never turned up at the helipad");
}

/// Darci reaches the helipad (cuboid 218), which stops Bane getting away, and arrests him:
/// won two seconds later.
#[test]
fn estate_of_emergency_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    to_bane(&mut r);
    r.put(r.point(218));
    assert!(r.until(30.0, |r| r.said("I'll give you THREE")), "{:?}", r.seen);
    let bane = r.alive(172).unwrap();
    assert!(r.arrest(bane), "couldn't arrest Bane: {:?}", r.g.people[bane].state);
    r.play(3.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(!r.said("Bane got away"), "{:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// He's wanted alive: killing him loses it.
#[test]
fn estate_of_emergency_needs_bane_alive() {
    let Some(mut r) = start() else { return };
    to_bane(&mut r);
    r.kill(172);
    r.play(3.0, Controls::default());
    assert!(r.said("We needed him ALIVE"), "nobody noticed: {:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}

/// Seen him but never reached the helipad: three minutes after his talk with the pilot,
/// he's gone.
#[test]
fn estate_of_emergency_lets_bane_get_away() {
    let Some(mut r) = start() else { return };
    to_bane(&mut r);
    assert!(r.until(30.0, |r| r.said("I'll give you THREE")), "{:?}", r.seen);
    // Somewhere on the same level with a clear view of him, outside the helipad's cuboid.
    let bane = r.g.people[r.alive(172).unwrap()].fig.pos;
    let (pad, half) = {
        let e = r.level.mission.event_points.iter().find(|e| e.index == 218).unwrap();
        (glam::vec2(e.x as f32, e.z as f32), glam::vec2((e.radius & 0xffff) as f32, (e.radius >> 16) as f32))
    };
    let head = vec3(0.0, 96.0, 0.0);
    let spot = (0..32)
        .flat_map(|k| [700.0, 1000.0, 1300.0].map(|d| bane + vec3((k as f32 * 0.196).sin(), 0.0, (k as f32 * 0.196).cos()) * d))
        .find(|p| {
            let off = (glam::vec2(p.x, p.z) - pad).abs();
            (off.x > half.x + 64.0 || off.y > half.y + 64.0)
                && (r.level.ground.surface(&r.level.map, p.x, p.z, bane.y + 64.0) - bane.y).abs() < 16.0
                && r.level.ground.line_of_sight(*p + head, bane + head)
        })
        .expect("nowhere in sight of Bane");
    r.put(spot);
    // A step towards him to face him.
    let to = glam::vec2(bane.x - spot.x, bane.z - spot.z).normalize();
    r.play(0.2, Controls { wish: to, ..Default::default() });
    r.play(185.0, Controls::default());
    assert!(r.said("Bane got away"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}
