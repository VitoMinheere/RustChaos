//! Drivers on patrol drive their route (`PCOM_process_driving_patrol`, pcom.cpp:6376) rather
//! than sitting at the wheel. Gang Order 1's three patrol cars (points 129, 132, 135, made
//! once 125 goes off, here at once) each have one point with the longest wait: they drive
//! there and park (`PCOM_set_person_move_park_car`). Their drivers start on the far side
//! of their cars, and get in at the nearer door. Skipped without an install.

use std::collections::BTreeMap;
use uc_formats::ucm::trigger;
use uc_game::{Controls, Game, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn gang_order_1s_patrol_cars_drive_to_their_points_and_park() {
    let Some(game) = uc_formats::default_install() else { return };
    let mut level = Level::load(&game, "Gangorder1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    level.mission.event_points.iter_mut().find(|e| e.index == 125).unwrap().trigger = trigger::NONE;
    let mut g = Game::new(&level, true);
    // Each patrol car, from where it was when its driver took the wheel.
    let mut from = BTreeMap::new();
    for _ in 0..(30.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        for (vi, v) in g.vehicles.iter().enumerate() {
            if let Some(pi) = v.driver.filter(|&pi| {
                let p = &g.people[pi];
                !p.player && matches!(p.mv, 2 | 3) && matches!(p.state, State::InCar { .. })
            }) {
                from.entry(vi).or_insert((v.pos, pi));
            }
        }
    }
    assert_eq!(from.len(), 3, "patrol cars: {from:?}");
    for (&vi, &(start, pi)) in &from {
        let p = &g.people[pi];
        let point = level.mission.event_points.iter()
            .find(|e| e.used != 0 && e.kind == 1 && e.colour == p.colour && e.group == p.group)
            .expect("its point");
        let at = glam::vec2(point.x as f32, point.z as f32);
        let v = &g.vehicles[vi];
        let (now, was) = (glam::vec2(v.pos.x, v.pos.z), glam::vec2(start.x, start.z));
        assert!((now - at).length() < 0x300 as f32 + 128.0, "car {vi} isn't at its point: {now:?} vs {at:?}");
        assert!(v.speed.abs() < 30.0, "car {vi} is still going");
        if (was - at).length() > 0x300 as f32 + 256.0 {
            assert!((now - was).length() > 512.0, "car {vi} never set off");
        }
    }
}
