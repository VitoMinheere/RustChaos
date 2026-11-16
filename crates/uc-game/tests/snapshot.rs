//! A game copied mid-mission (#544: `Game` is `Clone`, the snapshot a checkpoint is) plays
//! on as the one it was copied from: the same state hash turn for turn, given the same
//! inputs, and the same whether or not the original was played on first. Skipped without an
//! install.

use uc_game::{Controls, Game, TURN};

use crate::common;

/// The inputs on turn `t`: a second each of walking, running, fighting, crouching, jumping
/// and firing.
fn input(t: usize) -> Controls {
    let mut c = Controls::default();
    match t / 30 % 8 {
        0 | 1 => c.wish = glam::vec2(0.0, 1.0),
        2 => { c.wish = glam::vec2(1.0, 0.5); c.sprint = true }
        3 => c.punch = t.is_multiple_of(10),
        4 => c.kick = t.is_multiple_of(10),
        5 => c.crouch = true,
        6 => { c.wish = glam::vec2(-0.7, -0.7); c.jump = t.is_multiple_of(30) }
        _ => c.fire = t.is_multiple_of(6),
    }
    c
}

#[test]
fn a_copy_plays_on_as_the_original() {
    let missions = ["police1", "police3", "Gangorder1", "awol1", "bankbomb1", "carbomb1", "estate2", "factory1", "skymiss2", "snow2", "Finale1"];
    for mission in missions {
        let Some(level) = common::load(mission) else { return };
        let mut g = Game::new(&level, true);
        let (n, m) = (30 * 20 + 11, 30 * 20);
        for t in 0..n {
            if g.cutscene.is_some() && t == 45 {
                g.skip_cutscene();
            }
            g.update(TURN, &input(t), &level);
        }
        let kept = g.clone();
        assert_eq!(kept.state_hash(), g.state_hash(), "{mission}: the copy as it's made");
        assert_eq!(kept.turns(), g.turns());
        // The original played on, then two copies of the copy: all three the same.
        let on: Vec<u64> = (n..n + m).map(|t| {
            g.update(TURN, &input(t), &level);
            g.state_hash()
        }).collect();
        for _ in 0..2 {
            let mut c = kept.clone();
            for (k, t) in (n..n + m).enumerate() {
                c.update(TURN, &input(t), &level);
                assert_eq!(c.state_hash(), on[k], "{mission}: {} turns after the copy", k + 1);
            }
            assert_eq!(c.turns(), g.turns());
            assert_eq!(c.time, g.time);
        }
    }
}
