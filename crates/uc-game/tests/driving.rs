//! Drives Driving Bronze (testdrive1a) with a bot: walk to the squad car, get in, three laps
//! of the oval through the mission's checkpoints. Skipped without an install.

use glam::{vec2, Vec2};
use uc_game::{Controls, Drive, Game, Outcome, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn bot_drives_three_laps() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "testdrive1a", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    // The road round the island, through the lap checkpoints (enemy_radius points 58..63):
    // the checkpoints, with corners added where the straight line would cross the island.
    let point = |i: usize| {
        let e = level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec2(e.x as f32, e.z as f32)
    };
    let sq = |x: f32, z: f32| vec2(x, z) * 256.0;
    let checkpoints: Vec<Vec2> = vec![
        point(58), point(59), point(60), sq(60.0, 92.0), point(61), sq(48.0, 91.0), point(62), point(63),
        sq(48.0, 30.0), sq(60.0, 30.0),
    ];
    let mut next = 0;
    // Seconds stuck against something, and seconds left of backing away.
    let (mut stuck, mut backing) = (0.0f32, 0.0f32);
    let mut seen = Vec::new();
    for frame in 0..(240.0 / DT) as usize {
        g.skip_cutscene();
        let p = g.player().unwrap();
        let pos = vec2(p.fig.pos.x, p.fig.pos.z);
        let mut c = Controls::default();
        match g.player_vehicle() {
            None => {
                // Walk to the car and get in.
                let car = &g.vehicles[0];
                let door = car.door();
                let d = door - pos;
                if frame % 60 == 0 {
                    println!("  {:.1}s darci ({:.0},{:.0}) {:?} door ({:.0},{:.0}) car ({:.0},{:.0}) cutscene {}", g.time, pos.x, pos.y, p.state, door.x, door.y, car.pos.x, car.pos.z, g.cutscene.is_some());
                }
                // Next to the car: get in; otherwise walk to its door.
                if (vec2(car.pos.x, car.pos.z) - pos).length() < 300.0 {
                    c.action = frame % 10 == 0;
                } else {
                    c.wish = d.normalize();
                    c.run = true;
                }
            }
            Some(car) => {
                let at = vec2(car.pos.x, car.pos.z);
                if (checkpoints[next] - at).length() < 700.0 {
                    next = (next + 1) % checkpoints.len();
                }
                let want = (checkpoints[next] - at).normalize();
                let f = car.forward();
                // Positive cross: the target is to the car's right (steer +1 turns right).
                let cross = f.x * want.y - f.y * want.x;
                let ahead = f.dot(want);
                let steer = (cross * 3.0).clamp(-1.0, 1.0);
                // Brake for bends: slow down when the checkpoint after next is round a corner.
                let dist = (checkpoints[next] - at).length();
                let after = (checkpoints[(next + 1) % checkpoints.len()] - checkpoints[next]).normalize();
                let bend = 1.0 - want.dot(after).max(0.0);
                let target = if ahead < 0.7 { 900.0 } else if dist < 2500.0 { 3000.0 - bend * 2000.0 } else { 3000.0 };
                let throttle = if car.speed > target + 100.0 { -1.0 } else { 1.0 };
                stuck = if car.speed.abs() < 60.0 && backing <= 0.0 { stuck + DT } else { 0.0 };
                if stuck > 0.5 {
                    backing = 1.0;
                    stuck = 0.0;
                }
                c.drive = if backing > 0.0 {
                    backing -= DT;
                    Drive { throttle: -1.0, steer: -steer.signum(), handbrake: false }
                } else {
                    Drive { throttle, steer, handbrake: false }
                };
            }
        }
        g.update(DT, &c, &level);
        for m in &g.messages {
            if !seen.contains(&m.text) {
                println!("[{:6.1}s] {}", g.time, m.text);
                seen.push(m.text.clone());
            }
        }
        if std::env::var("DRIVE_TRACE").is_ok() && frame % 6 == 0 && g.time < 8.0 {
            if let Some(car) = g.player_vehicle() {
                println!("  {:.1}s car ({:.0},{:.0}) yaw {:.2} fwd {:.2?} speed {:.0} steer {:.2} drive {:?}", g.time, car.pos.x, car.pos.z, car.yaw, car.forward(), car.speed, car.steer, c.drive);
            }
        }
        if frame % 150 == 0 {
            if let Some(car) = g.player_vehicle() {
                println!("  {:.0}s car at ({:.0}, {:.0}) speed {:.0} health {} next {}", g.time, car.pos.x / 256.0, car.pos.z / 256.0, car.speed, car.health, next);
            }
        }
        if g.outcome.is_some() {
            break;
        }
    }
    println!("outcome {:?}, in car {}", g.outcome, matches!(g.player().unwrap().state, State::InCar { .. }));
    assert_eq!(g.outcome, Some(Outcome::Won));
}

#[test]
fn driving_silver_and_gold_scripts_run() {
    let Some(game) = uc_formats::default_install() else { return };
    for m in ["testdrive2", "testdrive3"] {
        let level = Level::load(&game, m, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
        let mut g = Game::new(&level, true);
        assert!(g.unsupported().is_empty(), "{m}: unsupported {:?}", g.unsupported());
        for _ in 0..(30.0 / DT) as usize {
            g.skip_cutscene();
            g.update(DT, &Controls::default(), &level);
        }
        println!("{m}: {} vehicles, {} cones and barrels, messages {:?}", g.vehicles.len(), g.barrels.len(),
                 g.messages.iter().map(|m| &m.text).collect::<Vec<_>>());
        assert_eq!(g.vehicles.len(), 1);
        assert!(g.barrels.len() > 100);
    }
}

#[test]
fn people_get_out_of_the_cars_way() {
    // Driving Silver lost with "the training officer has been killed" when the car mounted
    // the kerb where he stands: now he runs out of the way, as in the original.
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "testdrive2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(5.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let officer = g.people.iter().position(|p| p.way == 198).expect("the training officer");
    let pi = g.player.unwrap();
    // Darci at the wheel, aimed at him, foot down.
    g.vehicles[0].driver = Some(pi);
    g.people[pi].state = State::InCar { vehicle: 0 };
    let (car, at) = (g.vehicles[0].pos, g.people[officer].fig.pos);
    g.vehicles[0].yaw = (at.x - car.x).atan2(at.z - car.z);
    let foot_down = Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() };
    let mut closest = f32::MAX;
    for _ in 0..(4.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &foot_down, &level);
        let (v, o) = (&g.vehicles[0], &g.people[officer]);
        closest = closest.min(vec2(v.pos.x - o.fig.pos.x, v.pos.z - o.fig.pos.z).length());
    }
    let o = &g.people[officer];
    println!("closest {closest:.0}, officer health {}/{} {:?}, outcome {:?}", o.health, o.max_health, o.state, g.outcome);
    assert_eq!(o.health, o.max_health, "the officer was hit");
    assert!(g.outcome.is_none(), "{:?}", g.outcome);
}

#[test]
fn driving_silver_swaps_its_instructor_without_killing_him() {
    // Passing the raised section (point 250) moves the instructor to the finish: a
    // kill_waypoint takes the first one away and a new one appears. That isn't a death.
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "testdrive2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(5.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let pi = g.player.unwrap();
    g.vehicles[0].driver = Some(pi);
    g.people[pi].state = State::InCar { vehicle: 0 };
    let e = level.mission.event_points.iter().find(|e| e.index == 250).unwrap();
    g.vehicles[0].pos.x = e.x as f32;
    g.vehicles[0].pos.z = e.z as f32;
    for _ in 0..(3.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let msgs: Vec<_> = g.messages.iter().map(|m| m.text.clone()).collect();
    println!("{:?} {msgs:?}", g.outcome);
    assert!(g.people.iter().any(|p| p.way == 251), "no new instructor at the finish");
    assert!(g.outcome.is_none(), "{:?}", g.outcome);
}

#[test]
fn the_handbrake_slides_the_car_round_corners() {
    // Flat out, then hard left: with the handbrake the car turns further and slides
    // sideways; without, it grips.
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "testdrive1a", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let corner = |handbrake: bool| {
        let mut v = g_car(&level);
        let go = Drive { throttle: 1.0, ..Default::default() };
        for _ in 0..(1.5 / DT) as usize {
            v.update(go, DT, &level.ground, &level.map);
        }
        let (yaw0, mut slip) = (v.yaw, 0.0f32);
        for _ in 0..(0.6 / DT) as usize {
            v.update(Drive { throttle: 0.5, steer: -1.0, handbrake }, DT, &level.ground, &level.map);
            slip = slip.max(v.slip.abs());
        }
        ((v.yaw - yaw0).abs(), slip, v.speed.hypot(v.slip))
    };
    let (turn_grip, slip_grip, _) = corner(false);
    let (turn_slide, slip_slide, speed) = corner(true);
    println!("grip: turned {turn_grip:.2} slip {slip_grip:.0}; handbrake: turned {turn_slide:.2} slip {slip_slide:.0} speed {speed:.0}");
    assert!(turn_slide > turn_grip * 1.3, "the handbrake didn't swing the back out");
    assert!(slip_slide > slip_grip * 2.0 && slip_slide > 300.0, "no slide");
    assert!(speed > 600.0, "the handbrake stopped the car instead of sliding it");
}

#[test]
fn a_handbrake_turn_swings_the_car_right_round() {
    // At a good lick, full lock and the handbrake: the car spins through 180 degrees (a
    // bootlegger turn), ready to drive off the way it came.
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "testdrive1a", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut v = g_car(&level);
    for _ in 0..(0.8 / DT) as usize {
        v.update(Drive { throttle: 1.0, ..Default::default() }, DT, &level.ground, &level.map);
    }
    let (start, speed) = (v.forward(), v.speed);
    let mut t = 0.0;
    while v.forward().dot(start) > -0.95 && t < 3.0 {
        v.update(Drive { throttle: 0.0, steer: 1.0, handbrake: true }, DT, &level.ground, &level.map);
        t += DT;
    }
    println!("from {speed:.0}: facing back after {t:.2}s, still moving {:.0}", v.speed.hypot(v.slip));
    assert!(t < 1.5, "the car didn't come round");
}

/// The squad car at its start, on its own (no mission running).
fn g_car(level: &Level) -> uc_game::Vehicle {
    let g = Game::new(level, true);
    g.vehicles[0].clone()
}
