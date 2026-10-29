//! A car coming fast (#395, `PCOM_SOUND_VAN`): past `Velocity` 500, a civilian near where
//! it's headed runs from it, even off its path; slower, they don't. Skipped without an install.

use glam::vec3;
use uc_game::person::AI_CIV;
use uc_game::Controls;

mod common;

fn scared_by(speed: f32) -> Option<bool> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let v = r.g.vehicles.iter().position(|v| v.driver.is_none()).expect("no car");
    let (at, fwd) = (r.g.vehicles[v].pos, r.g.vehicles[v].forward());
    let side = glam::vec2(fwd.y, -fwd.x);
    let civ = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead() && q.scared.is_none()).expect("no civilian");
    // Well to the side of its path, where nobody would dodge it, but near its noise.
    let spot = glam::Vec2::new(at.x, at.z) + fwd * 450.0 + side * 340.0;
    r.g.people[civ].fig.pos = vec3(spot.x, at.y, spot.y);
    for _ in 0..(0.3 / common::DT) as usize {
        r.g.vehicles[v].speed = speed;
        r.play(common::DT, Controls::default());
        if r.g.people[civ].scared.is_some() {
            return Some(true);
        }
    }
    Some(false)
}

#[test]
fn a_fast_car_scares_civilians() {
    let Some(scared) = scared_by(1000.0) else { return };
    assert!(scared, "a civilian stood by a car coming at 1000");
}

#[test]
fn a_slow_one_doesnt() {
    let Some(scared) = scared_by(400.0) else { return };
    assert!(!scared, "a civilian ran from a car coming at 400");
}
