//! Gangs (#245, `PCOM_BENT_GANG`): when one of a gang goes into a fight, the rest of their
//! colour who aren't busy come for the same person, however far off; another gang doesn't.
//! Three of RTA's crowd made gang members. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, AI_GANG, BENT_GANG};
use uc_game::Controls;
use uc_world::character::person as ptype;

mod common;

#[test]
fn a_gang_backs_up_one_of_its_own() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let crowd: Vec<usize> = r.g.people.iter().enumerate().filter(|(_, q)| q.fake && q.ai == AI_CIV && !q.is_dead()).map(|(i, _)| i).collect();
    let (a, b, other) = (crowd[0], crowd[1], crowd[2]);
    let me = r.g.people[r.player()].fig.pos;
    // All facing away from her (along +x, angle -90 degrees), out of sight.
    for (i, colour, off) in [(a, 3, 8.0 * 256.0), (b, 3, 40.0 * 256.0), (other, 2, -40.0 * 256.0)] {
        let q = &mut r.g.people[i];
        (q.ai, q.kind, q.mv, q.fake, q.bent, q.colour) = (AI_GANG, ptype::THUG_RED, 1, false, BENT_GANG, colour);
        q.fig.pos = me + vec3(off, 0.0, 0.0);
        q.fig.angle = uc_world::character::angle_of(glam::vec2(off.signum(), 0.0));
    }
    r.play(0.5, Controls::default());
    assert!(!r.g.people[b].aggro() && !r.g.people[other].aggro(), "nobody's fighting yet");
    // One of them is hit by her: into the fight.
    let pi = r.player();
    let from = r.g.people[pi].fig.pos;
    r.g.people[a].take_hit(from, 1, 0, false, false, uc_game::Weapon::None);
    r.play(0.2, Controls::default());
    assert!(r.g.people[a].aggro());
    assert!(r.g.people[b].aggro(), "his mate forty squares off should come too");
    assert_eq!(r.g.people[b].foe, Some(pi));
    assert!(!r.g.people[other].aggro(), "another gang stays out of it");
}
