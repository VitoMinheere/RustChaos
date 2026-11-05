//! The panel's stamina marks (DDEngine/Source/panel.cpp:5021): a mark a 25 of breath, five at
//! most; sprinting wears them down. Skipped without an install.

use uc_game::{Controls, Game};

mod common;
use common::DT;

#[test]
fn a_mark_a_25_of_breath() {
    let Some(level) = common::load("fight2") else { return };
    let mut g = Game::new(&level, true);
    common::frames(&mut g, &level, 30);
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
