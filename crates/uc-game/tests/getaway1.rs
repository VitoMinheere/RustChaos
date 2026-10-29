//! Mr X's Getaway (`Getaway1`, a prototype on the TV centre's map): the police contact
//! (point 162) sends Darci after Mr X (131), who walks from the TV station to his van (164)
//! and drives off in it. His beacon takes over from the contact's once they've talked
//! (`uc_world::level::finish_unfinished`). Taking him down wins; his van getting away loses,
//! which needs him up the ramp to it, round the square too steep to walk up
//! (`MAV_precalculate`). Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Outcome, State, Weapon};

mod common;
use common::{Run, DT};

impl Run {
    /// Talks to the person made by `way` from `off` of them, facing them, then plays `secs`.
    fn talk_to(&mut self, way: usize, off: Vec3, secs: f32) {
        let at = self.g.people[self.who(way)].fig.pos;
        self.stand(at + off);
        let me = self.player();
        self.g.people[me].fig.angle = uc_world::character::angle_of(vec2(-off.x, -off.z));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }

    fn beacons(&self) -> Vec<String> {
        self.g.beacons().into_iter().map(|(l, _)| l).collect()
    }

    /// Meets the contact: then it's Mr X's beacon, not his.
    fn meet_the_contact(&mut self) {
        self.play(2.0, Controls::default());
        assert_eq!(self.beacons(), ["Police contact"]);
        self.talk_to(162, vec3(110.0, 0.0, 0.0), 15.0);
        assert!(self.said("intercept him before he gets away"), "{:?}", self.seen);
        assert_eq!(self.beacons(), ["Mr X"]);
    }
}

#[test]
fn mr_xs_beacon_leads_to_him() {
    let Some(mut r) = common::start("Getaway1") else { return };
    r.meet_the_contact();
    // Some way along his walk, the beacon is still on him.
    r.play(30.0, Controls::default());
    let x = r.who(131);
    let (_, at) = r.g.beacons()[0].clone();
    assert!(at.distance(r.g.people[x].fig.pos) < 1.0, "the beacon is at {at}, Mr X at {}", r.g.people[x].fig.pos);
    r.stand(at + vec3(150.0, 0.0, 0.0));
    r.play(DT, Controls::default());
    let me = r.player();
    let from = r.g.people[me].fig.pos;
    r.g.people[x].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    assert!(r.until(10.0, |r| r.g.outcome.is_some()));
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}

#[test]
fn mr_x_gets_away_if_left_alone() {
    let Some(mut r) = common::start("Getaway1") else { return };
    r.meet_the_contact();
    assert!(r.until(400.0, |r| r.g.outcome.is_some()), "Mr X is at {}", r.g.people[r.who(131)].fig.pos);
    let x = r.who(131);
    assert!(matches!(r.g.people[x].state, State::InCar { .. }), "Mr X isn't in his van: {:?}", r.g.people[x].state);
    assert!(r.said("Looks like he got away!"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}
