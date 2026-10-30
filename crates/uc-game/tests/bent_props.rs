//! Props bent by a knock (#296, `OB_damage`): a sign leans away, a crumpling prop crumples,
//! and the level no longer draws them itself. RTA's. Skipped without an install.

use glam::Vec3;
use std::collections::BTreeMap;
use uc_game::Controls;
use uc_world::level::prim_damage::{CRUMPLE, EXPLODES, LEAN};
use uc_world::Batch;

mod common;

/// Every vertex the bendable props are drawn with, in order.
fn drawn(r: &common::Run) -> Vec<Vec3> {
    let mut out = BTreeMap::<u32, Batch>::new();
    r.g.bends.batches(&mut out);
    out.values().flat_map(|b| b.vertices.iter().map(|v| Vec3::from(v.pos))).collect()
}

#[test]
fn a_knocked_prop_leans_or_crumples() {
    let Some(mut r) = common::start("police1") else { return };
    let bendable = r.level.bendable();
    assert!(bendable.len() > 100, "only {} bendable props", bendable.len());
    r.play(0.1, Controls::default());
    let before = drawn(&r);
    assert!(!before.is_empty());
    // One that leans and one that crumples, neither going up when it's hit.
    let find = |kind: u8| {
        *bendable.iter().find(|&&i| {
            let (d, f) = r.level.ground.prop_damage[i];
            d & kind != 0 && d & EXPLODES == 0 && f & 1 == 0
        }).expect("a prop of the kind")
    };
    for (i, kind, most) in [(find(LEAN), LEAN, 0.5), (find(CRUMPLE), CRUMPLE, 12.0 / 256.0)] {
        let was = drawn(&r);
        r.g.damage_prop(&r.level, i, (0.0, 0.0));
        r.play(0.1, Controls::default());
        assert!(r.g.prop_bends.contains_key(&i), "{kind}: not bent");
        let now = drawn(&r);
        assert_eq!(was.len(), now.len(), "{kind}: the props aren't all drawn");
        let moved: Vec<f32> = was.iter().zip(&now).map(|(a, b)| (*a - *b).length()).filter(|d| *d > 1e-4).collect();
        assert!(!moved.is_empty(), "{kind}: nothing moved");
        let far = moved.iter().copied().fold(0.0, f32::max);
        assert!(far < most, "{kind}: moved {far} squares");
    }
}
