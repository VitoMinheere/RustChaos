//! The running vault (#252, `SUB_STATE_RUNNING_VAULT`): running into a low fence (two blocks
//! of 16, 128 high), Darci vaults it and runs on from the far side; walking into it, she still
//! climbs it. One of RTA's low fences. Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::{Controls, State};

mod common;
use common::{Run, DT};

/// RTA with Darci 150 in front of a low fence standing on open ground, facing it. Returns the
/// way over it.
fn at_a_low_fence() -> Option<(Run, Vec2, Vec2)> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let g = &r.level.ground;
    let map = &r.level.map;
    let (mid, over) = g.fences.iter().find_map(|f| {
        if ((f.y1 - f.y0) - 128.0).abs() > 1.0 {
            return None;
        }
        let (a, b) = (Vec2::from(f.a), Vec2::from(f.b));
        if (b - a).length() < 200.0 {
            return None;
        }
        let mid = (a + b) / 2.0;
        let n = vec2(-(b - a).y, (b - a).x).normalize();
        let floor = |p: Vec2| map.ground_height(p.x, p.y);
        let (near, far) = (mid - n * 150.0, mid + n * 150.0);
        let flat = (floor(near) - f.y0).abs() < 4.0 && (floor(far) - f.y0).abs() < 4.0;
        flat.then_some((mid, n))
    }).expect("RTA has a low fence on open ground");
    let at = mid - over * 150.0;
    let y = r.level.ground.surface(&r.level.map, at.x, at.y, 64.0);
    let pi = r.player();
    r.g.people[pi].fig.pos = vec3(at.x, y, at.y);
    r.g.people[pi].fig.angle = uc_world::character::angle_of(over);
    r.g.people[pi].state = State::Move;
    Some((r, mid, over))
}

#[test]
fn running_at_a_low_fence_she_vaults_it() {
    let Some((mut r, mid, over)) = at_a_low_fence() else { return };
    let pi = r.player();
    let run = Controls { wish: over, run: true, ..Default::default() };
    let mut vaulted = false;
    for _ in 0..(2.0 / DT) as usize {
        r.play(DT, run);
        vaulted |= matches!(r.g.people[pi].state, State::Vault { .. });
    }
    assert!(vaulted, "she never vaulted: {:?}", r.g.people[pi].state);
    let p = r.g.people[pi].fig.pos;
    assert!((vec2(p.x, p.z) - mid).dot(over) > 0.0, "she's still on the near side");
}

#[test]
fn walking_into_it_she_climbs() {
    let Some((mut r, _, over)) = at_a_low_fence() else { return };
    let pi = r.player();
    let walk = Controls { wish: over, ..Default::default() };
    let mut climbed = false;
    for _ in 0..(2.0 / DT) as usize {
        r.play(DT, walk);
        climbed |= matches!(r.g.people[pi].state, State::Fence { .. });
        assert!(!matches!(r.g.people[pi].state, State::Vault { .. }), "vaulted at a walk");
    }
    assert!(climbed);
}
