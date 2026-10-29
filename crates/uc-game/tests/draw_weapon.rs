//! Getting a weapon out (#342, `set_person_draw_item`): switching to one, she stands and
//! draws it (`ANIM_PISTOL_DRAW`, `_SHOTGUN_DRAW`, `_KNIFE_DRAW`, `_BAT_DRAW`), in hand from
//! the fourth frame; putting it away is at once. Skipped without an install.

mod common;

use uc_game::person::State;
use uc_game::{Controls, Weapon};
use uc_world::character::anim;

fn switch() -> Controls {
    Controls { switch: true, ..Default::default() }
}

/// Gives the player `w` and switches round to it: what she plays the turn it comes out,
/// then, a second and a half on, whether she's done and has it in hand.
fn draw(r: &mut common::Run, w: Weapon) -> usize {
    let me = r.player();
    r.g.people[me].give(w);
    for _ in 0..10 {
        r.play(common::DT, switch());
        let p = &r.g.people[me];
        if p.weapon != w {
            r.play(1.5, Controls::default());
            continue;
        }
        assert!(matches!(p.state, State::Draw), "{w:?} came out at once");
        assert!(!p.in_hand(), "{w:?} in hand on the first frame");
        let a = p.fig.anim();
        r.play(1.5, Controls::default());
        let p = &r.g.people[me];
        assert!(matches!(p.state, State::Move) && p.in_hand(), "{w:?} never drawn");
        return a;
    }
    panic!("never switched to {w:?}");
}

#[test]
fn she_draws_each_weapon() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    // The bat takes the knife's place: one hand weapon at a time.
    for (w, a) in [(Weapon::Knife, anim::KNIFE_DRAW), (Weapon::Bat, anim::BAT_DRAW), (Weapon::Pistol, anim::PISTOL_DRAW), (Weapon::Shotgun, anim::SHOTGUN_DRAW)] {
        assert_eq!(draw(&mut r, w), a, "{w:?}");
    }
}

#[test]
fn putting_it_away_is_at_once() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    draw(&mut r, Weapon::Knife);
    let me = r.player();
    r.play(common::DT, switch());
    let p = &r.g.people[me];
    assert_eq!(p.weapon, Weapon::None);
    assert!(!matches!(p.state, State::Draw));
}
