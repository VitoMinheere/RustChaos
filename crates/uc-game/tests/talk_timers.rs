//! Mission timers stop while a conversation is being said (#258, eway.cpp:2569).

mod common;

use glam::Vec3;
use uc_formats::ucm::trigger;
use uc_game::script::{PersonState, Query, ThingState};
use uc_game::Script;

/// Nobody about; only whether someone's talking.
struct Quiet(bool);

impl Query for Quiet {
    fn player_pos(&self) -> Option<Vec3> {
        None
    }
    fn person(&self, _way: usize) -> Option<PersonState> {
        None
    }
    fn thing(&self, _way: usize) -> Option<ThingState> {
        None
    }
    fn in_vehicle(&self, _person: usize, _vehicle: usize) -> bool {
        false
    }
    fn player_grabbing(&self) -> bool {
        false
    }
    fn player_used(&self) -> bool {
        false
    }
    fn talking(&self) -> bool {
        self.0
    }
}

#[test]
fn the_countdown_waits_for_a_conversation() {
    // Assault Course's two minutes on the clock, running from the start.
    let Some(level) = common::load("Assault1") else { return };
    let mut m = level.mission.clone();
    let ep = m.event_points.iter_mut().find(|e| e.trigger == trigger::VISIBLE_COUNTDOWN).expect("the course's countdown");
    ep.ep_ref = 0;
    let mut s = Script::new(&m);
    for _ in 0..20 {
        s.update(0.05, &Quiet(false));
    }
    let before = s.countdown().expect("the clock is running");
    for _ in 0..100 {
        s.update(0.05, &Quiet(true));
    }
    assert_eq!(s.countdown(), Some(before), "the clock ran on through a conversation");
    for _ in 0..20 {
        s.update(0.05, &Quiet(false));
    }
    assert!(s.countdown().unwrap() < before - 0.9, "the clock didn't start again");
}
