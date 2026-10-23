//! Builds real levels from an install (skipped when none is found; see uc-formats tests).

use uc_world::character::{anim, person};
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
    let geo = level.build();
    assert!(geo.pages().contains(&284), "the chain link isn't drawn");}

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
