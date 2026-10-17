//! Where people are made (`create_person`, Person.cpp:724, and `find_face_for_this_pos`,
//! walkable.cpp:466): on the surface nearest their point's own height, no more than 0xa0
//! above it. Skipped without an install.

mod common;

use common::Run;
use uc_formats::ucm::waypoint;

/// The missions in the menu (`PLAYABLE`, uc-viewer's menu.rs).
const PLAYABLE: [&str; 35] = ["ftutor1", "assault1", "testdrive1a", "fight1", "fight2", "police1", "testdrive2",
    "testdrive3", "police2", "Bankbomb1", "police3", "Gangorder2", "Gordout1", "Stealtst1", "Baalrog3",
    "Gangorder1", "police4", "bball2", "wstores1", "e3", "westcrime1", "carbomb1", "botanicc", "park2",
    "factory1", "snow2", "MIB", "Semtex", "estate2", "mission2", "skymiss2", "AWOL1", "Finale1", "FreeCD1", "jung3"];

/// Every person's point in every playable mission: they're made on the surface nearest it,
/// never more than 0xa0 above it but on the street. Lists the points with no surface within a step, which the
/// original leaves where they are, to fall. None of them has a roof or a prop over a point
/// in the street that the old rule (the highest surface up to half a storey above) picked
/// over a nearer one.
#[test]
fn people_are_made_on_the_surface_nearest_their_point() {
    let mut points = 0;
    for mission in PLAYABLE {
        let Some(level) = common::load(mission) else { return };
        for e in &level.mission.event_points {
            if e.used == 0 || !matches!(e.kind, waypoint::CREATE_PLAYER | waypoint::CREATE_ENEMIES) {
                continue;
            }
            points += 1;
            let (x, y, z) = (e.x as f32, e.y as f32, e.z as f32);
            let (at, ground) = (level.ground.spawn_height(&level.map, x, z, y), level.map.ground_height(x, z));
            // Points under the street are put on it.
            assert!(at - y <= 160.0 || at == ground, "{mission} point {}: made at {at}, {} above its point", e.index, at - y);
            if (at - y).abs() > uc_world::ground::STEP_HEIGHT {
                println!("{mission} point {}: at {y} over ground {ground}, made at {at}", e.index);
            }
        }
    }
    assert!(points > 500, "{points} points");
}

/// The Jump's jumper (`police2` point 54) is 64 below his ledge, and is made on it.
#[test]
fn the_jumper_is_made_on_his_ledge() {
    let Some(level) = common::load("police2") else { return };
    let mut r = Run::new(level);
    r.play(0.5, Default::default());
    let i = r.who(54);
    let (point, at) = (r.point(54), r.g.people[i].fig.pos);
    assert!((at.y - (point.y + 64.0)).abs() < 1.0, "the jumper is at {at}, his point at {point}");
}
