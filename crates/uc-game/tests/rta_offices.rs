//! RTA (police1): up to the offices over the police station's lobby, which were built with no
//! way up (`uc_world::level::up_to_rtas_offices`). Darci walks from her start in the lobby to
//! the ladder on its west wall, climbs it through the doorway over it onto the landing, goes
//! into the office across from it, and back along the landing into the other. Skipped without
//! an install.

use glam::{vec2, Vec2};
use uc_game::{Controls, State};

use crate::common;
use common::{Run, DT};

/// Up a storey: the offices' floors.
const UPSTAIRS: f32 = 256.0;

fn at(r: &Run) -> glam::Vec3 {
    r.g.people[r.player()].fig.pos
}

/// Walks straight at each of `pts` (squares) in turn.
fn walk_by(r: &mut Run, pts: &[(f32, f32)]) {
    for &(x, z) in pts {
        let to = vec2(x, z) * 256.0;
        let mut done = false;
        for _ in 0..(8.0 / DT) as usize {
            let d = to - vec2(at(r).x, at(r).z);
            if d.length() < 40.0 {
                done = true;
                break;
            }
            r.play(DT, Controls { wish: d.normalize(), ..Default::default() });
        }
        assert!(done, "Darci got stuck at {} on the way to ({x}, {z}) ({:?})", at(r), r.g.people[r.player()].state);
    }
}

/// Walks into the ladder ahead (the way `dir`) and up it, till she stands at `top`.
fn climb(r: &mut Run, dir: Vec2, top: f32) {
    for _ in 0..(15.0 / DT) as usize {
        if (at(r).y - top).abs() < 16.0 && matches!(r.g.people[r.player()].state, State::Move) {
            return;
        }
        r.play(DT, Controls { wish: dir, ..Default::default() });
    }
    panic!("not up the ladder to {top}: at {} ({:?})", at(r), r.g.people[r.player()].state);
}

#[test]
fn up_the_ladder_into_both_offices() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    // To the foot of the ladder, round the armchair by it.
    walk_by(&mut r, &[(81.0, 85.5), (79.7, 85.5)]);
    climb(&mut r, vec2(-1.0, 0.0), UPSTAIRS);
    // Across the landing into office A, where two of the cops are.
    walk_by(&mut r, &[(78.5, 85.5), (76.0, 85.5)]);
    assert!((at(&r).y - UPSTAIRS).abs() < 16.0, "fell through office A's floor: {}", at(&r));
    // Back out, along the landing to its north end, and into office B.
    walk_by(&mut r, &[(78.5, 85.5), (78.5, 80.5), (80.5, 80.5), (84.0, 81.5)]);
    assert!((at(&r).y - UPSTAIRS).abs() < 16.0, "fell through office B's floor: {}", at(&r));
    // Back to the top of the ladder, and down it with action, to the lobby.
    walk_by(&mut r, &[(80.5, 80.5), (78.5, 80.5), (78.5, 85.5), (78.85, 85.5)]);
    r.play(DT, Controls { action: true, ..Default::default() });
    for _ in 0..(10.0 / DT) as usize {
        if at(&r).y < 16.0 && matches!(r.g.people[r.player()].state, State::Move) {
            break;
        }
        r.play(DT, Controls { wish: vec2(0.0, 0.0), ..Default::default() });
    }
    assert!(at(&r).y < 16.0 && at(&r).x > 79.0 * 256.0, "not back down in the lobby: {} ({:?})", at(&r), r.g.people[r.player()].state);
}
