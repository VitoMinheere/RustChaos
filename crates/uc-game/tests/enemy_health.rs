//! The health over whoever Darci is fighting or aiming at (#300, `OVERLAY_draw_enemy_health`):
//! with her gun on someone, their health against their type's; with nothing drawn and no fight,
//! none. Skipped without an install.

mod common;

use glam::vec3;
use uc_game::{Controls, Weapon};
use uc_world::character::person as ptype;

#[test]
fn her_gun_on_someone_shows_their_health() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    assert!(r.g.enemy_health().is_none(), "a health bar with nobody to fight");
    let me = r.player();
    let i = r.g.people.iter().position(|q| q.kind == ptype::CIV && !q.is_dead() && !q.removed).expect("no crowd");
    let at = r.g.people[me].fig.pos;
    let f = r.g.people[me].facing_dir();
    r.g.people[i].fig.pos = at + vec3(f.x, 0.0, f.y) * 300.0;
    r.g.people[i].health = 100;
    r.g.people[me].give(Weapon::Pistol);
    r.g.people[me].weapon = Weapon::Pistol;
    r.g.target = Some(i);
    r.play(0.5, Controls::default());
    let full = ptype::HEALTH[ptype::CIV as usize] as f32;
    let (pos, frac) = r.g.enemy_health().expect("no health bar over her target");
    assert!((pos - r.g.people[i].fig.pos).length() < 1.0);
    assert!((frac - 100.0 / full).abs() < 0.01, "{frac}");
}
