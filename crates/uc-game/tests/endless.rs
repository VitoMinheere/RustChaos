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
