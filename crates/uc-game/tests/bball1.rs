//! bball1 (The Purple Palace, unreleased): an earlier Gatecrasher on a map of its own. Georgie
//! sends Darci to Big Joe in the club, who phones Wild Bill; Bill at Posheeta's walks her to
//! the back alley and has her let in. It's won with all three gang bosses (points 73, 74 and
//! 75) down and one of them arrested. The walking is done by putting Darci where she has to
//! be and the fighting by knocking people down. Skipped without an install.

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
}

#[test]
fn big_joe_and_wild_bill_let_her_in_and_the_bosses_go_down() {
    let Some(mut r) = common::start("bball1") else { return };
    r.play(2.0, Controls::default());
    let gate = r.point(252);
    r.stand(gate);
    r.play(20.0, Controls::default());
    assert!(r.said("Talk to big joe"), "Georgie didn't send her to the club: {:?}", r.seen);

    r.talk_to(213, vec3(-110.0, 0.0, 0.0), 30.0);
    assert!(r.said("I'll call him up"), "Big Joe didn't talk: {:?}", r.seen);
    // He walks across the club to the phone, and Bill turns up at Posheeta's.
    assert!(r.until(60.0, |r| r.g.people.iter().any(|p| p.way == 188)), "Big Joe never got to the phone: at {:.0?}", r.g.people[r.who(213)].fig.pos);

    let diner = r.point(96);
    r.stand(diner);
    r.play(5.0, Controls::default());
    r.talk_to(188, vec3(130.0, 0.0, 0.0), 30.0);
    assert!(r.said("follow me"), "Bill didn't talk: {:?}", r.seen);
    assert!(r.until(90.0, |r| r.said("go ahead to let you through")), "Bill never got to the alley: at {:.0?}", r.g.people[r.who(188)].fig.pos);

    // One boss arrested, the other two knocked out.
    let boss = r.who(74);
    r.look_at(boss, 160.0);
    r.play(1.0, Controls::default());
    let at = r.g.people[boss].fig.pos;
    r.g.people[boss].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, Weapon::None);
    for _ in 0..60 {
        let me = r.player();
        if matches!(r.g.people[me].state, State::Move) {
            break;
        }
        r.play(DT * 1.01, Controls::default());
    }
    let body = r.g.people[boss].body_pos();
    r.stand(body + vec3(60.0, 0.0, 0.0));
    r.play(0.2, Controls { action: true, ..Default::default() });
    r.play(4.0, Controls::default());
    assert!(r.g.people[boss].arrested || matches!(r.g.people[boss].state, State::Cuffed), "the boss wasn't arrested: {:?}", r.g.people[boss].state);
    assert_eq!(r.g.outcome, None, "won with two bosses still up");
    for way in [73, 75] {
        let k = r.who(way);
        let at = r.g.people[k].fig.pos;
        r.g.people[k].take_hit(at + vec3(50.0, 0.0, 0.0), 10_000, ko::FINISHERS, false, false, Weapon::None);
    }
    assert!(r.until(20.0, |r| r.g.outcome.is_some()), "no ending: {:?}", r.seen);
    assert!(r.said("Your comin with me pal"), "{:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
