//! A mission's `visual_effect` points (#257): only a fireball bangs, and a bonfire burns for
//! good.

mod common;

use uc_game::{Controls, Sfx};

#[test]
fn the_taxi_burns_on_without_a_bang() {
    // RTA's crashed taxi is a bonfire point: it crackles, it doesn't go off.
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let (fire, _) = r.g.pyro.fires().next().expect("RTA's taxi burns");
    assert!(!r.g.take_sounds().iter().any(|(s, _)| *s == Sfx::Explosion), "a bonfire shouldn't bang");
    assert_eq!(r.g.pyro.nearest_fire(fire), Some(fire));
    // Long past the half minute a blast's fire lasts.
    r.play(60.0, Controls::default());
    assert!(r.g.pyro.fires().any(|(f, _)| f == fire), "the taxi has gone out");
}
