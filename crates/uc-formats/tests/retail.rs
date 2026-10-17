//! Checks against a retail install. Set `UC_GAME_DIR` (or install the GOG version in its
//! default location); without an install these tests print a note and pass.
//!
//! Expected results come from tools/uc_formats/verify_formats.py (docs/ARCHITECTURE.md §5).

use std::fs;
use std::path::{Path, PathBuf};
use uc_formats::{facet, iam::Map, prm::Prim, resolve, tma::StyleTable, txc::Clump, ucm::Mission};

fn game() -> Option<PathBuf> {
    let g = uc_formats::default_install();
    if g.is_none() {
        eprintln!("no Urban Chaos install found (set UC_GAME_DIR); skipping");
    }
    g
}

fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case(ext)).unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Mission file names listed in the shipped English mission tree.
fn shipped_missions(game: &Path) -> Vec<String> {
    let text = fs::read(game.join("data").join("urban.sty")).unwrap();
    let text: String = text.iter().map(|&c| c as char).collect();
    text.lines()
        .filter(|l| !l.starts_with("//") && !l.starts_with('['))
        .filter_map(|l| l.split(" : ").nth(7).map(|s| s.trim().to_string()))
        .filter(|s| s.to_ascii_lowercase().ends_with(".ucm"))
        .collect()
}

#[test]
fn every_map_parses_to_eof() {
    let Some(g) = game() else { return };
    let mut paths = files(&g.join("data"), "iam");
    paths.extend(files(&g.join("levels"), "iam"));
    assert_eq!(paths.len(), 72, "the GOG release has 72 maps");
    let mut old = 0;
    for p in &paths {
        let m = Map::parse(&fs::read(p).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        if m.walkables_layout == uc_formats::iam::WalkablesLayout::Old {
            old += 1;
        }
    }
    assert_eq!(old, 3, "jumper1, submiss1a and supply1 use the old walkables layout");
}

#[test]
fn every_shipped_mission_parses_to_eof() {
    let Some(g) = game() else { return };
    let names = shipped_missions(&g);
    assert_eq!(names.len(), 33);
    for name in names {
        let path = resolve(&g, &format!("levels\\{name}")).expect(&name);
        let m = Mission::parse(&fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(m.player_start().is_some(), "{name} has no player spawn");
        assert!(resolve(&g, &m.map).is_some(), "{name}: map {} missing", m.map);
    }
}

#[test]
fn every_prim_parses() {
    let Some(g) = game() else { return };
    let paths = files(&g.join("server").join("prims"), "prm");
    assert_eq!(paths.len(), 433);
    for p in paths {
        Prim::parse(&fs::read(&p).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    }
}

#[test]
fn every_style_table_parses() {
    let Some(g) = game() else { return };
    let dir = g.join("server").join("textures");
    let mut n = 0;
    for world in fs::read_dir(&dir).unwrap().flatten() {
        if let Some(p) = resolve(&world.path(), "style.tma") {
            StyleTable::parse(&fs::read(&p).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            n += 1;
        }
    }
    assert!(n >= 17, "expected a style.tma per texture world, found {n}");
}

#[test]
fn police1_clump_decodes_completely() {
    let Some(g) = game() else { return };
    let clump = Clump::parse(fs::read(resolve(&g, "clumps\\police1.txc").unwrap()).unwrap()).unwrap();
    let (mut images, mut entries) = (0, 0);
    for id in clump.ids().collect::<Vec<_>>() {
        entries += 1;
        if let Some(t) = clump.texture(id).unwrap() {
            assert_eq!(t.rgba.len(), (t.width * t.height * 4) as usize);
            images += 1;
        }
    }
    assert_eq!(entries, 940);
    assert!(images > 800, "only {images} images");
}

/// Two crinkles whose sizes were checked by hand, and every crinkle in the install parses.
#[test]
fn crinkles_read_from_the_clumps() {
    let Some(g) = game() else { return };
    let clump = |name: &str| Clump::parse(fs::read(resolve(&g, &format!("clumps\\{name}.txc")).unwrap()).unwrap()).unwrap();
    let size = |c: &uc_formats::txc::Crinkle| (c.points.len(), c.faces.len());
    assert_eq!(clump("baalrog3").crinkle(1).unwrap().as_ref().map(size), Some((20, 22)));
    assert_eq!(clump("ftutor1").crinkle(12).unwrap().as_ref().map(size), Some((12, 18)));
    let mut n = 0;
    for f in files(&g.join("clumps"), "txc") {
        let c = Clump::parse(fs::read(&f).unwrap()).unwrap();
        for page in 0..uc_formats::txc::CRINKLE_PAGES {
            if let Some(k) = c.crinkle(page).unwrap_or_else(|e| panic!("{} page {page}: {e}", f.display())) {
                assert!(k.points.iter().all(|p| (-0.01..=1.01).contains(&p.u) && (-0.01..=1.01).contains(&p.v)));
                n += 1;
            }
        }
    }
    eprintln!("{n} crinkles");
    assert!(n > 2000, "only {n} crinkles");
}

#[test]
fn police_station_walls_find_their_textures() {
    let Some(g) = game() else { return };
    let map = Map::parse(&fs::read(resolve(&g, "data\\disturb1.iam").unwrap()).unwrap()).unwrap();
    let world = map.texture_set.unwrap();
    let styles = StyleTable::parse(
        &fs::read(resolve(&g, &format!("server\\textures\\world{world}\\style.tma")).unwrap()).unwrap(),
    )
    .unwrap();
    let clump = Clump::parse(fs::read(resolve(&g, "clumps\\police1.txc").unwrap()).unwrap()).unwrap();
    let ids: std::collections::HashSet<u32> = clump.ids().collect();
    let quads: Vec<_> = (1..map.facets.len()).flat_map(|i| facet::wall_quads(&map, &styles, i)).collect();
    let found = quads.iter().filter(|q| ids.contains(&q.page)).count();
    eprintln!("{} wall quads, {} with a texture in police1.txc", quads.len(), found);
    assert!(quads.len() > 1000);
    assert!(found * 100 / quads.len() >= 90, "too many wall pages missing from the clump");
}

#[test]
fn every_lighting_file_parses() {
    let Some(g) = game() else { return };
    let paths = files(&resolve(&g, r"data\Lighting").unwrap(), "lgt");
    assert_eq!(paths.len(), 61);
    for p in paths {
        uc_formats::lgt::Lighting::parse(&fs::read(&p).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    }
}

#[test]
fn darci_animations_load() {
    let Some(g) = game() else { return };
    let chunk = uc_formats::all::AnimChunk::parse(&fs::read(resolve(&g, r"data\darci1.all").unwrap()).unwrap()).unwrap();
    assert_eq!(chunk.element_count, 15);
    assert_eq!(chunk.bodies.len(), 4);
    for i in 0..15 {
        assert!(chunk.part_mesh(0, 0, i).is_some(), "Darci part {i} has no mesh");
    }
    // ANIM_STAND_READY, ANIM_WALK, ANIM_RUN (fallen/Headers/animate.h)
    for (name, id) in [("stand", 3usize), ("walk", 20), ("run", 119)] {
        let start = chunk.anims[id].unwrap_or_else(|| panic!("{name} missing"));
        let (mut f, mut n) = (start, 0);
        loop {
            assert!(chunk.pose(f).is_some(), "{name}: frame {f} has no pose");
            n += 1;
            let kf = chunk.keyframes[f];
            match kf.next {
                Some(next) if n < 200 && kf.flags & uc_formats::all::FLAG_LAST_FRAME == 0 => f = next,
                _ => break,
            }
        }
        eprintln!("{name}: {n} frames, loops back to start: {:?}", chunk.keyframes[f].next == Some(start));
    }
}

#[test]
fn every_parsable_mission_round_trips_byte_for_byte() {
    let Some(g) = game() else { return };
    let (mut ok, mut skipped) = (0, 0);
    for p in files(&g.join("levels"), "ucm") {
        let bytes = fs::read(&p).unwrap();
        let Ok(m) = Mission::parse(&bytes) else { skipped += 1; continue };
        let out = m.write().unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        if out != bytes {
            let at = out.iter().zip(&bytes).position(|(a, b)| a != b).unwrap_or(out.len().min(bytes.len()));
            panic!("{}: differs at byte {at} (sizes {} vs {})", p.display(), out.len(), bytes.len());
        }
        ok += 1;
    }
    eprintln!("{ok} missions round-trip exactly ({skipped} dev files don't parse)");
    assert_eq!(ok, 80);
}

#[cfg(feature = "json")]
#[test]
fn missions_round_trip_through_json() {
    use uc_formats::ucm_json::{from_json, to_json, MissionJson};
    let Some(g) = game() else { return };
    let mut n = 0;
    for p in files(&g.join("levels"), "ucm") {
        let bytes = fs::read(&p).unwrap();
        let Ok(m) = Mission::parse(&bytes) else { continue };
        // Lossless: JSON text -> mission -> bytes must equal the original file.
        let text = serde_json::to_string(&to_json(&m, true)).unwrap();
        let back: MissionJson = serde_json::from_str(&text).unwrap();
        let out = from_json(&back).unwrap_or_else(|e| panic!("{}: {e}", p.display())).write().unwrap();
        assert!(out == bytes, "{}: lossless JSON round trip changed the file", p.display());
        // Clean: same game-relevant content after a trip through the file format.
        let clean: MissionJson = serde_json::from_str(&serde_json::to_string(&to_json(&m, false)).unwrap()).unwrap();
        let reparsed = Mission::parse(&from_json(&clean).unwrap().write().unwrap()).unwrap();
        assert_eq!(to_json(&reparsed, false), to_json(&m, false), "{}: clean round trip changed content", p.display());
        assert!(reparsed.unused_slots.is_empty());
        n += 1;
    }
    assert_eq!(n, 80);
}
