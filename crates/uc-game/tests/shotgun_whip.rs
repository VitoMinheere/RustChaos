//! Clubbing with a two-handed gun (#346): hit in a fight, the player goes into fight mode
//! (combat.cpp:2183), gun or no gun, and punch is then a punch (`set_player_punch`,
//! interfac.cpp:747): with a shotgun in hand, a club with it (`ANIM_SHOTGUN_WHIP1`, `_2`).
//! Running leaves fight mode. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::Move;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::character::{anim, Play};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Combat Silver with one of its fighters squared up to Darci, everyone else gone, and a
/// shotgun in her hands.
fn setup() -> Option<(Level, Game, usize, usize)> {
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
    let at = g.people[pi].fig.pos;
    g.people[ei].fig.pos = at + vec3(0.0, 0.0, 110.0);
    g.people[ei].fig.angle = uc_world::character::angle_of(vec2(0.0, -1.0));
    g.people[pi].fig.angle = uc_world::character::angle_of(vec2(0.0, 1.0));
    g.people[pi].give(Weapon::Shotgun);
    g.people[pi].weapon = Weapon::Shotgun;
    Some((level, g, pi, ei))
}

/// The enemy punches her, and she's back on her feet.
fn punched(g: &mut Game, level: &Level, pi: usize, ei: usize) {
    let e = &mut g.people[ei];
    e.fig.restart(anim::PUNCH_COMBO1, Play::Once);
    e.state = State::Attack { mv: Move::Punch1, hit: false, queued: None };
    let health = g.people[pi].health;
    for _ in 0..90 {
        g.update(DT, &Controls::default(), level);
        if g.people[pi].health < health && matches!(g.people[pi].state, State::Move) {
            return;
        }
    }
    panic!("not punched: {:?}, health {}", g.people[pi].state, g.people[pi].health);
}

fn punch(g: &mut Game, level: &Level) {
    g.update(DT, &Controls { punch: true, fire: true, ..Default::default() }, level);
}

#[test]
fn hit_in_a_fight_she_clubs_with_the_shotgun() {
    let Some((level, mut g, pi, ei)) = setup() else { return };
    // Not in a fight: it fires.
    punch(&mut g, &level);
    assert!(matches!(g.people[pi].state, State::Shoot { .. }), "{:?}", g.people[pi].state);
    for _ in 0..60 {
        g.update(DT, &Controls::default(), &level);
    }
    punched(&mut g, &level, pi, ei);
    punch(&mut g, &level);
    let p = &g.people[pi];
    assert!(matches!(p.state, State::Attack { mv: Move::Whip1 | Move::Whip2, .. }), "{:?}", p.state);
    assert!(matches!(p.fig.anim(), anim::SHOTGUN_WHIP1 | anim::SHOTGUN_WHIP2));
}

#[test]
fn running_leaves_fight_mode() {
    let Some((level, mut g, pi, ei)) = setup() else { return };
    punched(&mut g, &level, pi, ei);
    g.people[ei].removed = true;
    for _ in 0..10 {
        g.update(DT, &Controls { wish: vec2(1.0, 0.0), run: true, ..Default::default() }, &level);
    }
    for _ in 0..30 {
        g.update(DT, &Controls::default(), &level);
    }
    punch(&mut g, &level);
    assert!(matches!(g.people[pi].state, State::Shoot { .. }), "{:?}", g.people[pi].state);
}
