//! The `shake_camera` point (`EWAY_DO_SHAKE_CAMERA`): while it's active the camera trembles,
//! stirred up by 3 a turn below 100 (eway.cpp:6649), and it settles once it's over. Day Of
//! Reckoning's Finale has one, as does factory1's (point 479, active while 478 is).
//! Skipped without an install.

use uc_formats::ucm::trigger;
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn factory(shaking: bool) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let mut level = Level::load(&game, "factory1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    if shaking {
        // Active from the start, while it's true.
        let ep = level.mission.event_points.iter_mut().find(|e| e.index == 479).unwrap();
        ep.trigger = trigger::NONE;
    }
    let g = Game::new(&level, true);
    Some((level, g))
}

fn run(level: &Level, g: &mut Game, secs: f32) -> f32 {
    let mut most: f32 = 0.0;
    for _ in 0..(secs / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
        most = most.max(g.shake);
    }
    most
}

#[test]
fn the_camera_trembles_while_it_is_active() {
    let Some((level, mut g)) = factory(true) else { return };
    run(&level, &mut g, 1.0);
    let most = run(&level, &mut g, 2.0);
    assert!(g.shake > 0.0 && most < 100.0, "a steady tremble: now {}, at most {most}", g.shake);
}

#[test]
fn not_until_it_goes_active() {
    let Some((level, mut g)) = factory(false) else { return };
    assert_eq!(run(&level, &mut g, 3.0), 0.0);
}
