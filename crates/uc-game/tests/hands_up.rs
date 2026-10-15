//! Hands up (`PCOM_cop_aiming_at_you`, fallen/Source/pcom.cpp:14092): Darci's gun turned on a
//! passer-by has them put their hands up and keep them there; twenty seconds on, they put
//! them down once she's aiming elsewhere. The guilty go for her instead, and cops pay no
//! heed. In RTA, among its crowd. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, AI_GANG};
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

fn run(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
    }
}

/// Someone of the crowd, put in front of Darci, and her aim on them.
fn aim_at(g: &mut Game, level: &Level, i: usize) {
    let p = g.player.unwrap();
    let me = g.people[p].fig.pos;
    let f = g.people[p].facing_dir();
    let at = me + vec3(f.x, 0.0, f.y) * 300.0;
    let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
    g.people[i].fig.pos = vec3(at.x, y, at.z);
    g.people[i].unseen = 0.0;
    g.target = Some(i);
}

#[test]
fn a_passer_by_puts_their_hands_up() {
    let Some((level, mut g)) = start() else { return };
    let p = g.player.unwrap();
    let i = g.people.iter().position(|q| q.fake && q.ai == AI_CIV && q.kind == ptype::CIV && !q.is_dead()).expect("no crowd");
    aim_at(&mut g, &level, i);
    run(&mut g, &level, DT);
    assert!(matches!(g.people[i].state, State::HandsUp { by, .. } if by == p), "{:?}", g.people[i].state);
    run(&mut g, &level, 2.0);
    assert_eq!(g.people[i].fig.anim(), anim::HANDS_UP_LOOP);
    // Still aimed at: they stay that way.
    for _ in 0..25 {
        g.target = Some(i);
        run(&mut g, &level, 1.0);
    }
    assert!(matches!(g.people[i].state, State::HandsUp { .. }));
    // Aimed elsewhere, after the twenty seconds: down again.
    g.people[p].weapon = Weapon::None;
    run(&mut g, &level, 0.5);
    assert!(matches!(g.people[i].state, State::Move), "{:?}", g.people[i].state);
}

#[test]
fn the_guilty_go_for_her() {
    let Some((level, mut g)) = start() else { return };
    let i = g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("no crowd");
    g.people[i].guilty = true;
    aim_at(&mut g, &level, i);
    run(&mut g, &level, DT);
    assert!(!matches!(g.people[i].state, State::HandsUp { .. }));
    assert_eq!(g.people[i].ai, AI_GANG);
    assert!(g.people[i].aggro());
}

#[test]
fn cops_pay_no_heed() {
    let Some((level, mut g)) = start() else { return };
    let Some(i) = g.people.iter().position(|q| q.kind == ptype::COP && !q.player && !q.is_dead() && !q.removed) else { return };
    aim_at(&mut g, &level, i);
    run(&mut g, &level, DT);
    assert!(!matches!(g.people[i].state, State::HandsUp { .. }));
}

/// Red marks (`RedMarks`, Person.cpp:2340): Darci killing a passer-by or a cop who isn't
/// guilty earns one; a guilty one doesn't.
#[test]
fn killing_innocents_earns_red_marks() {
    let Some((level, mut g)) = start() else { return };
    let p = g.player.unwrap();
    let from = g.people[p].fig.pos;
    let civ = g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead() && q.mv == uc_game::person::MOVE_WANDER).expect("no passer-by");
    g.people[civ].hurt_by = Some(p);
    g.people[civ].take_hit(from, 10_000, uc_game::person::ko::FINISHERS, false, false, Weapon::Pistol);
    run(&mut g, &level, DT);
    assert_eq!(g.red_marks, 1);
    // Someone guilty, killed: no mark.
    let thug = g.people.iter().position(|q| !q.player && !q.is_dead() && q.guilty).or_else(|| { let k = g.people.iter().position(|q| !q.player && !q.is_dead() && q.ai == AI_CIV)?; g.people[k].guilty = true; Some(k) }).expect("nobody");
    g.people[thug].hurt_by = Some(p);
    g.people[thug].take_hit(from, 10_000, uc_game::person::ko::FINISHERS, false, false, Weapon::Pistol);
    run(&mut g, &level, DT);
    assert_eq!(g.red_marks, 1);
    // Killed by someone else: no mark either.
    let other = g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead() && q.mv == uc_game::person::MOVE_WANDER).expect("no passer-by");
    g.people[other].hurt_by = None;
    g.people[other].take_hit(from, 10_000, uc_game::person::ko::FINISHERS, false, false, Weapon::Pistol);
    run(&mut g, &level, DT);
    assert_eq!(g.red_marks, 1);
}
