//! The game is deterministic: two runs of a mission given the same inputs agree on
//! `Game::state_hash` every second. Lockstep multiplayer and replays depend on it (#114).
//! Each run's `HashMap`s get their own random keys, so anything that iterates one in a way
//! that matters shows up here. Skipped without an install.

use uc_game::Controls;

mod common;
use common::Run;

/// The inputs at second `t`: walking about, running, fighting, crouching, a shot or two.
fn input(t: usize) -> Controls {
    let mut c = Controls::default();
    match t % 8 {
        0 | 1 => c.wish = glam::vec2(0.0, 1.0),
        2 => { c.wish = glam::vec2(1.0, 0.5); c.sprint = true }
        3 => c.punch = true,
        4 => c.kick = true,
        5 => c.crouch = true,
        6 => { c.wish = glam::vec2(-0.7, -0.7); c.jump = true }
        _ => { c.fire = true; c.switch = t % 16 == 7 }
    }
    c
}

/// The state hash after every second of `secs`, playing `mission` with the inputs above.
fn hashes(mission: &str, secs: usize) -> Option<Vec<u64>> {
    hashes_with(mission, secs, input)
}

fn hashes_with(mission: &str, secs: usize, input: impl Fn(usize) -> Controls) -> Option<Vec<u64>> {
    let mut r = Run::new(common::load(mission)?);
    let mut out = Vec::new();
    for t in 0..secs {
        r.play(1.0, input(t));
        out.push(r.g.state_hash());
    }
    Some(out)
}

#[test]
fn two_runs_of_a_mission_agree_every_second() {
    for mission in ["police1", "Gangorder1", "Finale1"] {
        let Some(a) = hashes(mission, 40) else { return };
        let b = hashes(mission, 40).unwrap();
        if let Some(t) = a.iter().zip(&b).position(|(x, y)| x != y) {
            panic!("{mission}: the runs part after {} s", t + 1);
        }
    }
}

/// And the hash does see a difference: a run that stands still for one second of it parts
/// from the others from then on.
#[test]
fn a_different_input_shows_in_the_hash() {
    let Some(a) = hashes("police1", 10) else { return };
    let b = hashes_with("police1", 10, |t| if t == 4 { Controls::default() } else { input(t) }).unwrap();
    assert_eq!(a[..4], b[..4]);
    assert_ne!(a[4], b[4]);
}
