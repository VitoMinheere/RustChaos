//! Clips (#401, Person.cpp:6085-6187 and 6898-6930): a clip's worth of shots, then a pull of
//! the trigger that changes the clip, a dry click and no shot; out altogether, a click and on
//! to her best weapon with rounds left. Skipped without an install.

use uc_game::{Controls, Sfx, Weapon};

mod common;

/// What each of `n` pulls of the trigger did: 's' a shot, 'c' a click.
fn pulls(r: &mut common::Run, n: usize) -> String {
    let pi = r.player();
    let mut out = String::new();
    r.g.take_sounds();
    for _ in 0..n {
        r.play(common::DT, Controls { punch: true, ..Default::default() });
        r.play(0.8, Controls::default());
        let sounds = r.g.take_sounds();
        if sounds.iter().any(|(s, _)| matches!(s, Sfx::Shot(_))) {
            out.push('s');
        } else if sounds.iter().any(|(s, _)| matches!(s, Sfx::Click)) {
            out.push('c');
        }
        if r.g.people[pi].weapon != Weapon::Pistol {
            break;
        }
    }
    out
}

#[test]
fn a_clip_then_a_change_then_her_bat() {
    let Some(mut r) = common::start("fight2") else { return };
    r.play(1.0, Controls::default());
    let pi = r.player();
    let p = &mut r.g.people[pi];
    p.give(Weapon::Pistol);
    p.give(Weapon::Bat);
    p.weapon = Weapon::Pistol;
    p.ammo[Weapon::Pistol.slot().unwrap()] = 20;
    let did = pulls(&mut r, 30);
    assert_eq!(did, format!("{}c{}c", "s".repeat(15), "s".repeat(5)));
    assert_eq!(r.g.people[pi].weapon, Weapon::Bat, "she didn't go to her bat");
}
