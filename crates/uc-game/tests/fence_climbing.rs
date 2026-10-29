//! Which fences can be climbed (#358): not one flagged unclimbable (`FACET_FLAG_UNCLIMBABLE`,
//! `set_person_land_on_fence`, Person.cpp:10146), and only a flat-topped one climbed over: on
//! an angle-topped one (`STOREY_TYPE_FENCE`) the hands come off at the top and she drops
//! back down (Person.cpp:14357). Crimescene has both (Capflag1 does too, but is exempt).
//! Skipped without an install.

use glam::{vec3, Vec2};
use uc_game::{Controls, State};

mod common;
use common::{Run, DT};

/// Darci a little way in front of the middle of the `nth` fence `pick` likes, facing it;
/// then walking into it for up to four seconds. Returns where she ends up, and whether she
/// got onto it; None when there's no such fence.
fn walk_into(r: &mut Run, nth: usize, pick: impl Fn(&uc_world::ground::FenceClimb, f32) -> bool) -> Option<(f32, bool)> {
    let ground = &r.level.ground;
    let k = (0..ground.fences.len())
        .filter(|&k| pick(&ground.fence_climb[k], ground.fences[k].y1 - ground.fences[k].y0))
        .nth(nth)?;
    let w = ground.fences[k];
    let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
    let mid = (a + b) / 2.0;
    let across = (b - a).perp().normalize();
    let start = mid - across * 80.0;
    r.stand(vec3(start.x, w.y0, start.y));
    r.play(0.5, Controls::default());
    let p = r.player();
    let mut climbed = false;
    for _ in 0..120 {
        r.play(DT, Controls { wish: across, ..Default::default() });
        climbed |= matches!(r.g.people[p].state, State::Fence { .. });
    }
    r.play(2.0, Controls::default());
    let at = r.g.people[p].fig.pos;
    Some(((Vec2::new(at.x, at.z) - mid).dot(across), climbed))
}

#[test]
fn an_unclimbable_fence_is_a_wall() {
    let Some(mut r) = common::start("Crimescene") else { return };
    let (side, climbed) = walk_into(&mut r, 0, |c, h| !c.climbable && h > 200.0).expect("no such fence");
    assert!(!climbed && side < 0.0, "got onto it ({climbed}) or past it ({side})");
}

#[test]
fn an_angle_topped_fence_is_climbed_but_not_over() {
    let Some(mut r) = common::start("Crimescene") else { return };
    // The first she can get to: some stand against a wall or up on a roof.
    let (side, climbed) = (0..)
        .map_while(|n| walk_into(&mut r, n, |c, h| c.climbable && !c.over && h > 200.0))
        .find(|&(_, climbed)| climbed)
        .unwrap_or((0.0, false));
    assert!(climbed, "never climbed one");
    assert!(side < 0.0, "over it: {side}");
}

#[test]
fn a_flat_topped_fence_is_climbed_over() {
    let Some(mut r) = common::start("Crimescene") else { return };
    let (side, climbed) = walk_into(&mut r, 0, |c, h| c.climbable && c.over && h > 200.0).expect("no such fence");
    assert!(climbed && side > 0.0, "climbed {climbed}, ended {side} from it");
}
