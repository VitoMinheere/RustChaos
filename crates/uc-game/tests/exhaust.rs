//! The bike's exhaust (#107, bike.cpp:1621, fallen/Source/ribbon.cpp): two ribbons of smoke
//! that trail behind it as it's ridden, fading out at their far end. Skipped without an
//! install.

use std::collections::BTreeMap;
use glam::Vec3;
use uc_game::{Controls, Drive, State};
use uc_world::geometry::SMOKER_PAGE;

mod common;
use common::{Run, DT};

#[test]
fn riding_the_bike_trails_smoke() {
    let Some(level) = common::load("train1") else { return };
    let mut r = Run::new(level);
    r.until(60.0, |r| r.g.time > 1.0 && !r.g.in_conversation());
    let k = r.g.bikes.iter().position(|(b, _)| b.way == 66).expect("no bike");
    let exhaust = r.g.bikes[k].0.exhaust.map(|e| e.expect("no exhaust ribbon"));
    let p = r.player();
    let at = r.g.bikes[k].0.pos();
    r.stand(at + Vec3::new(100.0, 0.0, 0.0));
    r.g.people[p].state = State::Move;
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(3.0, Controls::default());
    assert!(matches!(r.g.people[p].state, State::Bike { phase: 1, .. }), "not riding: {:?}", r.g.people[p].state);

    let go = Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() };
    let from = r.g.bikes[k].0.pos();
    let scroll = r.g.ribbons.get(exhaust[0]).unwrap().scroll;
    r.play(1.0, go);
    let bike = r.g.bikes[k].0.pos();
    let ahead = (bike - from).with_y(0.0).normalize();
    println!("bike went {:.0} from {from:?} to {bike:?}", (bike - from).length());
    assert!((bike - from).length() > 100.0, "the bike hardly moved");

    for e in exhaust {
        let ribbon = r.g.ribbons.get(e).unwrap();
        let points: Vec<Vec3> = ribbon.points().collect();
        // Full: ten in the ring, nine kept.
        assert_eq!(points.len(), 9);
        // Behind the bike, the oldest furthest back.
        let along: Vec<f32> = points.iter().map(|q| (*q - bike).dot(ahead)).collect();
        println!("along the way: {along:?}");
        assert!(along.iter().all(|&a| a < 0.0), "smoke ahead of the bike: {along:?}");
        assert!(along[0] < along[8] - 20.0, "not trailing: {along:?}");
        // Near the ground, by the bike.
        assert!(points.iter().all(|q| (q.y - bike.y).abs() < 100.0));
    }
    // Its texture slides along it.
    assert_ne!(r.g.ribbons.get(exhaust[0]).unwrap().scroll, scroll);

    // Drawn with the smoke page: clear at the far end, solid at the bike.
    let mut out = BTreeMap::new();
    r.g.ribbons.draw(&mut out);
    let smoke = &out[&SMOKER_PAGE];
    assert_eq!(smoke.indices.len(), 2 * 7 * 3, "two strips of seven triangles");
    let alphas: Vec<u8> = smoke.vertices.iter().map(|v| v.color[3]).collect();
    assert_eq!(alphas[0], 0, "the far end isn't clear");
    assert_eq!(*alphas.last().unwrap(), 255, "the near end isn't solid");
    assert!(smoke.vertices.iter().all(|v| v.color[..3] == [0x6f; 3]));
    // And the install has the page.
    assert!(r.level.texture(SMOKER_PAGE).is_some(), "no smoker2.tga");
}
