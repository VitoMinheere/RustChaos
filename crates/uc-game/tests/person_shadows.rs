//! People's own shadows (#241, `SMAP_person`): the four nearest Darci within six squares
//! cast their shapes onto the ground round them, fading out by 256 off them, in place of the
//! round blobs. Skipped without an install.

mod common;

use glam::Vec3;
use uc_game::smap;
use uc_game::Controls;
use uc_world::{SHADOW, SMAP, UNIT};

#[test]
fn people_near_her_cast_their_own_shapes() {
    let Some(mut r) = common::start("Assault1") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    let at = r.g.people[me].fig.pos;
    let casters = r.g.shadow_casters();
    assert!(!casters.is_empty() && casters.len() <= smap::MAX, "{} casters", casters.len());
    // Darci first: nearest the focus, and everyone within six squares of it.
    assert_eq!(casters[0].0, me);
    for (i, _) in &casters {
        let p = r.g.people[*i].fig.pos;
        assert!((p.x - at.x).abs() + (p.z - at.z).abs() < smap::RANGE);
    }
    // Her map has her in it.
    let solid = casters[0].1.pixels.iter().filter(|&&c| c > 128).count();
    assert!(solid > 100, "only {solid} pixels of her");
    let eye = at + Vec3::new(0.0, 300.0, -500.0);
    let b = r.g.batches(&r.level, eye);
    let hers = b.get(&SMAP).expect("her shadow on the ground");
    assert!(!hers.indices.is_empty());
    // On the ground by her, away from the light, and gone by 256 off her.
    let body = r.g.people[me].body_pos();
    for v in &hers.vertices {
        let p = Vec3::from(v.pos) * UNIT;
        assert!((p.y - body.y).abs() < 64.0, "a shadow at {p}, her at {body}");
        let d = (p.x - body.x).abs() + (p.z - body.z).abs();
        assert!(d < 400.0, "a shadow {d} off her");
        if d > 257.0 {
            assert_eq!(v.color[3], 0, "not faded at {d}");
        }
        assert!(v.uv.iter().all(|c| (0.0..=1.0).contains(c)), "off the map: {:?}", v.uv);
    }
    // The light shines towards +x and -z: so does her shadow.
    let mid = hers.vertices.iter().map(|v| Vec3::from(v.pos) * UNIT).sum::<Vec3>() / hers.vertices.len() as f32;
    assert!(mid.x - body.x > 0.0 && mid.z - body.z < 0.0, "her shadow lies towards {}", mid - body);
    // No round blobs for anyone standing.
    assert!(b.get(&SHADOW).is_none_or(|s| s.vertices.is_empty()), "blob shadows still drawn");
    assert_eq!(r.g.shadow_maps().len(), casters.len());
}

#[test]
fn with_shadows_off_nobody_casts_one() {
    let Some(mut r) = common::start("Assault1") else { return };
    r.play(1.0, Controls::default());
    r.g.show.shadows = false;
    let at = r.g.people[r.player()].fig.pos;
    let b = r.g.batches(&r.level, at + Vec3::new(0.0, 300.0, -500.0));
    assert!((0..smap::MAX as u32).all(|k| !b.contains_key(&(SMAP - k))));
    assert!(r.g.shadow_maps().is_empty());
}
