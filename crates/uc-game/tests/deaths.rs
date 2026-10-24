//! How people die (#238): shot dead on the street they fall as the shot had it
//! (`PERSON_DEATH_TYPE_SHOT_*`, `ANIM_SHOT_DEAD_*`); and fired at with their hands up they
//! get down, unhurt, for Darci to cuff ("Get Down.", `PERSON_DEATH_TYPE_GET_DOWN`). In RTA,
//! among its crowd. Skipped without an install.

use glam::vec3;
use uc_game::person::AI_CIV;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::character::{anim, person as ptype};
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
    let p = g.player.unwrap();
    g.people[p].give(Weapon::Pistol);
    g.people[p].weapon = Weapon::Pistol;
    Some((level, g))
}

/// One of the crowd, put 300 in front of Darci and facing `towards` her (or away), with her
/// aim on them.
fn in_front(g: &mut Game, level: &Level, towards: bool) -> usize {
    let p = g.player.unwrap();
    let i = g.people.iter().position(|q| q.fake && q.ai == AI_CIV && q.kind == ptype::CIV && !q.is_dead()).expect("no crowd");
    let me = g.people[p].fig.pos;
    let f = g.people[p].facing_dir();
    let at = me + vec3(f.x, 0.0, f.y) * 300.0;
    let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
    g.people[i].fig.pos = vec3(at.x, y, at.z);
    g.people[i].fig.angle = g.people[p].fig.angle + if towards { std::f32::consts::PI } else { 0.0 };
    g.people[i].unseen = 0.0;
    g.target = Some(i);
    i
}

/// Person `i` stands where they are, takes no notice and has a shot's worth of health left:
/// no AI, not one of the crowd, and a robot so the gun doesn't put their hands up.
fn stand_still(g: &mut Game, i: usize) {
    let q = &mut g.people[i];
    (q.ai, q.mv, q.fake, q.health) = (0, 1, false, 1);
    q.bent |= uc_game::person::BENT_ROBOT;
}

/// Shoots until person `i` is down.
fn shoot(g: &mut Game, level: &Level, i: usize) {
    let fire = Controls { punch: true, fire: true, ..Default::default() };
    for _ in 0..(10.0 / DT) as usize {
        g.target = Some(i);
        g.update(DT, &fire, level);
        if g.people[i].is_down() {
            return;
        }
    }
}

#[test]
fn shot_dead_from_the_front_they_go_over_backwards() {
    let Some((level, mut g)) = start() else { return };
    let i = in_front(&mut g, &level, true);
    stand_still(&mut g, i);
    shoot(&mut g, &level, i);
    assert!(g.people[i].is_dead(), "{:?}", g.people[i].state);
    assert_eq!(g.people[i].fig.anim(), anim::SHOT_DEAD_BACK);
}

#[test]
fn shot_dead_from_behind_they_go_over_forwards() {
    let Some((level, mut g)) = start() else { return };
    let i = in_front(&mut g, &level, false);
    stand_still(&mut g, i);
    shoot(&mut g, &level, i);
    assert!(g.people[i].is_dead(), "{:?}", g.people[i].state);
    let a = g.people[i].fig.anim();
    assert!([anim::SHOT_DEAD_HEAD, anim::SHOT_DEAD_GUT, anim::SHOT_DEAD_CHEST].contains(&a), "anim {a}");
}

#[test]
fn fired_at_with_their_hands_up_they_get_down() {
    let Some((level, mut g)) = start() else { return };
    let i = in_front(&mut g, &level, true);
    for _ in 0..30 {
        g.target = Some(i);
        g.update(DT, &Controls::default(), &level);
    }
    assert!(matches!(g.people[i].state, State::HandsUp { .. }), "{:?}", g.people[i].state);
    let health = g.people[i].health;
    shoot(&mut g, &level, i);
    let q = &g.people[i];
    assert!(q.is_down() && !q.is_dead(), "{:?}", q.state);
    assert_eq!(q.fig.anim(), anim::HANDS_UP_LIE);
    assert_eq!(q.health, health, "getting down doesn't hurt");
    assert!(g.messages.iter().any(|m| m.text == "Get Down."), "she should say it");
    for _ in 0..(5.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.people[i].is_down(), "they stay down to be cuffed");
    assert!(g.people[i].snap().arrestable, "and can be cuffed");
}

#[test]
fn a_blast_kills_flat_out() {
    // Semtex's drum 324 going up beside someone.
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "semtex", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let k = g.barrels.iter().position(|b| b.way == 324).expect("drum 324");
    let at = g.barrels[k].pos;
    let i = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead()).expect("someone");
    stand_still(&mut g, i);
    let y = level.ground.surface(&level.map, at.x + 100.0, at.z, at.y + 64.0);
    g.people[i].fig.pos = vec3(at.x + 100.0, y, at.z);
    let p = g.player.unwrap();
    g.shoot_barrel(k, p, &level);
    for _ in 0..15 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.people[i].is_dead(), "{:?}", g.people[i].state);
    let a = g.people[i].fig.anim();
    assert!([anim::KO_BACK, anim::KO_BEHIND_BIG].contains(&a), "anim {a}");
}

#[test]
fn a_fall_that_kills_ends_in_a_heap() {
    let Some((level, mut g)) = start() else { return };
    let i = in_front(&mut g, &level, true);
    stand_still(&mut g, i);
    // Coming down fast enough to hurt (20000 a tick at the original's scale).
    g.people[i].fig.pos.y += 300.0;
    g.people[i].state = State::Fall { vy: -2400.0, vel: glam::Vec2::ZERO };
    for _ in 0..(4.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        if g.people[i].is_dead() {
            break;
        }
    }
    assert!(g.people[i].is_dead(), "{:?}", g.people[i].state);
    assert_eq!(g.people[i].fig.anim(), anim::BIGLAND_DIE);
}
