//! Taunts and Roper's grapples (#51): a gang member who sees the player taunts them first
//! (`PCOM_set_person_ai_taunt`, `ANIM_WANKER`), and goes for them in the end; one without a
//! gun can only taunt a driver. Roper doesn't grab arms: he strangles, or headbutts from
//! further off (`grapples[]`, combat.cpp:179), and leaves them lying. Skipped without an
//! install.

use glam::{vec2, vec3};
use uc_game::person::{Grab, Taunt, AI_GANG};
use uc_game::{Controls, Game, State, Weapon};
use uc_world::character::anim;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(name: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, name, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
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

fn run(g: &mut Game, level: &Level, secs: f32, c: &Controls) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, c, level);
    }
}

/// Puts person `i` `dist` in front of the player, facing them, and clears everyone else
/// away who might join in.
fn face_off(g: &mut Game, level: &Level, i: usize, dist: f32) {
    let p = g.player.unwrap();
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != p && j != i && (q.hostile() || q.kind == uc_world::character::person::COP) {
            q.removed = true;
        }
    }
    let me = g.people[p].fig.pos;
    let f = g.people[p].facing_dir();
    let at = me + vec3(f.x, 0.0, f.y) * dist;
    let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
    g.people[i].fig.pos = vec3(at.x, y, at.z);
    g.people[i].fig.angle = uc_world::character::angle_of(-f);
    g.people[i].state = State::Move;
    g.people[i].unseen = 0.0;
}

/// RTA, Darci out on open ground, and someone of the crowd made a gang member.
fn rta_gang() -> Option<(Level, Game, usize)> {
    let (level, mut g) = load("police1")?;
    let p = g.player.unwrap();
    let at = vec3(66.0 * 256.0, 0.0, 74.0 * 256.0);
    let y = level.ground.surface(&level.map, at.x, at.z, 1.0e4);
    g.people[p].fig.pos = vec3(at.x, y, at.z);
    g.people[p].fig.angle = uc_world::character::angle_of(vec2(0.0, 1.0));
    g.people[p].state = State::Move;
    let i = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && q.kind != uc_world::character::person::COP && q.weapon == Weapon::None).expect("nobody");
    g.people[i].ai = AI_GANG;
    g.people[i].fake = false;
    face_off(&mut g, &level, i, 600.0);
    Some((level, g, i))
}

#[test]
fn a_gang_member_taunts_then_comes_on() {
    let Some((level, mut g, i)) = rta_gang() else { return };
    let mut taunted = false;
    let mut came = false;
    for _ in 0..(20.0 / DT) as usize {
        run(&mut g, &level, DT, &Controls::default());
        if matches!(g.people[i].state, State::Taunt { .. }) {
            taunted = true;
            assert_eq!(g.people[i].fig.anim(), anim::WANKER);
        }
        if taunted && g.people[i].taunt.is_none() && g.people[i].aggro() {
            came = true;
            break;
        }
    }
    assert!(taunted, "never taunted: {:?}", g.people[i].state);
    assert!(came, "only ever taunted: {:?}", g.people[i].taunt);
}

#[test]
fn hitting_a_taunter_ends_it() {
    let Some((level, mut g, i)) = rta_gang() else { return };
    for _ in 0..(5.0 / DT) as usize {
        run(&mut g, &level, DT, &Controls::default());
        if g.people[i].taunt.is_some() {
            break;
        }
    }
    assert!(g.people[i].taunt.is_some(), "no taunt");
    let from = g.people[g.player.unwrap()].fig.pos;
    g.people[i].take_hit(from, 1, 0, false, false, Weapon::None);
    assert_eq!(g.people[i].taunt, None);
}

#[test]
fn nobody_fist_fights_a_car() {
    let Some((level, mut g, i)) = rta_gang() else { return };
    // Taunted once and come on, then she's off in a car: he taunts her there.
    for _ in 0..(20.0 / DT) as usize {
        run(&mut g, &level, DT, &Controls::default());
        if g.people[i].aggro() && g.people[i].taunt.is_none() {
            break;
        }
    }
    assert!(g.people[i].aggro());
    let p = g.player.unwrap();
    let Some(v) = (0..g.vehicles.len()).find(|&v| g.vehicles[v].driver.is_none()) else { return };
    g.people[p].state = State::InCar { vehicle: v };
    g.vehicles[v].driver = Some(p);
    let me = g.people[p].fig.pos;
    g.vehicles[v].pos = me;
    let mut taunted = 0;
    for _ in 0..(10.0 / DT) as usize {
        run(&mut g, &level, DT, &Controls::default());
        if matches!(g.people[i].state, State::Taunt { .. }) {
            taunted += 1;
        }
        assert!(!matches!(g.people[i].state, State::Attack { .. }), "swung at a car");
    }
    assert!(taunted > 0, "never taunted the car: {:?} {:?}", g.people[i].state, g.people[i].taunt);
    assert!(matches!(g.people[i].taunt, Some(Taunt::Doing | Taunt::Pause(_) | Taunt::Now)));
}

/// Day Of Reckoning: Roper, and one of Bane's men in front of him.
fn roper_and(dist: f32) -> Option<(Level, Game, usize)> {
    let (level, mut g) = load("Finale1")?;
    let p = g.player.unwrap();
    assert_eq!(g.people[p].kind, uc_world::character::person::ROPER);
    let i = g.people.iter().position(|q| q.hostile() && !q.removed && !q.is_dead()).expect("nobody to fight");
    g.people[i].weapon = Weapon::None;
    g.people[i].max_health = 200;
    g.people[i].health = 200;
    face_off(&mut g, &level, i, dist);
    // Hold him still: it's Roper's move that's tested.
    g.people[i].removed = false;
    Some((level, g, i))
}

/// Until the grapple's over.
fn see_through(g: &mut Game, level: &Level) {
    let p = g.player.unwrap();
    for _ in 0..(8.0 / DT) as usize {
        run(g, level, DT, &Controls::default());
        if !matches!(g.people[p].state, State::Grapple { .. }) {
            return;
        }
    }
    panic!("never let go");
}

fn grapple(g: &mut Game, level: &Level, i: usize) -> Option<Grab> {
    let p = g.player.unwrap();
    let d = g.people[i].fig.pos - g.people[p].fig.pos;
    let c = Controls { punch: true, wish: vec2(d.x, d.z).normalize(), ..Default::default() };
    run(g, level, DT, &c);
    match g.people[p].state {
        State::Grapple { victim, grab, .. } if victim == i => Some(grab),
        _ => None,
    }
}

#[test]
fn roper_strangles_close_up() {
    let Some((level, mut g, i)) = roper_and(55.0) else { return };
    let skill = g.people[i].skill as i32;
    assert_eq!(grapple(&mut g, &level, i), Some(Grab::Strangle), "{:?}", g.people[g.player.unwrap()].state);
    assert!(matches!(g.people[i].state, State::Held { .. }));
    assert_eq!(g.people[i].fig.anim(), anim::STRANGLE_VICTIM);
    see_through(&mut g, &level);
    // Seven squeezes: frames 14 to 24 every other one, and 27.
    let lost = 200 - g.people[i].health;
    assert_eq!(lost, 7 * (((40 - skill) >> 3) + 1), "lost {lost} (skill {skill})");
    assert!(g.people[i].is_down() || g.people[i].is_dead(), "{:?}", g.people[i].state);
    assert!(matches!(g.people[g.player.unwrap()].state, State::Move));
}

#[test]
fn roper_headbutts_from_further() {
    let Some((level, mut g, i)) = roper_and(100.0) else { return };
    let skill = g.people[i].skill as i32;
    assert_eq!(grapple(&mut g, &level, i), Some(Grab::Headbutt), "{:?}", g.people[g.player.unwrap()].state);
    assert_eq!(g.people[i].fig.anim(), anim::HEADBUTT_VICTIM);
    see_through(&mut g, &level);
    assert_eq!(200 - g.people[i].health, 40 - skill);
    assert!(g.people[i].is_down() || g.people[i].is_dead());
}
