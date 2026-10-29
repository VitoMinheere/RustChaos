//! The player's run (#342): from standing she pushes off (`ANIM_YOMP_START`) into the yomp
//! (`ANIM_YOMP`), and sprinting is the run (`ANIM_RUN`); with a bat both are the bat's own
//! (Person.cpp:7947, `get_yomp_anim`). Skipped without an install.

mod common;

use glam::Vec2;
use uc_game::{Controls, Weapon};
use uc_world::character::anim;

/// The animations she plays, in order, over `secs` with `c` held.
fn anims(r: &mut common::Run, secs: f32, c: Controls) -> Vec<usize> {
    let me = r.player();
    let mut seen: Vec<usize> = Vec::new();
    for _ in 0..(secs / common::DT) as usize {
        r.play(common::DT, c);
        let a = r.g.people[me].fig.anim();
        if seen.last() != Some(&a) {
            seen.push(a);
        }
    }
    seen
}

fn run(sprint: bool) -> Controls {
    Controls { wish: Vec2::new(0.0, 1.0), run: true, sprint, ..Default::default() }
}

#[test]
fn she_pushes_off_into_the_yomp() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    let seen = anims(&mut r, 1.5, run(false));
    assert_eq!(seen[..2], [anim::YOMP_START, anim::YOMP], "{seen:?}");
}

#[test]
fn sprinting_is_the_run() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    anims(&mut r, 1.0, run(false));
    let seen = anims(&mut r, 1.0, run(true));
    assert_eq!(seen, [anim::RUN], "from the yomp, no push off");
}

#[test]
fn with_a_bat_its_own() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    let me = r.player();
    r.g.people[me].give(Weapon::Bat);
    r.g.people[me].weapon = Weapon::Bat;
    r.play(1.0, Controls::default());
    let seen = anims(&mut r, 1.5, run(false));
    assert_eq!(seen[..2], [anim::YOMP_START_BAT, anim::YOMP_BAT], "{seen:?}");
}
