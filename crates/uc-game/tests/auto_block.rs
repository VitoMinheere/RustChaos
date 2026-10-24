//! The manual's fighting (#210): stepping back from someone swinging at her, Darci blocks
//! (`should_i_block`, combat.cpp:951), and kick from the guard sweeps their feet
//! (`set_person_leg_sweep`, Person.cpp:9545). Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::person::Move;
use uc_game::{Controls, Game, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Combat Silver with one of its fighters squared up to Darci, everyone else gone.
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
    Some((level, g, pi, ei))
}

/// The enemy throws a punch at her.
fn swing(g: &mut Game, ei: usize) {
    let e = &mut g.people[ei];
    e.fig.restart(uc_world::character::anim::PUNCH_COMBO1, uc_world::character::Play::Once);
    e.state = State::Attack { mv: Move::Punch1, hit: false, queued: None };
}

#[test]
fn backing_off_from_a_punch_blocks_it_and_kick_sweeps_their_feet() {
    let Some((level, mut g, pi, ei)) = setup() else { return };
    let back = Controls { wish: Vec2::new(0.0, -1.0), ..Default::default() };
    // Backing off with nobody swinging is just a step back.
    g.update(DT, &back, &level);
    assert!(!matches!(g.people[pi].state, State::Block { .. }), "{:?}", g.people[pi].state);
    // With a punch coming: a block, held while she keeps backing off.
    swing(&mut g, ei);
    let health = g.people[pi].health;
    g.update(DT, &back, &level);
    assert!(matches!(g.people[pi].state, State::Block { auto: true }), "{:?}", g.people[pi].state);
    for _ in 0..10 {
        g.update(DT, &back, &level);
    }
    assert!(matches!(g.people[pi].state, State::Block { auto: true }));
    assert_eq!(g.people[pi].health, health, "the punch was blocked");
    // Kick from the guard: the foot sweep, which puts them down.
    let enemy_health = g.people[ei].health;
    g.update(DT, &Controls { kick: true, ..Default::default() }, &level);
    assert!(matches!(g.people[pi].state, State::Attack { mv: Move::LegSweep, .. }), "{:?}", g.people[pi].state);
    for _ in 0..(1.5 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        if g.people[ei].is_down() {
            break;
        }
    }
    assert!(g.people[ei].is_down(), "swept off their feet");
    assert!(g.people[ei].health <= enemy_health - 49 || g.people[ei].health <= 0, "health {} from {enemy_health}", g.people[ei].health);
}

#[test]
fn letting_go_of_back_ends_the_auto_block() {
    let Some((level, mut g, pi, ei)) = setup() else { return };
    swing(&mut g, ei);
    g.update(DT, &Controls { wish: Vec2::new(0.0, -1.0), ..Default::default() }, &level);
    assert!(matches!(g.people[pi].state, State::Block { auto: true }));
    g.update(DT, &Controls::default(), &level);
    assert!(!matches!(g.people[pi].state, State::Block { .. }));
}
