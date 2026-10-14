//! Who Darci can arrest: someone knocked down and waiting to get up, never someone lying dead
//! (`find_arrestee` wants `is_person_ko_and_lay_down`, fallen/Source/Person.cpp). Uses one of
//! the people RTA starts with, brought out into the street. Skipped without an install.

use glam::Vec3;
use uc_game::person::ko;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn run(level: &Level, g: &mut Game, secs: f32, c: Controls) {
    for _ in 0..((secs / DT) as usize).max(1) {
        let pi = g.player.unwrap();
        g.people[pi].health = g.people[pi].max_health;
        g.skip_cutscene();
        g.update(DT, &c, level);
    }
}

/// Puts Darci in the street with somebody lying beside her, knocked down with `damage`, and
/// has her press action. Returns whether they ended up arrested.
fn try_arrest(damage: i32) -> Option<bool> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    run(&level, &mut g, 1.0, Controls::default());
    let pi = g.player.unwrap();
    let at = Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0);
    let at = Vec3::new(at.x, level.ground.surface(&level.map, at.x, at.z, 1.0e4), at.z);
    g.people[pi].fig.pos = at;
    let other = (0..g.people.len())
        .find(|&i| i != pi && !g.people[i].removed && !g.people[i].is_dead() && matches!(g.people[i].state, State::Move))
        .expect("someone besides Darci");
    g.people[other].fig.pos = at + Vec3::new(80.0, 0.0, 0.0);
    g.people[other].take_hit(at, damage, ko::KNOCKDOWN, false, false, Weapon::None);
    // Down on the floor first.
    run(&level, &mut g, 1.5, Controls::default());
    assert!(g.people[other].is_down(), "should be lying down");
    // A mission that ends over it would stop her doing anything at all, arrests included.
    assert_eq!(g.outcome, None, "the mission ended");
    // Beside wherever they came to rest.
    let body = g.people[other].body_pos();
    g.people[pi].fig.pos = body + Vec3::new(60.0, 0.0, 0.0);
    run(&level, &mut g, 0.2, Controls { action: true, ..Default::default() });
    run(&level, &mut g, 4.0, Controls::default());
    Some(g.people[other].arrested || matches!(g.people[other].state, State::Cuffed))
}

#[test]
fn the_knocked_down_can_be_arrested() {
    let Some(arrested) = try_arrest(0) else { return };
    assert!(arrested, "someone knocked down should be arrested");
}

#[test]
fn the_dead_cant_be_arrested() {
    let Some(arrested) = try_arrest(10_000) else { return };
    assert!(!arrested, "arrested a dead man");
}
