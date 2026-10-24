//! Followers ride along (#250, `PCOM_AI_STATE_HITCH`): when whoever they follow gets into a
//! car they climb in too, and out again after them; and when they crouch or crawl, so do the
//! followers (`PERSON_SPEED_CRAWL`). Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::MOVE_FOLLOW;
use uc_game::{Controls, State};

mod common;
use common::{Run, DT};

impl Run {
    /// One of the level's people made to follow the player, two squares behind her.
    fn follower(&mut self) -> usize {
        let pi = self.player();
        let me = self.g.people[pi].fig.pos;
        let i = self.g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && !matches!(q.state, State::InCar { .. })).expect("someone");
        let at = me + vec3(-300.0, 0.0, 0.0);
        let y = self.level.ground.surface(&self.level.map, at.x, at.z, at.y + 64.0);
        let q = &mut self.g.people[i];
        (q.mv, q.follow, q.fake, q.ai, q.bent) = (MOVE_FOLLOW, 0, false, 1, 0);
        q.fig.pos = vec3(at.x, y, at.z);
        i
    }
}

#[test]
fn a_follower_rides_along() {
    let Some(mut r) = common::start("Bankbomb1") else { return };
    r.play(1.0, Controls::default());
    // Mako into the car bomb (point 2), with a follower two squares behind him.
    let v = r.g.vehicles.iter().position(|v| v.way == 2).expect("the car bomb");
    let door = r.g.vehicles[v].door();
    r.stand(vec3(door.x, r.g.vehicles[v].pos.y, door.y));
    let i = r.follower();
    r.play(DT, Controls { action: true, ..Default::default() });
    assert!(r.until(5.0, |r| r.g.player_vehicle().is_some()), "Mako should get in");
    assert!(r.until(10.0, |r| matches!(r.g.people[i].state, State::InCar { vehicle } if vehicle == v)), "the follower never got in: {:?}", r.g.people[i].state);
    // Out again after him.
    r.play(DT, Controls { action: true, ..Default::default() });
    assert!(r.until(5.0, |r| r.g.player_vehicle().is_none()), "Mako should get out");
    r.play(0.5, Controls::default());
    assert!(!matches!(r.g.people[i].state, State::InCar { .. }), "the follower stayed in the car");
}

#[test]
fn a_follower_crawls_when_she_does() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let i = r.follower();
    // Darci crawls off, away from them.
    let crawl = Controls { crouch: true, wish: vec2(1.0, 0.0), ..Default::default() };
    let mut crouched = false;
    for _ in 0..(4.0 / DT) as usize {
        r.play(DT, crawl);
        crouched |= matches!(r.g.people[i].state, State::Crouch { .. });
    }
    assert!(crouched, "the follower never got down: {:?}", r.g.people[i].state);
}
