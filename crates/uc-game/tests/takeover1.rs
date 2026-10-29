//! Takeover1 (Bank Sector Takeover, unreleased): the two hostages wait where they are until
//! Darci comes up to them where they can see her (`PCOM_MOVE_FOLLOW_ON_SEE`), then follow
//! her; brought to the extraction point, each is "rescued", and with both the mission is won
//! (an ending added here: the original's never had one). Beacons, also added, show each
//! hostage until they're rescued, and the extraction point. Darci is moved from one to the
//! next rather than walked across the city. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::{person, Controls, Outcome, State};

mod common;
use common::Run;

/// The points that make the two hostages, and where the script counts them rescued.
const WORKERS: [usize; 2] = [62, 63];
const EXTRACTION: Vec3 = Vec3::new(9455.0, 0.0, 9628.0);

impl Run {
    /// Stands Darci about `dist` in front of person `i`, on the floor they stand on (off to
    /// one side if straight ahead is off their roof or up against a taller one), facing them.
    fn before(&mut self, i: usize, dist: f32) {
        let q = &self.g.people[i];
        let (from, f) = (q.fig.pos, q.facing_dir());
        // Up close they see well to either side (`can_a_see_b`).
        let spots = [0.0f32, 0.5, -0.5, 1.0, -1.0, 1.5, -1.5, 2.0, -2.0].into_iter().flat_map(|a| [1.0, 0.6].map(|k| (a, k)));
        let at = spots
            .map(|(a, k)| {
                let d = vec2(f.x * a.cos() - f.y * a.sin(), f.x * a.sin() + f.y * a.cos()) * dist * k;
                let (x, z) = (from.x + d.x, from.z + d.y);
                vec3(x, self.level.ground.surface(&self.level.map, x, z, from.y + 64.0), z)
            })
            .find(|p| (p.y - from.y).abs() < 32.0)
            .expect("nowhere to stand in front of them");
        let me = self.player();
        self.g.people[me].fig.pos = at;
        self.g.people[me].fig.angle = uc_world::character::angle_of(vec2(from.x - at.x, from.z - at.z));
        self.g.people[me].state = State::Move;
    }
}

#[test]
fn the_hostages_follow_darci_out_and_she_wins() {
    let Some(mut r) = common::start("Takeover1") else { return };
    r.play(1.0, Controls::default());
    // Nobody else to get in the way: the gang would make it a fight.
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| p.hostile()) {
            p.removed = true;
        }
    });
    let beacons = |r: &Run| r.g.beacons().into_iter().map(|(l, _)| l).collect::<Vec<_>>();
    assert_eq!(beacons(&r), ["Worker 1", "Worker 2", "Extraction point"]);
    let workers = WORKERS.map(|w| r.who(w));
    // The hostages' beacons are on them.
    for (k, &w) in workers.iter().enumerate() {
        let at = r.g.beacons()[k].1;
        assert!(at.distance(r.g.people[w].fig.pos) < 1.0, "worker {}'s beacon is at {at}", k + 1);
    }
    for &w in &workers {
        assert_eq!(r.g.people[w].mv, person::MOVE_FOLLOW_ON_SEE);
        // Far off, they stay put.
        r.before(w, 1500.0);
        r.play(1.0, Controls::default());
        assert_eq!(r.g.people[w].mv, person::MOVE_FOLLOW_ON_SEE, "followed from far off");
        // Close, in sight: they follow.
        r.before(w, 180.0);
        assert!(r.until(3.0, |r| r.g.people[w].mv == person::MOVE_FOLLOW), "worker {w} never followed");
    }
    // At the extraction point, with the hostages brought along close behind her.
    let me = r.player();
    let y = r.level.ground.surface(&r.level.map, EXTRACTION.x, EXTRACTION.z, 64.0);
    r.g.people[me].fig.pos = vec3(EXTRACTION.x, y, EXTRACTION.z);
    for (k, &w) in workers.iter().enumerate() {
        let at = vec2(EXTRACTION.x + 700.0, EXTRACTION.z + 200.0 * k as f32);
        let wy = r.level.ground.surface(&r.level.map, at.x, at.y, 64.0);
        r.g.people[w].fig.pos = vec3(at.x, wy, at.y);
    }
    assert!(r.until(20.0, |r| r.said("Worker 1 rescued") && r.said("Worker 2 rescued")), "{:?}", r.seen);
    r.play(0.2, Controls::default());
    assert_eq!(beacons(&r), ["Extraction point"]);
    assert!(r.until(5.0, |r| r.g.outcome.is_some()), "not over");
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// A hostage follows Darci down off his roof and across the sector to the keycard gate
/// (point 190): the roofs of `jumper1`, an older map's, are on the navigation map.
#[test]
fn a_hostage_follows_darci_down_off_his_roof() {
    let Some(mut r) = common::start("Takeover1") else { return };
    r.play(1.0, Controls::default());
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| p.hostile()) {
            p.removed = true;
        }
    });
    let w = r.who(WORKERS[0]);
    r.before(w, 180.0);
    assert!(r.until(3.0, |r| r.g.people[w].mv == person::MOVE_FOLLOW), "never followed");
    let gate = r.point(190);
    let me = r.player();
    for _ in 0..240 {
        r.stand(gate);
        r.play(0.5, Controls::default());
        if r.g.people[w].fig.pos.distance(r.g.people[me].fig.pos) < 300.0 {
            break;
        }
    }
    let at = r.g.people[w].fig.pos;
    assert!(at.distance(r.g.people[me].fig.pos) < 300.0 && at.y < 64.0, "the hostage is at {at}");
}

