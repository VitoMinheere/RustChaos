//! The flying kick (`ANIM_FLYKICK_START`, Person.cpp:13613): kick in a running jump and Darci
//! goes in feet first, knocking down whoever she meets on the way, and lands out of it
//! (`ANIM_FLYKICK_LAND`). In RTA, with the traffic stopped. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::AI_GANG;
use uc_game::{Controls, Game, State};
use uc_world::character::anim;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn step(g: &mut Game, level: &Level, c: &Controls) {
    let p = g.player.unwrap();
    g.people[p].health = g.people[p].max_health;
    for v in g.vehicles.iter_mut() {
        v.speed = 0.0;
    }
    g.update(DT, c, level);
}

/// RTA, Darci on open ground facing +z, and a gang member `dist` in front of her, facing
/// her: (level, game, Darci, him).
fn start(dist: f32) -> Option<(Level, Game, usize, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..120 {
        g.skip_cutscene();
        step(&mut g, &level, &Controls::default());
    }
    let p = g.player.unwrap();
    let at = vec3(66.0 * 256.0, 0.0, 74.0 * 256.0);
    let y = level.ground.surface(&level.map, at.x, at.z, 1.0e4);
    g.people[p].fig.pos = vec3(at.x, y, at.z);
    g.people[p].fig.angle = uc_world::character::angle_of(vec2(0.0, 1.0));
    g.people[p].state = State::Move;
    let i = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && q.kind != uc_world::character::person::COP)?;
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != p && j != i && (q.hostile() || q.kind == uc_world::character::person::COP) {
            q.removed = true;
        }
    }
    let h = &mut g.people[i];
    (h.ai, h.fake, h.mv, h.max_health, h.health) = (AI_GANG, false, 1, 500, 500);
    h.fig.pos = vec3(at.x, y, at.z + dist);
    h.fig.angle = uc_world::character::angle_of(vec2(0.0, -1.0));
    h.state = State::Move;
    Some((level, g, p, i))
}

/// Running at him, jump, then kick in the air: feet first, and down he goes; she lands and
/// carries on.
#[test]
fn a_flying_kick_knocks_him_down() {
    let Some((level, mut g, p, i)) = start(700.0) else { return };
    let run = Controls { wish: vec2(0.0, 1.0), run: true, ..Default::default() };
    for _ in 0..12 {
        step(&mut g, &level, &run);
    }
    step(&mut g, &level, &Controls { jump: true, ..run });
    assert!(matches!(g.people[p].state, State::Fall { .. }), "no jump: {:?}", g.people[p].state);
    step(&mut g, &level, &run);
    step(&mut g, &level, &Controls { kick: true, ..run });
    assert_eq!(g.people[p].fig.anim(), anim::FLYKICK_START, "not feet first");
    let (mut down, mut fell) = (false, false);
    for _ in 0..60 {
        step(&mut g, &level, &run);
        down |= g.people[i].is_down();
        fell |= g.people[p].fig.anim() == anim::FLYKICK_FALL;
        if !matches!(g.people[p].state, State::Fall { .. }) {
            break;
        }
    }
    assert!(fell, "never fell out of the kick");
    assert_eq!(g.people[p].fig.anim(), anim::FLYKICK_LAND, "landed some other way");
    assert!(down, "he's still up: {:?}, {} away", g.people[i].state, (g.people[i].fig.pos - g.people[p].fig.pos).length());
    assert!(matches!(g.people[p].state, State::Land { .. } | State::Move), "she didn't land: {:?}", g.people[p].state);
    assert!(g.people[i].health < 500, "no damage");
}

/// Kicking in a standing jump does nothing of the sort.
#[test]
fn not_from_a_standing_jump() {
    let Some((level, mut g, p, _)) = start(700.0) else { return };
    step(&mut g, &level, &Controls { jump: true, ..Default::default() });
    step(&mut g, &level, &Controls::default());
    step(&mut g, &level, &Controls { kick: true, ..Default::default() });
    assert_ne!(g.people[p].fig.anim(), anim::FLYKICK_START);
}
