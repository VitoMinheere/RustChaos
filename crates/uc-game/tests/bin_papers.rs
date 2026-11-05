//! A bin knocked over throws out four bits of paper from its top (`DIRT_create_papers`, from
//! `BARREL_convert_stationary_to_moving`, barrel.cpp:248), which flutter down as leaves.
//! Skipped without an install.

use uc_game::dirt::Kind;
use uc_game::{Controls, Game};

mod common;
use common::DT;

#[test]
fn a_bin_knocked_over_throws_out_papers() {
    let Some(level) = common::load("police1") else { return };
    let mut g = Game::new(&level, true);
    common::frames(&mut g, &level, 30);
    let b = (0..g.barrels.len()).find(|&b| g.barrels[b].bin() && g.barrels[b].spheres.is_none()).expect("a bin standing");
    let at = g.barrels[b].pos;
    let near = |g: &Game| g.dirt.pieces.iter().flatten()
        .filter(|p| p.kind == Kind::Leaf && !p.still && ((p.x as f32 - at.x).powi(2) + (p.z as f32 - at.z).powi(2)).sqrt() < 80.0)
        .count();
    assert_eq!(near(&g), 0);
    // Darci walks into it.
    let me = g.player.unwrap();
    g.people[me].fig.pos = at + glam::vec3(-120.0, 0.0, 0.0);
    // The frame it goes over, four bits of paper come out of it.
    let mut knocked = None;
    for _ in 0..30 {
        let before = near(&g);
        g.update(DT, &Controls { wish: glam::vec2(1.0, 0.0), ..Default::default() }, &level);
        if g.barrels[b].spheres.is_some() {
            knocked = Some(near(&g) - before);
            break;
        }
    }
    // (Walking up to it she may kick up a leaf of her own.)
    assert!(knocked.is_some_and(|n| n >= 4), "four bits of paper as it goes over: {knocked:?}");
}
