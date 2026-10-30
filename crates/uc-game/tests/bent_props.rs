//! Damaged props drawn bent (#296, `OB_damage`): a lamppost leans, a crumpling prop
//! crumples a step further each time, and both are chunks of their own in the static world
//! for the viewer to leave out once they are. On RTA's. Skipped without an install.

use glam::Vec3;
use uc_game::Controls;

mod common;

/// The first prop of RTA whose prim has `PRIM_DAMAGE_DAMAGABLE` and `flag`.
fn first(r: &common::Run, flag: u8) -> usize {
    r.level.ground.prop_damage.iter().position(|&(d, _)| d & 1 != 0 && d & flag != 0).expect("no such prop")
}

fn at(r: &common::Run, i: usize) -> Vec3 {
    let o = &r.level.map.objects[i];
    Vec3::new(o.x as f32, o.y as f32, o.z as f32)
}

#[test]
fn a_lamppost_leans() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(0.5, Controls::default());
    let i = first(&r, 8);
    let base = at(&r, i);
    assert!(r.g.bent.is_empty());
    r.g.damage_prop(&r.level, i, base + Vec3::new(300.0, 0.0, 0.0));
    let b = r.g.bent.get(&i).expect("not bent");
    // 40 of 2048 a way, give or take up to 28: between 2 and 12 degrees.
    for a in [b.pitch, b.roll] {
        assert!((2.0..12.0).contains(&a.abs().to_degrees()), "leans {} degrees", a.to_degrees());
    }
    // Its foot stays put and its top has gone over.
    assert!((b.place(Vec3::ZERO) - base).length() < 1.0);
    let up = b.place(Vec3::new(0.0, 400.0, 0.0)) - base;
    assert!(Vec3::new(up.x, 0.0, up.z).length() > 20.0, "the top hasn't moved: {up}");
    assert!(r.g.bent_props().any(|p| p == i));
}

#[test]
fn others_crumple_further_each_time() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(0.5, Controls::default());
    let i = first(&r, 4);
    let base = at(&r, i);
    let mut seen = Vec::new();
    for _ in 0..6 {
        r.g.damage_prop(&r.level, i, base);
        seen.push(r.g.bent[&i].crumple);
    }
    assert_eq!(seen, [1, 2, 3, 4, 3, 4]);
    let b = &r.g.bent[&i];
    assert_eq!((b.pitch, b.roll), (0.0, 0.0), "a crumpling prop doesn't lean");
    let v = Vec3::new(10.0, 50.0, -20.0);
    assert!((b.place(v) - base).length() > 0.0);
}

/// Every prop that bends is a chunk of its own in the static world, to leave out.
#[test]
fn bending_props_have_chunks_of_their_own() {
    let Some(r) = common::start("police1") else { return };
    let geo = r.level.build();
    let own: std::collections::BTreeSet<usize> = geo.chunks.iter().filter_map(|c| c.prop).collect();
    let bends: Vec<usize> = r.level.ground.prop_damage.iter().enumerate()
        .filter(|(i, &(d, _))| d & 1 != 0 && d & 12 != 0 && !r.level.barrel_props.contains(i) && !r.level.platforms.iter().any(|p| p.1 == *i))
        .map(|(i, _)| i)
        .collect();
    assert!(!bends.is_empty());
    for i in bends {
        assert!(own.contains(&i) || uc_world::level::load_prim(&r.level.game, r.level.map.objects[i].prim).is_none(), "prop {i} isn't in a chunk of its own");
    }
}
