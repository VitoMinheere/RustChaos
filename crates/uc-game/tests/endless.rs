//! Endless mode (#129): waves come after a breather, each bigger than the last; downed
//! and arrested thugs score; the same seed gives the same waves; the crime rate loses the
//! run if the waves are left standing. Skipped without an install.

use uc_game::game::endless::{wave_size, Phase};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Weapon};
use uc_world::Level;

mod common;
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
    run(&mut g, &level, 6.0);
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
        run(&mut g, &level, 6.0);
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
    run(&mut g, &level, 6.0);
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
    run(&mut g, &level, 6.0);
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
    run(&mut g, &level, 6.0);
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

/// The radio before each wave and when one's cleared, and the sirens with a boss (#149).
#[test]
fn the_radio_calls_the_waves() {
    use uc_game::Sfx;
    let Some((level, mut g)) = start(7) else { return };
    // The radio and the sirens heard over `secs`.
    let listen = |g: &mut Game, level: &Level, secs: f32| -> Vec<Sfx> {
        let mut heard = Vec::new();
        for _ in 0..(secs / DT) as usize {
            g.update(DT, &Controls::default(), level);
            heard.extend(g.take_sounds().into_iter().map(|(s, _)| s).filter(|s| matches!(s, Sfx::Radio | Sfx::Siren)));
        }
        heard
    };
    // Before the first wave.
    let heard = listen(&mut g, &level, 2.5);
    assert!(heard.contains(&Sfx::Radio), "no radio before the first wave");
    assert!(g.messages.iter().any(|m| m.text.starts_with("Despatch")));
    listen(&mut g, &level, 3.5);
    knock_out_the_wave(&mut g);
    let heard = listen(&mut g, &level, 1.0);
    assert!(heard.contains(&Sfx::Radio), "no radio when the wave was cleared");
    // The boss wave.
    let e = g.endless_mut().unwrap();
    e.wave = 9;
    e.phase = Phase::Breather(4.0);
    let heard = listen(&mut g, &level, 5.0);
    assert!(heard.contains(&Sfx::Siren), "no sirens with the boss");
}
