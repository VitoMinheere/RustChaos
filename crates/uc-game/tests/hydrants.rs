//! Hydrants (#296, `OB_damage`, `OB_process`): one knocked over gushes water for fifteen
//! seconds, with a hiss, then dries up. One of RTA's. Skipped without an install.

use glam::Vec3;
use uc_game::Controls;

mod common;

#[test]
fn a_broken_hydrant_gushes_then_dries_up() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let i = r.level.map.objects.iter().position(|o| o.prim == 228).expect("a hydrant");
    let o = &r.level.map.objects[i];
    let at = Vec3::new(o.x as f32, o.y as f32, o.z as f32);
    r.g.damage_prop(&r.level, i);
    r.play(1.0, Controls::default());
    assert_eq!(r.g.hydrant_noise(at), Some(at), "no hiss");
    let water = r.g.drops.drops.iter().filter(|d| d.water && (d.pos - at).length() < 200.0).count();
    assert!(water >= 3, "only {water} drops of water by it");
    r.play(15.0, Controls::default());
    assert!(r.g.hydrant_noise(at).is_none(), "still gushing after fifteen seconds");
}
