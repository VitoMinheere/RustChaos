//! Mr Bates the other way round (`mib_bates`, `uc_world::level::make_port_mission`): played as
//! one of the Men in Black, with Men in Black in the den, against Mr Bates the Wildcat.
//! Skipped without an install.

use glam::{vec3, Vec2};
use uc_game::{Controls, Outcome, State, Weapon};
use uc_world::character::person as ptype;

use crate::common;

const MIB: [u8; 3] = [ptype::MIB1, ptype::MIB2, ptype::MIB3];

/// The player and the den are Men in Black, Bates and his guards Wildcats, and the den's
/// words are theirs.
#[test]
fn the_characters_are_swapped() {
    let Some(mut r) = common::start("mib_bates") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    assert_eq!(r.g.people[me].kind, ptype::MIB1);
    assert!(r.g.people[me].has(Weapon::Ak47), "no AK-47");
    for way in [10, 11, 110] {
        let k = r.g.people[r.who(way)].kind;
        assert!(MIB.contains(&k), "point {way} is {k}, not one of the Men in Black");
        assert!(!r.g.people[r.who(way)].aggro(), "point {way} is after the player");
    }
    for way in [113, 114, 185, 193] {
        assert_eq!(r.g.people[r.who(way)].kind, ptype::THUG_RASTA, "point {way}");
    }
    assert!(r.level.mission.point_texts()[&111].contains("Razor"));
    // With the gun out, it isn't aimed at any of them.
    r.take_out(Weapon::Ak47);
    r.play(1.0, Controls::default());
    assert!(r.g.target.is_none_or(|t| !MIB.contains(&r.g.people[t].kind)), "aimed at one of the den");
    assert!(!r.level.mission.point_texts().values().any(|t| t.contains("Wild cats")));
}

/// Walking or holding run, a Man in Black goes no faster: he strides, faster than a walk.
#[test]
fn the_men_in_black_never_run() {
    let Some(mut r) = common::start("mib_bates") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    let mut walked = |run: bool| {
        let from = r.g.people[me].fig.pos;
        r.play(1.5, Controls { wish: Vec2::new(0.0, 1.0), run, ..Default::default() });
        let d = (r.g.people[me].fig.pos - from).length();
        r.play(0.5, Controls::default());
        d
    };
    let (walk, run) = (walked(false), walked(true));
    // A stride: well past a walk's 190 a second, well short of a run's 520.
    assert!((350.0..600.0).contains(&walk), "walked {walk} in a second and a half");
    assert!((run - walk).abs() < walk * 0.2, "walked {walk}, ran {run}");
}

/// The AK-47 fires a burst of three for each pull of the trigger, and never runs dry.
#[test]
fn the_ak_fires_bursts_of_three() {
    let Some(mut r) = common::start("mib_bates") else { return };
    r.play(1.0, Controls::default());
    r.take_out(Weapon::Ak47);
    for burst in 1..=12 {
        r.play(common::DT, Controls { punch: true, fire: true, ..Default::default() });
        r.play(1.0, Controls { fire: true, ..Default::default() });
        r.play(0.5, Controls::default());
        assert_eq!(r.g.record.shots, 3 * burst, "after {burst} pulls");
    }
}

/// Bates down, the den's beacon comes up, and back there it's won.
#[test]
fn bates_down_and_back_to_the_den() {
    let Some(mut r) = common::start("mib_bates") else { return };
    r.play(2.0, Controls::default());
    let i = r.who(113);
    let at = r.g.people[i].fig.pos;
    r.g.people[i].take_hit(at, 10_000, uc_game::person::ko::FINISHERS, false, false, Weapon::None);
    assert!(r.until(5.0, |r| r.said("return to the den")), "{:?}", r.seen);
    let den = r.point(116);
    r.stand(den);
    assert!(r.until(40.0, |r| r.g.outcome.is_some()));
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
    assert!(r.said("The Wildcats will remember"), "{:?}", r.seen);
}

/// Coming up to the corner of the road to the park, the Wildcats there go for the player.
#[test]
fn wildcats_on_the_way_go_for_the_player() {
    let Some(mut r) = common::start("mib_bates") else { return };
    r.play(1.0, Controls::default());
    r.stand(vec3(18976.0, 0.0, 10035.0 - 600.0));
    let me = r.player();
    assert!(
        r.until(10.0, |r| r.g.people.iter().any(|q| q.kind == ptype::THUG_RASTA && q.aggro() && q.foe == Some(me))),
        "no Wildcat came for the player"
    );
}

/// Near Bates: the alarm goes up, his guards come for the player, and he walks to his car
/// and gets in. Left alone, he's away in 25 seconds and the mission is lost.
#[test]
fn bates_runs_for_his_car_and_gets_away() {
    let Some(mut r) = common::start("mib_bates") else { return };
    r.play(1.0, Controls::default());
    let bates = r.who(113);
    let at = r.g.people[bates].fig.pos;
    r.stand(at + vec3(5.0 * 256.0, 0.0, 0.0));
    assert!(r.until(3.0, |r| r.said("making for his car")), "{:?}", r.seen);
    let me = r.player();
    assert!(r.until(5.0, |r| [114, 185, 193].iter().any(|&w| {
        let g = &r.g.people[r.who(w)];
        g.aggro() && g.foe == Some(me)
    })), "his guards didn't come for the player");
    // Out of the way, so as not to be in the fight.
    r.stand(vec3(25382.0, 0.0, 4911.0));
    assert!(r.until(60.0, |r| matches!(r.g.people[bates].state, State::InCar { .. })), "Bates is at {}", r.g.people[bates].fig.pos);
    assert!(r.until(3.0, |r| r.said("He's in his car")), "{:?}", r.seen);
    assert!(r.until(35.0, |r| r.g.outcome.is_some()));
    assert!(r.said("Bates got away"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}

/// Blowing up his car throws him out; then he can be finished, and back at the den it's won
/// with no clock running out.
#[test]
fn blowing_up_his_car_stops_him() {
    let Some(mut r) = common::start("mib_bates") else { return };
    r.play(1.0, Controls::default());
    let bates = r.who(113);
    let at = r.g.people[bates].fig.pos;
    r.stand(at + vec3(5.0 * 256.0, 0.0, 0.0));
    assert!(r.until(3.0, |r| r.said("making for his car")), "{:?}", r.seen);
    r.stand(vec3(25382.0, 0.0, 4911.0));
    assert!(r.until(60.0, |r| matches!(r.g.people[bates].state, State::InCar { .. })), "Bates never got in");
    let car = r.g.vehicles.iter().position(|v| v.driver == Some(bates)).unwrap();
    r.g.vehicles[car].health = 0;
    assert!(r.until(3.0, |r| !matches!(r.g.people[bates].state, State::InCar { .. })), "still in his car");
    // Thrown out, he's down: finished off once he's up again.
    for _ in 0..300 {
        if r.g.people[bates].is_dead() {
            break;
        }
        let at = r.g.people[bates].fig.pos;
        r.g.people[bates].take_hit(at, 10_000, uc_game::person::ko::FINISHERS, false, false, Weapon::None);
        r.play(common::DT, Controls::default());
    }
    assert!(r.until(5.0, |r| r.said("return to the den")), "{:?}", r.seen);
    // The player's still in the den: it's won, and the clock never runs out.
    assert!(r.until(40.0, |r| r.g.outcome.is_some()));
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
    assert!(!r.said("Bates got away"), "{:?}", r.seen);
}
