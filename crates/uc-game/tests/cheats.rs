//! The developer console's commands and the cheats (#106): teleports, the nearest event
//! point, ending the mission, playing as Roper, the full kit. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Game, Outcome, State, Weapon};
use uc_world::character::person as ptype;

mod common;

/// `telw`: to an event point, standing; `wpt` then names it.
#[test]
fn telw_goes_to_an_event_point() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let n = r.level.mission.event_points.iter().map(|e| e.index).find(|&i| i > 50).unwrap();
    let at = Game::point_pos(&r.level, n).unwrap();
    assert!(r.g.teleport(at, &r.level));
    let p = r.g.player().unwrap();
    assert!((p.fig.pos.x - at.x).abs() < 1.0 && (p.fig.pos.z - at.z).abs() < 1.0);
    assert!(matches!(p.state, State::Move));
    let (near, d) = r.g.nearest_point(&r.level).unwrap();
    assert!(d < 1.0 || near == n, "the nearest point should be the one she's at");
}

/// A teleport over a building puts her on its roof, and she stays there.
#[test]
fn a_teleport_over_a_building_lands_on_the_roof() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    // The first square of the map with a roof well above the street.
    let roof = (0..128 * 128).find_map(|k| {
        let (x, z) = ((k % 128) as f32 * 256.0 + 128.0, (k / 128) as f32 * 256.0 + 128.0);
        let top = r.level.ground.surface(&r.level.map, x, z, 1.0e5);
        (top > 600.0).then_some(vec3(x, top, z))
    }).expect("a roof");
    assert!(r.g.teleport(roof, &r.level));
    r.play(2.0, Controls::default());
    let y = r.g.player().unwrap().fig.pos.y;
    assert!((y - roof.y).abs() < 40.0, "she should be on the roof at {}, is at {y}", roof.y);
}

/// `win` and `lose` end the mission.
#[test]
fn win_and_lose_end_the_mission() {
    let Some(mut r) = common::start("police1") else { return };
    r.g.end_mission(true);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
    let Some(mut r) = common::start("police1") else { return };
    r.g.end_mission(false);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))));
}

/// The full kit: every gun's ammo, and an AK, a shotgun and a grenade at her feet.
#[test]
fn the_full_kit_lays_out_the_guns() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = false;
    r.play(1.0, Controls::default());
    r.g.full_kit(&r.level);
    r.play(1.0, Controls { wish: glam::vec2(0.0, 1.0), ..Default::default() });
    let p = r.g.player().unwrap();
    assert_eq!(p.health, p.max_health);
    let has = |w| p.has(w) || r.g.items.iter().any(|it| Weapon::from_item(it.kind) == Some(w));
    assert!(has(Weapon::Ak47) && has(Weapon::Shotgun), "the guns should be there, or picked up");
}

/// `roper`: where the mission has his body (Cop Killers), Darci becomes Roper; where it
/// doesn't (RTA), nothing changes.
#[test]
fn play_as_roper_where_his_body_is_loaded() {
    let Some(mut r) = common::start("AWOL1") else { return };
    r.play(1.0, Controls::default());
    assert!(r.g.play_as(ptype::ROPER, &r.level));
    assert_eq!(r.g.player().unwrap().kind, ptype::ROPER);
    r.play(1.0, Controls { wish: glam::vec2(0.0, 1.0), ..Default::default() });

    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    if r.level.people.as_ref().is_some_and(|p| p.roper.is_none()) {
        assert!(!r.g.play_as(ptype::ROPER, &r.level));
        assert_eq!(r.g.player().unwrap().kind, ptype::DARCI);
    }
}
