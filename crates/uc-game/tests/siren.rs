//! The siren (#206, the manual's pursuit mode): `VEH_SIREN` switches it on and off in the
//! police car, the ambulance and the meat wagon, with their light bars flashing, and it goes
//! off when the driver gets out (fallen/Source/vehicle.cpp:3548). Bronze's squad car (point 2)
//! in the driving test. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::vehicle::{self, Key, Vehicle};
use uc_game::{Controls, Drive, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load() -> Option<(Level, Game, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "testdrive1a", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
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

/// The lights are in the light bar, on top of the body about its middle.
#[test]
fn the_light_bars_are_on_the_roof() {
    let Some((level, ..)) = load() else { return };
    for (kind, &(body, wheel, ..)) in vehicle::TYPES.iter().enumerate() {
        let v = Vehicle::new(0, kind as u8, vec3(1000.0, 0.0, 1000.0), 0.3, Key::None);
        let lift = vehicle::wheel_radius(level.item_prims.get(&wheel)) - vehicle::WHEEL_ABOVE_BODY;
        let Some(([a, b], _)) = v.light_bar(lift) else {
            assert!(!v.has_siren(), "type {kind}");
            continue;
        };
        let (_, hi) = uc_world::level::prim_bounds(&level.item_prims[&body]);
        let top = hi.y + lift;
        for at in [a, b] {
            println!("type {kind}: light at {at:?}, the body's top {top}");
            // In the light bar: the top 40 of the body.
            assert!(at.y <= top && at.y > top - 40.0, "type {kind}'s light is at {} for a top at {top}", at.y);
            assert!((vec2(at.x, at.z) - vec2(1000.0, 1000.0)).length() < 80.0, "type {kind}'s light is off the middle");
        }
        assert!((a - b).length() > 60.0, "type {kind}'s lights are a pair");
    }
}

#[test]
fn the_siren_goes_on_and_off_and_stops_when_she_gets_out() {
    let Some((level, mut g, vi)) = load() else { return };
    assert!(g.vehicles[vi].has_siren(), "the squad car is a police car");
    let pi = g.player.unwrap();
    // Into the car.
    let door = g.vehicles[vi].door();
    g.people[pi].fig.pos = vec3(door.x, g.people[pi].fig.pos.y, door.y);
    for frame in 0..(10.0 / DT) as usize {
        if g.player_vehicle().is_some() {
            break;
        }
        g.update(DT, &Controls { action: frame % 10 == 0, ..Default::default() }, &level);
    }
    assert!(g.player_vehicle().is_some(), "she should be in the car");
    let press = |g: &mut Game, siren: bool| g.update(DT, &Controls { drive: Drive { siren, ..Default::default() }, ..Default::default() }, &level);
    // A press switches it on, and holding still leaves it on.
    press(&mut g, true);
    assert!(g.vehicles[vi].siren, "the first press switches it on");
    for _ in 0..30 {
        press(&mut g, false);
    }
    assert!(g.vehicles[vi].siren);
    assert!(g.siren_noise(g.vehicles[vi].pos).is_some(), "it can be heard");
    press(&mut g, true);
    assert!(!g.vehicles[vi].siren, "the next switches it off");
    press(&mut g, true);
    assert!(g.vehicles[vi].siren);
    // Out of the car: off.
    for frame in 0..(10.0 / DT) as usize {
        if g.player_vehicle().is_none() && g.vehicles[vi].driver.is_none() {
            break;
        }
        g.update(DT, &Controls { action: frame % 10 == 0, ..Default::default() }, &level);
    }
    g.update(DT, &Controls::default(), &level);
    assert!(g.vehicles[vi].driver.is_none(), "she should be out");
    assert!(!g.vehicles[vi].siren, "the siren stops with nobody at the wheel");
    assert!(g.siren_noise(g.vehicles[vi].pos).is_none());
}
