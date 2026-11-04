//! Builds real levels from an install (skipped when none is found; see uc-formats tests).

use uc_world::character::{anim, person};
use glam::Vec3;
use uc_world::{Layer, Level, LoadOptions, Play};

#[test]
fn police1_builds_with_people() {
    let Some(game) = uc_formats::default_install() else {
        eprintln!("no Urban Chaos install found (set UC_GAME_DIR); skipping");
        return;
    };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: false, character: true, overrides: None }).unwrap();
    let geo = level.build();
    for layer in [Layer::Floor, Layer::Roofs, Layer::Walls, Layer::Props, Layer::Detail] {
        assert!(geo.count(layer) > 1000, "{layer:?} has only {} triangles", geo.count(layer));
    }
    assert_eq!(geo.chunks.len(), 256);
    let missing: Vec<u32> = geo.pages().into_iter().filter(|&p| level.texture(p).is_none()).collect();
    assert!(missing.len() * 20 < geo.pages().len(), "too many pages without a texture: {missing:?}");

    // Every person type can be posed and drawn with Darci's animations.
    let people = level.people.as_ref().unwrap();
    for t in 0..=person::MIB3 {
        let mut f = people.figure(t, 0, level.spawn.0, level.spawn.1).unwrap();
        assert!(f.play(anim::RUN, Play::Loop));
        f.advance(0.5);
        let mut batches = Default::default();
        f.append_batches(&mut batches, level.lighter().as_ref());
        assert!(!batches.is_empty(), "person type {t} drew nothing");
    }
}

#[test]
fn one_shot_animations_finish() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "ftutor1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut f = level.people.as_ref().unwrap().figure(person::DARCI, 0, level.spawn.0, 0.0).unwrap();
    for a in [anim::PUNCH_COMBO1, anim::KICK_COMBO3, anim::KD_FRONT_MID, anim::PULL_UP, anim::ARREST_CROUTCH] {
        assert!(f.restart(a, Play::Once), "animation {a} missing");
        let mut t = 0.0;
        while !f.finished() && t < 10.0 {
            f.advance(1.0 / 30.0);
            t += 1.0 / 30.0;
        }
        assert!(f.finished(), "animation {a} never finished");
        assert!((f.progress() - 1.0).abs() < 1e-6);
    }
    // The tutorial's rooftop is reachable: three ladders, roofs at 640.
    assert_eq!(level.ground.ladders.len(), 3);
    assert_eq!(level.ground.surface(&level.map, 55.6 * 256.0, 55.0 * 256.0, 640.0), 640.0);
}

/// Capflag1's fences, whose own styles come out as windows and doors, are drawn with the
/// downtown missions' chain link, which its borrowed textures have.
#[test]
fn capflag_fences_are_chain_link() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "Capflag1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    assert!(level.clump.texture(284).unwrap().is_some_and(|t| t.has_alpha), "no see-through chain link");
    // (The game draws the fences: the level hands them over.)
    assert!(level.fences().iter().flat_map(|f| &f.1).any(|q| q.page == 284), "the chain link isn't drawn");
}

/// Walls with a crinkle are raised near the camera and nowhere else, quickly enough to do
/// every frame (#113).
#[test]
fn crinkles_raise_near_walls() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: false, character: false, overrides: None }).unwrap();
    let geo = level.build();
    let relief = geo.relief();
    assert!(!geo.crinkles.is_empty() && geo.crinkled_count() > 1000, "{} crinkled wall triangles", geo.crinkled_count());
    // From high above, nothing is near enough to raise.
    let mut out = std::collections::BTreeMap::new();
    assert_eq!(relief.draw(glam::vec3(64.0, 100.0, 64.0), &mut out), 0);
    // At head height in the middle of every chunk with a crinkle: the worst frame and the
    // average.
    let t = std::time::Instant::now();
    let (mut most, mut tris, mut total) = (0, 0, 0);
    for ch in &relief.chunks {
        let mid = (ch.min + ch.max) / 2.0;
        let mut out = std::collections::BTreeMap::new();
        let n = relief.draw(glam::vec3(mid.x, ch.min.y + 0.7, mid.z), &mut out);
        total += n;
        if n > most {
            (most, tris) = (n, out.values().map(|b| b.indices.len() / 3).sum());
        }
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0 / relief.chunks.len() as f64;
    eprintln!("{} chunks with crinkles: at most {most} quads raised ({tris} triangles), {:.0} on average, {ms:.3} ms a frame",
              relief.chunks.len(), total as f64 / relief.chunks.len() as f64);
    assert!(most > 0);
}

/// Relief is back-face culled (#172): seen from in front of a crinkled wall its faces are
/// drawn, from behind only the sides of a few that face neither way. [`Relief::draw`] draws a
/// wall seen from behind flat, like the walls without a crinkle.
#[test]
fn crinkles_cull_from_behind() {
    use uc_world::crinkle::{raise, ReliefChunk};
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: false, character: false, overrides: None }).unwrap();
    let relief = level.build().relief();
    let mut one = uc_world::Relief { crinkles: relief.crinkles.clone(), chunks: vec![] };
    let (mut total, mut front, mut back) = (0, 0, 0);
    for q in relief.chunks.iter().flat_map(|ch| &ch.quads) {
        let c = &relief.crinkles[q.crinkle as usize];
        let p = q.corners;
        // Relief stands out of a wall on the side its normal points away from. (Some quads
        // have no width.)
        let Some(n) = (p[1] - p[0]).cross(p[2] - p[0]).try_normalize() else { continue };
        let middle = p.iter().copied().sum::<glam::Vec3>() / 4.0;
        let (ahead, behind) = (middle - n * 3.0, middle + n * 3.0);
        let raised = |cam| {
            let mut b = uc_world::Batch::default();
            raise(c, q, 1.0, cam, &mut b);
            b.indices.len() / 3
        };
        total += c.faces.len();
        front += raised(ahead);
        back += raised(behind);

        one.chunks = vec![ReliefChunk { min: p[0].min(p[3]), max: p[0].max(p[3]), quads: vec![*q] }];
        let mut out = std::collections::BTreeMap::new();
        assert_eq!(one.draw(ahead, &mut out), 1);
        let mut out = std::collections::BTreeMap::new();
        assert_eq!(one.draw(behind, &mut out), 0);
        assert_eq!(out[&q.page].indices.len(), 6, "not drawn flat from behind");
    }
    eprintln!("{total} relief faces: {front} drawn from in front, {back} from behind");
    assert!(front * 10 > total * 9 && back * 10 < total, "{front} and {back} of {total}");
}

/// Police 1's world (3) has lit windows (#469): its 'I' pages are on its walls, and the
/// page each draws over itself is in the clump, clear but for the panes, and never drawn
/// on its own. The prims' 'S' pages come with it.
#[test]
fn police1_has_lit_windows() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: false, character: false, overrides: None }).unwrap();
    let geo = level.build();
    assert_eq!(geo.self_lit, level.self_lit);
    let lit = &level.self_lit;
    for page in [137, 139, 183, 185, 187, 189, 220, 250, 252, 254] {
        assert!(lit.two_pass.contains(&page), "page {page} isn't two-pass");
    }
    assert!(lit.unlit.contains(&(uc_world::level::PRIM_PAGES + 74)));
    let pages = geo.pages();
    let drawn: Vec<u32> = lit.two_pass.iter().copied().filter(|p| pages.contains(p)).collect();
    assert!(drawn.len() >= 8, "only {drawn:?} drawn");
    for page in drawn {
        let over = lit.overlay(page).unwrap();
        assert!(!pages.contains(&over), "page {over} is drawn on its own");
        let (t, _) = level.texture(over).unwrap_or_else(|| panic!("no page {over}"));
        let clear = t.rgba.chunks(4).filter(|p| p[3] == 0).count();
        assert!(t.has_alpha && clear * 2 > t.rgba.len() / 4 && clear < t.rgba.len() / 4, "page {over}: {clear} clear");
    }
}

/// RTA's police station is a warehouse: its walls are built twice, back to back, the outside
/// and the inside (`FACET_FLAG_INSIDE`). Each is drawn from its own side only, so they don't
/// fight over the same pixels (the facade flickered black).
#[test]
fn warehouse_walls_are_one_sided() {
    let Some(game) = uc_formats::default_install() else { return };
    let l = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let w = l.build();
    let walls = w.chunks.iter().flat_map(|c| c.batches.iter().filter(|((l, _), _)| *l == Layer::Walls).map(|(_, b)| b).chain(c.crinkled.values()));
    let (one, two) = walls.flat_map(|b| b.vertices.iter()).fold((0, 0), |(o, t), v| if v.facing == [0; 4] { (o, t + 1) } else { (o + 1, t) });
    assert!(one > 1000, "{one} one-sided wall vertices");
    assert!(two > one, "the other buildings' walls stay two-sided");
}

/// Round RTA's door, inside and out, no wall has another face in the same place that's seen
/// from the same side: the station's walls back to back, its rooms' (`BUILDING_TYPE_CRATE_IN`)
/// on them and on each other, its neighbour's, and the props flat on them: the flicker by
/// the door.
#[test]
fn nothing_fights_the_walls_by_rtas_door() {
    let Some(game) = uc_formats::default_install() else { return };
    let l = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let w = l.build();
    let near = |p: Vec3| (72.0..92.0).contains(&p.x) && (82.0..94.0).contains(&p.z);
    let mut tris: Vec<(bool, [Vec3; 3], Vec3)> = Vec::new();
    for c in &w.chunks {
        let batches = c.batches.iter().filter(|((l, _), _)| matches!(l, Layer::Walls | Layer::Props))
            .map(|((l, _), b)| (*l == Layer::Walls, b))
            .chain(c.crinkled.values().map(|b| (true, b)));
        for (wall, b) in batches {
            for t in b.indices.chunks(3) {
                let v = [0, 1, 2].map(|i| &b.vertices[t[i] as usize]);
                let p = v.map(|v| Vec3::from(v.pos));
                if p.iter().all(|&p| near(p)) {
                    let f = v[0].facing;
                    tris.push((wall, p, Vec3::new(f[0] as f32, f[1] as f32, f[2] as f32)));
                }
            }
        }
    }
    let mut fights = Vec::new();
    for (i, a) in tris.iter().enumerate() {
        let n = (a.1[1] - a.1[0]).cross(a.1[2] - a.1[0]).normalize_or_zero();
        if !a.0 || n == Vec3::ZERO {
            continue;
        }
        for (j, b) in tris.iter().enumerate() {
            // Seen from opposite sides only, they never meet.
            if i == j || a.2.dot(b.2) < 0.0 || b.1.iter().any(|p| (*p - a.1[0]).dot(n).abs() > 0.004) {
                continue;
            }
            let u = if n.y.abs() > 0.9 { Vec3::X } else { n.cross(Vec3::Y).normalize() };
            let v = n.cross(u);
            let flat = |t: &[Vec3; 3]| t.map(|p| (p.dot(u), p.dot(v)));
            if overlap(&flat(&a.1), &flat(&b.1)) > 0.002 {
                fights.push((a.1, b.1));
            }
        }
    }
    // A wall's own two triangles share an edge, not an area.
    assert!(fights.is_empty(), "{} pairs, e.g. {:?}", fights.len(), fights.first());
}

/// The area two triangles have in common (Sutherland-Hodgman).
fn overlap(a: &[(f32, f32); 3], b: &[(f32, f32); 3]) -> f32 {
    fn area(p: &[(f32, f32)]) -> f32 {
        (0..p.len()).map(|i| { let (q, r) = (p[i], p[(i + 1) % p.len()]); q.0 * r.1 - r.0 * q.1 }).sum::<f32>() / 2.0
    }
    let mut clip = b.to_vec();
    if area(&clip) < 0.0 {
        clip.reverse();
    }
    let mut out = a.to_vec();
    for i in 0..clip.len() {
        let (s, e) = (clip[i], clip[(i + 1) % clip.len()]);
        let side = |p: (f32, f32)| (e.0 - s.0) * (p.1 - s.1) - (e.1 - s.1) * (p.0 - s.0);
        let input = std::mem::take(&mut out);
        for k in 0..input.len() {
            let (p, q) = (input[k], input[(k + 1) % input.len()]);
            let (dp, dq) = (side(p), side(q));
            let cross = || { let t = dp / (dp - dq); (p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)) };
            match (dp >= 0.0, dq >= 0.0) {
                (true, true) => out.push(q),
                (true, false) => out.push(cross()),
                (false, true) => { out.push(cross()); out.push(q) }
                _ => {}
            }
        }
        if out.is_empty() {
            return 0.0;
        }
    }
    area(&out).abs()
}

/// RTA's offices upstairs, over its lobby, where three cops stand: their floors (the roofs
/// of the rooms below, on page 174, which no clump has) are textured from above, and black
/// from below as the original draws them.
#[test]
fn rtas_offices_have_floors() {
    let Some(game) = uc_formats::default_install() else { return };
    let l = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let w = l.build();
    // Room 61's floor, one storey up, over (75, 84).
    let at = |b: &uc_world::geometry::Batch| b.vertices.iter().filter(|v| v.pos[1] == 1.0 && (75.0..=76.0).contains(&v.pos[0]) && (84.0..=85.0).contains(&v.pos[2])).map(|v| v.facing).collect::<Vec<_>>();
    let roofs = w.chunks.iter().flat_map(|c| c.batches.iter()).filter(|((l, _), _)| *l == Layer::Roofs);
    let (mut up, mut down) = (false, false);
    for ((_, page), b) in roofs {
        for f in at(b) {
            up |= f[1] > 0 && *page != uc_world::geometry::UNTEXTURED;
            down |= f[1] < 0 && *page == uc_world::geometry::UNTEXTURED;
        }
    }
    assert!(up && down, "textured from above {up}, black from below {down}");
}

/// RTA's police station is a warehouse, whose roof textures were never shipped: from above
/// its roof has the page most of the city's roofs have, from inside it's black.
#[test]
fn rtas_station_has_a_roof() {
    let Some(game) = uc_formats::default_install() else { return };
    let l = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let w = l.build();
    // Over (100, 90), five storeys up.
    let over = |v: &uc_world::geometry::Vertex| v.pos[1] == 5.0 && (100.0..=101.0).contains(&v.pos[0]) && (90.0..=91.0).contains(&v.pos[2]);
    let roofs = w.chunks.iter().flat_map(|c| c.batches.iter()).filter(|((l, _), _)| *l == Layer::Roofs);
    let (mut up, mut down) = (false, false);
    for ((_, page), b) in roofs {
        for v in b.vertices.iter().filter(|v| over(v)) {
            up |= v.facing[1] > 0 && *page != uc_world::geometry::UNTEXTURED;
            down |= v.facing[1] < 0 && *page == uc_world::geometry::UNTEXTURED;
        }
    }
    assert!(up && down, "textured from above {up}, black from below {down}");
}

/// The rooftop tiles world 3 (RTA's city) has for each shape of roof, from the released maps
/// (`uc textures roofs`), turned with the shape: a kerb along an edge, shaded asphalt along
/// a raised part, their corners, and the plain middle.
#[test]
fn world_3s_rooftop_tiles() {
    use uc_world::roofs::{tile, Shape, Side::*};
    let all = |_: u32| true;
    let at = |sides, notches| tile(3, Shape::new(sides, notches), all).unwrap();
    assert_eq!(at([Same; 4], [false; 4]), (421, 0));
    assert_eq!(at([Lower, Same, Same, Same], [false; 4]), (438, 0));
    // The same edge on the next side over: the same kerb, turned once.
    assert_eq!(at([Same, Lower, Same, Same], [false; 4]), (438, 1));
    assert_eq!(at([Higher, Same, Same, Same], [false; 4]), (428, 0));
    assert_eq!(at([Lower, Lower, Same, Same], [false; 4]).0, 439);
    assert_eq!(at([Same; 4], [true, false, false, false]).0, 430);
}

/// RTA's station roof is laid out from those tiles: kerbs on its edges, shading by what's
/// raised over it.
#[test]
fn rtas_station_roof_has_kerbs_and_shading() {
    let Some(game) = uc_formats::default_install() else { return };
    let l = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let w = l.build();
    let station = |v: &uc_world::geometry::Vertex| v.pos[1] >= 5.0 && (74.0..=109.0).contains(&v.pos[0]) && (80.0..=100.0).contains(&v.pos[2]) && v.facing[1] > 0;
    let pages: std::collections::BTreeSet<u32> = w.chunks.iter().flat_map(|c| c.batches.iter())
        .filter(|((l, _), b)| *l == Layer::Roofs && b.vertices.iter().any(station))
        .map(|((_, p), _)| *p).collect();
    for want in [421, 428, 438, 439] {
        assert!(pages.contains(&want), "no page {want} on the station's roof: {pages:?}");
    }
}

/// An older map's roofs, laid as one page over every square (army1's, of pages its world
/// has no picture of), are laid out as its world's released rooftops are: plain middles,
/// kerbs on the edges, corners ([`uc_world::roofs`]).
#[test]
fn army1s_roofs_are_laid_as_rooftops() {
    let Some(game) = uc_formats::default_install() else { return };
    let l = Level::load(&game, "army1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let mut pages = std::collections::BTreeMap::<u16, usize>::new();
    for rf in l.map.roof_faces.iter().skip(1) {
        *pages.entry(l.map.square((rf.rx & 127) as usize, (rf.rz & 127) as usize).page()).or_default() += 1;
    }
    for want in [421, 438, 439] {
        assert!(pages.get(&want).is_some_and(|&n| n > 20), "army1's roofs have little of page {want}: {pages:?}");
    }
    let rooftop = pages.iter().filter(|(&p, _)| uc_world::roofs::rooftop_page(3, p as u32)).map(|(_, n)| n).sum::<usize>();
    assert!(rooftop * 10 >= pages.values().sum::<usize>() * 9, "{pages:?}");
}

/// A released map's roofs are left as they shipped.
#[test]
fn a_released_maps_roofs_are_as_shipped() {
    let Some(game) = uc_formats::default_install() else { return };
    let l = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let raw = uc_formats::iam::Map::parse(&std::fs::read(uc_formats::resolve(&game, &l.mission.map).unwrap()).unwrap()).unwrap();
    let changed = (0..raw.squares.len()).filter(|&i| raw.squares[i].texture != l.map.squares[i].texture).count();
    assert_eq!(changed, 0);
}
