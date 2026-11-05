//! Getting into a car: she stands where the original stands her for the animation
//! (`position_person_for_vehicle`), so it carries her into the seat, not out beside the
//! front wheel; and the door swings open while she gets in, and shut again (not in the
//! original). Skipped without an install.

use glam::{Vec2, Vec3};
use uc_game::vehicle::Key;
use uc_game::{Controls, Game, State, Vehicle};
use uc_world::Level;

mod common;
use common::DT;

/// Rio Canal with a car of `kind` in an open spot, and her at door `door`: (level, game, car).
fn at_door(kind: u8, door: usize) -> Option<(Level, Game, usize)> {
    let level = common::load("wstores1")?;
    let mut g = Game::new(&level, true);
    common::frames(&mut g, &level, 60);
    let me = g.player().unwrap().fig.pos;
    let grid = uc_game::game::endless::arena(&level, me, 30.0 * 256.0);
    let open = grid.iter()
        .find(|((x, z), _)| (-4..=4).all(|dx| (-4..=4).all(|dz| grid.contains_key(&(x + dx, z + dz)))))
        .map(|(_, (at, _))| *at)?;
    g.vehicles.push(Vehicle::new(0, kind, open, 0.3, Key::None));
    let vi = g.vehicles.len() - 1;
    let v = &g.vehicles[vi];
    // Beside the door, as the driver's or the other.
    let d = if door == 0 { v.door() } else { v.to_world(-200.0, 20.0) };
    let pi = g.player.unwrap();
    g.people[pi].fig.pos = Vec3::new(d.x, open.y, d.y);
    Some((level, g, vi))
}

#[test]
fn getting_in_ends_in_the_seat() {
    for kind in [7, 1, 0] {
        for door in [0, 1] {
            let Some((level, mut g, vi)) = at_door(kind, door) else { return };
            let pi = g.player.unwrap();
            g.update(DT, &Controls { action: true, ..Default::default() }, &level);
            assert!(matches!(g.people[pi].state, State::Enter { .. }), "kind {kind} door {door}: not getting in");
            let mut last = Vec3::ZERO;
            for _ in 0..150 {
                if !matches!(g.people[pi].state, State::Enter { .. }) {
                    break;
                }
                last = g.people[pi].fig.hips();
                g.update(DT, &Controls::default(), &level);
            }
            assert!(matches!(g.people[pi].state, State::InCar { vehicle } if vehicle == vi));
            // Her hips at the end, in the car's own space: inside its sides, and between its
            // axles, not out by the front wheel.
            let v = &g.vehicles[vi];
            let (s, c) = v.yaw.sin_cos();
            let (dx, dz) = (last.x - v.pos.x, last.z - v.pos.z);
            let (x, z) = (c * dx - s * dz, s * dx + c * dz);
            let (_, _, wx, front, back) = uc_game::vehicle::TYPES[kind as usize];
            assert!(x.abs() < wx, "kind {kind} door {door}: out of the side, at x {x:.0}");
            assert!(back < z && z < front * 0.6, "kind {kind} door {door}: at z {z:.0}, not in a seat");
            assert!(v.covers(Vec2::new(last.x, last.z), 0.0));
        }
    }
}

#[test]
fn the_door_swings_open_and_shut() {
    let Some((level, mut g, vi)) = at_door(7, 0) else { return };
    assert_eq!(g.vehicles[vi].door_open(), None);
    g.update(DT, &Controls { action: true, ..Default::default() }, &level);
    let mut widest: f32 = 0.0;
    for _ in 0..90 {
        g.update(DT, &Controls::default(), &level);
        if let Some((door, open)) = g.vehicles[vi].door_open() {
            assert_eq!(door, 0);
            widest = widest.max(open);
        }
    }
    assert!(widest > 0.99, "never all the way open: {widest}");
    assert_eq!(g.vehicles[vi].door_open(), None, "left open");
    // And getting out swings it again.
    g.update(DT, &Controls { action: true, ..Default::default() }, &level);
    g.update(DT, &Controls::default(), &level);
    assert!(g.vehicles[vi].door_open().is_some(), "not opened to get out");
}
