//! The danger stings (#409, `set_danger_level` and `context_music`, Controls.cpp:1672 and
//! 1389): a thug coming within 0xc00 plays the green sting at two to three 0x400s off, and the
//! red one within 0x400. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Sfx};
use uc_world::character::person as ptype;

mod common;

/// The danger stings heard over `secs`.
fn stings(r: &mut common::Run, secs: f32) -> Vec<bool> {
    let mut out = Vec::new();
    for _ in 0..(secs / common::DT) as usize {
        r.play(common::DT, Controls::default());
        out.extend(r.g.take_sounds().into_iter().filter_map(|(s, _)| match s {
            Sfx::Danger { near } => Some(near),
            _ => None,
        }));
    }
    out
}

#[test]
fn a_thug_drawing_near() {
    let Some(mut r) = common::start("fight2") else { return };
    r.play(1.0, Controls::default());
    let pi = r.player();
    let me = r.g.people[pi].fig.pos;
    // Nobody about but the one thug, standing still.
    let thug = r.g.people.iter().position(|q| !q.player && !q.removed).expect("nobody");
    for (i, q) in r.g.people.iter_mut().enumerate() {
        if i != pi && i != thug {
            q.removed = true;
        }
    }
    let q = &mut r.g.people[thug];
    (q.kind, q.ai) = (ptype::THUG_RED, 0);
    q.fig.pos = me + vec3(4000.0, 0.0, 0.0);
    r.g.take_sounds();
    assert!(stings(&mut r, 2.0).is_empty(), "a sting with nobody near");
    r.g.people[thug].fig.pos = me + vec3(2500.0, 0.0, 0.0);
    assert_eq!(stings(&mut r, 2.0), vec![false], "no green sting at 2500");
    r.g.people[thug].fig.pos = me + vec3(500.0, 0.0, 0.0);
    assert_eq!(stings(&mut r, 2.0), vec![true], "no red sting at 500");
}
