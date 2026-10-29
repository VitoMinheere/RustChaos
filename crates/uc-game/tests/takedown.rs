//! The crouch key, and the takedown from behind (`grapples[0]`, `ANIM_PISTOL_WHIP`,
//! fallen/Source/combat.cpp:181): punching someone close by with their back to Darci knocks
//! them out cold, whatever their health, to lie there until cuffed. In RTA. Skipped without
//! an install.

use glam::{vec2, vec3};
use uc_game::person::{Grab, AI_GANG};
use uc_game::{Controls, Game, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// RTA, Darci on open ground facing +z, and a gang member from the crowd `dist` in front of
/// her, facing the way given (`away`: his back to her).
fn start(dist: f32, away: bool) -> Option<(Level, Game, usize, usize)> {
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
    let p = g.player.unwrap();
    let at = vec3(66.0 * 256.0, 0.0, 74.0 * 256.0);
    let y = level.ground.surface(&level.map, at.x, at.z, 1.0e4);
    g.people[p].fig.pos = vec3(at.x, y, at.z);
    g.people[p].fig.angle = uc_world::character::angle_of(vec2(0.0, 1.0));
    g.people[p].state = State::Move;
    let i = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && q.kind != uc_world::character::person::COP).expect("nobody");
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != p && j != i && (q.hostile() || q.kind == uc_world::character::person::COP) {
            q.removed = true;
        }
    }
    let him = &mut g.people[i];
    him.ai = AI_GANG;
    him.fake = false;
    him.mv = 1;
    him.max_health = 500;
    him.health = 500;
    him.fig.pos = vec3(at.x, y, at.z + dist);
    him.fig.angle = uc_world::character::angle_of(if away { vec2(0.0, 1.0) } else { vec2(0.0, -1.0) });
    him.state = State::Move;
    Some((level, g, p, i))
}

fn run(g: &mut Game, level: &Level, secs: f32, c: &Controls) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, c, level);
    }
}

#[test]
fn the_crouch_key() {
    let Some((level, mut g, p, _)) = start(2000.0, true) else { return };
    let crouch = Controls { crouch: true, ..Default::default() };
    run(&mut g, &level, 1.0, &crouch);
    assert!(g.people[p].crouching(), "{:?}", g.people[p].state);
    let crawl = Controls { crouch: true, wish: vec2(0.0, 1.0), ..Default::default() };
    run(&mut g, &level, 1.0, &crawl);
    assert!(matches!(g.people[p].state, State::Crouch { phase: 2 }), "not crawling: {:?}", g.people[p].state);
    run(&mut g, &level, 2.0, &Controls::default());
    assert!(matches!(g.people[p].state, State::Move), "didn't get up: {:?}", g.people[p].state);
}

#[test]
fn out_cold_from_behind() {
    let Some((level, mut g, p, i)) = start(80.0, true) else { return };
    run(&mut g, &level, DT, &Controls { punch: true, ..Default::default() });
    assert!(matches!(g.people[p].state, State::Grapple { grab: Grab::PistolWhip, victim, .. } if victim == i), "{:?}", g.people[p].state);
    run(&mut g, &level, 4.0, &Controls::default());
    let him = &g.people[i];
    assert!(him.is_down() && him.out_cold, "{:?}", him.state);
    assert!(him.health > 0, "he's alive");
    assert!(him.arrestable(), "and can be cuffed");
    // And stays down.
    run(&mut g, &level, 6.0, &Controls::default());
    assert!(g.people[i].is_down(), "got up: {:?}", g.people[i].state);
}

#[test]
fn crawling_up_and_whipping_him() {
    let Some((level, mut g, p, i)) = start(80.0, true) else { return };
    run(&mut g, &level, 1.0, &Controls { crouch: true, ..Default::default() });
    assert!(g.people[p].crouching());
    run(&mut g, &level, DT, &Controls { crouch: true, punch: true, ..Default::default() });
    assert!(matches!(g.people[p].state, State::Grapple { grab: Grab::PistolWhip, .. }), "{:?}", g.people[p].state);
    run(&mut g, &level, 4.0, &Controls::default());
    assert!(g.people[i].out_cold);
}

#[test]
fn not_face_to_face() {
    let Some((level, mut g, p, i)) = start(80.0, false) else { return };
    run(&mut g, &level, DT, &Controls { punch: true, ..Default::default() });
    assert!(!matches!(g.people[p].state, State::Grapple { grab: Grab::PistolWhip, .. }), "whipped him face to face");
    run(&mut g, &level, 2.0, &Controls::default());
    assert!(!g.people[i].out_cold);
}

/// He goes down once, in the whip's own animation, and lies as it leaves him: not stood up
/// and knocked over again when it's done.
#[test]
fn down_once() {
    use uc_world::character::anim;
    let Some((level, mut g, _, i)) = start(80.0, true) else { return };
    run(&mut g, &level, DT, &Controls { punch: true, ..Default::default() });
    for _ in 0..(4.0 / DT) as usize {
        run(&mut g, &level, DT, &Controls::default());
        let a = g.people[i].fig.anim();
        assert!(!matches!(a, anim::KD_BACK_MID | anim::KD_FRONT_MID | anim::KD_BACK_HI | anim::KD_FRONT_HI | anim::KD_BACK_LOW | anim::KD_FRONT_LOW | anim::KD_BACK_LAND | anim::KD_FRONT_LAND),
                "knocked over again: anim {a}, {:?}", g.people[i].state);
    }
    assert_eq!(g.people[i].fig.anim(), anim::PISTOL_WHIP_TAKE);
    assert!(g.people[i].out_cold);
}
