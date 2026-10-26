//! Gatecrasher (`bball2`): Darci goes to the Wildcat leaders' meet at the ballpark and brings
//! one of them in. Checks the script end to end, with the walking done by putting people where
//! they have to be and the fighting by knocking them down, as the other mission tests do.
//! Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    common::start("bball2")
}

/// One of the three leaders at the meet is knocked down and arrested: that is counter 2, and
/// the win follows. (Killing all three is the way to lose.)
#[test]
fn gatecrasher_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let leader = r.who(73);
    r.look_at(leader, 160.0);
    r.play(1.0, Controls::default());
    let at = r.g.people[leader].fig.pos;
    r.g.people[leader].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, uc_game::Weapon::None);
    // Let her get over any blow she took first: a flinch eats the button.
    for _ in 0..60 {
        let me = r.g.player.unwrap();
        if matches!(r.g.people[me].state, uc_game::State::Move) {
            break;
        }
        r.play(DT * 1.01, Controls::default());
    }
    let body = r.g.people[leader].body_pos();
    let beside = (0..8)
        .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
        .find(|p| (r.level.ground.surface(&r.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
        .expect("somewhere to stand");
    r.stand(beside);
    r.play(0.2, Controls { action: true, ..Default::default() });
    r.play(4.0, Controls::default());
    let l = &r.g.people[leader];
    assert!(l.arrested || matches!(l.state, uc_game::State::Cuffed), "the leader wasn't arrested: {:?}", l.state);

    r.play(10.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("I GOT HIM"), "the arrest didn't count: {:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

impl Run {
    /// Talks to the person made by `way` from `off` of them, facing them, then plays `secs`.
    fn talk_to(&mut self, way: usize, off: glam::Vec3, secs: f32) {
        let at = self.g.people[self.who(way)].fig.pos;
        self.stand(at + off);
        let me = self.player();
        self.g.people[me].fig.angle = uc_world::character::angle_of(glam::vec2(-off.x, -off.z));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }
}

/// The club and Posheeta's have floors 256 up, under the warehouses' roofs (at 2304 and
/// 768): everyone made on them stays on them rather than dropping through to the street
/// level under them, as does Darci walking in.
#[test]
fn people_stand_on_the_club_and_diner_floors() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    // Club-goers at the bar, two of them dancing, and Bill's neighbours at the diner.
    for way in [212, 214, 216, 217, 181, 184] {
        let p = &r.g.people[r.who(way)];
        assert!((p.fig.pos.y - 256.0).abs() < 8.0, "point {way} is at {:.0?}, not on the floor", p.fig.pos);
    }
    let me = r.player();
    r.stand(vec3(23750.0, 256.0, 9866.0));
    r.play(1.0, Controls { wish: glam::vec2(0.0, -1.0), ..Default::default() });
    assert!((r.g.people[me].fig.pos.y - 256.0).abs() < 8.0, "Darci fell through the club's floor: {:.0?}", r.g.people[me].fig.pos);
}

/// Georgie sends Darci to the club; the doorman there goes off to phone Bill (walking across
/// the club to the phone, which has to be within 160 of it, `enemy_radius`), and Bill turns
/// up at Posheeta's. Once he's had his say he walks out of the diner and up the street, and
/// on getting near the alley he's let Darci in.
#[test]
fn the_doorman_phones_wild_bill_and_bill_calls_georgie() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let gate = r.point(252);
    r.stand(gate);
    r.play(20.0, Controls::default());
    assert!(r.said("he's in the club"), "Georgie didn't send her to the club: {:?}", r.seen);

    r.talk_to(213, vec3(-110.0, 0.0, 0.0), 30.0);
    assert!(r.said("I'll go phone Bill"), "the doorman didn't talk: {:?}", r.seen);
    for _ in 0..60 {
        if r.g.people.iter().any(|p| p.way == 188) {
            break;
        }
        r.play(1.0, Controls::default());
    }
    assert!(r.g.people.iter().any(|p| p.way == 188), "the doorman never got to the phone: at {:.0?}", r.g.people[r.who(213)].fig.pos);

    let diner = r.point(367);
    r.stand(diner);
    r.play(5.0, Controls::default());
    r.talk_to(188, vec3(130.0, 0.0, 0.0), 40.0);
    assert!(r.said("make the call to Georgie"), "Bill didn't talk: {:?}", r.seen);
    for _ in 0..60 {
        if r.said("I gave the guys the go-ahead") {
            break;
        }
        r.play(1.0, Controls::default());
    }
    assert!(r.said("I gave the guys the go-ahead"), "Bill never got near the alley: at {:.0?}", r.g.people[r.who(188)].fig.pos);
}

/// Darci stood about next to a dancer in the club for three seconds dances along, the same
/// dance, until she's asked to move.
#[test]
fn darci_dances_along_with_the_club_goers() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    let dancer = r.who(434);
    let uc_game::State::Pose { anim } = r.g.people[dancer].state else { panic!("point 434 isn't dancing: {:?}", r.g.people[dancer].state) };
    // Stood about from now: a step first, as she's been stood about since the start.
    r.play(DT, Controls { wish: glam::vec2(1.0, 0.0), ..Default::default() });
    r.look_at(dancer, 120.0);
    let me = r.player();
    r.play(2.0, Controls::default());
    assert!(matches!(r.g.people[me].state, uc_game::State::Move), "dancing too soon");
    r.play(1.5, Controls::default());
    assert_eq!(r.g.people[me].state, uc_game::State::Pose { anim }, "Darci didn't join in");
    r.play(DT, Controls { wish: glam::vec2(1.0, 0.0), ..Default::default() });
    assert!(matches!(r.g.people[me].state, uc_game::State::Move), "still dancing: {:?}", r.g.people[me].state);
}
