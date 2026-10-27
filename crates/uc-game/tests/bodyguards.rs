//! Darci's bodyguards (`PCOM_AI_BODYGUARD`): they go for whoever is going for her, at once
//! and in the thick of it, and arm themselves with what's lying about when they have no gun.
//! Uses the people RTA starts with, brought out into the street. Skipped without an install.

use glam::Vec3;
use uc_game::person::AI_BODYGUARD;
use uc_game::{Controls, Game, Item, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn run(level: &Level, g: &mut Game, secs: f32) {
    for _ in 0..((secs / DT) as usize).max(1) {
        let pi = g.player.unwrap();
        g.people[pi].health = g.people[pi].max_health;
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
    }
}

/// RTA, with Darci in a quiet street and a bodyguard beside her, unarmed. The others it's
/// made by now are returned too.
fn start() -> Option<(Level, Game, usize, Vec<usize>)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    run(&level, &mut g, 1.0);
    let pi = g.player.unwrap();
    let at = Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0);
    let at = Vec3::new(at.x, level.ground.surface(&level.map, at.x, at.z, 1.0e4), at.z);
    g.people[pi].fig.pos = at;
    let mut others: Vec<usize> = (0..g.people.len())
        .filter(|&i| i != pi && !g.people[i].removed && !g.people[i].is_dead() && matches!(g.people[i].state, State::Move))
        .collect();
    let guard = others.remove(0);
    let p = &mut g.people[guard];
    (p.ai, p.other, p.mv) = (AI_BODYGUARD, 0, 0);
    p.weapon = Weapon::None;
    p.fig.pos = at + Vec3::new(-120.0, 0.0, 0.0);
    Some((level, g, guard, others))
}

/// Three of a gang go for her together; her bodyguard lands a blow on one of them within a
/// couple of seconds, though they stand close round her.
#[test]
fn a_bodyguard_wades_in() {
    let Some((level, mut g, guard, others)) = start() else { return };
    let at = g.player().unwrap().fig.pos;
    assert!(others.len() >= 3);
    for (k, &i) in others[..3].iter().enumerate() {
        let p = &mut g.people[i];
        (p.ai, p.mv) = (6, 0);
        p.weapon = Weapon::None;
        p.fig.pos = at + Vec3::new(150.0, 0.0, -150.0 + 150.0 * k as f32);
        p.go_for();
    }
    let mut hit = None;
    for t in 0..(8.0 / DT) as usize {
        run(&level, &mut g, DT);
        if others[..3].iter().any(|&i| g.people[i].hurt_by == Some(guard)) {
            hit = Some(t as f32 * DT);
            break;
        }
    }
    let hit = hit.expect("the bodyguard never landed a blow");
    assert!(hit < 3.0, "the bodyguard took {hit:.1}s to land a blow");
}

/// Unarmed, with nothing to fight, a bodyguard picks up a pistol lying near.
#[test]
fn a_bodyguard_arms_himself() {
    let Some((level, mut g, guard, _)) = start() else { return };
    let from = g.people[guard].fig.pos;
    let darci = g.player().unwrap().fig.pos;
    let eye = Vec3::new(0.0, 120.0, 0.0);
    // Three squares off, wherever he can see it.
    let at = (0..8)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 8.0;
            let v = from + Vec3::new(a.cos(), 0.0, a.sin()) * 3.0 * 256.0;
            Vec3::new(v.x, level.ground.surface(&level.map, v.x, v.z, from.y + 64.0), v.z)
        })
        // On his side of her, so he doesn't push her onto it on the way.
        .filter(|&v| (v - from).dot(from - darci) > 0.0)
        .find(|&v| (v.y - from.y).abs() < 32.0 && level.ground.line_of_sight(from + eye, v + eye))
        .expect("nowhere in sight to put it");
    g.items.push(Item { way: 0, kind: 2, pos: at });
    run(&level, &mut g, 8.0);
    assert!(g.people[guard].weapon == Weapon::Pistol, "still has {:?}", g.people[guard].weapon);
    assert!(!g.items.iter().any(|it| it.way == 0 && it.kind == 2), "the pistol's still lying there");
}

/// Someone else's bodyguard hit by the player fights back (`PCOM_attack_happened`: anyone
/// but his client): in Mr Bates (gang1), one of the cops guarding Bates (point 114),
/// punched face on by Mako.
#[test]
fn a_bodyguard_hit_by_the_player_fights_back() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "gang1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    run(&level, &mut g, 2.0);
    let pi = g.player.unwrap();
    let guard = g.people.iter().position(|p| p.way == 114).expect("no guard");
    assert_eq!(g.people[guard].ai, AI_BODYGUARD);
    let at = g.people[guard].fig.pos;
    g.people[pi].fig.pos = at + Vec3::new(90.0, 0.0, 0.0);
    g.people[pi].fig.angle = uc_world::character::angle_of(glam::vec2(-1.0, 0.0));
    g.people[pi].state = State::Move;
    g.people[guard].fig.angle = uc_world::character::angle_of(glam::vec2(1.0, 0.0));
    g.update(DT, &Controls { punch: true, ..Default::default() }, &level);
    let mut hurt = false;
    for _ in 0..(8.0 / DT) as usize {
        let before = g.people[pi].health;
        g.update(DT, &Controls::default(), &level);
        hurt |= g.people[pi].health < before;
        g.people[pi].health = g.people[pi].max_health;
    }
    assert_eq!(g.people[guard].foe, Some(pi), "he isn't after Mako");
    assert!(hurt, "he never hit back: {:?}", g.people[guard].state);
}
