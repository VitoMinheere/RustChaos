//! The editor's oldest missions (file versions 0 to 5, #470): every one loads with every
//! page it draws textured and runs; those whose maps are missing are played on stand-ins
//! (`uc_world::level::stand_in_map`); version 0's points are renumbered as later versions
//! number them (`Mission::renumber_version0`); and what the five with a story script does
//! happens. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Outcome};

use crate::common;
use common::{Run, DT};

/// Every mission in the install older than version 6.
const OLD: [&str; 16] = ["army1", "Assassin1", "bang", "downight", "e7", "ECTS3C", "four", "Rooftop1",
    "supply1", "test", "three", "three_n", "timetest", "tm3", "upperclass1", "wibble"];

fn start(mission: &str) -> Option<Run> {
    let mut r = common::start(mission)?;
    r.play(1.0, Controls::default());
    Some(r)
}

impl Run {
    /// Picks up the item point `way` puts down.
    fn fetch_checked(&mut self, way: usize) {
        let at = self.g.items.iter().find(|it| it.way == way).unwrap_or_else(|| panic!("no item from point {way}")).pos;
        self.stand(at);
        self.play(0.5, Controls::default());
        assert!(!self.g.items.iter().any(|it| it.way == way), "item {way} not picked up");
    }

    /// Stands on point `way` (the floor under it) and presses action there.
    fn stand_and_use(&mut self, way: usize) {
        self.stand(self.point(way));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(1.0, Controls::default());
    }
}

#[test]
fn every_old_mission_loads_textured_and_runs() {
    let Some(game) = uc_formats::default_install() else { return };
    for name in OLD {
        let level = common::load(name).unwrap();
        assert!(uc_formats::resolve(&game, &level.mission.map).is_some(), "{name}: no map");
        let pages = level.build().pages();
        let bare: Vec<u32> = pages.iter().copied().filter(|&p| level.texture(p).is_none()).collect();
        // Rooftop1's stand-in has one prim page (909) no clump in the install has.
        assert!(bare.len() * 20 <= pages.len(), "{name}: pages without a texture: {bare:?} of {}", pages.len());
        let mut r = Run::new(level);
        assert!(r.g.unsupported().is_empty(), "{name}: unsupported {:?}", r.g.unsupported());
        r.stop_at_outcome = false;
        r.play(30.0, Controls::default());
        assert!(!r.g.people[r.player()].is_dead(), "{name}: the player died standing still");
    }
}

/// The two version 0 files put down their people, cars and things once renumbered: e7 its
/// seven enemies, ECTS3C eleven people and the eleven things on its rooftops.
#[test]
fn version_0_missions_put_down_people_and_things() {
    let Some(r) = start("e7") else { return };
    assert_eq!(r.level.mission.event_points[0].kind, uc_formats::ucm::waypoint::CREATE_PLAYER);
    assert_eq!(r.g.people.iter().filter(|p| !p.player).count(), 7);
    let Some(r) = start("ECTS3C") else { return };
    assert_eq!(r.g.people.iter().filter(|p| !p.player).count(), 11);
    assert_eq!(r.g.items.len(), 11);
}

/// Rooftop1's lost `ROOFIN1.iam` is stood in for by `ROOFnoin.iam`, the same rooftops: all
/// but one of the file's points have a roof under them, and Darci starts up on one.
#[test]
fn rooftop1_is_on_the_rooftops() {
    let Some(game) = uc_formats::default_install() else { return };
    let Some(r) = start("Rooftop1") else { return };
    assert!(r.level.mission.map.eq_ignore_ascii_case(r"Data\ROOFnoin.iam"));
    let file = uc_formats::ucm::Mission::parse(&std::fs::read(uc_formats::resolve(&game, r"levels\Rooftop1.ucm").unwrap()).unwrap()).unwrap();
    let pts: Vec<_> = file.event_points.iter().filter(|e| e.used != 0).collect();
    let off: Vec<usize> = pts.iter().filter(|e| {
        let (x, y, z) = (e.x as f32, e.y as f32, e.z as f32);
        (r.level.ground.surface(&r.level.map, x, z, y + 40.0) - y).abs() > 40.0
    }).map(|e| e.index).collect();
    assert_eq!(off, [25], "points with no roof under them");
    let me = r.g.people[r.player()].fig.pos;
    assert!(me.y > 500.0, "Darci isn't up on the roof: {me}");
}

/// Radar Tower (`army1`): a beacon on the gate's switch, then one on the tower's console;
/// the system sabotaged (point 67, action up on the tower's platform) wins it. Its soldiers
/// (the old `ET_ARMY_SOLDIER`, 16) are grey thugs, not hostages.
#[test]
fn radar_tower_is_won_by_sabotaging_the_system() {
    use uc_world::character::person as ptype;
    let Some(mut r) = start("army1") else { return };
    assert!(r.said("Sabatage the rader tower."), "{:?}", r.seen);
    let soldier = r.who(4);
    assert_eq!(r.g.people[soldier].kind, ptype::THUG_GREY, "a soldier isn't a grey thug");
    assert!(!r.g.people.iter().any(|p| p.kind == ptype::HOSTAGE), "a soldier in a hostage's clothes");
    let labels = |r: &Run| r.g.beacons().into_iter().map(|(l, _)| l).collect::<Vec<_>>();
    assert!(labels(&r).iter().any(|l| l.contains("Gate switch")), "{:?}", labels(&r));
    assert!(!labels(&r).iter().any(|l| l.contains("Radar tower")), "{:?}", labels(&r));
    // Worked from where the switch is worked.
    assert!(r.stand_at_switch(42), "the gate's point isn't a switch");
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(1.0, Controls::default());
    assert!(r.said("Gate open"), "{:?}", r.seen);
    assert!(!labels(&r).iter().any(|l| l.contains("Gate switch")), "{:?}", labels(&r));
    let tower = r.g.beacons().into_iter().find(|(l, _)| l.contains("Radar tower")).expect("no beacon on the tower").1;
    assert!((tower - r.point(67)).length() < 64.0, "the beacon isn't on the console: {tower}");
    r.stand_and_use(67);
    assert!(r.said("System Sabataged"), "{:?}", r.seen);
    r.play(3.0, Controls::default());
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}

/// Early Grim Gardens (`upperclass1`): with the crowbar (point 129) in hand, each of the four
/// supplies (130 to 133) worked is sabotaged, and all four win it.
#[test]
fn early_grim_gardens_is_won_at_the_four_supplies() {
    let Some(mut r) = start("upperclass1") else { return };
    r.fetch_checked(129);
    assert!(r.said("Got crow bar."), "{:?}", r.seen);
    for way in [130, 131, 132, 133] {
        assert_eq!(r.g.outcome, None, "{:?}", r.seen);
        r.stand_and_use(way);
    }
    for n in ["Supply 1", "Supply 2", "Supply 3", "Supply 4"] {
        assert!(r.said(n), "{n} never sabotaged: {:?}", r.seen);
    }
    r.play(3.0, Controls::default());
    assert!(r.said("All supplies sabataged"), "{:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}

/// Rooftop1: the keycard (point 24) in hand and both panels worked (25 and 21) opens the door.
#[test]
fn rooftop1_keycard_opens_the_door() {
    let Some(mut r) = start("Rooftop1") else { return };
    r.fetch_checked(24);
    r.stand_and_use(25);
    assert!(!r.said("Door opened"), "opened with one panel: {:?}", r.seen);
    r.stand_and_use(21);
    assert!(r.said("Door opened"), "{:?}", r.seen);
}

/// TV Station Rooftops (`Assassin1`): the briefing, and near the transmitter (point 47) it's
/// pointed out.
#[test]
fn tv_station_rooftops_points_out_the_transmitter() {
    let Some(mut r) = start("Assassin1") else { return };
    r.play(20.0, Controls::default());
    assert!(r.said("Caution: This sector is patrolled heavily"), "{:?}", r.seen);
    r.stand(r.point(47));
    r.play(1.0, Controls::default());
    assert!(r.said("Thats the transmitter"), "{:?}", r.seen);
}

/// Supply Base (`supply1`): crossing a tripwire (points 16 and 17) sounds the alert.
#[test]
fn supply_base_tripwire_sounds_the_alert() {
    let Some(mut r) = start("supply1") else { return };
    assert!(!r.g.trips.wires.is_empty(), "no tripwires");
    let w = r.g.trips.wires.iter().min_by(|a, b| {
        let p = r.point(16);
        let d = |w: &uc_game::trip::Wire| ((w.a + w.b) / 2.0 - glam::vec2(p.x, p.z)).length();
        d(a).total_cmp(&d(b))
    }).unwrap();
    let (a, b, y) = (w.a, w.b, w.y);
    let mid = (a + b) / 2.0;
    let across = (b - a).perp().normalize();
    let start = mid - across * 200.0;
    r.stand(vec3(start.x, y, start.y));
    let me = r.player();
    r.g.people[me].fig.angle = uc_world::character::angle_of(across);
    r.play(2.0, Controls { wish: across, ..Default::default() });
    assert!(r.seen.iter().any(|m| m.eq_ignore_ascii_case("Security Alert!")), "{:?}", r.seen);
}

/// timetest's race to the cinema (point 5, on a roof of the lost map; on the stand-in, the
/// ground under it): standing there wins it, and waiting out the clock loses it.
#[test]
fn timetest_is_a_race_to_the_cinema() {
    let Some(mut r) = start("timetest") else { return };
    assert!(r.said("You have four minutes to get to the cinema."), "{:?}", r.seen);
    r.stand(r.point(5));
    r.play(2.0, Controls::default());
    assert!(r.said("Congratulations, you made it"), "{:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));

    let Some(mut r) = start("timetest") else { return };
    r.play(250.0, Controls::default());
    assert!(r.said("You ran out of time"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}
