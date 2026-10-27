//! Breaking out of a hold (`SUB_STATE_GRAPPLE_HELD`, Person.cpp:19230): someone Darci has
//! by the arm struggles five times a second, and past 23 less half their skill they break
//! free (`ANIM_GRAB_ARM_ESCAPEV`, and `ANIM_GRAB_ARM_ESCAPE` for her), though not in the
//! middle of a knee. Then they come for her. In RTA, with the traffic stopped. Skipped
//! without an install.

use glam::{vec2, vec3};
use uc_game::person::{Grab, AI_GANG};
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

/// Darci walks up to him and grabs his arm, and holds on.
fn hold() -> Option<(Level, Game, usize, usize)> {
    let (level, mut g, p, i) = start(100.0)?;
    let mut c = Controls { wish: vec2(0.0, 1.0), punch: true, ..Default::default() };
    for _ in 0..90 {
        step(&mut g, &level, &c);
        c.punch = false;
        if matches!(g.people[p].state, State::Grapple { grab: Grab::Hold, .. }) {
            break;
        }
        c.wish = if matches!(g.people[p].state, State::Grapple { .. }) { vec2(0.0, 0.0) } else { vec2(0.0, 1.0) };
    }
    assert!(matches!(g.people[p].state, State::Grapple { grab: Grab::Hold, .. }), "no hold: {:?}", g.people[p].state);
    Some((level, g, p, i))
}

#[test]
fn held_too_long_he_breaks_free() {
    let Some((level, mut g, p, i)) = hold() else { return };
    let skill = g.people[i].skill as f32;
    // Counting from the grab, as he's struggled since.
    let State::Grapple { timer, .. } = g.people[p].state else { unreachable!() };
    let mut t = timer;
    while matches!(g.people[p].state, State::Grapple { .. }) {
        step(&mut g, &level, &Controls::default());
        t += DT;
        assert!(t < 10.0, "never broke free");
    }
    assert!(matches!(g.people[p].state, State::Escape) && g.people[p].fig.anim() == anim::GRAB_ARM_ESCAPE, "{:?}", g.people[p].state);
    assert!(matches!(g.people[i].state, State::Escape) && g.people[i].fig.anim() == anim::GRAB_ARM_ESCAPEV, "{:?}", g.people[i].state);
    // 23 less half his skill, at five a second.
    let due = (23.0 - (skill / 2.0).floor()) / 5.0;
    assert!((t - due).abs() < 1.0, "free after {t:.1}s, due after {due:.1}s");
    for _ in 0..90 {
        step(&mut g, &level, &Controls::default());
        if !matches!(g.people[i].state, State::Escape) {
            break;
        }
    }
    assert!(matches!(g.people[p].state, State::Move), "{:?}", g.people[p].state);
    assert!(!matches!(g.people[i].state, State::Escape | State::Held { .. }), "{:?}", g.people[i].state);
    assert!(g.people[i].aggro(), "he should come for her");
}

#[test]
fn not_while_hes_being_kneed() {
    let Some((level, mut g, p, i)) = hold() else { return };
    // Knee after knee: he never gets the chance.
    for k in 0..(6.0 / DT) as usize {
        let knee = k % 20 == 0;
        step(&mut g, &level, &Controls { punch: knee, ..Default::default() });
        if g.people[i].is_down() {
            break;
        }
        assert!(!matches!(g.people[p].state, State::Escape), "broke free mid-knee after {:.1}s", k as f32 * DT);
    }
}
