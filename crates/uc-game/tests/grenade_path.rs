//! The grenade's path shown before the throw (#419, `show_grenade_path`, grenade.cpp:440):
//! standing with one in hand, where it would go, ending about where a throw then comes to
//! rest; none walking. Skipped without an install.

use glam::vec2;
use uc_game::{Controls, Weapon};

mod common;

#[test]
fn the_path_shows_where_it_lands() {
    let Some(mut r) = common::start("fight2") else { return };
    r.play(1.0, Controls::default());
    let pi = r.player();
    let p = &mut r.g.people[pi];
    (p.weapon, p.grenades) = (Weapon::Grenade, 3);
    r.play(1.0, Controls::default());
    let path = r.g.grenade_path(&r.level).expect("no path standing with a grenade");
    assert!(path.len() > 5, "{} points", path.len());
    let end = *path.last().unwrap();
    // Prime it, throw it, and see where it stops.
    let punch = Controls { punch: true, ..Default::default() };
    r.play(common::DT, punch);
    r.play(0.2, Controls::default());
    r.play(common::DT, punch);
    let mut rest = None;
    for _ in 0..(5.0 / common::DT) as usize {
        r.play(common::DT, Controls::default());
        if let Some(g) = r.g.grenades.first().filter(|g| g.vel == glam::Vec3::ZERO) {
            rest = Some(g.pos);
            break;
        }
    }
    let rest = rest.expect("the grenade never came to rest");
    assert!(vec2(rest.x - end.x, rest.z - end.z).length() < 100.0, "shown landing at {end:?}, landed at {rest:?}");
    // Walking, no path.
    r.play(0.5, Controls { wish: vec2(1.0, 0.0), ..Default::default() });
    assert!(r.g.grenade_path(&r.level).is_none(), "a path while walking");
}
