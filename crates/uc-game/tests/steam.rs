//! Steam vents (`create_trap`): Assassin (`Gangorder1`) has two blowing down from a ledge,
//! and Grim Gardens (`botanicc`) one that stays active for good and so, as in the original,
//! never blows. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::steam::{dir, Vent};
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    Some((level, g))
}

fn play(level: &Level, g: &mut Game, secs: f32) {
    for _ in 0..(secs / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
    }
}

fn point(level: &Level, i: usize) -> Vec3 {
    let e = level.mission.event_points.iter().find(|e| e.index == i).unwrap();
    vec3(e.x as f32, e.y as f32, e.z as f32)
}

/// The routine: Assassin's point 241 is `speed 2, steps 5, routine 1`, so it blows for the
/// first fifth of every 5 * 128 / 2 hundredths, 3.2 seconds.
#[test]
fn a_vent_keeps_to_its_routine() {
    let v = Vent { pos: Vec3::ZERO, angle: 0.0, dir: dir::DOWN, routine: 1, speed: 2, steps: 5, range: 3 };
    assert!(v.blowing(0.0));
    assert!(v.blowing(0.6));
    assert!(!v.blowing(0.7), "a step is 64 hundredths");
    assert!(!v.blowing(3.1));
    assert!(v.blowing(3.2), "and round again");
    let none = Vent { steps: 0, ..v };
    assert!(!none.blowing(0.0), "no steps, no steam (the original would divide by nothing)");
}

#[test]
fn assassins_vents_blow_and_scald() {
    let Some((level, mut g)) = load("Gangorder1") else { return };
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    play(&level, &mut g, 4.0);
    let vent = point(&level, 241);
    let near = g.steam.puffs.iter().filter(|p| (p.pos - vent).length() < 512.0).count();
    assert!(near > 0, "no steam at point 241");
    // Blowing down, it heads for the ground.
    assert!(g.steam.puffs.iter().filter(|p| (p.pos - vent).length() < 512.0).all(|p| p.pos.y <= vent.y + 8.0));

    // Stand in it. Every puff in reach scalds on its own, and a column holds dozens, so it
    // doesn't take long: these are traps.
    let me = g.player.unwrap();
    let floor = level.ground.surface(&level.map, vent.x, vent.z, vent.y - 16.0);
    g.people[me].fig.pos = vec3(vent.x, floor, vent.z);
    let before = g.people[me].health;
    play(&level, &mut g, 0.1);
    assert!(g.people[me].health < before, "the steam should scald: {} -> {}", before, g.people[me].health);
    play(&level, &mut g, 2.0);
    assert!(g.people[me].knocked_out, "two seconds in a vent's column is the end of her");
}

#[test]
fn a_trap_that_stays_active_never_blows() {
    let Some((level, mut g)) = load("botanicc") else { return };
    play(&level, &mut g, 1.0);
    assert!(g.point_active(149), "point 149 has no trigger, so it's active straight away");
    play(&level, &mut g, 6.0);
    let vent = point(&level, 149);
    assert!(g.steam.puffs.iter().all(|p| (p.pos - vent).length() > 512.0), "149 is dead as it fires, so no steam");
}
