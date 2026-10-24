//! Where people are made (`create_person`, Person.cpp:724, and `find_face_for_this_pos`,
//! walkable.cpp:466): on the surface nearest their point's own height, no more than 0xa0
//! above it, or the ground within 0x50; with neither, in the air, to fall. Skipped without
//! an install.

mod common;

use common::Run;
use uc_game::State;
use uc_formats::ucm::waypoint;

/// The missions in the menu (`PLAYABLE`, uc-viewer's menu.rs).
const PLAYABLE: [&str; 35] = ["ftutor1", "assault1", "testdrive1a", "fight1", "fight2", "police1", "testdrive2",
    "testdrive3", "police2", "Bankbomb1", "police3", "Gangorder2", "Gordout1", "Stealtst1", "Baalrog3",
    "Gangorder1", "police4", "bball2", "wstores1", "e3", "westcrime1", "carbomb1", "botanicc", "park2",
    "factory1", "snow2", "MIB", "Semtex", "estate2", "mission2", "skymiss2", "AWOL1", "Finale1", "FreeCD1", "jung3"];

/// Whether someone made at (x, y, z) is left in the air: made at the point's own height,
/// with nothing within a step under it.
fn in_air(level: &uc_world::Level, x: f32, y: f32, z: f32) -> bool {
    level.ground.spawn_height(&level.map, x, z, y) == y && level.ground.surface(&level.map, x, z, y) < y - uc_world::ground::STEP_HEIGHT
}

/// Every person's point in every playable mission: they're made on the surface nearest it,
/// never more than 0xa0 above it, or with none on the ground within 0x50 of it, or under it.
/// Otherwise they're left where the point is, in the air, to fall: bball2's warehouse points
/// and factory1's points 133 and 156, among others. None of them has a roof or a prop over a
/// point in the street that the old rule (the highest surface up to half a storey above)
/// picked over a nearer one.
#[test]
fn people_are_made_on_the_surface_nearest_their_point() {
    let (mut points, mut aloft) = (0, Vec::new());
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
            if in_air(&level, x, y, z) {
                println!("{mission} point {}: at {y} over ground {ground}, left there", e.index);
                aloft.push((mission, e.index));
            }
        }
    }
    assert!(points > 500, "{points} points");
    for m in [("factory1", 133), ("factory1", 156)] {
        assert!(aloft.contains(&m), "{m:?} isn't left in the air");
    }
    assert!(aloft.iter().any(|&(m, _)| m == "bball2"), "none of bball2's warehouse points is left in the air");
}

/// Someone made in the air falls, and lands: bball2's warehouse people, 256 over its floor,
/// and factory1's, 1024 over the ground (point 307 grabs a ledge beside him on the way down,
/// and pulls up onto it).
#[test]
fn people_made_in_the_air_fall() {
    for mission in ["bball2", "factory1"] {
        let Some(level) = common::load(mission) else { return };
        let aloft: Vec<usize> = level.mission.event_points.iter()
            .filter(|e| e.used != 0 && matches!(e.kind, waypoint::CREATE_PLAYER | waypoint::CREATE_ENEMIES))
            .filter(|e| in_air(&level, e.x as f32, e.y as f32, e.z as f32))
            .map(|e| e.index)
            .collect();
        let mut r = Run::new(level);
        // Who's made in the air (checked on the turn they're made, falling), whether they've
        // dropped since, and whether they've then stood on something.
        let mut made: Vec<(usize, usize, bool, bool)> = Vec::new();
        for _ in 0..(20.0 / common::DT) as usize {
            r.play(common::DT, Default::default());
            for (i, p) in r.g.people.iter().enumerate() {
                let point = || r.point(p.way);
                match made.iter_mut().find(|m| m.0 == i) {
                    Some(m) => {
                        let at = p.fig.pos;
                        let floor = r.level.ground.surface(&r.level.map, at.x, at.z, at.y);
                        m.2 |= at.y < point().y - 32.0;
                        // On their feet again, or dancing as their move has them do.
                        m.3 |= m.2 && matches!(p.state, State::Move | State::Pose { .. }) && (at.y - floor).abs() < 8.0;
                    }
                    None if aloft.contains(&p.way) => {
                        assert!(matches!(p.state, State::Fall { .. }), "{mission} point {}: made {:?}", p.way, p.state);
                        assert!((p.fig.pos.y - point().y).abs() < 40.0, "{mission} point {}: made at {}, its point at {}", p.way, p.fig.pos, point());
                        made.push((i, p.way, false, false));
                    }
                    None => {}
                }
            }
        }
        println!("{mission}: {} of {} points in the air made", made.len(), aloft.len());
        assert!(!made.is_empty(), "{mission}: nobody made in the air");
        for (i, way, fell, landed) in made {
            println!("{mission} point {way}: now at {} ({:?})", r.g.people[i].fig.pos, r.g.people[i].state);
            assert!(fell && landed, "{mission} point {way}: fell {fell}, landed {landed}, from {}", r.point(way));
        }
    }
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
