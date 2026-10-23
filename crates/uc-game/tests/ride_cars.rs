//! Riding on cars (`WMOVE`, fallen/Source/wmove.cpp): every vehicle has walkable faces on
//! top that move with it, so Darci can stand on one, be carried along and get off again; a
//! van's she can catch the edge of from a jump.
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

/// Darci, `from` units off vehicle `vi`'s side, turned to type `kind`, walks at it and jumps,
/// for `secs`, the car going at `speed` once she's pulling up. Returns the states she went through.
fn jump_at(kind: u8, from: f32, speed: f32, secs: f32) -> Option<(Level, Game, usize, Vec<State>)> {
    let (level, mut g, vi) = load()?;
    g.vehicles[vi].kind = kind;
    for _ in 0..30 {
        g.update(DT, &Controls::default(), &level);
    }
    let pi = g.player.unwrap();
    let v = g.vehicles[vi].clone();
    let side = vec2(v.forward().y, -v.forward().x);
    let at = vec2(v.pos.x, v.pos.z) + side * from;
    let y = level.ground.surface(&level.map, at.x, at.y, v.pos.y + 32.0);
    g.people[pi].fig.pos = vec3(at.x, y, at.y);
    g.people[pi].fig.angle = uc_world::character::angle_of(-side);
    g.people[pi].state = State::Move;
    let mut seen: Vec<State> = Vec::new();
    for f in 0..(secs / DT) as usize {
        let c = Controls { wish: if f < 45 { -side } else { vec2(0.0, 0.0) }, jump: f == 6, ..Default::default() };
        if seen.iter().any(|s| matches!(s, State::PullUp { .. })) {
            g.vehicles[vi].speed = speed;
        }
        g.update(DT, &c, &level);
        let s = g.people[pi].state.clone();
        if seen.last().is_none_or(|l| std::mem::discriminant(l) != std::mem::discriminant(&s)) {
            seen.push(s);
        }
    }
    Some((level, g, vi, seen))
}

/// The top of vehicle `vi`'s highest face, and where Darci is in the car's own space.
fn roof_and_local(level: &Level, g: &Game, vi: usize) -> (f32, glam::Vec2) {
    let v = &g.vehicles[vi];
    let (body, wheel, ..) = vehicle::TYPES[v.kind as usize];
    let roof = v.tops(&level.item_prims[&body], vehicle::wheel_radius(level.item_prims.get(&wheel)))
        .map(|f| f.at(vec2(0.5, 0.5)).y)
        .fold(f32::MIN, f32::max);
    let d = vec2(g.people[g.player.unwrap()].fig.pos.x - v.pos.x, g.people[g.player.unwrap()].fig.pos.z - v.pos.z);
    let f = v.forward();
    (roof, vec2(d.dot(f), d.dot(vec2(f.y, -f.x))))
}

/// A van's top has edges to hold (`FACE_FLAG_SLIDE_EDGE_*`, wmove.cpp:510): jumping at its
/// side, Darci catches the edge of the roof and, it being a moving face, pulls straight up
/// onto it (Person.cpp:13009).
#[test]
fn darci_grabs_a_vans_edge_and_pulls_up() {
    let Some((level, g, vi, seen)) = jump_at(0, 220.0, 0.0, 4.0) else { return };
    println!("{seen:?}");
    assert!(seen.iter().any(|s| matches!(s, State::PullUp { .. })), "she should catch the edge: {seen:?}");
    let (roof, at) = roof_and_local(&level, &g, vi);
    let p = &g.people[g.player.unwrap()];
    assert!(matches!(p.state, State::Move) && (p.fig.pos.y - roof).abs() < 8.0, "she should be up on it: {:?} at {} for {roof}", p.state, p.fig.pos.y);
    assert!(at.y.abs() < 120.0 && at.x.abs() < 170.0, "and on top of it: {at:?}");
}

/// The edge moves with the van: pulling up as it drives off, she still ends up on its roof.
#[test]
fn a_vans_edge_moves_with_it() {
    let Some((level, g, vi, seen)) = jump_at(8, 220.0, 300.0, 4.0) else { return };
    let (roof, at) = roof_and_local(&level, &g, vi);
    let p = &g.people[g.player.unwrap()];
    println!("{seen:?}, she's at {at:?} on it, {} for {roof}", p.fig.pos.y);
    assert!(seen.iter().any(|s| matches!(s, State::PullUp { .. })), "she should catch the edge: {seen:?}");
    assert!(g.vehicles[vi].speed > 200.0, "the van should be going");
    assert!((p.fig.pos.y - roof).abs() < 24.0 && at.y.abs() < 120.0 && at.x.abs() < 170.0, "she should be up on it: at {at:?}, {} for {roof}", p.fig.pos.y);
}

/// The other types' faces have no edges to hold: a jump at a jeep's side, whose roof is as
/// out of a jump's reach as a van's, isn't caught on it, nor at a car's.
#[test]
fn a_cars_top_has_no_edge_to_grab() {
    for kind in [5, 1] {
        let Some((.., seen)) = jump_at(kind, 220.0, 0.0, 3.0) else { return };
        println!("type {kind}: {seen:?}");
        assert!(!seen.iter().any(|s| matches!(s, State::PullUp { .. } | State::Dangle { .. })), "type {kind}: {seen:?}");
    }
}

