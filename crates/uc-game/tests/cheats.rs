//! The developer console's commands and the cheats (#106, #147): teleports, the nearest
//! event point, ending the mission, playing as Roper, the full kit, god mode, unlimited
//! ammo, give, kill, freeze and the crime rate. Skipped without an install.

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

/// `god`: blows land but take nothing off her.
#[test]
fn god_mode_takes_no_damage() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = false;
    r.play(1.0, Controls::default());
    r.g.cheats.god = true;
    r.play(0.1, Controls::default());
    let p = r.g.player.unwrap();
    let full = r.g.people[p].max_health;
    let from = r.g.people[p].fig.pos + vec3(50.0, 0.0, 0.0);
    r.g.people[p].take_hit(from, 1000, 0, false, false, Weapon::None);
    r.play(0.5, Controls::default());
    assert_eq!(r.g.people[p].health, full);
    assert!(!r.g.people[p].knocked_out);
    // Off again, blows land.
    r.g.cheats.god = false;
    r.play(0.1, Controls::default());
    r.g.people[p].take_hit(from, 10, 0, false, false, Weapon::None);
    assert!(r.g.people[p].health < full);
}

/// `give` and `ammo`: a shotgun in her hands, and its rounds never run down.
#[test]
fn give_and_unlimited_ammo() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    assert!(r.g.give("shotgun"));
    assert!(!r.g.give("banana"));
    let p = r.g.player.unwrap();
    assert_eq!(r.g.people[p].weapon, Weapon::Shotgun);
    r.g.cheats.ammo = true;
    for _ in 0..10 {
        r.play(0.1, Controls { punch: true, ..Default::default() });
        r.play(0.4, Controls::default());
    }
    assert!(r.g.people[p].rounds().unwrap() >= 240, "ran down to {:?}", r.g.people[p].rounds());
    assert!(r.g.give("grenades"));
    assert_eq!(r.g.people[p].grenades, 3);
}

/// `kill`: everyone after her goes down; `kill near` only those close.
#[test]
fn kill_knocks_out_the_enemies() {
    let Some(level) = common::load("police1") else { return };
    let mut g = Game::new_endless(&level, 5, Some(glam::vec2(8827.0, 10274.0)), 9.0);
    for _ in 0..(8.0 / common::DT) as usize {
        g.update(common::DT, &Controls::default(), &level);
    }
    let up = |g: &Game| g.people.iter().filter(|p| !p.player && !p.removed && !p.is_down() && p.hostile()).count();
    assert!(up(&g) > 0);
    assert_eq!(g.knock_out_enemies(Some(1.0)), 0, "none is that close");
    let n = g.knock_out_enemies(None);
    assert!(n > 0 && up(&g) == 0, "{n} knocked out, {} still up", up(&g));
}

/// `freeze`: nobody else moves.
#[test]
fn freeze_stops_everyone_else() {
    let Some(level) = common::load("police1") else { return };
    let mut g = Game::new_endless(&level, 5, Some(glam::vec2(8827.0, 10274.0)), 9.0);
    for _ in 0..(8.0 / common::DT) as usize {
        g.update(common::DT, &Controls::default(), &level);
    }
    g.cheats.freeze = true;
    g.update(common::DT, &Controls::default(), &level);
    let me = g.player.unwrap();
    let at: Vec<_> = g.people.iter().map(|p| p.fig.pos).collect();
    for _ in 0..(3.0 / common::DT) as usize {
        g.update(common::DT, &Controls::default(), &level);
    }
    for (k, p) in g.people.iter().enumerate().filter(|(k, p)| *k != me && p.hostile() && !p.removed) {
        assert!((p.fig.pos - at[k]).length() < 32.0, "{k} moved {:.0}", (p.fig.pos - at[k]).length());
    }
}
