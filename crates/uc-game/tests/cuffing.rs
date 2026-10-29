//! A grapple's throw and an arrest, both where the body lies. Thrown (`ANIM_GRAB_ARM_THROWV`),
//! a victim lies where the throw puts him (`PERSON_DEATH_TYPE_STAY_ALIVE_PRONE`), not knocked
//! on again as by a punch. Cuffed lying on his back, he's rolled over first (`ANIM_ARREST_ROLL`),
//! then cuffed (`ANIM_ARREST_BE_CUFFED`), both kept where he is (`set_locked_anim`), and from
//! then on he struggles now and then (`ANIM_ARREST_STRUGGLE`, every 256 turns). In RTA, with
//! the traffic stopped so no car shoves the body. Skipped without an install.

use glam::{vec2, vec3, Vec3};
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

/// RTA, Darci on open ground facing +z, and a tough gang member a step in front of her,
/// facing her: (level, game, Darci, him).
fn start() -> Option<(Level, Game, usize, usize)> {
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
    h.fig.pos = vec3(at.x, y, at.z + 100.0);
    h.fig.angle = uc_world::character::angle_of(vec2(0.0, -1.0));
    h.state = State::Move;
    Some((level, g, p, i))
}

/// Darci grabs him and throws him; returns once the throw's over for her, and where his
/// hips were as he hit the ground.
fn throw(level: &Level, g: &mut Game, p: usize, i: usize) -> Vec3 {
    let mut c = Controls { wish: vec2(0.0, 1.0), punch: true, ..Default::default() };
    for _ in 0..90 {
        step(g, level, &c);
        c.punch = false;
        if matches!(g.people[p].state, State::Grapple { grab: Grab::Hold, .. }) {
            break;
        }
        c.wish = if matches!(g.people[p].state, State::Grapple { .. }) { vec2(0.0, 0.0) } else { vec2(0.0, 1.0) };
    }
    assert!(matches!(g.people[p].state, State::Grapple { grab: Grab::Hold, .. }), "no hold: {:?}", g.people[p].state);
    step(g, level, &Controls { kick: true, ..Default::default() });
    let mut landed = None;
    for _ in 0..150 {
        step(g, level, &Controls::default());
        if landed.is_none() && g.people[i].fig.finished() {
            landed = Some(g.people[i].body_pos());
        }
        if landed.is_some() && matches!(g.people[p].state, State::Move) {
            break;
        }
    }
    landed.expect("the throw never finished")
}

fn flat(a: Vec3, b: Vec3) -> f32 {
    vec2(a.x - b.x, a.z - b.z).length()
}

#[test]
fn a_thrown_man_lies_where_the_throw_puts_him() {
    let Some((level, mut g, p, i)) = start() else { return };
    let landed = throw(&level, &mut g, p, i);
    let h = &g.people[i];
    assert!(matches!(h.state, State::Down), "{:?}", h.state);
    assert_eq!(h.fig.anim(), anim::GRAB_ARM_THROWV, "knocked down again on top of the throw");
    assert!(flat(h.body_pos(), landed) < 8.0, "slid from {landed} to {}", h.body_pos());
    // And gets up again later, as from any knockdown.
    let mut up = false;
    for _ in 0..(6.0 / DT) as usize {
        step(&mut g, &level, &Controls::default());
        up |= matches!(g.people[i].state, State::GetUp);
    }
    assert!(up, "never got up: {:?}", g.people[i].state);
}

#[test]
fn cuffed_where_he_lies_rolled_over_and_struggling() {
    let Some((level, mut g, p, i)) = start() else { return };
    throw(&level, &mut g, p, i);
    let body = g.people[i].body_pos();
    g.people[p].fig.pos = body + vec3(60.0, 0.0, 0.0);
    for _ in 0..6 {
        step(&mut g, &level, &Controls { action: true, ..Default::default() });
    }
    assert!(matches!(g.people[i].state, State::Cuffed), "not cuffed: {:?}", g.people[i].state);
    // On his back from the throw: rolled over, then cuffed, his hips where they were.
    let (mut rolled, mut cuffed, mut moved) = (false, false, 0.0f32);
    for _ in 0..(3.0 / DT) as usize {
        step(&mut g, &level, &Controls::default());
        let h = &g.people[i];
        rolled |= h.fig.anim() == anim::ARREST_ROLL;
        cuffed |= rolled && h.fig.anim() == anim::ARREST_BE_CUFFED;
        moved = moved.max(flat(h.body_pos(), body));
    }
    assert!(rolled && cuffed, "rolled {rolled}, cuffed {cuffed}");
    assert!(moved < 24.0, "moved {moved} from where he lay");
    assert!(g.people[i].arrested);
    // A struggle within 256 turns, and still where he was.
    let mut struggled = false;
    for _ in 0..(13.0 / DT) as usize {
        step(&mut g, &level, &Controls::default());
        struggled |= g.people[i].fig.anim() == anim::ARREST_STRUGGLE;
        moved = moved.max(flat(g.people[i].body_pos(), body));
    }
    assert!(struggled, "never struggled");
    assert!(moved < 24.0, "moved {moved} from where he lay");
    assert!(matches!(g.people[i].state, State::Cuffed), "{:?}", g.people[i].state);
}

/// Held too long without a throw or a knee, he wriggles out of it (`ANIM_GRAB_ARM_ESCAPEV`)
/// and the hold's broken (`ANIM_GRAB_ARM_ESCAPE`) (#342, Person.cpp:19285).
#[test]
fn held_too_long_he_wriggles_free() {
    let Some((level, mut g, p, i)) = start() else { return };
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
    let (mut broke, mut escaped) = (false, false);
    for _ in 0..(7.0 / DT) as usize {
        step(&mut g, &level, &Controls::default());
        broke |= g.people[p].fig.anim() == anim::GRAB_ARM_ESCAPE;
        escaped |= g.people[i].fig.anim() == anim::GRAB_ARM_ESCAPEV;
    }
    assert!(broke && escaped, "broke {broke}, escaped {escaped}");
    assert!(!matches!(g.people[p].state, State::Grapple { .. } | State::Recover { .. }), "{:?}", g.people[p].state);
}
