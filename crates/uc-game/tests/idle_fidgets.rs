//! Standing about (#306, `set_person_random_idle`): left standing still, Darci now and then
//! scratches (`ANIM_IDLE_SCRATCH1`, `_2`), every 400 to 655 turns; with something in hand
//! she doesn't. Skipped without an install.

mod common;

use uc_game::{Controls, Weapon};
use uc_world::character::anim;

/// The fidgets she does in `secs` standing still, and when.
fn fidgets(r: &mut common::Run, secs: f32) -> Vec<f32> {
    let me = r.player();
    let mut seen = Vec::new();
    let mut was = false;
    for k in 0..(secs / common::DT) as usize {
        r.play(common::DT, Controls::default());
        let now = matches!(r.g.people[me].fig.anim(), anim::IDLE_SCRATCH1 | anim::IDLE_SCRATCH2);
        if now && !was {
            seen.push(k as f32 * common::DT);
        }
        was = now;
    }
    seen
}

#[test]
fn standing_about_she_scratches_now_and_then() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    let at = r.g.people[r.player()].fig.pos;
    let seen = fidgets(&mut r, 150.0);
    assert!(!seen.is_empty(), "no fidget in 150 s");
    // Never sooner than 400 turns apart, and she stays where she is.
    for w in seen.windows(2) {
        assert!(w[1] - w[0] >= 19.9, "fidgets {} s apart", w[1] - w[0]);
    }
    assert!((r.g.people[r.player()].fig.pos - at).length() < 32.0, "she wandered off");
}

#[test]
fn not_with_something_in_hand() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    let me = r.player();
    r.g.people[me].give(Weapon::Bat);
    r.g.people[me].weapon = Weapon::Bat;
    r.play(1.0, Controls::default());
    assert!(fidgets(&mut r, 90.0).is_empty(), "she fidgeted holding a bat");
}
