//! Darci's upper body twists to her target (#302, `twist_darci_body_to_angle`): with her
//! pistol on someone a little off to one side, she points that way without turning; further
//! round, she turns. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::person::AI_CIV;
use uc_game::{Controls, Weapon};

mod common;

/// Darci aiming at one of RTA's crowd `deg` degrees round from straight ahead: her right
/// hand's place, and how far she turned.
fn aim(deg: f32) -> Option<(Vec3, f32)> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let me = r.player();
    r.g.people[me].give(Weapon::Pistol);
    r.g.people[me].weapon = Weapon::Pistol;
    r.g.people[me].fig.angle = uc_world::character::angle_of(vec2(1.0, 0.0));
    let angle = r.g.people[me].fig.angle;
    let i = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("crowd");
    let at = r.g.people[me].fig.pos;
    let f = r.g.people[me].facing_dir();
    let a = deg.to_radians();
    let dir = vec2(f.x * a.cos() - f.y * a.sin(), f.x * a.sin() + f.y * a.cos());
    for _ in 0..(0.6 / common::DT) as usize {
        let y = r.level.ground.surface(&r.level.map, at.x + dir.x * 400.0, at.z + dir.y * 400.0, at.y + 64.0);
        r.g.people[i].fig.pos = vec3(at.x + dir.x * 400.0, y, at.z + dir.y * 400.0);
        r.g.target = Some(i);
        r.play(common::DT, Controls::default());
    }
    let hand = r.g.people[me].fig.part_world(7).map(|(_, p)| p)?;
    Some((hand - r.g.people[me].fig.pos, (r.g.people[me].fig.angle - angle).abs()))
}

#[test]
fn she_points_to_the_side_without_turning() {
    let (Some((a, ta)), Some((b, tb)), Some((c, _))) = (aim(30.0), aim(-30.0), aim(0.0)) else { return };
    assert!(ta < 0.01 && tb < 0.01, "she turned {ta} and {tb}");
    // The gun hand goes round her towards the side the target is on: its bearing from her
    // middle turns the way the target's does.
    let bearing = |h: Vec3| uc_world::character::angle_of(vec2(h.x, h.z));
    let turn = |h: Vec3| (bearing(h) - bearing(c) + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    assert!(turn(a) * turn(b) < 0.0, "the hand went the same way both times");
    // The way the target at +30 lies, by the same bearing (`aim` turns her facing by +30).
    let f = uc_world::character::facing(uc_world::character::angle_of(vec2(1.0, 0.0)));
    let r = 30f32.to_radians();
    let dir = vec2(f.x * r.cos() - f.y * r.sin(), f.x * r.sin() + f.y * r.cos());
    let target = (uc_world::character::angle_of(dir) - uc_world::character::angle_of(f) + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    assert!(turn(a) * target > 0.0, "the hand went away from the target ({} against {target})", turn(a));
}

#[test]
fn further_round_she_turns() {
    let Some((_, t)) = aim(90.0) else { return };
    assert!(t > 0.3, "she only turned {t}");
}
