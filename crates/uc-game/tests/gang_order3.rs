//! Gang order3 (Getaway Van, unreleased): an early Mako mission. Mako brings a Wildcat along,
//! picks up what the one guarding the den hands him, and shakes down the man in the store for
//! the files from the last drop-off. With them in hand the getaway van turns up, and getting
//! it to the hideout in the north-west wins half a minute later. The walking and driving are
//! done by putting Mako and the van where they have to be. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::{Controls, Outcome, State};

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
}

#[test]
fn mako_gets_the_files_to_the_getaway_van() {
    let Some(mut r) = common::start("Gang order3") else { return };
    r.play(2.0, Controls::default());
    let me = r.player();
    // Mako is a Rasta Wildcat (`PERSON_THUG_RASTA`).
    assert_eq!(r.g.people[me].kind, uc_world::character::person::THUG_RASTA);

    r.talk_to(12, vec3(110.0, 0.0, 0.0), 10.0);
    assert!(r.said("lets trsh that place"), "{:?}", r.seen);
    r.talk_to(111, vec3(110.0, 0.0, 0.0), 15.0);
    assert!(r.said("you might find em useful"), "{:?}", r.seen);

    // The store's floor is 256 up, under its rooftop: he's on it, not under it.
    let man = r.who(134);
    assert!((r.g.people[man].fig.pos.y - 256.0).abs() < 8.0, "the man in the store is at {}", r.g.people[man].fig.pos);
    r.talk_to(134, vec3(110.0, 0.0, 0.0), 30.0);
    assert!(r.said("here they are"), "{:?}", r.seen);
    let file = r.g.items.iter().find(|i| i.way == 136).map(|i| i.pos).expect("no files");
    assert!((file.y - 256.0).abs() < 8.0, "the files are at {file}, not on the store's floor");
    r.stand(file);
    assert!(r.until(5.0, |r| r.said("now get to the van")), "{:?}", r.seen);

    let van = r.g.vehicles.iter().position(|v| v.way == 170).expect("no van");
    for side in [-200.0, 200.0, -120.0, 120.0] {
        let at = r.g.vehicles[van].to_world(side, 20.0);
        r.stand(vec3(at.x, r.g.vehicles[van].pos.y, at.y));
        r.play(DT, Controls { action: true, ..Default::default() });
        r.play(3.0, Controls::default());
        if matches!(r.g.people[me].state, State::InCar { .. }) {
            break;
        }
    }
    assert!(matches!(r.g.people[me].state, State::InCar { vehicle } if vehicle == van), "not in the van: {:?}", r.g.people[me].state);
    let hideout = r.point(137);
    let y = r.g.vehicles[van].pos.y;
    r.g.vehicles[van].pos = vec3(hideout.x, y, hideout.z);
    assert!(r.until(5.0, |r| r.said("you should be safe here")), "{:?}", r.seen);
    assert!(r.until(40.0, |r| r.g.outcome.is_some()));
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
