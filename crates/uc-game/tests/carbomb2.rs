//! carbomb2, a copy of Auto-Destruct (`carbomb1`) with its debrief's points deleted, which
//! `finish_unfinished` puts back: both endings, played through as `carbomb1.rs` plays Car
//! Bomb's. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Outcome, Search};

mod common;
use common::{Run, DT};

/// Both bomb cars in and both bombers cuffed, as in Car Bomb.
fn to_the_search() -> Option<Run> {
    let mut r = common::start("carbomb2")?;
    r.play(2.0, Controls::default());
    r.deliver_car(6, 8);
    assert!(r.g.point_active(8), "the first car didn't count: {:?}", r.seen);
    r.play(2.0, Controls::default());
    r.deliver_car(82, 92);
    assert!(r.g.point_active(92), "the second car didn't count: {:?}", r.seen);
    r.play(3.0, Controls::default());
    r.cuff(147);
    r.cuff(148);
    assert!(r.said("You got the bombers"), "{:?}", r.seen);
    Some(r)
}

impl Run {
    /// Back at base until it's over: whether it was won.
    fn back_to_base(&mut self) -> bool {
        self.stand(self.point(169));
        for _ in 0..90 {
            if self.g.outcome.is_some() {
                break;
            }
            self.play(1.0, Controls::default());
        }
        self.g.outcome == Some(Outcome::Won)
    }
}

/// The tape found at the hideout and brought back: the Sarge's debrief about it.
#[test]
fn the_tape_brought_back() {
    let Some(mut r) = to_the_search() else { return };
    r.play(6.0, Controls::default());
    let ob = r.g.hidden.iter().find(|(_, it)| it.way == 181).expect("the video, stashed").0;
    let o = r.level.map.objects[ob];
    let prop = vec3(o.x as f32, o.y as f32, o.z as f32);
    r.stand(prop + vec3(110.0, 0.0, 0.0));
    let me = r.g.player.unwrap();
    r.g.people[me].face_towards(prop);
    r.play(DT, Controls { action: true, sprint: true, ..Default::default() });
    r.play(Search::FOUND + 0.2, Controls { sprint: true, ..Default::default() });
    let video = r.g.items.iter().find(|it| it.way == 181).expect("the search turned up nothing").pos;
    r.play(Search::DONE, Controls::default());
    r.stand(video);
    r.play(1.0, Controls::default());
    assert!(r.said("UCNN video"), "the video was never lifted: {:?}", r.seen);
    assert!(r.back_to_base(), "{:?}", r.seen);
    assert!(r.said("I found this old Video tape"), "not the tape's debrief: {:?}", r.seen);
}

/// Back without it once despatch has lost patience: the other debrief.
#[test]
fn back_with_nothing() {
    let Some(mut r) = to_the_search() else { return };
    assert!(r.until(60.0, |r| r.said("what the hell you up to")), "despatch never called: {:?}", r.seen);
    assert!(r.back_to_base(), "{:?}", r.seen);
    assert!(r.said("place was full of dust"), "not the empty-handed debrief: {:?}", r.seen);
    assert!(!r.said("I found this old Video tape"), "{:?}", r.seen);
}
