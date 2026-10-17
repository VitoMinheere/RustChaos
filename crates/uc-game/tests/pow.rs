//! POW (fallen/Source/pow.cpp), the PlayStation's explosions: a bang of sprites that sets off
//! smaller bangs, which set off smaller ones again, all gone in under three seconds. The PC
//! game never makes one: a barrel going up is a `PYRO_FIREBOMB` (barrel.cpp:1891).

use glam::{vec3, Vec3};
use uc_game::pow::{self, Pow};

mod common;

/// Turns of `POW_process`, 20 a second.
fn turns(p: &mut Pow, n: usize) {
    for _ in 0..n {
        p.turn();
    }
}

/// `POW_CREATE_LARGE_SEMI`, every PlayStation barrel on the ground: a half-ball of 33 sprites
/// flung upwards, its three children (23 sprites each) at its 3rd, 6th and 8th turns, each
/// of those with three small bangs (10 sprites) of its own.
#[test]
fn a_large_bang_spawns_children_and_grandchildren() {
    let mut p = Pow::default();
    let at = vec3(1000.0, 200.0, 1000.0);
    p.create(pow::CREATE_LARGE_SEMI, at);
    assert_eq!(p.bangs.len(), 1);
    assert_eq!(p.sprite_count(), 33);
    turns(&mut p, 1);
    assert!(p.sprites().all(|(s, _)| s.y > at.y), "a semi-sphere goes up");
    turns(&mut p, 1);
    assert_eq!(p.bangs.len(), 1, "no child before the timer is under 3/8 of a second");
    turns(&mut p, 1);
    assert_eq!(p.bangs.len(), 2);
    assert_eq!(p.bangs[1].kind, pow::SPAWN_SPHERE_MEDIUM);
    assert_eq!(p.bangs[1].sprites.len(), 23);
    turns(&mut p, 5);
    assert_eq!(p.bangs.iter().filter(|b| b.kind == pow::SPAWN_SPHERE_MEDIUM).count(), 3);
    let mut most = 0;
    let mut small = 0;
    for _ in 0..20 {
        turns(&mut p, 1);
        most = most.max(p.bangs.len());
        small = small.max(p.bangs.iter().filter(|b| b.kind == pow::BASIC_SPHERE_SMALL).count());
    }
    assert!(small >= 3, "the medium bangs should set off small ones");
    assert!(most <= 13, "one bang, three children and nine grandchildren at most: {most}");
}

/// A sprite plays its sixteen frames at 96 to 191 256ths of the full rate (a frame every
/// 1/25 second at full), so it lasts between 18 and 35 turns. The last grandchild comes at
/// turn 16, so it's all over by turn 51, and still going at 1.6 seconds.
#[test]
fn it_dies_out_in_the_originals_time() {
    let mut p = Pow::default();
    p.create(pow::CREATE_LARGE_SEMI, Vec3::ZERO);
    let mut frames = vec![0u8; 0];
    turns(&mut p, 32);
    assert!(p.sprite_count() > 0, "nothing left at 1.6 seconds");
    for _ in 32..51 {
        frames.extend(p.sprites().map(|(_, f)| f));
        turns(&mut p, 1);
    }
    assert!(frames.iter().all(|&f| f < pow::FRAMES));
    assert!(p.bangs.is_empty(), "{} bangs left after 51 turns", p.bangs.len());
    assert_eq!(p.sprite_count(), 0);
}

/// The single sprites of the root bang: none outlives 35 turns, and none goes before 18.
#[test]
fn a_sprite_lasts_eighteen_to_thirty_five_turns() {
    let mut p = Pow::default();
    p.create(pow::CREATE_LARGE_SEMI, Vec3::ZERO);
    turns(&mut p, 17);
    assert_eq!(p.bangs[0].sprites.len(), 33, "a sprite of the first bang went too soon");
    turns(&mut p, 18);
    assert!(p.bangs.iter().all(|b| b.kind != pow::MULTISPAWN_SEMI), "the first bang outlived 35 turns");
}

/// A full-sized sphere (`POW_CREATE_LARGE`, a PlayStation barrel in the air) is 59 sprites,
/// and it plays in real time: [`Pow::update`] runs whole turns at 20 a second.
#[test]
fn a_large_sphere_plays_at_twenty_turns_a_second() {
    let mut p = Pow::default();
    p.create(pow::CREATE_LARGE, Vec3::ZERO);
    assert_eq!(p.sprite_count(), 59);
    assert!(p.sprites().any(|(s, _)| s.y < 0.0) || { p.turn(); p.sprites().any(|(s, _)| s.y < 0.0) });
    let mut q = Pow::default();
    q.create(pow::CREATE_LARGE, Vec3::ZERO);
    // A second at 30 frames a second is 20 turns, as the turns are.
    for _ in 0..30 {
        q.update(1.0 / 30.0);
    }
    let mut r = Pow::default();
    r.create(pow::CREATE_LARGE, Vec3::ZERO);
    turns(&mut r, 20);
    assert_eq!(q.bangs.len(), r.bangs.len());
    assert_eq!(q.sprite_count(), r.sprite_count());
}

/// There are only 31 bangs and 255 sprites, as on the PC; past that, nothing more is made.
#[test]
fn it_runs_out_of_bangs_and_sprites_quietly() {
    let mut p = Pow::default();
    for k in 0..40 {
        p.create(pow::CREATE_LARGE, vec3(k as f32 * 300.0, 0.0, 0.0));
    }
    assert!(p.bangs.len() <= pow::MAX_POWS);
    assert!(p.sprite_count() <= pow::MAX_SPRITES);
    turns(&mut p, 60);
    assert!(p.bangs.is_empty());
}

/// The PC game makes no POW: a barrel shot on Semtex goes up as a fireball, and the console's
/// `pow` is the only way to see one. Skipped without an install.
#[test]
fn a_barrel_makes_no_pow_on_the_pc() {
    let Some(mut r) = common::start("Semtex") else { return };
    r.play(1.0, uc_game::Controls::default());
    let k = r.g.barrels.iter().position(|b| b.explodes && !b.gone).expect("a barrel");
    let p = r.g.player.unwrap();
    r.g.shoot_barrel(k, p, &r.level);
    r.play(0.2, uc_game::Controls::default());
    assert!(r.g.barrels[k].gone);
    assert!(!r.g.pyro.puffs.is_empty(), "the barrel should go up in a fireball");
    assert!(r.g.pow.bangs.is_empty());
    // The console's `pow`, a square in front of her.
    r.g.pow_test();
    assert_eq!(r.g.pow.bangs.len(), 1);
    let her = r.g.player().unwrap().fig.pos;
    let at = r.g.pow.sprites().next().unwrap().0;
    assert!(((at - her) * vec3(1.0, 0.0, 1.0)).length() > 200.0);
    r.play(3.0, uc_game::Controls::default());
    assert!(r.g.pow.bangs.is_empty(), "gone in three seconds");
}
