//! Jumping in a fight goes the way Darci pushes (`ACTION_STANDING_JUMP`, interfac.cpp:4541):
//! back, a back flip (`set_person_standing_jump_backwards`, `ANIM_BACK_FLIP`); to a side, a
//! flip that way; else straight up. In RTA, squared up to a gang member, with the traffic
//! stopped. Skipped without an install.

use glam::{vec2, vec3, Vec2};
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

/// Squared up to him, she pushes `wish` (relative to facing him along +z) and jumps.
fn jump_pushing(wish: Vec2) -> Option<(Level, Game, usize)> {
    let (level, mut g, p, _) = start(150.0)?;
    for _ in 0..10 {
        step(&mut g, &level, &Controls::default());
    }
    step(&mut g, &level, &Controls { wish, jump: true, ..Default::default() });
    Some((level, g, p))
}

#[test]
fn pulling_back_is_a_back_flip() {
    let Some((level, mut g, p)) = jump_pushing(vec2(0.0, -1.0)) else { return };
    assert_eq!(g.people[p].fig.anim(), anim::BACK_FLIP);
    let State::Fall { vel, .. } = g.people[p].state else { panic!("not in the air: {:?}", g.people[p].state) };
    assert!(vel.normalize().dot(g.people[p].facing_dir()) < -0.9, "not backwards: {vel}");
    let from = g.people[p].fig.pos;
    for _ in 0..60 {
        step(&mut g, &level, &Controls::default());
        if !matches!(g.people[p].state, State::Fall { .. }) {
            break;
        }
    }
    assert!(matches!(g.people[p].state, State::Land { .. } | State::Move), "never landed: {:?}", g.people[p].state);
    assert!(g.people[p].fig.pos.z < from.z - 100.0, "didn't go back: {} -> {}", from, g.people[p].fig.pos);
}

#[test]
fn pushing_to_a_side_is_a_flip() {
    for (wish, left) in [(vec2(1.0, 0.0), true), (vec2(-1.0, 0.0), false)] {
        let Some((_, g, p)) = jump_pushing(wish) else { return };
        // Facing +z, +x is her left.
        assert!(matches!(g.people[p].state, State::Flip { left: l } if l == left), "{wish}: {:?}", g.people[p].state);
    }
}

#[test]
fn standing_still_is_straight_up() {
    let Some((_, g, p)) = jump_pushing(Vec2::ZERO) else { return };
    assert!(matches!(g.people[p].state, State::Fall { vel, .. } if vel.length() < 1.0), "{:?}", g.people[p].state);
    assert_eq!(g.people[p].fig.anim(), anim::STANDING_JUMP);
}
