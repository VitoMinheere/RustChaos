//! Beat Cop (#130): dispatch calls incidents in with a beacon and a time limit; taking the
//! culprits down handles one, letting the time run out misses it; the crime rate and score
//! follow; the shift ends in a win. Skipped without an install.

use uc_game::game::beat::{Kind, Shift};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Weapon};
use uc_world::Level;

use crate::common;
use common::DT;

fn start(seed: u32) -> Option<(Level, Game)> {
    let level = common::load("police1")?;
    let g = Game::new_beat(&level, seed, None, Shift::Minutes(20));
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
        if !matches!(inc.kind, Kind::Mugging | Kind::ArmedSuspect | Kind::GangFight | Kind::WantedMan) {
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
    panic!("no call with people to take down");
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
    let g = Game::new_beat(&level, 11, None, Shift::Minutes(5));
    assert_eq!(g.beat().unwrap().shift, Some(300.0));
    let mut g = Game::new_beat(&level, 11, None, Shift::Open);
    assert_eq!(g.beat().unwrap().shift, None);
    g.beat_mut().unwrap().on = 3600.0;
    run(&mut g, &level, 3.0);
    assert_eq!(g.outcome, None, "an open-ended shift ended on the clock");
    assert!(g.beat().unwrap().on > 3600.0);
}

/// An unlimited shift (#520) has no clock and goes on with the crime rate at 100, dispatch
/// saying so once; an open-ended one is lost there. Ending it from the pause menu is a win.
#[test]
fn an_unlimited_shift_goes_on() {
    let Some(level) = common::load("police1") else { return };
    let mut g = Game::new_beat(&level, 11, None, Shift::Unlimited);
    assert_eq!(g.beat().unwrap().shift, None);
    g.crime_rate = 100;
    run(&mut g, &level, 3.0);
    assert_eq!(g.outcome, None, "an unlimited shift ended at 100");
    let said = |g: &Game| g.messages.iter().filter(|m| m.text.contains("out of control")).count();
    assert_eq!(said(&g), 1, "dispatch should say the city's out of control, once");
    g.end_shift();
    assert_eq!(g.outcome, Some(Outcome::Won));
    assert!(g.beat().unwrap().ended);

    let mut g = Game::new_beat(&level, 11, None, Shift::Open);
    g.crime_rate = 100;
    run(&mut g, &level, 1.0);
    assert!(matches!(g.outcome, Some(Outcome::Lost(_))), "an open-ended shift goes on at 100");
}

/// Calls `kind` in, with nothing else on.
fn call(seed: u32, kind: Kind) -> Option<(Level, Game)> {
    let (level, mut g) = start(seed)?;
    run(&mut g, &level, 1.0);
    assert!(g.call_in_now(kind, &level), "nowhere for a {}", kind.label());
    Some((level, g))
}

/// The person an incident's about.
fn culprit(g: &Game) -> usize {
    let at = g.beat().unwrap().incidents[0].at;
    g.people.iter().position(|p| !p.player && p.guilty && !p.removed && (p.fig.pos - at).length() < 64.0).expect("nobody there")
}

/// A drunk driver weaves down the road; wreck his car and he's thrown out, and taking him
/// in handles it.
#[test]
fn a_drunk_driver_is_stopped() {
    let Some((level, mut g)) = call(11, Kind::DrunkDriver) else { return };
    let d = culprit(&g);
    let uc_game::State::InCar { vehicle } = g.people[d].state else { panic!("not in a car") };
    let from = g.vehicles[vehicle].pos;
    run(&mut g, &level, 4.0);
    assert!((g.vehicles[vehicle].pos - from).length() > 256.0, "the car never went anywhere");
    g.vehicles[vehicle].health = 0;
    run(&mut g, &level, 1.0);
    assert!(!matches!(g.people[d].state, uc_game::State::InCar { .. }), "still in the wreck");
    assert!(g.people[d].is_down(), "not knocked down by the crash");
    // Cuffed where he lies.
    g.people[d].arrested = true;
    run(&mut g, &level, 0.5);
    let b = g.beat().unwrap();
    assert_eq!((b.handled, b.missed), (1, 0));
}

/// A drunk driver pulls over for a siren close by, gets out with his hands up, and with
/// her beside him that's handled.
#[test]
fn a_drunk_driver_pulls_over_for_a_siren() {
    let Some((level, mut g)) = call(11, Kind::DrunkDriver) else { return };
    let d = culprit(&g);
    let uc_game::State::InCar { vehicle } = g.people[d].state else { panic!("not in a car") };
    run(&mut g, &level, 3.0);
    assert!(g.vehicles[vehicle].speed.abs() > 60.0, "not driving");
    // A police car with its siren going, just behind him.
    let at = g.vehicles[vehicle].pos;
    let mut cop = uc_game::Vehicle::new(0, 3, at - glam::vec3(0.0, 0.0, 600.0), 0.0, uc_game::vehicle::Key::None);
    cop.siren = true;
    g.vehicles.push(cop);
    let mut out = false;
    for _ in 0..40 {
        run(&mut g, &level, 0.25);
        if matches!(g.people[d].state, uc_game::State::HandsUp { .. }) {
            out = true;
            break;
        }
    }
    assert!(out, "never pulled over: {:?} at {}", g.people[d].state, g.vehicles[vehicle].speed);
    let me = g.player.unwrap();
    g.people[me].fig.pos = g.people[d].fig.pos + glam::vec3(200.0, 0.0, 0.0);
    run(&mut g, &level, 0.5);
    let b = g.beat().unwrap();
    assert_eq!((b.handled, b.missed), (1, 0));
}

/// A drunk driver stops for her standing in the road, late, and gets out with her at the
/// door.
#[test]
fn a_drunk_driver_stops_for_her_in_the_road() {
    let Some((level, mut g)) = call(11, Kind::DrunkDriver) else { return };
    let d = culprit(&g);
    let uc_game::State::InCar { vehicle } = g.people[d].state else { panic!("not in a car") };
    run(&mut g, &level, 3.0);
    let me = g.player.unwrap();
    let mut out = false;
    for _ in 0..80 {
        // A little ahead of the car, whichever way it's pointing now.
        let v = &g.vehicles[vehicle];
        let ahead = glam::vec3(v.yaw.sin(), 0.0, v.yaw.cos()) * 180.0;
        let to = v.pos + ahead;
        g.people[me].fig.pos = glam::vec3(to.x, level.ground.surface(&level.map, to.x, to.z, v.pos.y + 64.0), to.z);
        run(&mut g, &level, 0.1);
        if !matches!(g.people[d].state, uc_game::State::InCar { .. }) {
            out = true;
            break;
        }
    }
    assert!(out, "never stopped for her: {}", g.vehicles[vehicle].speed);
    assert_eq!(g.people[me].health, g.people[me].max_health, "ran her down");
}

/// The getaway car boxed in or smashed up: he bails out to fight.
#[test]
fn a_getaway_driver_bails_out_of_a_smashed_car() {
    let Some((level, mut g)) = call(11, Kind::Getaway) else { return };
    let d = culprit(&g);
    let uc_game::State::InCar { vehicle } = g.people[d].state else { panic!("not in a car") };
    run(&mut g, &level, 6.0);
    g.vehicles[vehicle].health = 100;
    let mut out = false;
    for _ in 0..40 {
        run(&mut g, &level, 0.25);
        if !matches!(g.people[d].state, uc_game::State::InCar { .. }) {
            out = true;
            break;
        }
    }
    assert!(out, "still driving the wreck");
}

/// A getaway driver frightened out of his car turns gang member, and fights, gun and all.
#[test]
fn a_getaway_driver_fights_once_out() {
    let Some((level, mut g)) = call(11, Kind::Getaway) else { return };
    let d = culprit(&g);
    assert!(g.people[d].has(Weapon::Pistol));
    run(&mut g, &level, 2.0);
    // A shot fired close by.
    let at = g.people[d].fig.pos + glam::vec3(300.0, 0.0, 0.0);
    g.people[d].scare(None, at);
    let mut out = false;
    for _ in 0..40 {
        run(&mut g, &level, 0.25);
        if !matches!(g.people[d].state, uc_game::State::InCar { .. }) && g.people[d].ai == uc_game::person::AI_GANG {
            out = true;
            break;
        }
    }
    assert!(out, "he never got out to fight");
}

/// Staying beside the jumper a moment talks them down.
#[test]
fn a_jumper_is_talked_down() {
    let Some((level, mut g)) = call(11, Kind::Jumper) else { return };
    let j = g.beat().unwrap().incidents[0].jumper.unwrap();
    let at = g.people[j].fig.pos;
    assert!(at.y > 2.0 * 256.0, "not up on a roof: {at:?}");
    run(&mut g, &level, 3.0);
    assert_eq!(g.people[j].fig.pos.x, at.x, "the jumper wandered off the ledge");
    assert!(g.teleport(at + glam::vec3(100.0, 0.0, 0.0), &level));
    run(&mut g, &level, 3.0);
    let b = g.beat().unwrap();
    assert_eq!((b.handled, b.missed), (1, 0), "not talked down");
    assert!(!g.people[j].removed);
}

/// Staying with the bomb defuses it; leaving it, it goes up.
#[test]
fn a_bomb_is_defused_or_goes_up() {
    let Some((level, mut g)) = call(11, Kind::Bomb) else { return };
    let at = g.beat().unwrap().incidents[0].at;
    assert!(g.teleport(at + glam::vec3(150.0, 0.0, 0.0), &level));
    run(&mut g, &level, 4.0);
    let b = g.beat().unwrap();
    assert_eq!((b.handled, b.missed), (1, 0), "not defused");

    let Some((level, mut g)) = call(11, Kind::Bomb) else { return };
    let k = g.beat().unwrap().incidents[0].bomb.unwrap();
    let crime = g.crime_rate;
    run(&mut g, &level, 95.0);
    assert!(g.mines[k].gone, "it never went off");
    assert!(g.beat().unwrap().missed >= 1 && g.crime_rate > crime);
}

/// Knocks person `i` out.
fn floor(g: &mut Game, i: usize) {
    let p = &mut g.people[i];
    let from = p.fig.pos + glam::vec3(50.0, 0.0, 0.0);
    p.take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
}

/// Calls `kinds` in one after another, each settled away before the next.
fn calls(level: &Level, g: &mut Game, kinds: &[Kind]) -> Vec<glam::Vec3> {
    kinds.iter().map(|&k| {
        assert!(g.call_in_now(k, level), "nowhere for a {}", k.label());
        let at = g.beat().unwrap().incidents[0].at;
        g.beat_mut().unwrap().incidents.clear();
        at
    }).collect()
}

/// A dumped gun goes in a bin or a skip on the beat, a different one each time, not
/// whatever prop is nearest a spot (a lamp post, a tree).
#[test]
fn a_dumped_gun_is_in_a_bin_or_a_skip() {
    let Some((level, mut g)) = start(11) else { return };
    run(&mut g, &level, 1.0);
    let at = calls(&level, &mut g, &[Kind::DumpedWeapon; 4]);
    // (Not the mission's own stashed things: Beat Cop's are numbered from 50,000.)
    let dumped: Vec<usize> = g.hidden.iter().filter(|(_, it)| it.way >= 50_000).map(|(ob, _)| *ob).collect();
    assert_eq!(dumped.len(), 4);
    for ob in dumped {
        let prim = level.map.objects[ob].prim;
        assert!([33, 134, 221].contains(&prim), "hidden in prim {prim}");
    }
    for (k, a) in at.iter().enumerate() {
        assert!(at[..k].iter().all(|b| (*a - *b).length() > 256.0), "the same bin twice: {at:?}");
    }
}

/// The last few incidents were all somewhere else: calls aren't piled up in one spot.
#[test]
fn incidents_are_spread_about() {
    for mission in ["police1", "Gangorder1"] {
        let Some(level) = common::load(mission) else { return };
        let mut g = Game::new_beat(&level, 3, None, Shift::Minutes(20));
        run(&mut g, &level, 1.0);
        let at = calls(&level, &mut g, &[Kind::Mugging, Kind::Bomb, Kind::WantedMan, Kind::GangFight, Kind::DrugDeal, Kind::ManDown]);
        for (k, a) in at.iter().enumerate() {
            for b in &at[k.saturating_sub(5)..k] {
                let d = glam::vec2(a.x - b.x, a.z - b.z).length();
                assert!(d >= 6.0 * 256.0, "{mission}: two calls {:.1} squares apart: {at:?}", d / 256.0);
            }
        }
    }
}

/// Southside Beach starts her up on a roof; the shift puts her down in the street, with
/// somewhere for incidents.
#[test]
fn southside_beach_gets_calls() {
    let Some(level) = common::load("e3") else { return };
    let mut g = Game::new_beat(&level, 11, None, Shift::Minutes(20));
    assert!(g.player().unwrap().fig.pos.y < 256.0, "still up on the roof");
    run(&mut g, &level, 1.0);
    calls(&level, &mut g, &[Kind::Mugging, Kind::Bomb, Kind::DrunkDriver]);
}

/// A bag snatcher is off and running from the start.
#[test]
fn a_bag_snatcher_runs() {
    let Some((level, mut g)) = call(11, Kind::BagSnatch) else { return };
    let at = g.beat().unwrap().incidents[0].at;
    let t = g.people.iter().position(|p| !p.player && p.guilty && !p.removed && (p.fig.pos - at).length() < 200.0).expect("no thief");
    assert!(g.people[t].scared.is_some(), "not running");
    run(&mut g, &level, 3.0);
    assert!((g.people[t].fig.pos - at).length() > 3.0 * 256.0, "he never got away from the spot");
    floor(&mut g, t);
    run(&mut g, &level, 0.5);
    assert_eq!(g.beat().unwrap().handled, 1);
}

/// A dealer and his buyer stand together till she comes near, then both run; it's handled
/// once both are down.
#[test]
fn a_drug_deal_scatters() {
    let Some((level, mut g)) = call(11, Kind::DrugDeal) else { return };
    let at = g.beat().unwrap().incidents[0].at;
    let two: Vec<usize> = g.people.iter().enumerate().filter(|(_, p)| !p.player && p.guilty && !p.removed && (p.fig.pos - at).length() < 200.0).map(|(i, _)| i).collect();
    assert_eq!(two.len(), 2);
    run(&mut g, &level, 2.0);
    assert!(two.iter().all(|&i| (g.people[i].fig.pos - at).length() < 200.0 && g.people[i].scared.is_none()), "they didn't wait");
    assert!(g.teleport(at + glam::vec3(5.0 * 256.0, 0.0, 0.0), &level) || g.teleport(at + glam::vec3(0.0, 0.0, 5.0 * 256.0), &level));
    run(&mut g, &level, 0.5);
    assert!(two.iter().all(|&i| g.people[i].scared.is_some()), "they didn't run");
    floor(&mut g, two[0]);
    run(&mut g, &level, 0.5);
    assert_eq!(g.beat().unwrap().handled, 0, "handled with one still loose");
    floor(&mut g, two[1]);
    run(&mut g, &level, 0.5);
    assert_eq!(g.beat().unwrap().handled, 1);
}

/// A troublemaker picks on the passers-by.
#[test]
fn a_troublemaker_is_a_bully() {
    let Some((level, mut g)) = call(11, Kind::Troublemaker) else { return };
    let t = culprit(&g);
    assert_eq!(g.people[t].ai, uc_game::person::AI_BULLY);
    floor(&mut g, t);
    run(&mut g, &level, 0.5);
    assert_eq!(g.beat().unwrap().handled, 1);
}

/// A man down lies hurt; staying with him till the ambulance comes handles it.
#[test]
fn a_man_down_is_seen_to() {
    let Some((level, mut g)) = call(11, Kind::ManDown) else { return };
    let m = g.beat().unwrap().incidents[0].patient.unwrap();
    run(&mut g, &level, 1.0);
    assert!(matches!(g.people[m].state, uc_game::State::Injured), "not lying hurt: {:?}", g.people[m].state);
    let at = g.people[m].fig.pos;
    assert!(g.teleport(at + glam::vec3(120.0, 0.0, 0.0), &level) || g.teleport(at + glam::vec3(0.0, 0.0, 120.0), &level));
    run(&mut g, &level, 5.0);
    let b = g.beat().unwrap();
    assert_eq!((b.handled, b.missed), (1, 0), "not seen to");
}

/// A gunman on a roof waits there, armed, and turns on her once she's up there with him.
#[test]
fn a_rooftop_gunman_turns_on_her_up_there() {
    let Some((level, mut g)) = call(11, Kind::Gunman) else { return };
    let t = culprit(&g);
    let at = g.people[t].fig.pos;
    assert!(at.y > 2.0 * 256.0, "not up on a roof: {at:?}");
    assert!(g.people[t].has(Weapon::Pistol));
    run(&mut g, &level, 2.0);
    assert!(!g.people[t].aggro(), "went for her from across the beat");
    // Up on the roof, a couple of squares from him.
    let me = g.player.unwrap();
    let on = (0..16).map(|k| k as f32 * std::f32::consts::TAU / 16.0)
        .map(|a| at + glam::vec3(a.sin(), 0.0, a.cos()) * 2.0 * 256.0)
        .map(|p| glam::vec3(p.x, level.ground.surface(&level.map, p.x, p.z, at.y + 64.0), p.z))
        .find(|p| (p.y - at.y).abs() < 32.0)
        .expect("no roof round him");
    g.people[me].fig.pos = on;
    run(&mut g, &level, 0.5);
    let p = &g.people[t];
    assert!(p.aggro(), "never turned on her: {:?} ai {} from {:?}, at {:?}", p.state, p.ai, g.people[me].fig.pos, p.fig.pos);
}

/// Dispatch doesn't call the same kind in twice running.
#[test]
fn never_the_same_kind_twice_running() {
    let Some((level, mut g)) = start(5) else { return };
    let mut kinds = Vec::new();
    for _ in 0..12 {
        g.beat_mut().unwrap().incidents.clear();
        for _ in 0..240 {
            run(&mut g, &level, 0.25);
            if let Some(inc) = g.beat().unwrap().incidents.first() {
                kinds.push(inc.kind);
                break;
            }
        }
    }
    assert!(kinds.len() >= 8, "too few calls: {kinds:?}");
    assert!(kinds.windows(2).all(|w| w[0] != w[1]), "{kinds:?}");
}

/// A 20-minute shift ended with the crime rate under 20 counts in the stats (#517); one with
/// it higher, or a shorter shift, doesn't.
#[test]
fn a_quiet_shift_counts() {
    for (on, rate, quiet) in [(1198.0, 10, 1), (1198.0, 25, 0), (500.0, 10, 0)] {
        let Some((level, mut g)) = start(11) else { return };
        let b = g.beat_mut().unwrap();
        (b.shift, b.on) = (Some(2.0), on);
        g.crime_rate = rate;
        run(&mut g, &level, 3.0);
        assert_eq!(g.outcome, Some(Outcome::Won));
        assert_eq!(g.record.quiet_shifts, quiet, "{on}s on, crime rate {rate}");
        assert!(g.record.best_shift as i64 >= g.beat().unwrap().score.max(0));
    }
}
