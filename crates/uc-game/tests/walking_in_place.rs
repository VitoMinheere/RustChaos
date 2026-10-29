//! Nobody walks on the spot: a patrol turning back and forth between two of its points, a
//! passer-by against a wall, a parked car or the edge of an awning they were put on top of.
//! A minute of five street missions, anyone in a walk or run for three seconds having gone
//! somewhere. Skipped without an install.

use glam::Vec3;
use uc_game::Controls;
use uc_world::character::anim;

mod common;

#[test]
fn nobody_walks_on_the_spot() {
    let mut stuck = Vec::new();
    for mission in ["police1", "park2", "Gangorder1", "westcrime1", "Bankbomb1"] {
        let Some(mut r) = common::start(mission) else { return };
        r.stop_at_outcome = false;
        // Each person's place and animation, a second apart.
        let mut seen: Vec<Vec<(Vec3, usize)>> = vec![Vec::new(); r.g.people.len()];
        for _ in 0..60 {
            r.play(1.0, Controls::default());
            for (i, p) in r.g.people.iter().enumerate().take(seen.len()) {
                seen[i].push((p.fig.pos, p.fig.anim()));
                let last = &seen[i][seen[i].len().saturating_sub(4)..];
                if last.len() < 4 || p.removed || p.player {
                    continue;
                }
                let walking = last.iter().all(|&(_, a)| uc_game::person::walking_anim(a) || a == anim::RUN);
                let moved = last.windows(2).map(|w| (w[1].0 - w[0].0).length()).fold(0.0, f32::max);
                if walking && moved < 30.0 && !stuck.iter().any(|s: &(&str, usize, f32)| s.0 == mission && s.1 == i) {
                    stuck.push((mission, i, r.g.time));
                }
            }
        }
    }
    assert!(stuck.is_empty(), "walking on the spot (mission, person, time): {stuck:?}");
}
