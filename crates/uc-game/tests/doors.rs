//! Doors: solid until the mission opens one, which is the only way one ever opens
//! (`DOOR_open`'s only caller is `EWAY_DO_CONTROL_DOOR`). Skipped without an install.

use glam::{vec2, Vec2};
use uc_game::Game;
use uc_world::mav::{caps, Mav, GOTO};
use uc_world::{Level, LoadOptions};

fn load(mission: &str) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap())
}

/// Media Trouble's one `activate_prim` point opens a door, and has no trigger, so it fires as
/// the mission starts.
#[test]
fn a_mission_opens_its_door() {
    let Some(level) = load("e3") else { return };
    assert!(!level.ground.doors.is_empty(), "the map should have doors");
    assert!(level.ground.doors.iter().all(|d| !d.is_open()), "they all start shut");
    let shut = level.ground.blocking().count();

    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());

    let opened = level.ground.doors.iter().filter(|d| d.is_open()).count();
    assert_eq!(opened, 1, "the mission opens one of them");
    assert_eq!(level.ground.blocking().count(), shut - 1, "an open door stops blocking");
}

/// A shut door turns someone back; the same door open lets them through.
#[test]
fn a_shut_door_blocks_the_way() {
    let Some(level) = load("botanicc") else { return };
    // A door tall enough to walk into rather than step over.
    let door = level.ground.doors.iter().find(|d| d.wall.y1 - d.wall.y0 > 128.0).expect("a full-height door");
    let (a, b) = (Vec2::from(door.wall.a), Vec2::from(door.wall.b));
    let ab = b - a;
    let through = vec2(-ab.y, ab.x).normalize();
    let mid = door.mid;
    let feet = door.wall.y0;

    // Walk at it the way a person does, a frame at a time: `slide` pushes out of a wall, so
    // it wants the steps a run actually takes rather than one long stride over the top.
    let walk = |ground: &uc_world::ground::Ground| {
        let mut p = mid - through * 200.0;
        for _ in 0..40 {
            p = ground.slide(p, feet, through * 17.0, 40.0);
        }
        (p - mid).dot(through)
    };

    let shut = walk(&level.ground);
    assert!(shut < 0.0, "a shut door should turn you back, but they got {shut:.0} past it");

    assert!(level.ground.open_door_near(mid.x, mid.y).is_some(), "there is a door to open here");
    let open = walk(&level.ground);
    assert!(open > 0.0, "an open door should let you through, but they got {open:.0} past it");
}

/// `DOOR_find` looks within two squares of the point and takes the nearest; nothing in range
/// means nothing opens.
#[test]
fn only_a_door_near_the_point_opens() {
    let Some(level) = load("botanicc") else { return };
    let far = vec2(1.0, 1.0);
    assert!(level.ground.open_door_near(far.x, far.y).is_none(), "the map corner has no door to open");
    assert!(level.ground.doors.iter().all(|d| !d.is_open()), "and nothing should have opened");
}

/// A door the mission opens is opened on the navigation map too, so people finding their own
/// way can go through it (`DOOR_open`, door.cpp:206; #439): each square along it can be
/// walked out of across it, both ways. Media Trouble opens its door as it starts, after the
/// map was worked out with it shut.
#[test]
fn an_opened_door_opens_the_navigation_map() {
    let Some(level) = load("e3") else { return };
    // Along x or z, the squares either side of each stretch of it, and the ways across.
    let across = |m: &Mav, d: &uc_world::ground::Door| -> Vec<bool> {
        let (a, b) = (Vec2::from(d.wall.a) / 256.0, Vec2::from(d.wall.b) / 256.0);
        let along_x = a.y == b.y;
        let (lo, hi) = if along_x { (a.x.min(b.x), a.x.max(b.x)) } else { (a.y.min(b.y), a.y.max(b.y)) };
        (lo as i32..hi as i32).flat_map(|k| {
            let line = if along_x { a.y } else { a.x } as i32;
            // (square before the line, way over it), (square after it, way back)
            let (before, after) = if along_x { ((k, line - 1), (k, line)) } else { ((line - 1, k), (line, k)) };
            let (fwd, back) = if along_x { (3, 2) } else { (1, 0) };
            [m.opt(before.0, before.1, fwd), m.opt(after.0, after.1, back)].map(|o| o & caps(GOTO) != 0)
        }).collect()
    };
    let shut = Mav::new(&level);
    let door = &level.ground.doors[0];
    assert!(across(&shut, door).iter().all(|&w| !w), "the shut door can be walked through");
    let g = Game::new(&level, true);
    assert!(door.is_open(), "the mission opened its door");
    assert!(across(g.nav(), door).iter().all(|&w| w), "still shut on the navigation map");
}
