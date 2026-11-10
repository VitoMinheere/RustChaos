//! Endless mode (#129): waves come after a breather, each bigger than the last; downed
//! and arrested thugs score; the same seed gives the same waves; the crime rate loses the
//! run if the waves are left standing. Skipped without an install.

use uc_game::game::endless::{wave_size, Phase, Riot, FIRST_BREATHER};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Weapon};
use uc_world::Level;

use crate::common;
use common::DT;

fn start(seed: u32) -> Option<(Level, Game)> {
    let level = common::load("police1")?;
    let g = Game::new_endless(&level, seed, None, 10.0);
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

/// The wave's people on their feet.
fn standing(g: &Game) -> Vec<usize> {
    (0..g.people.len()).filter(|&i| {
        let p = &g.people[i];
        !p.player && p.aggro() && !p.removed && !p.is_down() && !p.is_dead() && !p.knocked_out
    }).collect()
}

/// Knocks out everyone of the wave standing.
fn knock_out_the_wave(g: &mut Game) {
    for i in standing(g) {
        let from = g.people[i].fig.pos + glam::vec3(50.0, 0.0, 0.0);
        g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }
}

/// The first wave comes after its breather, with as many as `wave_size(1)`, unarmed and away
/// from Darci; knocking them all out clears it, scores, and the next is bigger.
#[test]
fn waves_come_and_grow() {
    let Some((level, mut g)) = start(7) else { return };
    assert!(matches!(g.endless().unwrap().phase, Phase::Breather(_)));
    run(&mut g, &level, FIRST_BREATHER + 1.0);
    let e = g.endless().unwrap();
    assert_eq!((e.wave, e.phase), (1, Phase::Wave));
    let wave = standing(&g);
    assert_eq!(wave.len() as u32, wave_size(1), "the whole first wave should be about");
    let me = g.player().unwrap().fig.pos;
    for &i in &wave {
        let p = &g.people[i];
        assert_eq!(p.weapon, Weapon::None, "the first wave is unarmed");
        assert!((p.fig.pos - me).length() > 2.0 * 256.0, "made too near her");
    }
    // Down they go.
    for _ in 0..10 {
        knock_out_the_wave(&mut g);
        run(&mut g, &level, 0.5);
    }
    let e = g.endless().unwrap();
    assert!(matches!(e.phase, Phase::Breather(_)), "the wave should be cleared: {:?}", e.phase);
    assert!(e.score > 0 && e.downed == wave_size(1));
    // The next one is bigger.
    run(&mut g, &level, 16.0);
    let e = g.endless().unwrap();
    assert_eq!(e.wave, 2);
    assert!(wave_size(2) > wave_size(1));
    assert_eq!(standing(&g).len() as u32, wave_size(2));
}

/// The same seed makes the same wave, in the same places; another seed doesn't.
#[test]
fn the_seed_decides_the_waves() {
    let spots = |seed| {
        let (level, mut g) = start(seed)?;
        run(&mut g, &level, FIRST_BREATHER + 1.0);
        Some(standing(&g).iter().map(|&i| g.people[i].fig.pos).collect::<Vec<_>>())
    };
    let Some(a) = spots(7) else { return };
    assert_eq!(Some(a.clone()), spots(7));
    assert_ne!(Some(a), spots(8));
}

/// Waves left standing push the crime rate up until the city is lost.
#[test]
fn the_crime_rate_ends_a_run_left_alone() {
    let Some((level, mut g)) = start(7) else { return };
    let before = g.crime_rate;
    for _ in 0..600 {
        run(&mut g, &level, 1.0);
        if g.outcome.is_some() {
            break;
        }
    }
    assert!(g.crime_rate >= 100 || g.outcome.is_some(), "crime rate only got to {} from {before}", g.crime_rate);
    assert!(matches!(g.outcome, Some(Outcome::Lost(_))));
}

/// In a small arena (RTA's park), everyone of the wave is made in it, on the ground, and
/// gets to Darci.
#[test]
fn the_wave_comes_at_her_in_the_park() {
    let Some(level) = common::load("police1") else { return };
    let park = glam::vec2(8827.0, 10274.0);
    let mut g = Game::new_endless(&level, 3, Some(park), 10.0);
    let me = g.player().unwrap().fig.pos;
    assert!(glam::vec2(me.x, me.z).distance(park) < 256.0, "she should start in the park");
    let spots = &g.endless().unwrap().spots;
    assert!(spots.len() > 40, "the park should be a fair size: {} squares", spots.len());
    run(&mut g, &level, FIRST_BREATHER + 1.0);
    let wave = standing(&g);
    assert_eq!(wave.len() as u32, wave_size(1));
    for &i in &wave {
        let p = g.people[i].fig.pos;
        assert!(glam::vec2(p.x, p.z).distance(park) < 11.0 * 256.0, "made outside the park at {p:?}");
        assert!((p.y - me.y).abs() < 200.0, "made off her level at {p:?}");
    }
    // They come for her.
    run(&mut g, &level, 20.0);
    let me = g.player().unwrap().fig.pos;
    let near = wave.iter().filter(|&&i| (g.people[i].fig.pos - me).length() < 3.0 * 256.0).count();
    assert!(near >= 2, "only {near} of the wave got to her");
}

/// Plays through wave `n`'s start: skips to just before it and lets it come.
fn skip_to_wave(g: &mut Game, level: &Level, n: u32) {
    let e = g.endless_mut().unwrap();
    e.wave = n - 1;
    e.phase = Phase::Breather(0.1);
    run(g, level, 1.0);
}

/// Every tenth wave has a boss: made first, said, marked, and one a fist can land on.
#[test]
fn wave_ten_brings_a_boss() {
    let Some((level, mut g)) = start(7) else { return };
    skip_to_wave(&mut g, &level, 10);
    let e = g.endless().unwrap();
    assert_eq!(e.wave, 10);
    let boss = e.boss(&g).expect("no boss on wave ten");
    let b = &g.people[boss];
    assert!(b.max_health > uc_world::character::person::HEALTH[b.kind as usize], "the boss is no tougher");
    use uc_world::character::person::{MIB1, MIB2, MIB3};
    assert!(![MIB1, MIB2, MIB3].contains(&b.kind), "a boss who jumps clear of every blow");
    assert!(g.messages.iter().any(|m| m.text.contains("boss")), "nobody said a boss was coming");
    assert!(g.beacons().iter().any(|(l, _)| l == "Boss"));
    // Not on the others.
    let Some((level, mut g)) = start(7) else { return };
    skip_to_wave(&mut g, &level, 9);
    assert!(g.endless().unwrap().boss(&g).is_none());
}

/// Wave ten isn't a wall: no more than a couple of guns about at once, and some fists still.
#[test]
fn wave_ten_is_not_all_guns() {
    let Some((level, mut g)) = start(7) else { return };
    skip_to_wave(&mut g, &level, 10);
    let wave = standing(&g);
    let guns = wave.iter().filter(|&&i| g.people[i].weapon.is_gun()).count();
    assert!(!wave.is_empty());
    assert!(guns <= 3, "{guns} guns out of {}", wave.len());
}

/// No spot of the arena is inside a building, on RTA's map round the start (inside the police
/// station: she's put outside) or in the park.
#[test]
fn nobody_is_made_inside_a_building() {
    let Some(level) = common::load("police1") else { return };
    for centre in [None, Some(glam::vec2(8827.0, 10274.0))] {
        let g = Game::new_endless(&level, 3, centre, 10.0);
        for s in &g.endless().unwrap().spots {
            let sq = level.map.square((s.x / 256.0) as usize, (s.z / 256.0) as usize);
            assert!(!sq.has(uc_formats::iam::flags::HIDDEN), "a spot inside a building at {s:?}");
        }
    }
}

/// Each wave comes from another side of the arena than the last, with a beacon there.
#[test]
fn waves_come_from_another_side_each_time() {
    let Some((level, mut g)) = start(7) else { return };
    let mut last = None;
    for n in 1..=5 {
        skip_to_wave(&mut g, &level, n);
        let e = g.endless().unwrap();
        assert_ne!(Some(e.side), last, "wave {n} came from the same side");
        last = Some(e.side);
        assert!(g.beacons().iter().any(|(l, _)| l.starts_with("Wave")), "no beacon for wave {n}");
        knock_out_the_wave(&mut g);
        run(&mut g, &level, 0.5);
    }
}

/// Fighting, however slowly, holds the crime rate: a blow every few seconds and it doesn't
/// climb, where keeping out of it does (`the_crime_rate_ends_a_run_left_alone`).
#[test]
fn the_crime_rate_holds_while_she_fights() {
    let Some((level, mut g)) = start(7) else { return };
    run(&mut g, &level, FIRST_BREATHER + 1.0);
    let before = g.crime_rate;
    for k in 0..40 {
        // A light blow to one of them every three seconds.
        if let Some(&i) = standing(&g).get(k % 3) {
            let from = g.people[i].fig.pos + glam::vec3(50.0, 0.0, 0.0);
            g.people[i].take_hit(from, 1, 0, false, false, Weapon::None);
        }
        run(&mut g, &level, 3.0);
    }
    assert!(g.crime_rate <= before, "the crime rate went from {before} to {} while she fought", g.crime_rate);
}

/// One stuck where he can't get out (shut in a building) is made again in the arena.
#[test]
fn a_stuck_one_is_made_again() {
    let Some((level, mut g)) = start(7) else { return };
    run(&mut g, &level, FIRST_BREATHER + 1.0);
    let me = g.player().unwrap().fig.pos;
    // A square under a roof, well away from her.
    let n = uc_formats::iam::MAP_SIZE;
    let (x, z) = (0..n * n).map(|k| (k % n, k / n))
        .find(|&(x, z)| level.map.square(x, z).has(uc_formats::iam::flags::HIDDEN)
            && glam::vec2(x as f32 * 256.0 + 128.0 - me.x, z as f32 * 256.0 + 128.0 - me.z).length() > 12.0 * 256.0)
        .expect("no building");
    let i = standing(&g)[0];
    let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
    let y = level.ground.map_height(&level.map, cx, cz);
    g.people[i].fig.pos = glam::vec3(cx, y, cz);
    g.people[i].steer = None;
    run(&mut g, &level, 12.0);
    let p = g.people[i].fig.pos;
    let sq = level.map.square((p.x / 256.0) as usize, (p.z / 256.0) as usize);
    assert!(!sq.has(uc_formats::iam::flags::HIDDEN), "still shut in at {p:?}");
}

/// The radio: dispatch's intro and Darci's answer before the first wave, a call before each
/// one after it, a line when one's cleared, and the sirens with a boss (#149). Each line is
/// heard, not only shown.
#[test]
fn the_radio_calls_the_waves() {
    use uc_game::Sfx;
    let Some((level, mut g)) = start(7) else { return };
    // The radio's lines and the sirens heard over `secs`.
    let listen = |g: &mut Game, level: &Level, secs: f32| -> Vec<Sfx> {
        let mut heard = Vec::new();
        for _ in 0..(secs / DT) as usize {
            g.update(DT, &Controls::default(), level);
            heard.extend(g.take_sounds().into_iter().map(|(s, _)| s).filter(|s| matches!(s, Sfx::Riot(_) | Sfx::Siren)));
        }
        heard
    };
    let said = |heard: &[Sfx], f: fn(Riot) -> bool| heard.iter().any(|s| matches!(s, Sfx::Riot(l) if f(*l)));
    // Before the first wave: the intro, and Darci.
    let mut heard = listen(&mut g, &level, 1.0);
    assert!(g.messages.iter().any(|m| m.text.starts_with("Despatch: This is dispatch")), "the intro isn't shown");
    heard.extend(listen(&mut g, &level, FIRST_BREATHER - 0.5));
    assert!(said(&heard, |l| l == Riot::Intro), "no intro: {heard:?}");
    assert!(said(&heard, |l| l == Riot::Ready), "Darci didn't answer");
    knock_out_the_wave(&mut g);
    let heard = listen(&mut g, &level, 1.0);
    assert!(said(&heard, |l| matches!(l, Riot::Cleared(_))), "nothing said when the wave was cleared: {heard:?}");
    // The call before the next.
    let heard = listen(&mut g, &level, uc_game::game::endless::BREATHER);
    assert!(said(&heard, |l| matches!(l, Riot::Incoming(_))), "no call before wave 2: {heard:?}");
    // The boss wave: its own call, and the sirens.
    let e = g.endless_mut().unwrap();
    e.wave = 9;
    e.phase = Phase::Breather(4.0);
    let heard = listen(&mut g, &level, 5.0);
    assert!(said(&heard, |l| matches!(l, Riot::BossIncoming(_))), "no call for the boss");
    assert!(heard.contains(&Sfx::Siren), "no sirens with the boss");
}

/// Every line has words and a file of its own, and the takes don't repeat straight away.
#[test]
fn every_line_is_its_own() {
    let lines = uc_game::game::endless::lines();
    let files: std::collections::BTreeSet<&str> = lines.iter().map(|(f, _, _)| f.as_str()).collect();
    let words: std::collections::BTreeSet<&str> = lines.iter().map(|(_, t, _)| t.as_str()).collect();
    assert_eq!(files.len(), lines.len(), "two lines in one file");
    assert_eq!(words.len(), lines.len(), "two lines say the same");
}

/// Darci knocked down by a slide in the middle of a wave: the thugs leave her be while she's
/// down, then come back to her without shouting again. Each used to shout every turn she lay
/// there, so many at once it sounded like her screaming (#529).
#[test]
fn a_wave_doesnt_shout_while_darci_is_down() {
    let Some((level, mut g)) = start(1) else { return };
    run(&mut g, &level, FIRST_BREATHER + 5.0);
    assert!(!standing(&g).is_empty(), "no wave");
    g.take_sounds();
    let me = g.player.unwrap();
    let from = g.people[me].fig.pos + glam::vec3(0.0, 0.0, 60.0);
    g.people[me].take_hit(from, 30, ko::SLIDE, false, true, Weapon::None);
    assert!(g.people[me].is_down());
    let mut shouts = 0;
    for _ in 0..(5.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        shouts += g.take_sounds().iter().filter(|(s, _)| matches!(s, uc_game::Sfx::Alert { .. })).count();
    }
    assert!(!g.people[me].is_down(), "Darci didn't get up");
    assert_eq!(shouts, 0, "the wave shouted {shouts} times while Darci was down and getting up");
}

/// Clears the wave on: knocks them down as they come, at most `secs`. Darci's health is
/// left as it is.
fn clear_wave(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / 0.5) as usize {
        if matches!(g.endless().unwrap().phase, Phase::Breather(_)) {
            return;
        }
        knock_out_the_wave(g);
        for _ in 0..(0.5 / DT) as usize {
            g.update(DT, &Controls::default(), level);
        }
    }
}

/// A boss wave cleared without Darci losing any health counts in the stats (#517); one where
/// she's hurt doesn't. The wave reached counts as soon as it comes.
#[test]
fn a_boss_wave_cleared_unhurt_counts() {
    let Some((level, mut g)) = start(7) else { return };
    skip_to_wave(&mut g, &level, 10);
    assert_eq!(g.record.furthest_wave, 10);
    clear_wave(&mut g, &level, 60.0);
    assert!(matches!(g.endless().unwrap().phase, Phase::Breather(_)), "wave ten wasn't cleared");
    assert_eq!(g.record.boss_unhurt, 1);
    // Hurt this time.
    let Some((level, mut g)) = start(7) else { return };
    skip_to_wave(&mut g, &level, 10);
    let p = g.player.unwrap();
    g.people[p].health -= 10;
    clear_wave(&mut g, &level, 60.0);
    assert!(matches!(g.endless().unwrap().phase, Phase::Breather(_)), "wave ten wasn't cleared");
    assert_eq!(g.record.boss_unhurt, 0);
}

/// A run with modifiers `mods` (#518).
fn start_with(seed: u32, mods: &[uc_game::game::endless::Mod]) -> Option<(Level, Game)> {
    use uc_game::game::endless::Mods;
    let level = common::load("police1")?;
    let mods = mods.iter().fold(Mods::NONE, |m, &x| m.toggled(x));
    let g = Game::new_endless_with(&level, seed, None, 10.0, mods);
    Some((level, g))
}

/// The modifiers as a set: fists only and a gun run can't go together, the multipliers
/// multiply, and the ids go to words and back in one order whatever order they're given in.
#[test]
fn modifiers_make_a_set() {
    use uc_game::game::endless::{Mod, Mods};
    let m = Mods::NONE.toggled(Mod::Fists).toggled(Mod::OneHit);
    assert!(m.has(Mod::Fists) && m.has(Mod::OneHit));
    let m = m.toggled(Mod::GunRun);
    assert!(m.has(Mod::GunRun) && !m.has(Mod::Fists), "fists only and a gun run together");
    assert_eq!(m.percent(), 200 * 130 / 100);
    assert_eq!(m.scaled(1000), 2600);
    assert_eq!(Mods::NONE.percent(), 100);
    assert_eq!(m.ids(), "onehit-gunrun");
    assert_eq!(Mods::named("gunrun,onehit"), Some(m));
    assert_eq!(Mods::named("onehit,nonsense"), None);
    assert_eq!(Mods::from_bits(m.bits()), m);
    assert_eq!(m.toggled(Mod::OneHit).toggled(Mod::GunRun), Mods::NONE);
}

/// Fists only: no weapons on the map, nobody in the waves armed, even late on, and nothing
/// but health found on them. Heavy hitters: each of the wave four times as tough.
#[test]
fn fists_only_and_heavy_hitters() {
    use uc_game::game::endless::Mod;
    let Some((level, mut g)) = start_with(7, &[Mod::Fists, Mod::Heavy]) else { return };
    assert!(g.items.iter().all(|it| Weapon::from_item(it.kind).is_none()), "a weapon left on the map");
    skip_to_wave(&mut g, &level, 16);
    let wave = standing(&g);
    assert!(!wave.is_empty());
    for &i in &wave {
        let p = &g.people[i];
        assert_eq!(p.weapon, Weapon::None, "armed with fists only");
        assert!(p.drop.is_none_or(|d| d == 3), "ammo on him with fists only: {:?}", p.drop);
        let base = uc_world::character::person::HEALTH[p.kind as usize];
        assert!(p.max_health >= base * 4, "{} of {base}: no heavier", p.max_health);
    }
}

/// A gun run: Darci starts with a pistol, and the wave carries guns from the first.
#[test]
fn a_gun_run_arms_everyone() {
    use uc_game::game::endless::Mod;
    let Some((level, mut g)) = start_with(7, &[Mod::GunRun]) else { return };
    assert!(g.player().unwrap().has(Weapon::Pistol), "no pistol to start with");
    let first = g.endless().unwrap().breather().unwrap();
    run(&mut g, &level, first + 1.0);
    let wave = standing(&g);
    assert!(!wave.is_empty());
    assert!(wave.iter().all(|&i| g.people[i].weapon.is_gun()), "someone unarmed on a gun run");
}

/// One hit: she has a point of health, so any hit puts her down; and dispatch says so
/// before the first wave, which comes a little later for it.
#[test]
fn one_hit_and_the_rules_are_said() {
    use uc_game::game::endless::Mod;
    let Some((level, mut g)) = start_with(3, &[Mod::OneHit]) else { return };
    assert!(g.endless().unwrap().breather().unwrap() > FIRST_BREATHER);
    for _ in 0..(9.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    assert_eq!(g.player().unwrap().health, 1);
    assert!(g.messages.iter().any(|m| m.text.contains("One hit and you're down")), "dispatch didn't say the rules");
    // Two together: the stacked deck, then each rule in turn, all before the first wave
    // (twenty seconds at speed being thirty).
    let Some((level, mut g)) = start_with(3, &[Mod::OneHit, Mod::Speed]) else { return };
    let mut said = Vec::new();
    for _ in 0..(20.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        for m in &g.messages {
            if m.text.starts_with("Despatch") && !said.contains(&m.text) {
                said.push(m.text.clone());
            }
        }
    }
    assert_eq!(g.endless().unwrap().wave, 0, "the first wave came before the rules were said");
    let at = |t: &str| said.iter().position(|s| s.contains(t)).unwrap_or_else(|| panic!("not said: {t} in {said:?}"));
    assert!(at("stacked the deck") < at("One hit and you're down"));
    assert!(at("One hit and you're down") < at("all happening fast"));
}

/// Speed: a second plays half as many turns again. Low gravity: a standing jump goes higher.
#[test]
fn speed_and_low_gravity() {
    use uc_game::game::endless::Mod;
    let Some((level, mut g)) = start_with(3, &[Mod::Speed]) else { return };
    let before = g.turns();
    g.update(1.0, &Controls::default(), &level);
    assert_eq!(g.turns() - before, 45);
    let peak = |mods: &[Mod]| {
        let (level, mut g) = start_with(3, mods).unwrap();
        g.update(0.5, &Controls::default(), &level);
        let ground = g.player().unwrap().fig.pos.y;
        g.update(uc_game::TURN, &Controls { jump: true, ..Default::default() }, &level);
        let mut top = ground;
        for _ in 0..90 {
            g.update(uc_game::TURN, &Controls::default(), &level);
            top = top.max(g.player().unwrap().fig.pos.y);
        }
        top - ground
    };
    let (normal, floaty) = (peak(&[]), peak(&[Mod::LowGravity]));
    assert!(normal > 20.0, "no jump: {normal}");
    assert!(floaty > normal * 1.5, "jumped {floaty} against {normal}");
}

/// Each modifier has its line for dispatch, and the files are all different.
#[test]
fn every_modifier_has_its_line() {
    use uc_game::game::endless::Mod;
    let lines = uc_game::game::endless::lines();
    for m in Mod::ALL {
        assert!(lines.iter().any(|(f, _, _)| f.contains(m.id())), "no line for {}", m.id());
    }
}
