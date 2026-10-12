//! Runs the scripts of Combat Silver and Gold and of Physical Training to their end, with the
//! fighting and the course work done by cheating: enemies are knocked out directly and Darci
//! is moved to the switch. Checks the mission logic (waves, groups, random placement, timers,
//! fly-bys, conversations, switches), not the player's skill. Skipped without an install.

use glam::Vec3;
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "{mission}: unsupported {:?}", g.unsupported());
    Some((level, g))
}

/// Plays a combat grade, knocking every enemy out a moment after it appears. Returns the
/// weapons the enemies carried.
fn beat_the_grade(mission: &str, done_text: &str) -> Vec<Weapon> {
    let Some((level, mut g)) = load(mission) else { return Vec::new() };
    let mut weapons = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut knocked = 0;
    for frame in 0..(900.0 / DT) as usize {
        g.skip_cutscene();
        if frame % 30 == 0 {
            let p = g.player.unwrap();
            let from = g.people[p].fig.pos;
            g.people[p].health = g.people[p].max_health;
            for i in 0..g.people.len() {
                let e = &mut g.people[i];
                if !e.player && !e.removed && !e.is_dead() && !e.is_down() {
                    weapons.push(e.weapon);
                    e.take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
                    knocked += 1;
                }
            }
        }
        g.update(DT, &Controls::default(), &level);
        for m in &g.messages {
            if !seen.contains(&m.text) {
                println!("[{:6.1}s] {}", g.time, m.text);
                seen.push(m.text.clone());
            }
        }
        if g.outcome.is_some() {
            break;
        }
    }
    println!("{mission}: knocked out {knocked}, outcome {:?}", g.outcome);
    assert!(seen.iter().any(|m| m.contains(done_text)), "never saw {done_text:?}");
    assert_eq!(g.outcome, Some(Outcome::Won));
    weapons
}

#[test]
fn combat_silver_completes() {
    beat_the_grade("fight1", "BRONZE GRADE COMPLETED");
}

#[test]
fn combat_gold_completes_against_armed_enemies() {
    let weapons = beat_the_grade("fight2", "GOLD GRADE COMPLETED");
    println!("weapons: {weapons:?}");
    if !weapons.is_empty() {
        for w in [Weapon::Knife, Weapon::Bat, Weapon::Pistol] {
            assert!(weapons.contains(&w), "no {w:?} among {weapons:?}");
        }
    }
}

#[test]
fn physical_training_switch_stops_the_clock() {
    let Some((level, mut g)) = load("assault1") else { return };
    // The intro: three fly-bys, then "you ready?", then the clock starts.
    let mut clock = None;
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if let Some(t) = g.countdown() {
            clock = Some(t);
            break;
        }
    }
    let clock = clock.expect("the course clock never started");
    println!("clock started at {clock:.0}s, after {:.1}s", g.time);
    assert!(clock > 100.0);
    // Teleport to the finish and press the switch.
    let p = g.player.unwrap();
    g.people[p].fig.pos = Vec3::new(18639.0, 1536.0, 7842.0);
    let press = Controls { action: true, ..Default::default() };
    g.update(DT, &press, &level);
    for _ in 0..(3.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    println!("{:?}", g.messages.iter().map(|m| &m.text).collect::<Vec<_>>());
    assert_eq!(g.outcome, Some(Outcome::Won));
}

#[test]
fn physical_training_runs_out_of_time() {
    let Some((level, mut g)) = load("assault1") else { return };
    for _ in 0..(200.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.outcome.is_some() {
            break;
        }
    }
    assert!(matches!(g.outcome, Some(Outcome::Lost(_))), "{:?}", g.outcome);
}
