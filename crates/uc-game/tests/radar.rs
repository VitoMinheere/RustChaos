//! Who's on the radar (#411, panel.cpp:5329): gang members and Men in Black within 16
//! squares whatever they're doing, the Men in Black white, and anyone after her pulsing; not
//! civilians. Skipped without an install.

use glam::vec3;
use uc_game::Controls;
use uc_world::character::person as ptype;

mod common;

#[test]
fn gang_and_mib_whatever_they_are_doing() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let pi = r.player();
    let me = r.g.people[pi].fig.pos;
    let others: Vec<usize> = (0..r.g.people.len()).filter(|&i| i != pi && !r.g.people[i].removed && !r.g.people[i].is_dead()).take(3).collect();
    assert_eq!(others.len(), 3);
    for (k, (&i, kind)) in others.iter().zip([ptype::THUG_GREY, ptype::MIB1, ptype::CIV]).enumerate() {
        let q = &mut r.g.people[i];
        (q.kind, q.ai, q.foe) = (kind, 0, None);
        q.fig.pos = me + vec3(1000.0 + 300.0 * k as f32, 0.0, 0.0);
    }
    let dots = r.g.radar();
    let at = |i: usize| r.g.people[i].fig.pos;
    let gang = dots.iter().find(|d| d.pos == at(others[0])).expect("no gang member on the radar");
    assert!(!gang.mib && !gang.after_her);
    assert!(dots.iter().find(|d| d.pos == at(others[1])).expect("no MIB on the radar").mib);
    assert!(!dots.iter().any(|d| d.pos == at(others[2])), "a civilian on the radar");
    // Out past 16 squares, off it.
    r.g.people[others[0]].fig.pos = me + vec3(17.0 * 256.0, 0.0, 0.0);
    assert!(!r.g.radar().iter().any(|d| d.pos == r.g.people[others[0]].fig.pos));
}
