//! Firing on the run (`set_person_running_shoot`, Person.cpp:6607): punch while running with
//! a gun fires at whoever's targeted without stopping, no sooner than the gun allows after
//! the last; an AK goes on firing while it's held. Skipped without an install.

use glam::vec2;
use uc_game::person::running_anim;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Combat Silver, Darci armed with `w` and running, one of its fighters off to the side.
fn setup(w: Weapon) -> Option<(Level, Game, usize, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "fight1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let pi = g.player.unwrap();
    let ei = (0..g.people.len()).find(|&i| i != pi && !g.people[i].removed && g.people[i].hostile())?;
    for (i, p) in g.people.iter_mut().enumerate() {
        if i != pi && i != ei {
            p.removed = true;
        }
    }
    g.people[ei].fig.pos = g.people[pi].fig.pos + glam::vec3(0.0, 0.0, 700.0);
    let p = &mut g.people[pi];
    p.give(w);
    p.weapon = w;
    p.ammo[w.slot().unwrap()] = 50;
    for _ in 0..20 {
        g.update(DT, &run(), &level);
    }
    Some((level, g, pi, ei))
}

fn run() -> Controls {
    Controls { wish: vec2(1.0, 0.0), run: true, ..Default::default() }
}

fn rounds(g: &Game, pi: usize) -> u32 {
    let p = &g.people[pi];
    p.ammo[p.weapon.slot().unwrap()]
}

#[test]
fn she_fires_on_the_run_without_stopping() {
    let Some((level, mut g, pi, ei)) = setup(Weapon::Pistol) else { return };
    let before = rounds(&g, pi);
    let fire = Controls { punch: true, target: Some(ei), ..run() };
    g.update(DT, &fire, &level);
    // Again at once: too soon.
    g.update(DT, &fire, &level);
    let p = &g.people[pi];
    assert_eq!(rounds(&g, pi), before - 1, "one round");
    assert!(matches!(p.state, State::Move) && running_anim(p.fig.anim()), "still running: {:?} {}", p.state, p.fig.anim());
    // A fifth of a second on, it'll go off again.
    for _ in 0..6 {
        g.update(DT, &run(), &level);
    }
    g.update(DT, &fire, &level);
    assert_eq!(rounds(&g, pi), before - 2);
}

#[test]
fn an_ak_keeps_firing_while_held() {
    let Some((level, mut g, pi, ei)) = setup(Weapon::Ak47) else { return };
    let before = rounds(&g, pi);
    g.update(DT, &Controls { punch: true, fire: true, target: Some(ei), ..run() }, &level);
    for _ in 0..29 {
        g.update(DT, &Controls { fire: true, target: Some(ei), ..run() }, &level);
    }
    let fired = before - rounds(&g, pi);
    assert!((4..=6).contains(&fired), "{fired} rounds in a second");
    assert!(matches!(g.people[pi].state, State::Move));
}
