//! Items a person drops (`EWAY_ARG_ITEM_FOLLOW_PERSON`, #233): a flagged `create_item` puts
//! its item by the person its trigger is about, wherever they are by then, not at the point.
//! The Jump's point 187 is a baseball bat that the man from point 14 hands over when Darci
//! talks to him. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::Controls;

mod common;
use common::DT;

#[test]
fn a_flagged_item_turns_up_by_its_person() {
    let Some(mut r) = common::start("police2") else { return };
    // Past the opening talk.
    r.play(1.0, Controls::default());
    for _ in 0..240 {
        if !r.g.in_conversation() {
            break;
        }
        r.play(0.25, Controls::default());
    }
    let i = r.who(14);
    // Somewhere else than where the point is, so the two can't be confused.
    let at = r.g.people[i].fig.pos + vec3(-900.0, 0.0, 0.0);
    let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    r.g.people[i].fig.pos = vec3(at.x, y, at.z);
    let him = r.g.people[i].fig.pos;
    r.stand(him + vec3(120.0, 0.0, 0.0));
    r.play(DT, Controls { action: true, ..Default::default() });
    let mut found = None;
    for _ in 0..(20.0 / DT) as usize {
        r.play(DT, Controls::default());
        found = r.g.items.iter().find(|it| it.way == 187).map(|it| it.pos);
        if found.is_some() {
            break;
        }
    }
    let bat = found.expect("talking to him should give the bat");
    let point = r.point(187);
    let from_him = vec2(bat.x - him.x, bat.z - him.z).length();
    assert!(from_him < 256.0, "the bat is {from_him} from him, at {bat}");
    assert!(vec2(bat.x - point.x, bat.z - point.z).length() > 500.0, "the bat was put at its point, {point}");
}
