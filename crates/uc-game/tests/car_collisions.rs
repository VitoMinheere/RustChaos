//! Cars run into each other (`CollideCar` with the vehicles round it, vehicle.cpp:1656): the
//! one driven into the other stops short of going through it, and at speed both are hurt
//! (`DoDamage`). Skipped without an install.

use uc_game::{Controls, Game};

use crate::common;
use common::DT;

#[test]
fn a_car_driven_into_another_stops_and_both_are_hurt() {
    let Some(level) = common::load("police1") else { return };
    let mut g = Game::new(&level, true);
    common::frames(&mut g, &level, 30);
    let parked: Vec<usize> = (0..g.vehicles.len()).filter(|&v| g.vehicles[v].driver.is_none() && !g.vehicles[v].wrecked()).collect();
    assert!(parked.len() >= 2, "two parked cars");
    let (a, b) = (parked[0], parked[1]);
    // Put car a a little behind car b, facing it, and send it off at speed.
    let target = g.vehicles[b].pos;
    let back = glam::vec2(g.vehicles[b].forward().x, g.vehicles[b].forward().y);
    let start = target - glam::vec3(back.x, 0.0, back.y) * 900.0;
    g.vehicles[a].pos = start;
    g.vehicles[a].yaw = g.vehicles[b].yaw;
    let (ha, hb) = (g.vehicles[a].health, g.vehicles[b].health);
    let along = |g: &Game| {
        let d = g.vehicles[a].pos - start;
        glam::vec2(d.x, d.z).dot(back)
    };
    let mut furthest: f32 = 0.0;
    for _ in 0..60 {
        g.vehicles[a].speed = 1600.0;
        g.update(DT, &Controls::default(), &level);
        furthest = furthest.max(along(&g));
        if g.vehicles[a].health < ha {
            break;
        }
    }
    assert!(!g.vehicles[a].overlaps(&g.vehicles[b]), "drove into it");
    assert!(furthest < 900.0, "went through it: {furthest}");
    assert!(g.vehicles[a].health < ha && g.vehicles[b].health < hb, "health {} -> {}, {} -> {}", ha, g.vehicles[a].health, hb, g.vehicles[b].health);
}
