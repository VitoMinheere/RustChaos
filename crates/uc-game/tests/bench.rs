//! Darci sits on a bench and gets health back (#294, `set_person_sit_down`): action stood
//! at one sits her on it, a point of health every fifth of a second up to 39, once in a
//! while; pushing any way gets her up. On one of RTA's park benches. Skipped without an
//! install.

use glam::{vec2, vec3};
use uc_game::{Controls, State};
use uc_world::character::anim;

mod common;
use common::Run;

/// RTA with Darci at a park bench (`PRIM_OBJ_PARK_BENCH`), hurt.
fn at_a_bench() -> Option<Run> {
    let mut r = common::start("police1")?;
    r.heal = false;
    r.play(1.0, Controls::default());
    let bench = r.level.map.objects.iter().find(|o| o.prim == 89).expect("RTA has benches");
    let at = vec3(bench.x as f32, bench.y as f32, bench.z as f32);
    r.stand(at);
    let pi = r.player();
    r.g.people[pi].health = 100;
    Some(r)
}

#[test]
fn she_sits_and_gets_health_back() {
    let Some(mut r) = at_a_bench() else { return };
    let pi = r.player();
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(matches!(r.g.people[pi].state, State::Sit), "{:?}", r.g.people[pi].state);
    assert_eq!(r.g.people[pi].fig.anim(), anim::SIT_DOWN);
    r.play(12.0, Controls::default());
    assert!(matches!(r.g.people[pi].state, State::Sit), "got up by herself");
    assert_eq!(r.g.people[pi].health, 139, "39 back, no more");
    // Pushing any way gets her up.
    r.play(0.5, Controls { wish: vec2(1.0, 0.0), ..Default::default() });
    assert!(!matches!(r.g.people[pi].state, State::Sit), "still sitting");
}

/// Sitting again straight away gives nothing more.
#[test]
fn not_again_straight_away() {
    let Some(mut r) = at_a_bench() else { return };
    let pi = r.player();
    r.play(common::DT, Controls { action: true, ..Default::default() });
    r.play(1.0, Controls::default());
    r.play(0.3, Controls { wish: vec2(1.0, 0.0), ..Default::default() });
    let at = r.g.people[pi].fig.pos;
    r.stand(at);
    let had = r.g.people[pi].health;
    r.play(common::DT, Controls { action: true, ..Default::default() });
    assert!(matches!(r.g.people[pi].state, State::Sit), "{:?}", r.g.people[pi].state);
    r.play(10.0, Controls::default());
    assert_eq!(r.g.people[pi].health, had);
}
