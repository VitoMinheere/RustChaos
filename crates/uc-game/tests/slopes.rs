//! Ground too steep to stand on (#253, `check_on_slippy_slope`): someone on it slides down
//! until it's gentle enough to stand on again. Target UC's mountainside. Skipped without an
//! install.

use glam::vec3;
use uc_formats::iam::flags;
use uc_game::{Controls, State};

mod common;
use common::DT;

#[test]
fn she_slides_down_a_slope_too_steep_to_stand_on() {
    let Some(mut r) = common::start("snow2") else { return };
    r.play(1.0, Controls::default());
    let map = &r.level.map;
    // The middle of the steepest open square's first triangle.
    let steep = |x: usize, z: usize| {
        let h = |a, b| map.corner_alt(a, b) as f32;
        ((h(x + 1, z) - h(x, z)).abs() + (h(x, z + 1) - h(x, z)).abs()) / 8.0
    };
    let (mx, mz) = (2..125)
        .flat_map(|x| (2..125).map(move |z| (x, z)))
        .filter(|&(x, z)| !map.square(x, z).has(flags::HIDDEN))
        .max_by(|a, b| steep(a.0, a.1).total_cmp(&steep(b.0, b.1)))
        .unwrap();
    assert!(steep(mx, mz) > 50.0, "Target UC has nothing too steep");
    let (x, z) = (mx as f32 * 256.0 + 64.0, mz as f32 * 256.0 + 64.0);
    let y = map.ground_height(x, z);
    let pi = r.player();
    r.g.people[pi].fig.pos = vec3(x, y, z);
    r.g.people[pi].state = State::Move;
    let mut slipped = false;
    for _ in 0..(1.0 / DT) as usize {
        r.play(DT, Controls::default());
        slipped |= matches!(r.g.people[pi].state, State::Slip);
    }
    assert!(slipped, "she stood on it: {:?}", r.g.people[pi].state);
    assert!(r.g.people[pi].fig.pos.y < y - 32.0, "she didn't go down: {} from {y}", r.g.people[pi].fig.pos.y);
}

#[test]
fn flat_ground_holds() {
    let Some(mut r) = common::start("police1") else { return };
    let pi = r.player();
    for _ in 0..(2.0 / DT) as usize {
        r.play(DT, Controls::default());
        assert!(!matches!(r.g.people[pi].state, State::Slip));
    }
}
