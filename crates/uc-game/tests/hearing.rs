//! What the AI hears (#251, `PCOM_oscillate_tympanum`): a landing is as loud as it was hard,
//! and a killer's "what you looking at" to someone ignoring him sends the passers-by round
//! about running. In RTA, with Darci and two of its crowd. Skipped without an install.

use glam::vec3;
use uc_game::person::{AI_BULLY, AI_CIV, MOVE_WANDER};
use uc_game::{Controls, State};
use uc_world::character::person as ptype;

mod common;
use common::DT;

#[test]
fn a_hard_landing_is_heard_further() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let pi = r.player();
    // Coming down at 20000 a tick at the original's scale: as loud as a landing gets.
    let mut heard = |vy: f32, up: f32| {
        let p = &mut r.g.people[pi];
        p.fig.pos.y += up;
        p.state = State::Fall { vy, vel: glam::Vec2::ZERO };
        let mut loudest: f32 = 0.0;
        for _ in 0..30 {
            r.play(DT, Controls::default());
            // Whatever the landing leaves her doing: a hard one hurts.
            if !matches!(r.g.people[pi].state, State::Fall { .. }) {
                loudest = loudest.max(r.g.people[pi].snap().noise);
            }
        }
        loudest
    };
    assert_eq!(heard(-1700.0, 64.0), 0x600 as f32);
    assert_eq!(heard(-1000.0, 64.0), 0x400 as f32);
    assert_eq!(heard(-500.0, 64.0), 0x200 as f32);
    assert_eq!(heard(-100.0, 8.0), 0.0, "a soft landing is too quiet to hear");
}

#[test]
fn what_you_looking_at_sends_the_crowd_running() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let crowd: Vec<usize> = r.g.people.iter().enumerate().filter(|(_, q)| q.fake && q.ai == AI_CIV && !q.is_dead()).map(|(i, _)| i).collect();
    let (bully, mark, passer) = (crowd[0], crowd[1], crowd[2]);
    let pi = r.player();
    let me = r.g.people[pi].fig.pos;
    // Darci well away; the bully, his mark three squares off, and a passer-by beside him.
    r.g.people[pi].fig.pos = me + vec3(30.0 * 256.0, 0.0, 0.0);
    for (i, off) in [(bully, 0.0), (mark, 768.0), (passer, -300.0)] {
        let at = me + vec3(0.0, 0.0, off);
        let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
        let q = &mut r.g.people[i];
        q.fig.pos = vec3(at.x, y, at.z);
        (q.fake, q.mv) = (false, if i == bully { 1 } else { MOVE_WANDER });
    }
    let q = &mut r.g.people[bully];
    (q.ai, q.kind, q.bent) = (AI_BULLY, ptype::THUG_RASTA, 0);
    let mut ran = false;
    for _ in 0..(6.0 / DT) as usize {
        r.play(DT, Controls::default());
        ran |= r.g.people[passer].scared.is_some();
    }
    assert!(ran, "the passer-by never ran from him");
}
