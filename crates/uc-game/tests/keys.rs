//! Locked cars and keys (`create_vehicle`'s `VK_*`, `IT_KEY`): Media Trouble's police car at
//! point 139 asks for the red key, which opens for whoever picks up a key; RTA's parked
//! squad cars are `VK_LOCKED`, which nothing opens; Driving Bronze's is unlocked. Trying a
//! locked car says so. Skipped without an install.

use glam::Vec3;
use uc_game::vehicle::Key;
use uc_game::{Controls, Game, Item, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    // Everyone in sight, so nothing moves about under the test.
    g.view_dist = f32::MAX;
    // Past any opening scene: a filmed conversation holds the player still, action and all.
    for _ in 0..(120.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() && g.cutscene.is_none() {
            break;
        }
    }
    assert!(!g.in_conversation(), "{mission}: still talking");
    Some((level, g))
}

fn car(g: &Game, way: usize) -> usize {
    g.vehicles.iter().position(|v| v.way == way).unwrap_or_else(|| panic!("no car from point {way}"))
}

/// Stands the player at car `vi`'s door, presses action and lets a second pass, and then
/// however long getting in takes. Returns whether they got in.
fn try_door(level: &Level, g: &mut Game, vi: usize) -> bool {
    let pi = g.player.unwrap();
    let door = g.vehicles[vi].door();
    let y = level.ground.surface(&level.map, door.x, door.y, g.vehicles[vi].pos.y + 64.0);
    g.people[pi].fig.pos = Vec3::new(door.x, y, door.y);
    g.people[pi].state = State::Move;
    g.messages.clear();
    assert!(!g.in_conversation() && g.cutscene.is_none(), "a scene would hold her still");
    g.update(DT, &Controls { action: true, ..Default::default() }, level);
    for _ in 0..30 {
        g.update(DT, &Controls::default(), level);
    }
    for _ in 0..150 {
        if !matches!(g.people[pi].state, State::Enter { .. }) {
            break;
        }
        g.update(DT, &Controls::default(), level);
    }
    matches!(g.people[pi].state, State::InCar { vehicle } if vehicle == vi)
}

#[test]
fn a_keyed_car_opens_for_a_key() {
    let Some((level, mut g)) = load("e3") else { return };
    let vi = car(&g, 139);
    assert_eq!(g.vehicles[vi].key, Key::Key, "point 139 asks for the red key");
    let pi = g.player.unwrap();
    assert!(!g.people[pi].has_key);
    assert!(!try_door(&level, &mut g, vi), "got in without the key");
    assert!(g.messages.iter().any(|m| m.text == "The car is locked."), "nothing said: {:?}", g.messages.iter().map(|m| &m.text).collect::<Vec<_>>());
    // A key at her feet: she picks it up, and the car opens.
    let at = g.people[pi].fig.pos;
    g.items.push(Item { way: 0, kind: 1, pos: at });
    for _ in 0..10 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.people[pi].has_key, "the key wasn't picked up");
    assert!(try_door(&level, &mut g, vi), "the key didn't open the car");
}

#[test]
fn a_locked_car_stays_locked_even_with_a_key() {
    let Some((level, mut g)) = load("police1") else { return };
    // A squad car parked at the station.
    let vi = car(&g, 2);
    assert_eq!(g.vehicles[vi].key, Key::Never);
    let pi = g.player.unwrap();
    g.people[pi].has_key = true;
    assert!(!try_door(&level, &mut g, vi), "a VK_LOCKED car opened");
    assert!(g.messages.iter().any(|m| m.text == "The car is locked."), "nothing said");
}

#[test]
fn an_unlocked_car_opens() {
    // Driving Bronze's squad car, `VK_UNLOCKED`.
    let Some((level, mut g)) = load("testdrive1a") else { return };
    let vi = car(&g, 2);
    assert_eq!(g.vehicles[vi].key, Key::None);
    assert!(try_door(&level, &mut g, vi), "the unlocked car wouldn't open");
}
