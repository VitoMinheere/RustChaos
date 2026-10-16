//! Beat Cop (#130): dispatch calls incidents in with a beacon and a time limit; taking the
//! culprits down handles one, letting the time run out misses it; the crime rate and score
//! follow; the shift ends in a win. Skipped without an install.

use uc_game::game::beat::Kind;
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Weapon};
use uc_world::Level;

mod common;
use common::DT;

fn start(seed: u32) -> Option<(Level, Game)> {
    let level = common::load("police1")?;
    let g = Game::new_beat(&level, seed, None, Some(20));
    Some((level, g))
}

/// Plays `secs`, Darci kept whole.
fn run(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / DT) as usize {
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &Controls::default(), level);
    }
}

/// Plays until the first call, at most `secs`.
fn first_call(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / 0.25) as usize {
        if !g.beat().unwrap().incidents.is_empty() {
            return;
        }
        run(g, level, 0.25);
    }
}

/// Knocks out everyone standing near an incident's spot who's after trouble (the guilty).
fn take_down_near(g: &mut Game, at: glam::Vec3) {
    for p in g.people.iter_mut().filter(|p| !p.player && p.guilty && !p.removed && (p.fig.pos - at).length() < 8.0 * 256.0) {
        let from = p.fig.pos + glam::vec3(50.0, 0.0, 0.0);
        p.take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }
}

#[test]
fn a_call_comes_in_with_a_beacon() {
    let Some((level, mut g)) = start(11) else { return };
    first_call(&mut g, &level, 10.0);
    let b = g.beat().unwrap();
    assert_eq!(b.incidents.len(), 1, "no call in the first ten seconds");
    let inc = &b.incidents[0];
    let label = inc.kind.label();
    assert!(g.beacons().iter().any(|(l, _)| l.starts_with(label)), "no beacon for the {label}");
    assert!(g.messages.iter().any(|m| m.text.starts_with("Despatch")), "dispatch said nothing");
    let me = g.player().unwrap().fig.pos;
    assert!((inc.at - me).length() > 6.0 * 256.0, "called in on top of her");
}

#[test]
fn the_same_seed_calls_the_same_thing() {
    let call = |seed| -> Option<(Kind, glam::Vec3)> {
        let (level, mut g) = start(seed)?;
        first_call(&mut g, &level, 10.0);
        let inc = &g.beat()?.incidents[0];
        Some((inc.kind, inc.at))
    };
    let Some(a) = call(11) else { return };
    assert_eq!(Some(a), call(11));
}

/// Taking the culprits down handles a call: points, and the crime rate comes down. A dumped
/// weapon's gun is handled by picking it up.
#[test]
fn handling_a_call_scores() {
    for seed in 1..40 {
        let Some((level, mut g)) = start(seed) else { return };
        first_call(&mut g, &level, 10.0);
        let inc = g.beat().unwrap().incidents[0].clone();
        if inc.kind == Kind::DumpedWeapon {
            continue;
        }
        let crime = g.crime_rate;
        take_down_near(&mut g, inc.at);
        run(&mut g, &level, 1.0);
        let b = g.beat().unwrap();
        assert_eq!((b.handled, b.missed), (1, 0), "the {} wasn't handled", inc.kind.label());
        assert!(b.score > 0);
        assert!(g.crime_rate < crime, "the crime rate should come down");
        return;
    }
    panic!("no call but dumped weapons");
}

/// Letting the time run out misses a call: the crime rate goes up.
#[test]
fn a_missed_call_costs() {
    let Some((level, mut g)) = start(11) else { return };
    first_call(&mut g, &level, 10.0);
    let crime = g.crime_rate;
    run(&mut g, &level, 160.0);
    let b = g.beat().unwrap();
    assert!(b.missed >= 1, "nothing was missed");
    assert!(g.crime_rate > crime, "the crime rate should go up: {crime} -> {}", g.crime_rate);
}

/// A mugger goes for his victim.
#[test]
fn a_mugger_goes_for_his_victim() {
    for seed in 1..60 {
        let Some((level, mut g)) = start(seed) else { return };
        first_call(&mut g, &level, 10.0);
        if g.beat().unwrap().incidents[0].kind != Kind::Mugging {
            continue;
        }
        let at = g.beat().unwrap().incidents[0].at;
        let victim = g.people.iter().position(|p| !p.player && !p.guilty && !p.removed && (p.fig.pos - at).length() < 300.0).unwrap();
        let before = g.people[victim].health;
        let mut hurt = false;
        for _ in 0..80 {
            run(&mut g, &level, 0.25);
            let v = &g.people[victim];
            if v.health < before || v.is_down() || v.knocked_out {
                hurt = true;
                break;
            }
        }
        assert!(hurt, "the mugger never laid a finger on his victim");
        return;
    }
    panic!("no mugging called in");
}

/// The shift ends in a win.
#[test]
fn the_shift_ends() {
    let Some((level, mut g)) = start(11) else { return };
    g.beat_mut().unwrap().shift = Some(2.0);
    run(&mut g, &level, 3.0);
    assert_eq!(g.outcome, Some(Outcome::Won));
}

/// The shift lengths: five minutes runs out after five minutes; an open-ended one doesn't.
#[test]
fn shift_lengths() {
    let Some(level) = common::load("police1") else { return };
    let g = Game::new_beat(&level, 11, None, Some(5));
    assert_eq!(g.beat().unwrap().shift, Some(300.0));
    let mut g = Game::new_beat(&level, 11, None, None);
    assert_eq!(g.beat().unwrap().shift, None);
    g.beat_mut().unwrap().on = 3600.0;
    run(&mut g, &level, 3.0);
    assert_eq!(g.outcome, None, "an open-ended shift ended on the clock");
    assert!(g.beat().unwrap().on > 3600.0);
}
