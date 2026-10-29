//! The panel's stamina marks (DDEngine/Source/panel.cpp:5021): a mark a 25 of breath, five at
//! most; sprinting wears them down. Skipped without an install.

use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn a_mark_a_25_of_breath() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "fight2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let me = g.player.unwrap();
    // Full breath, 128: five.
    assert_eq!(g.people[me].stamina_marks(), 5);
    for (stamina, marks) in [(24.0, 0), (25.0, 1), (74.0, 2), (99.0, 3), (100.0, 4), (300.0, 5)] {
        g.people[me].stamina = stamina;
        assert_eq!(g.people[me].stamina_marks(), marks, "at {stamina}");
    }
    // Sprinting wears them down.
    g.people[me].stamina = 128.0;
    let sprint = Controls { wish: glam::vec2(1.0, 0.0), run: true, sprint: true, ..Default::default() };
    for _ in 0..(10.0 / DT) as usize {
        g.update(DT, &sprint, &level);
    }
    assert!(g.people[me].stamina_marks() < 5, "breath {}", g.people[me].stamina);
}
