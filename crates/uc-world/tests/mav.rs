//! The navigation map and route search (fallen/Source/mav.cpp). Skipped without an install.

use uc_world::mav::{self, Mav};
use uc_world::{Level, LoadOptions};

fn level(name: &str) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, name, &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap())
}

#[test]
fn builds_quickly() {
    let Some(level) = level("police1") else { return };
    let t = std::time::Instant::now();
    let m = Mav::new(&level);
    let secs = t.elapsed().as_secs_f32();
    println!("built in {secs:.2}s");
    let walkable = (0..128).flat_map(|x| (0..128).map(move |z| (x, z))).filter(|&(x, z)| m.opt(x, z, 0) & mav::caps(mav::GOTO) != 0).count();
    println!("{walkable} squares walkable westwards");
    assert!(walkable > 2000);
}

/// Follows `route` a step at a time from `a` to `b`, walking only: the squares it goes
/// through, or `None` if it gets stuck.
fn walk(m: &Mav, a: (u8, u8), b: (u8, u8)) -> Option<Vec<(u8, u8)>> {
    let mut at = a;
    let mut path = vec![a];
    let mut rng = 1u32;
    for _ in 0..60 {
        if at == b {
            return Some(path);
        }
        let (act, _) = m.route(at, b, mav::caps(mav::GOTO), &mut || { rng = rng.wrapping_mul(69069).wrapping_add(1); rng >> 16 });
        if act.action != mav::GOTO || act.dest == at {
            return None;
        }
        assert!(m.can_walk(at, act.dest), "told to walk {at:?} -> {:?} through a wall", act.dest);
        at = act.dest;
        path.push(at);
    }
    None
}

/// Round corners: between squares a straight walk can't join but a route on foot can,
/// following the route gets there, every leg of it a straight walk.
#[test]
fn routes_round_buildings() {
    let Some(level) = level("police1") else { return };
    let m = Mav::new(&level);
    let open = |x: i32, z: i32| (0..4).any(|d| m.opt(x, z, d) & mav::caps(mav::GOTO) != 0);
    let (mut tried, mut reached) = (0, 0);
    let mut rng = 7u32;
    let mut next = || { rng = rng.wrapping_mul(69069).wrapping_add(1); (rng >> 16) as i32 };
    while tried < 40 {
        let a = (20 + next() % 88, 20 + next() % 88);
        let b = (a.0 + next() % 21 - 10, a.1 + next() % 21 - 10);
        if !open(a.0, a.1) || !open(b.0, b.1) || !(0..128).contains(&b.0) || !(0..128).contains(&b.1) {
            continue;
        }
        let (a, b) = ((a.0 as u8, a.1 as u8), (b.0 as u8, b.1 as u8));
        if m.can_walk(a, b) {
            continue;
        }
        // Only pairs a route on foot joins.
        let (_, found) = m.route(a, b, mav::caps(mav::GOTO), &mut || 0);
        if !found {
            continue;
        }
        tried += 1;
        if walk(&m, a, b).is_some() {
            reached += 1;
        }
    }
    println!("{reached} of {tried} reached");
    assert!(reached >= tried * 9 / 10, "only {reached} of {tried}");
}

/// The combat tutorial's ladder (from the platform rim up to the roof): the way up to the
/// roof is by it, for someone who can climb (`MAV_CAPS_DARCI`), and there's none on foot.
#[test]
fn up_the_ladder() {
    let Some(level) = level("ftutor1") else { return };
    let m = Mav::new(&level);
    let l = level.ground.ladders[0];
    let foot = l.foot + l.out * 128.0;
    let roof = l.foot - l.out * 384.0;
    let sq = |v: glam::Vec2| ((v.x / 256.0) as u8, (v.y / 256.0) as u8);
    let (a, b) = (sq(foot), sq(roof));
    println!("ladder {:?}: foot {a:?} h {} roof {b:?} h {}", l, m.height(a.0 as i32, a.1 as i32), m.height(b.0 as i32, b.1 as i32));
    let (act, _) = m.route(a, b, mav::CAPS_DARCI, &mut || 0);
    assert_eq!(act.action, mav::LADDER_UP, "{act:?}");
    let (_, found) = m.route(a, b, mav::caps(mav::GOTO), &mut || 0);
    assert!(!found, "walked onto the roof");
}
