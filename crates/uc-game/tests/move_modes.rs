//! What people's moves have them do at home (#247): Gatecrasher's crowd dance, Headline
//! Hostage's hostage stands with her hands up (and no gun), and someone set to warm their
//! hands walks over to a fire. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, MOVE_WARM_HANDS};
use uc_game::{Controls, State, Weapon};
use uc_world::character::anim;

mod common;

#[test]
fn the_dancers_dance() {
    let Some(mut r) = common::start("bball2") else { return };
    r.play(2.0, Controls::default());
    let dances = [anim::DANCE_BOOGIE, anim::DANCE_WOOGIE, anim::DANCE_HEADBANG];
    let dancers: Vec<_> = r.g.people.iter().filter(|p| p.mv == 8 && !p.removed).collect();
    assert!(dancers.len() > 10, "only {} dancers", dancers.len());
    for p in &dancers {
        assert!(matches!(p.state, State::Pose { .. }), "point {} isn't dancing: {:?}", p.way, p.state);
        assert!(dances.contains(&p.fig.anim()), "point {} plays {}", p.way, p.fig.anim());
    }
    // Not all to the same dance.
    let first = dancers[0].fig.anim();
    assert!(dancers.iter().any(|p| p.fig.anim() != first));
}

#[test]
fn a_hostage_keeps_her_hands_up() {
    let Some(mut r) = common::start("Gordout1") else { return };
    r.play(3.0, Controls::default());
    let p = &r.g.people[r.who(3)];
    assert!(matches!(p.state, State::Pose { anim: anim::HANDS_UP }), "{:?}", p.state);
    assert_eq!(p.fig.anim(), anim::HANDS_UP_LOOP);
    assert!(!p.weapon.is_gun() || p.weapon == Weapon::None);
}

#[test]
fn cold_hands_go_to_the_fire() {
    // RTA's crashed taxi is burning.
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let (fire, _) = r.g.pyro.fires().next().expect("RTA's taxi burns");
    let i = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("crowd");
    let at = fire + vec3(500.0, 0.0, 0.0);
    let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    let q = &mut r.g.people[i];
    // As Officer Training's one is, someone who isn't a civilian: a civilian would run from
    // the fire before getting there (collide.cpp:6261, #297).
    (q.mv, q.fake, q.ai) = (MOVE_WARM_HANDS, false, 5);
    q.fig.pos = vec3(at.x, y, at.z);
    r.play(10.0, Controls::default());
    let d = (r.g.people[i].fig.pos - fire).length();
    assert!(d < 200.0, "{d} from the fire after ten seconds");
}
