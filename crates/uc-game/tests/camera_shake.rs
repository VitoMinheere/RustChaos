//! A blast shakes the camera (#236, `FC_explosion`, fallen/Source/fc.cpp:2085): hard close to
//! it, less further off, not at all beyond sixteen times its force, and it dies down within
//! a second or two. Semtex's drum 324 is the blast. Skipped without an install.

use glam::vec3;
use uc_game::Controls;

mod common;
use common::Run;

/// Blows up Semtex's drum 324 with the player `dist` from it, and returns how hard the
/// camera shakes straight after.
fn shake_at(dist: f32) -> Option<(Run, f32)> {
    let mut r = common::start("Semtex")?;
    r.play(1.0, Controls::default());
    let k = r.g.barrels.iter().position(|b| b.way == 324).expect("drum 324");
    let at = r.g.barrels[k].pos;
    r.stand(at + vec3(dist, 0.0, 0.0));
    let p = r.player();
    r.g.shoot_barrel(k, p, &r.level);
    r.play(0.1, Controls::default());
    let s = r.g.shake;
    Some((r, s))
}

#[test]
fn a_blast_shakes_the_camera_and_it_dies_down() {
    let Some((mut r, near)) = shake_at(400.0) else { return };
    let Some((_, further)) = shake_at(1500.0) else { return };
    assert!(near > 100.0, "a blast four hundred off should shake it hard, got {near}");
    assert!(further > 0.0 && further < near, "further off it should shake less: {further} against {near}");
    r.play(3.0, Controls::default());
    assert_eq!(r.g.shake, 0.0, "the shake should have died down");
}

#[test]
fn a_blast_far_off_doesnt_shake_it() {
    let Some((_, far)) = shake_at(30.0 * 256.0) else { return };
    assert_eq!(far, 0.0);
}
