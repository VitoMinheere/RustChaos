//! What the music does (#235, `context_music`, fallen/Source/Controls.cpp:1372): driving and
//! crawling have tunes of their own, the training missions and Day Of Reckoning one for all
//! of them, a countdown under 30 seconds the timer tune, and losing with the player still
//! standing the chaos tune. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Mood, Sfx, State, Theme};

mod common;
use common::DT;

#[test]
fn the_training_missions_and_the_finale_have_their_own_tunes() {
    for (mission, theme) in [
        ("FTutor1", Theme::CombatTraining),
        ("fight1", Theme::CombatTraining),
        ("Assault1", Theme::AssaultTraining),
        ("testdrive1a", Theme::DrivingTraining),
        ("Finale1", Theme::Reckoning),
    ] {
        let Some(mut r) = common::start(mission) else { return };
        r.play(1.0, Controls::default());
        assert_eq!(r.g.mood(), Mood::Theme(theme), "{mission}");
    }
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    assert_eq!(r.g.mood(), Mood::Calm, "RTA has no tune of its own");
}

#[test]
fn crawling_and_driving_have_tunes() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    r.g.people[me].state = State::Crouch { phase: 2 };
    assert_eq!(r.g.mood(), Mood::Crawl);
    // Nitro Car's car bomb (point 2), got into from its door.
    let Some(mut r) = common::start("Bankbomb1") else { return };
    r.play(1.0, Controls::default());
    let v = r.g.vehicles.iter().position(|v| v.way == 2).expect("the car bomb");
    let door = r.g.vehicles[v].door();
    r.stand(vec3(door.x, r.g.vehicles[v].pos.y, door.y));
    r.play(DT, Controls { action: true, ..Default::default() });
    let got_in = r.until(5.0, |r| r.g.player_vehicle().is_some());
    assert!(got_in, "Mako should get into the car");
    assert_eq!(r.g.mood(), Mood::Drive);
}

#[test]
fn a_countdown_under_thirty_seconds_plays_the_timer_tune() {
    // Day Of Reckoning's eight-minute countdown, over its own theme.
    let Some(mut r) = common::start("Finale1") else { return };
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| p.hostile()) {
            p.removed = true;
        }
    });
    // The countdown waits for the 33-second talk with Roper at the start (#258).
    r.play(33.0 + 445.0, Controls::default());
    assert_eq!(r.g.mood(), Mood::Theme(Theme::Reckoning), "the countdown has {:?} left", r.g.countdown());
    r.play(10.0, Controls::default());
    assert_eq!(r.g.mood(), Mood::Timer, "the countdown has {:?} left", r.g.countdown());
}

#[test]
fn losing_on_your_feet_is_chaos() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    r.g.take_sounds();
    r.g.end_mission(false);
    assert_eq!(r.g.mood(), Mood::Chaos);
    assert!(r.g.take_sounds().iter().any(|(s, _)| *s == Sfx::Chaos), "the chaos sting should play");
}
