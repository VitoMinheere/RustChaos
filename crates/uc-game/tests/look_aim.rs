//! Looking round (#417, `apply_button_input_first_person`, interfac.cpp:7459; guns.cpp:379):
//! she turns with the view, and her gun goes for whoever's at the height she's looking at,
//! passing over someone at her own height to one up on a roof. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Weapon};

mod common;

#[test]
fn she_turns_with_the_view() {
    let Some(mut r) = common::start("fight2") else { return };
    r.play(1.0, Controls::default());
    let pi = r.player();
    let a = r.g.people[pi].fig.angle + 1.0;
    r.play(0.2, Controls { look: Some((a, 0.0)), ..Default::default() });
    assert!((r.g.people[pi].fig.angle - a).abs() < 1e-3, "facing {} not {a}", r.g.people[pi].fig.angle);
}

#[test]
fn looking_up_picks_out_the_one_up_high() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let pi = r.player();
    let p = &mut r.g.people[pi];
    p.give(Weapon::Pistol);
    p.weapon = Weapon::Pistol;
    let (me, ahead) = (p.fig.pos, p.fig.facing());
    let ahead = vec3(ahead.x, 0.0, ahead.y);
    let others: Vec<usize> = (0..r.g.people.len()).filter(|&i| i != pi && !r.g.people[i].removed && !r.g.people[i].is_dead()).take(2).collect();
    for i in 0..r.g.people.len() {
        if i != pi && !others.contains(&i) {
            r.g.people[i].removed = true;
        }
    }
    let angle = r.g.people[pi].fig.angle;
    // One just ahead at her height, the other further and 250 up.
    let (low, high) = (others[0], others[1]);
    let place = |r: &mut common::Run| {
        r.g.people[low].fig.pos = me + ahead * 150.0;
        r.g.people[high].fig.pos = me + ahead * 300.0 + vec3(0.0, 250.0, 0.0);
    };
    place(&mut r);
    r.play(common::DT, Controls { look: Some((angle, 0.0)), ..Default::default() });
    assert_eq!(r.g.target, Some(low), "looking straight ahead");
    r.g.target = None;
    place(&mut r);
    r.play(common::DT, Controls { look: Some((angle, (250.0f32).atan2(300.0))), ..Default::default() });
    assert_eq!(r.g.target, Some(high), "looking up");
}
