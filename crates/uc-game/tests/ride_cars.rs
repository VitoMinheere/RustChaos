//! Riding on cars (`WMOVE`, fallen/Source/wmove.cpp): every vehicle has walkable faces on
//! top that move with it, so Darci can stand on one, be carried along and get off again.
//! Bronze's squad car (point 2) in the driving test. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::vehicle::{self, Key, Vehicle};
use uc_game::{Controls, Game, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load() -> Option<(Level, Game, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "testdrive1a", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    g.view_dist = f32::MAX;
    for _ in 0..(120.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() && g.cutscene.is_none() {
            break;
        }
    }
    let vi = g.vehicles.iter().position(|v| v.way == 2)?;
    Some((level, g, vi))
}

/// Every type's faces are found in its body prim, and lie on top of it.
#[test]
fn every_vehicle_has_a_top() {
    let Some((level, ..)) = load() else { return };
    for (kind, &(body, wheel, ..)) in vehicle::TYPES.iter().enumerate() {
        let v = Vehicle::new(0, kind as u8, vec3(1000.0, 0.0, 1000.0), 0.3, Key::None);
        let prim = level.item_prims.get(&body).expect("body prim");
        let tops: Vec<_> = v.tops(prim, vehicle::wheel_radius(level.item_prims.get(&wheel))).collect();
        let want = match kind { 0 | 4 | 8 => 1, 5 | 6 => 4, _ => 5 };
        assert_eq!(tops.len(), want, "type {kind}");
        let (_, hi) = uc_world::level::prim_bounds(prim);
        for f in &tops {
            let mid = f.at(vec2(0.5, 0.5));
            println!("type {kind}: middle {mid:?}, a {:?}, b {:?}", f.a, f.b);
            assert!(mid.y > 60.0 && mid.y < hi.y + 60.0, "type {kind}'s face is at {}", mid.y);
            assert!((vec2(mid.x, mid.z) - vec2(1000.0, 1000.0)).length() < 300.0, "type {kind}'s face is off the car");
        }
    }
}

#[test]
fn darci_rides_a_car_and_gets_off() {
    let Some((level, mut g, vi)) = load() else { return };
    let pi = g.player.unwrap();
    // Standing on the roof: the highest of its faces.
    let car = g.vehicles[vi].clone();
    let (body, wheel, ..) = vehicle::TYPES[car.kind as usize];
    let roof = car.tops(&level.item_prims[&body], vehicle::wheel_radius(level.item_prims.get(&wheel)))
        .max_by(|a, b| a.at(vec2(0.5, 0.5)).y.total_cmp(&b.at(vec2(0.5, 0.5)).y))
        .unwrap();
    let on = roof.at(vec2(0.5, 0.5));
    let y = level.ground.surface(&level.map, on.x, on.z, on.y + 8.0);
    assert!((y - on.y).abs() < 1.0, "the roof should be walkable: {y} for {}", on.y);
    g.people[pi].fig.pos = on;
    g.people[pi].state = State::Move;
    for _ in 0..15 {
        g.update(DT, &Controls::default(), &level);
    }
    let p = g.people[pi].fig.pos;
    assert!((p.y - on.y).abs() < 4.0, "she should stand on it: at {} for {}", p.y, on.y);

    // Where she is in the car's own space, and its heading.
    let local = |g: &Game| {
        let v = &g.vehicles[vi];
        let d = vec2(g.people[pi].fig.pos.x - v.pos.x, g.people[pi].fig.pos.z - v.pos.z);
        let f = v.forward();
        vec2(d.dot(f), d.dot(vec2(f.y, -f.x)))
    };
    let (start, at) = (g.vehicles[vi].pos, local(&g));
    let turned = g.people[pi].fig.angle - g.vehicles[vi].yaw;
    // It drives off, round a bend.
    for f in 0..(3.0 / DT) as usize {
        g.vehicles[vi].speed = 700.0;
        g.vehicles[vi].steer = if f > 30 { 0.4 } else { 0.0 };
        g.update(DT, &Controls::default(), &level);
    }
    let moved = (g.vehicles[vi].pos - start).length();
    println!("the car went {moved:.0}, she's at {:?} on it (was {at:?}), {:?}", local(&g), g.people[pi].state);
    assert!(moved > 800.0, "the car should have gone somewhere: {moved:.0}");
    assert!((local(&g) - at).length() < 24.0, "she should have stayed where she was on it: {:?} for {at:?}", local(&g));
    let p = g.people[pi].fig.pos;
    let top = g.vehicles[vi].pos.y + (on.y - start.y);
    assert!((p.y - top).abs() < 24.0, "and on top: at {} for {top}", p.y);
    let turn = (g.people[pi].fig.angle - g.vehicles[vi].yaw - turned).sin().abs();
    assert!(turn < 0.1, "and turned with it");

    // It stops; she walks off the side and down to the road.
    for _ in 0..(2.0 / DT) as usize {
        g.vehicles[vi].speed = 0.0;
        g.update(DT, &Controls::default(), &level);
    }
    let v = g.vehicles[vi].clone();
    let side = vec2(v.forward().y, -v.forward().x);
    for _ in 0..(3.0 / DT) as usize {
        g.update(DT, &Controls { wish: side, ..Default::default() }, &level);
    }
    let p = g.people[pi].fig.pos;
    let off = (vec2(p.x, p.z) - vec2(v.pos.x, v.pos.z)).length();
    let road = level.ground.surface(&level.map, p.x, p.z, p.y + 8.0);
    println!("off at {off:.0} from the car, {:.0} above the road, {:?}", p.y - road, g.people[pi].state);
    assert!(off > 250.0, "she should have got off: {off:.0} from the car");
    assert!((p.y - v.pos.y).abs() < 48.0 && (p.y - road).abs() < 4.0, "and be down on the road: at {} for {}", p.y, v.pos.y);
    // And the car goes on without her.
    let was: Vec3 = g.people[pi].fig.pos;
    for _ in 0..(1.0 / DT) as usize {
        g.vehicles[vi].speed = 700.0;
        g.update(DT, &Controls::default(), &level);
    }
    assert!((g.people[pi].fig.pos - was).length() < 40.0, "she shouldn't go with it");
}

