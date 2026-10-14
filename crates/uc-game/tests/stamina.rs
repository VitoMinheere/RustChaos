//! Stamina and reflexes (Person.cpp:4315, 15700; combat.cpp:2070): sprinting runs Darci out of
//! breath, letting go of action gets it back, the stamina badges raise how much she has, and
//! her reflexes add to every shot. In RTA's streets. Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::{Controls, Game, Sfx, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start() -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    Some((level, g))
}

/// Runs, keeping her on the same spot so walls never get in the way; returns how far she got
/// each frame on average, and whether she panted.
fn run(g: &mut Game, level: &Level, secs: f32, sprint: bool) -> (f32, bool) {
    let p = g.player.unwrap();
    let home = g.people[p].fig.pos;
    let c = Controls { wish: vec2(1.0, 0.0), run: true, sprint, ..Default::default() };
    let (mut moved, mut panted) = (0.0, false);
    let frames = (secs / DT) as usize;
    for _ in 0..frames {
        g.skip_cutscene();
        g.people[p].fig.pos = home;
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &c, level);
        let at = g.people[p].fig.pos;
        moved += Vec2::new(at.x - home.x, at.z - home.z).length();
        panted |= g.take_sounds().iter().any(|(s, _)| matches!(s, Sfx::ById { id: 176, .. }));
    }
    (moved / frames as f32, panted)
}

#[test]
fn sprinting_runs_her_out_of_breath_and_letting_go_brings_it_back() {
    let Some((level, mut g)) = start() else { return };
    let p = g.player.unwrap();
    assert_eq!(g.people[p].stamina, 128.0);
    let (sprinting, _) = run(&mut g, &level, 2.0, true);
    let (running, _) = run(&mut g, &level, 2.0, false);
    assert!(sprinting > running * 1.2, "sprint {sprinting:.1} is no faster than a run {running:.1}");
    // A point every fourth tick: 128 lasts about 24 seconds.
    let (_, panted) = run(&mut g, &level, 25.0, true);
    assert!(panted, "never ran out of breath ({:.0} left)", g.people[p].stamina);
    assert_eq!(g.people[p].stamina, 0.0);
    // Still holding action: only running.
    let (winded, _) = run(&mut g, &level, 1.0, true);
    assert!(winded < sprinting * 0.9, "still sprinting out of breath: {winded:.1}");
    // Let go, and it comes back a point a tick.
    run(&mut g, &level, 2.0, false);
    assert!(g.people[p].stamina > 40.0, "only {:.0} back", g.people[p].stamina);
}

#[test]
fn stamina_badges_raise_her_breath() {
    let Some((_, mut g)) = start() else { return };
    g.set_stats([5, 0, 0, 0]);
    let p = g.player.unwrap();
    assert_eq!(g.people[p].max_stamina, 133.0);
    assert_eq!(g.people[p].stamina, 133.0);
}

/// Reflexes add to her shots: the first hit with ten of them takes ten more than without.
#[test]
fn reflexes_add_to_her_shots() {
    let first_hit = |reflexes: u8| -> Option<i32> {
        let (level, mut g) = start()?;
        g.set_stats([0, 0, reflexes, 0]);
        let p = g.player.unwrap();
        let me = g.people[p].fig.pos;
        let t = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead()).expect("nobody");
        let target = &mut g.people[t];
        target.fig.pos = me + vec3(200.0, 0.0, 0.0);
        target.ai = 6; // a gang member: someone the gun picks
        target.max_health = 10_000;
        target.health = 10_000;
        target.state = State::Move;
        let pl = &mut g.people[p];
        pl.give(Weapon::Pistol);
        pl.weapon = Weapon::Pistol;
        pl.fig.angle = uc_world::character::angle_of(vec2(1.0, 0.0));
        for _ in 0..40 {
            g.people[t].fig.pos = me + vec3(200.0, 0.0, 0.0);
            g.update(DT, &Controls { punch: true, ..Default::default() }, &level);
            for _ in 0..10 {
                g.update(DT, &Controls::default(), &level);
            }
            if g.people[t].health < 10_000 {
                return Some(10_000 - g.people[t].health);
            }
        }
        panic!("never hit");
    };
    let (Some(without), Some(with)) = (first_hit(0), first_hit(10)) else { return };
    assert_eq!(with - without, 10, "a shot did {without} without reflexes and {with} with ten");
}
