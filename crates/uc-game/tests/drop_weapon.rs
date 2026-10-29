//! Knocked down, someone lets go of what's in their hand (`drop_current_gun`,
//! Person.cpp:7130): a shotgun with what's in it, Darci's pistol with her rounds. It's
//! theirs again only once picked up. Skipped without an install.

use uc_game::person::ko;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Combat Silver with Darci and one of its fighters, everyone else gone.
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
    g.people[ei].fig.pos = g.people[pi].fig.pos + glam::vec3(600.0, 0.0, 0.0);
    Some((level, g, pi, ei))
}

fn knock_down(g: &mut Game, level: &Level, i: usize) {
    let from = g.people[i].fig.pos + glam::vec3(100.0, 0.0, 0.0);
    g.people[i].take_hit(from, 1, ko::KNOCKDOWN, false, false, Weapon::None);
    assert!(g.people[i].is_down());
    for _ in 0..45 {
        g.update(DT, &Controls::default(), level);
    }
}

#[test]
fn a_shotgun_falls_from_his_hands() {
    let Some((level, mut g, _, ei)) = setup() else { return };
    let e = &mut g.people[ei];
    e.give(Weapon::Shotgun);
    e.weapon = Weapon::Shotgun;
    let before = g.items.len();
    knock_down(&mut g, &level, ei);
    let e = &g.people[ei];
    assert_eq!(e.weapon, Weapon::None);
    assert!(!e.has(Weapon::Shotgun), "it's no longer his");
    let it = g.items.last().unwrap();
    assert_eq!((g.items.len(), it.kind), (before + 1, Weapon::Shotgun.item().unwrap()));
    assert_eq!(it.rounds, Some(Weapon::Shotgun.clip()), "with what was in it");
}

#[test]
fn darci_drops_her_pistol_and_picks_it_up_again() {
    let Some((level, mut g, pi, _)) = setup() else { return };
    let s = Weapon::Pistol.slot().unwrap();
    let p = &mut g.people[pi];
    p.give(Weapon::Pistol);
    p.weapon = Weapon::Pistol;
    p.ammo[s] = 20;
    knock_down(&mut g, &level, pi);
    let p = &g.people[pi];
    assert_eq!(p.weapon, Weapon::None);
    assert!(!p.has(Weapon::Pistol));
    assert_eq!(p.ammo[s], 5, "the gun took a clip's worth");
    let it = g.items.iter().find(|it| it.kind == Weapon::Pistol.item().unwrap()).expect("the pistol on the ground");
    assert_eq!(it.rounds, Some(15));
    let at = it.pos;
    // Up again, and over it.
    for _ in 0..240 {
        g.update(DT, &Controls::default(), &level);
        if matches!(g.people[pi].state, State::Move) {
            break;
        }
    }
    g.people[pi].fig.pos = at;
    for _ in 0..10 {
        g.update(DT, &Controls::default(), &level);
    }
    let p = &g.people[pi];
    assert!(p.has(Weapon::Pistol), "picked up again");
    assert_eq!(p.ammo[s], 20, "with its rounds");
}
