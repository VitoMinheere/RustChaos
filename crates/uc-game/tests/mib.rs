//! The Fallen (`MIB`): five clues, each dropped by somebody who has to be found first and then
//! taken down. `the_fallen_runs_to_a_win` walks the whole trail in order; the last two clues
//! are the long way round (the complaint about the rooftop, whose two suits both have to go
//! down before the yard where the Wildcats fall out comes to life). The older bot below rounds
//! up only what it can reach without following the trail, and pins the clue machinery.
//!
//! Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::Controls;

mod common;
use common::{Run, DT};

/// The points that create the five people carrying a clue.
const CLUE_HOLDERS: [usize; 5] = [39, 67, 316, 231, 40];

fn start() -> Option<Run> {
    common::start("MIB")
}

impl Run {
    /// Stands next to someone and looks at them, which is what both talking to them and
    /// `can_a_see_b` want.
    fn walk_up_to(&mut self, i: usize) {
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(130.0, 0.0, 0.0));
        let me = self.g.player.unwrap();
        let from = self.g.people[me].fig.pos;
        self.g.people[me].fig.angle =
            uc_world::character::angle_of(glam::vec2(at.x - from.x, at.z - from.z));
    }

    /// How many of the five clues are in hand.
    fn clues(&self) -> usize {
        // Each clue point sets `specific_item_held` once its item has been picked up; the
        // points watching for that are 367, 227, 366, 365 and 293.
        [367usize, 227, 366, 365, 293].iter().filter(|&&i| self.g.point_active(i)).count()
    }
}

/// Talking to Harney sends the camera up to the roof, looking back down at Darci with the MIB
/// in between, and leaves the bootlegger dead up there with the first clue.
#[test]
fn harney_sets_the_trail_going() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    let h = r.g.people.iter().position(|p| p.way == 13).unwrap();
    r.walk_up_to(h);
    let darci = r.g.people[r.g.player.unwrap()].fig.pos;
    let mut flown = false;
    for n in 0..(40.0 / DT) as usize {
        // He won't answer while he's still talking to Estevez, so keep asking.
        let ask = n % 90 == 0 && !r.g.point_active(21);
        r.g.update(DT, &Controls { action: ask, ..Default::default() }, &r.level);
        if r.g.cutscene.is_some() {
            // The fly-by's target is attached to the player's start point, so it follows her
            // rather than staring at that point across the map.
            let (_, look) = r.g.camera().unwrap();
            assert!((look - darci).length() < 300.0, "looking at {look:?}, Darci is at {darci:?}");
            flown = true;
        }
    }
    assert!(flown, "no fly-by after talking to Harney");
    // Albert Haines is made with `PCOM_AI_SUICIDE`: dead on arrival, which brings out the clue.
    let haines = r.g.people.iter().find(|p| p.way == 39).expect("Haines was never made");
    assert!(haines.is_dead());
    assert!(r.g.point_active(7), "the first clue never appeared");
}

/// The clues that can be reached are found, taken down and picked up.
#[test]
fn the_clues_within_reach_are_collected() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());

    // Harney starts the trail, and the woman looking for Pete is found by walking into her
    // corner of the map.
    let harney = r.g.people.iter().position(|p| p.way == 13);
    if let Some(h) = harney {
        r.walk_up_to(h);
        r.play(DT, Controls { action: true, ..Default::default() });
        r.play(40.0, Controls::default());
    }

    // Then just keep at it: put down whoever is carrying a clue, pick up what they drop, and
    // talk to anyone new who turns up.
    let mut rounds = 0;
    while r.clues() < 5 && r.g.outcome.is_none() && rounds < 40 {
        rounds += 1;
        // Anyone holding a clue goes down.
        for way in CLUE_HOLDERS {
            if let Some(i) = r.g.people.iter().position(|p| p.way == way && !p.is_dead() && !p.removed) {
                r.walk_up_to(i);
                let from = r.g.people[i].fig.pos;
                r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
            }
        }
        r.play(2.0, Controls::default());
        // Walk over everything lying about, not just the first thing: most of it is scenery
        // she can't pick up, and the clue is somewhere in among it.
        let lying: Vec<Vec3> = r.g.items.iter().map(|it| it.pos).collect();
        for at in lying {
            r.stand(at);
            r.play(0.4, Controls::default());
        }
        // Walk to the places the trail is gated on: the conversations that start when she
        // steps into a corner of the map, and the points that follow them.
        for gate in [65usize, 313, 309, 325, 41, 227, 315] {
            let at = r.point(gate);
            r.stand(at);
            r.play(1.0, Controls::default());
        }
        // And talk to whoever will talk.
        let talkable: Vec<usize> = (0..r.g.people.len())
            .filter(|&i| {
                let p = &r.g.people[i];
                !p.player && p.way != 0 && !p.removed && !p.is_dead() && !p.hostile()
            })
            .collect();
        for i in talkable.into_iter().take(6) {
            r.walk_up_to(i);
            r.play(DT, Controls { action: true, ..Default::default() });
            r.play(3.0, Controls::default());
        }
    }

    println!("clues {} of 5 after {rounds} rounds, outcome {:?}", r.clues(), r.g.outcome);
    assert!(r.said("5 CLUES"), "the chief never set the job: {:?}", r.seen);
    // Three is what the trail gives up without walking all of it. If this ever reads five the
    // mission is finishable and the note at the top of this file is out of date.
    assert!(r.clues() >= 3, "only {} clues, the trail has got worse", r.clues());
    assert_eq!(r.g.outcome, None, "it can't be won yet, so nothing should be claiming it is");
}

/// The ambulance at the scene (point 15) has a driver (16, `PCOM_AI_DRIVER`) who gets in and
/// waits, then drives off down the roads once Darci arrives (point 491 sets him wandering).
#[test]
fn the_ambulance_drives_off_when_darci_arrives() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    let amb = r.g.vehicles.iter().position(|v| v.way == 15).expect("no ambulance");
    let driver = r.g.people.iter().position(|p| p.way == 16).expect("no driver");
    assert!(matches!(r.g.people[driver].state, uc_game::State::InCar { .. }), "the driver didn't get in");
    let parked = r.g.vehicles[amb].pos;
    assert!(r.g.vehicles[amb].speed.abs() < 1.0, "it shouldn't leave before she gets there");
    // Darci walks into the scene (the cuboid at point 17).
    r.stand(r.point(17));
    r.play(8.0, Controls::default());
    let v = &r.g.vehicles[amb];
    assert!((v.pos - parked).length() > 2000.0, "only got from {parked:?} to {:?}", v.pos);
    assert!(!v.wrecked(), "it crashed on the way");
}

impl Run {
    /// Plays until point `i` is active (or `secs` pass), pressing action now and then to move
    /// conversations along.
    fn until_active(&mut self, i: usize, secs: f32) -> bool {
        for _ in 0..(secs as usize) {
            if self.g.point_active(i) {
                return true;
            }
            self.play(1.0, Controls::default());
        }
        self.g.point_active(i)
    }

    /// Plays until point `way` has made its person (or `secs` pass).
    fn wait_for(&mut self, way: usize, secs: f32) {
        for _ in 0..(secs as usize) {
            if self.g.people.iter().any(|p| p.way == way) {
                return;
            }
            self.play(1.0, Controls::default());
        }
        panic!("point {way} never made anyone: {:?}", self.seen);
    }

    /// Talks to the person point `way` made until point `then` goes active.
    fn talk_until(&mut self, way: usize, then: usize, secs: f32) {
        let i = self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}: {:?}", self.seen));
        for _ in 0..(secs / 3.0) as usize {
            if self.g.point_active(then) {
                return;
            }
            self.walk_up_to(i);
            self.play(DT, Controls { action: true, ..Default::default() });
            self.play(3.0, Controls::default());
        }
        assert!(self.g.point_active(then), "talking to {way} never set off {then}: {:?}", self.seen);
    }

    /// Puts down the person point `way` made and picks up what point `item` drops.
    fn take_clue(&mut self, way: usize, item: usize) {
        if let Some(i) = self.g.people.iter().position(|p| p.way == way && !p.is_dead() && !p.removed) {
            self.walk_up_to(i);
            self.play(DT, Controls::default());
            let from = self.g.people[i].fig.pos;
            self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
        }
        self.play(2.0, Controls::default());
        // Picked up already if she was standing near enough when it fell.
        if let Some(at) = self.g.items.iter().find(|it| it.way == item).map(|it| it.pos) {
            self.stand(at);
            self.play(1.0, Controls::default());
        }
    }
}

/// The whole trail: five clues, each from someone put down, and the Chief is satisfied.
#[test]
fn the_fallen_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());

    // 1. Harney's account, the fly-by, and the bootlegger dead on the roof with a napkin.
    r.talk_until(13, 21, 60.0);
    r.play(15.0, Controls::default());
    r.take_clue(39, 7);
    assert!(r.until_active(9, 40.0), "clue 1 not counted: {:?}", r.seen);

    // 2. The woman looking for Pete (her corner is cuboid 65), and Braddock at Scarper's Cove.
    r.stand(r.point(65));
    r.wait_for(67, 90.0);
    r.play(3.0, Controls::default());
    r.take_clue(67, 226);
    assert!(r.until_active(227, 20.0), "clue 2 not counted: {:?}", r.seen);

    // 3. The Wildcat with the paper about Claavus.
    r.play(3.0, Controls::default());
    r.take_clue(231, 289);
    assert!(r.until_active(291, 40.0), "clue 3 not counted: {:?}", r.seen);

    // 4. The man who called in the complaint, then the suit he sends Darci after.
    r.wait_for(48, 60.0);
    r.talk_until(48, 216, 60.0);
    assert!(r.until_active(41, 80.0), "his story never finished: {:?}", r.seen);
    r.play(3.0, Controls::default());
    r.take_clue(40, 292);
    assert!(r.until_active(294, 40.0), "clue 4 not counted: {:?}", r.seen);
    // His partner too: the yard (group 22) only comes to life once both suits are down.
    if let Some(i) = r.g.people.iter().position(|p| p.way == 46 && !p.is_dead()) {
        r.walk_up_to(i);
        r.play(DT, Controls::default());
        let from = r.g.people[i].fig.pos;
        r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
        // And away: a while after going down, the suits go up (Person.cpp:17536).
        r.stand(from + vec3(1500.0, 0.0, 0.0));
    }
    r.play(20.0, Controls::default());

    // 5. Into the yard (cuboid 301): the fly-by, the Wildcats' falling out, and the suit.
    r.stand(r.point(301));
    assert!(r.until_active(315, 90.0), "the falling-out never happened: {:?}", r.seen);
    r.play(3.0, Controls::default());
    r.take_clue(316, 344);

    for _ in 0..60 {
        if r.g.outcome.is_some() {
            break;
        }
        r.play(1.0, Controls::default());
    }
    println!("clues {} of 5, outcome {:?} at {:.1}s", r.clues(), r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(uc_game::Outcome::Won));
}
