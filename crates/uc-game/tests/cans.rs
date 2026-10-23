//! Cans (#211, the manual's "throwing a can into another area will draw the attention of
//! guards"): action standing by one picks it up (`set_person_can_pickup`), punch throws it
//! (`DIRT_release_can_or_head`), and where it lands the guards about turn to look
//! (`PCOM_SOUND_UNUSUAL`). Skipped without an install.

use glam::{vec2, Vec3};
use uc_game::dirt::Kind;
use uc_game::{Controls, Game, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn a_can_picked_up_and_thrown_draws_a_guard() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "Stealtst1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..60 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let pi = g.player.unwrap();
    // A guard who'll hear it, everyone else gone.
    let gi = (0..g.people.len()).find(|&i| i != pi && !g.people[i].removed && g.people[i].hostile() && !g.people[i].aggro()).expect("a guard");
    for (i, p) in g.people.iter_mut().enumerate() {
        if i != pi && i != gi {
            p.removed = true;
        }
    }
    // Darci at a can lying in the open, facing along an open stretch: it flies about eight
    // squares.
    let open = |p: &uc_game::dirt::Piece, dir: Vec3| {
        let at = Vec3::new(p.x as f32, p.y as f32 + 64.0, p.z as f32);
        level.ground.line_of_sight(at, at + dir * 2400.0)
            && (level.ground.surface(&level.map, at.x + dir.x * 2100.0, at.z + dir.z * 2100.0, at.y) - p.y as f32).abs() < 64.0
    };
    let dirs = [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z];
    let (ci, can, dir) = g.dirt.pieces.iter().enumerate()
        .filter_map(|(i, p)| p.filter(|p| p.kind == Kind::Can).map(|p| (i, p)))
        .find_map(|(i, p)| dirs.iter().find(|&&d| open(&p, d)).map(|&d| (i, p, d)))
        .expect("a can in the open");
    let at = Vec3::new(can.x as f32, can.y as f32, can.z as f32) - dir * 30.0;
    let at = Vec3::new(at.x, level.ground.surface(&level.map, at.x, at.z, at.y + 32.0), at.z);
    g.people[pi].fig.pos = at;
    g.people[pi].fig.angle = uc_world::character::angle_of(vec2(dir.x, dir.z));
    g.people[pi].weapon = uc_game::Weapon::None;
    // The guard beside where it'll come down, too far off to notice Darci, facing away.
    let side = Vec3::new(-dir.z, 0.0, dir.x);
    g.people[gi].fig.pos = at + dir * 2100.0 + side * 500.0;
    g.people[gi].fig.pos.y = level.ground.surface(&level.map, g.people[gi].fig.pos.x, g.people[gi].fig.pos.z, at.y + 64.0);
    g.people[gi].fig.angle = uc_world::character::angle_of(vec2(side.x, side.z));
    g.people[gi].heard = None;
    g.update(DT, &Controls::default(), &level);
    assert!(!g.people[gi].aggro(), "the guard hasn't seen her");
    assert!(g.people[pi].can_near, "the can's in reach");

    // Action: bend down for it, and it's in her hand.
    g.update(DT, &Controls { action: true, ..Default::default() }, &level);
    assert!(matches!(g.people[pi].state, State::PickUp { .. }), "{:?}", g.people[pi].state);
    for _ in 0..(2.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
    }
    assert_eq!(g.people[pi].can, Some(ci), "she has the can");
    assert_eq!(g.dirt.pieces[ci].unwrap().kind, Kind::HeldCan);
    // Punch: thrown.
    g.update(DT, &Controls { punch: true, ..Default::default() }, &level);
    assert!(matches!(g.people[pi].state, State::Throw { .. }), "{:?}", g.people[pi].state);
    let mut heard = false;
    let mut landed = None;
    // Over three seconds in the air, then bouncing to a stop: where it lies about as a can
    // again (before the dirt round Darci takes it back, so far off).
    for _ in 0..(8.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        heard |= g.people[gi].heard.is_some();
        if let (None, Some(d)) = (landed, g.dirt.pieces[ci].filter(|d| d.kind == Kind::Can)) {
            landed = Some(d);
        }
    }
    assert_eq!(g.people[pi].can, None);
    let landed = landed.expect("it came down and lies about again");
    let flew = (Vec3::new(landed.x as f32, at.y, landed.z as f32) - at).dot(dir);
    assert!(flew > 1000.0, "it flew {flew}");
    assert!(heard, "the guard heard it land");
}
