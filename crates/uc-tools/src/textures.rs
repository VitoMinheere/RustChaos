//! `uc textures missing`: the world pages the missions without a clump of their own draw
//! with another world's pictures (or with none), by world, to be painted as `world<N>/page_NNNN.png`
//! overrides (`uc_world::level::override_path`). `uc textures floors`: the table of floor
//! pages the maps older than the shipped texture sets take instead (`uc_world::refloor`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use uc_formats::txc::entry_page;
use uc_world::level::{override_path, world_page};
use uc_world::{Level, LoadOptions};

pub fn missing(game: &Path, overrides: Option<&Path>) -> Result<(), String> {
    let names = mission_names(game)?;
    let opt = LoadOptions { fullbright: true, character: false, overrides: None };
    // world -> page -> the missions that draw it.
    let mut worlds: BTreeMap<Option<i32>, BTreeMap<u32, Vec<String>>> = BTreeMap::new();
    let mut failed = Vec::new();
    for name in names {
        if uc_formats::resolve(game, &format!("clumps\\{name}.txc")).is_some() {
            continue;
        }
        let level = match Level::load(game, &name, &opt) {
            Ok(l) => l,
            Err(e) => {
                failed.push(format!("{name}: {e}"));
                continue;
            }
        };
        // The world pages its geometry draws with that are another world's, or that no clump
        // in the install has at all.
        let have: BTreeSet<u32> = level.clump.ids().map(entry_page).collect();
        let pages = worlds.entry(level.map.texture_set).or_default();
        let mut used = level.build().pages();
        // The doors' and fences' too, which the game draws.
        used.extend(level.doors().into_iter().flatten().map(|q| q.page));
        used.extend(level.fences().into_iter().flat_map(|(_, q)| q).map(|q| q.page));
        for p in used.into_iter().filter(|&p| world_page(p)) {
            if level.stand_in_pages.contains(&p) || !have.contains(&p) {
                pages.entry(p).or_default().push(name.clone());
            }
        }
    }
    for (world, pages) in &worlds {
        if pages.is_empty() {
            continue;
        }
        let w = world.map_or("?".to_string(), |w| w.to_string());
        let done: BTreeSet<u32> = pages.keys().copied()
            .filter(|&p| overrides.is_some_and(|d| override_path(d, "", *world, p).is_some()))
            .collect();
        let missions: BTreeSet<&str> = pages.values().flatten().map(String::as_str).collect();
        println!("world {w}: {} pages with another world's picture or none, {} overridden", pages.len(), done.len());
        println!("  missions: {}", missions.into_iter().collect::<Vec<_>>().join(", "));
        if world.is_some_and(|w| uc_formats::resolve(game, &format!("server\\textures\\world{w}\\style.tma")).is_none()) {
            println!("  no style table: its walls follow world 3's, so overrides change the pictures, not which wall gets which");
        }
        for (p, users) in pages {
            let mark = if done.contains(p) { "x" } else { " " };
            println!("  [{mark}] page_{p:04}.png  ({})", users.join(", "));
        }
    }
    if !failed.is_empty() {
        println!("not loaded (a map the install lacks, or a mission file that doesn't parse):");
        for f in failed {
            println!("  {f}");
        }
    }
    Ok(())
}

/// Where a world page is on a map's visible floor: how many squares, of each sort (roof,
/// road, pavement, water), how many of their sides it shares with itself, and what's on the
/// other side of the rest.
#[derive(Default)]
struct FloorUse {
    sort: [f64; 4],
    n: f64,
    same: f64,
    sides: f64,
    next: BTreeMap<u32, f64>,
}

impl FloorUse {
    /// Its sorts as fractions.
    fn mix(&self) -> [f64; 4] {
        self.sort.map(|v| v / self.n.max(1.0))
    }

    /// How much of it is one field of itself rather than a border or a spot.
    fn field(&self) -> f64 {
        self.same / self.sides.max(1.0)
    }

    fn add(&mut self, o: FloorUse) {
        for (a, b) in self.sort.iter_mut().zip(o.sort) {
            *a += b;
        }
        self.n += o.n;
        self.same += o.same;
        self.sides += o.sides;
        for (k, v) in o.next {
            *self.next.entry(k).or_default() += v;
        }
    }
}

/// The world pages on `map`'s squares that can be seen: rooftops (a fake roof, or a roof
/// face over the square, which takes its page), and floor with nothing walkable over it.
/// Page 0, every world's dark edge of the city, is left out, and so is the strip round the
/// map's edge, out of play, where the released maps put down whatever (Grim Gardens a row
/// of doorways). `level` is any mission on the map, for what's over the squares.
fn floor_use(level: &Level, map: &uc_formats::iam::Map) -> BTreeMap<u32, FloorUse> {
    use uc_formats::iam::flags;
    let mut out: BTreeMap<u32, FloorUse> = BTreeMap::new();
    let mut roofed = vec![false; 128 * 128];
    for rf in map.roof_faces.iter().skip(1) {
        roofed[(rf.rx as usize & 127) * 128 + (rf.rz as usize & 127)] = true;
    }
    const EDGE: usize = 12;
    for x in EDGE..128 - EDGE {
        for z in EDGE..128 - EDGE {
            let s = map.square(x, z);
            let p = s.page() as u32;
            if !world_page(p) || p == 0 {
                continue;
            }
            let sort = if s.has(flags::ROOF_EXISTS) || roofed[x * 128 + z] {
                0
            } else if s.has(flags::HIDDEN) {
                continue;
            } else {
                let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
                let floor = level.ground.surface(map, cx, cz, s.alt as f32 * 64.0 + 64.0);
                if level.ground.surface(map, cx, cz, 30_000.0) > floor + 128.0 {
                    continue;
                }
                if s.has(flags::WATER) { 3 } else if s.has(flags::SINK_SQUARE) { 1 } else { 2 }
            };
            let u = out.entry(p).or_default();
            u.sort[sort] += 1.0;
            u.n += 1.0;
            for (dx, dz) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                if !(0..128).contains(&nx) || !(0..128).contains(&nz) {
                    continue;
                }
                let q = map.square(nx as usize, nz as usize).page() as u32;
                u.sides += 1.0;
                if q == p {
                    u.same += 1.0;
                } else {
                    *u.next.entry(q).or_default() += 1.0;
                }
            }
        }
    }
    out
}

/// Each map's texture set, whether a mission with a clump of its own is on it (a released
/// map, whose pages are the set's as it shipped), and a level of it, by map file in lower case.
type Maps = BTreeMap<String, (i32, bool, Level)>;

/// How a map (by file, and a level of it) uses its pages.
type Usage<'a> = dyn Fn(&str, &Level) -> Result<BTreeMap<u32, FloorUse>, String> + 'a;

fn load_maps(game: &Path) -> Result<Maps, String> {
    let opt = LoadOptions { fullbright: true, character: false, overrides: None };
    let mut maps = Maps::new();
    for name in &mission_names(game)? {
        let Ok(level) = Level::load(game, name, &opt) else { continue };
        let Some(world) = level.map.texture_set else { continue };
        let clumped = uc_formats::resolve(game, &format!("clumps\\{name}.txc")).is_some();
        let key = level.mission.map.to_ascii_lowercase();
        match maps.get(&key) {
            Some(&(_, true, _)) => {}
            Some(_) if !clumped => {}
            _ => {
                maps.insert(key, (world, clumped, level));
            }
        }
    }
    Ok(maps)
}

/// For each map no mission with a clump of its own is on: its pages that `usage` finds
/// and no released map of its world (or of the stand-in world, for a world none is of) uses
/// the same way, each with the released page most like it, as the Rust table `name`. Most
/// like it: on the same sort of place (`FloorUse::sort`), next to the same pages, as much
/// one field of itself, and widely used; and the same page as seldom as may be twice on a map.
///
/// A page the released maps use `keep` times or more that way is left as it is; one put in
/// is used `min` times or more.
fn old_map_table(maps: &Maps, usage: &Usage,
                 keep: f64, min: f64, name: &str, doc: &str) -> Result<String, String> {
    // Each world's pages its clumps have, and how the released maps use them.
    let mut have: BTreeMap<i32, BTreeSet<u32>> = BTreeMap::new();
    for (world, clumped, level) in maps.values() {
        if *clumped {
            have.entry(*world).or_default().extend(level.clump.ids().map(entry_page));
        }
    }
    let mut released: BTreeMap<i32, BTreeMap<u32, FloorUse>> = BTreeMap::new();
    for (file, (world, clumped, level)) in maps {
        if !clumped {
            continue;
        }
        let into = released.entry(*world).or_default();
        for (p, u) in usage(file, level)?.into_iter().filter(|(p, _)| have[world].contains(p)) {
            into.entry(p).or_default().add(u);
        }
    }
    let mut out = format!("{doc}\npub const {name}: &[(&str, &[(u16, u16)])] = &[\n");
    for (file, (world, clumped, level)) in maps {
        if *clumped {
            continue;
        }
        let from = if released.contains_key(world) { *world } else { uc_world::level::STAND_IN_WORLD };
        let cand: Vec<(&u32, &FloorUse)> = released[&from].iter().filter(|(&q, u)| q != 0 && u.n >= min).collect();
        let fine = |p: u32| released[&from].get(&p).is_some_and(|u| p != 0 && u.n >= keep);
        let mut off: Vec<(u32, FloorUse)> = usage(file, level)?.into_iter().filter(|(p, _)| !fine(*p)).collect();
        off.sort_by(|a, b| b.1.n.total_cmp(&a.1.n).then(a.0.cmp(&b.0)));
        if off.is_empty() {
            continue;
        }
        let mut taken: BTreeMap<u32, f64> = BTreeMap::new();
        let mut pairs = Vec::new();
        for (p, u) in off {
            let mix = u.mix();
            let known: f64 = u.next.iter().filter(|(k, _)| fine(**k)).map(|(_, v)| v).sum::<f64>().max(1.0);
            let score = |q: u32, c: &FloorUse| {
                let sort: f64 = mix.iter().zip(c.mix()).map(|(a, b)| a.min(b)).sum();
                let total: f64 = c.next.values().sum::<f64>().max(1.0);
                let next: f64 = u.next.iter().filter(|(k, _)| fine(**k))
                    .filter_map(|(k, v)| c.next.get(k).map(|w| (v / known).min(w / total))).sum();
                sort + 1.5 * next - (u.field() - c.field()).abs() + c.n.ln() * 0.03 - taken.get(&q).unwrap_or(&0.0) * 0.1
            };
            let best = cand.iter().map(|(&q, c)| (score(q, c), q)).max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
            if let Some((_, q)) = best {
                *taken.entry(q).or_default() += 1.0;
                pairs.push(format!("({p}, {q})"));
            }
        }
        out += &format!("    ({file:?}, &[{}]),\n", pairs.join(", "));
    }
    out += "];\n";
    Ok(out)
}

/// `uc textures floors`: for each map no mission with a clump of its own is on, the world
/// pages on its visible floor and roofs that no released map has there, each with the
/// released page to draw instead ([`old_map_table`]), as `uc_world::refloor`'s table. Those
/// maps are older than the texture sets that shipped: army1's paves its yards with what is
/// now a door.
pub fn floors(game: &Path) -> Result<String, String> {
    let maps = load_maps(game)?;
    // The map as it is in the install, not as loaded (which has the table's pages already).
    let raw = |file: &str| -> Result<uc_formats::iam::Map, String> {
        let p = uc_formats::resolve(game, file).ok_or(format!("{file} not found"))?;
        uc_formats::iam::Map::parse(&std::fs::read(p).map_err(|e| e.to_string())?).map_err(|e| format!("{file}: {e}"))
    };
    old_map_table(&maps, &|file, level| Ok(floor_use(level, &raw(file)?)), 4.0, 4.0, "REFLOOR",
        "//! Generated by `uc textures floors` (crates/uc-tools/src/textures.rs): don't edit.\n\
         //! For each map no mission with a clump of its own is on, its world pages that no released\n\
         //! map has on its visible floor or roofs, and the released maps' page put in their place.\n")
}

/// The world pages on `level`'s walls and fences, as they're drawn (`facet::wall_quads`, with
/// none of [`uc_world::rewall`]'s swaps): of which sort each piece is (a fence, a wall's
/// bottom row, a row above), what else is on the same facet, and how much of its facet it is.
fn wall_use(level: &Level) -> BTreeMap<u32, FloorUse> {
    use uc_formats::{facet, iam::facet_type};
    let mut out: BTreeMap<u32, FloorUse> = BTreeMap::new();
    let Some(styles) = &level.styles else { return out };
    for i in 1..level.map.facets.len() {
        let f = &level.map.facets[i];
        let fence = matches!(f.kind, facet_type::FENCE | facet_type::FENCE_BRICK | facet_type::FENCE_FLAT);
        let quads = facet::wall_quads(&level.map, styles, i);
        let base = quads.iter().map(|q| q.corners.iter().map(|c| c[1]).fold(f32::MAX, f32::min)).fold(f32::MAX, f32::min);
        let mut on: BTreeMap<u32, f64> = BTreeMap::new();
        for q in &quads {
            *on.entry(q.page).or_default() += 1.0;
        }
        for q in &quads {
            let p = q.page;
            if !world_page(p) || p == 0 {
                continue;
            }
            let bottom = q.corners.iter().map(|c| c[1]).fold(f32::MAX, f32::min) - base < 1.0;
            let u = out.entry(p).or_default();
            u.sort[if fence { 0 } else if bottom { 1 } else { 2 }] += 1.0;
            u.n += 1.0;
            u.sides += quads.len() as f64 - 1.0;
            u.same += on[&p] - 1.0;
            for (&k, &v) in on.iter().filter(|(&k, _)| k != p) {
                *u.next.entry(k).or_default() += v;
            }
        }
    }
    out
}

/// `uc textures walls`: the same for the old maps' walls and fences: their world pages no
/// released map has on a wall, each with the released wall page to draw instead, as
/// `uc_world::rewall`'s table. Rooftop1's map's walls are pages world 12 hasn't got.
pub fn walls(game: &Path) -> Result<String, String> {
    let maps = load_maps(game)?;
    old_map_table(&maps, &|_, level| Ok(wall_use(level)), 1.0, 8.0, "REWALL",
        "//! Generated by `uc textures walls` (crates/uc-tools/src/textures.rs): don't edit.\n\
         //! For each map no mission with a clump of its own is on, its world pages that no released\n\
         //! map has on a wall or fence, and the released maps' wall page put in their place.\n")
}

/// `uc textures roofs`: for each world, the page and turn the released maps' rooftops
/// (`ROOF_EXISTS` squares with a page their clump has) have most for each shape of the roof
/// round a square ([`uc_world::roofs::Shape`], turned upright), as `uc_world::rooftiles`'s
/// table. A shape seen fewer than [`MIN_ROOFS`] times is left out. With `report`, how many of
/// each shape had the page chosen goes to stderr.
pub fn roofs(game: &Path, report: bool) -> Result<String, String> {
    use uc_world::roofs::Tops;
    let maps = load_maps(game)?;
    // World, shape upright, (page, turn): how many squares.
    let mut seen: BTreeMap<i32, BTreeMap<u16, Uses>> = BTreeMap::new();
    for (world, clumped, level) in maps.values() {
        if !*clumped {
            continue;
        }
        let have: BTreeSet<u32> = level.clump.ids().map(entry_page).collect();
        let map = &level.map;
        let tops = Tops::new(map);
        for x in 0..uc_formats::iam::MAP_SIZE {
            for z in 0..uc_formats::iam::MAP_SIZE {
                let sq = map.square(x, z);
                if !sq.has(uc_formats::iam::flags::ROOF_EXISTS) || !have.contains(&(sq.page() as u32)) {
                    continue;
                }
                let (shape, k) = tops.shape(x as i32, z as i32, sq.height as i32 * 64).upright();
                let turn = (sq.rotation() + k) % 4;
                *seen.entry(*world).or_default().entry(shape.0).or_default().entry((sq.page(), turn)).or_default() += 1;
            }
        }
    }
    let mut out = String::from(
        "//! Generated by `uc textures roofs` (crates/uc-tools/src/textures.rs): don't edit.\n\
         //! For each world, the page and turn its released maps' rooftops have most for each\n\
         //! shape of roof round a square (`uc_world::roofs::Shape`, turned upright).\n\n\
         pub const ROOF_TILES: &[(i32, &[crate::roofs::RoofTile])] = &[\n");
    for (world, shapes) in &seen {
        let mut tiles = Vec::new();
        for (shape, uses) in shapes {
            let total: usize = uses.values().sum();
            // The most, and of those the least page and turn, so it's the same every time.
            let (&(page, turn), &n) = uses.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).unwrap();
            if total < MIN_ROOFS {
                continue;
            }
            if report {
                eprintln!("world {world} shape {shape:#06x}: page {page} turn {turn} on {n} of {total}");
            }
            tiles.push(format!("({shape:#06x}, {page}, {turn})"));
        }
        out += &format!("    ({world}, &[{}]),\n", tiles.join(", "));
    }
    out += "];\n";
    Ok(out)
}

/// How many squares of a shape have each (page, turn).
type Uses = BTreeMap<(u16, u8), usize>;

/// The fewest squares of a shape [`roofs`] keeps a tile for.
const MIN_ROOFS: usize = 3;

/// The missions in the install's `levels`, by file name without `.ucm`, sorted.
fn mission_names(game: &Path) -> Result<Vec<String>, String> {
    let dir = game.join("levels");
    let mut names: Vec<String> = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .filter_map(|e| {
            let f = e.file_name().to_string_lossy().into_owned();
            Some(f.strip_suffix(".ucm").or_else(|| f.strip_suffix(".UCM"))?.to_string())
        })
        .collect();
    names.sort_by_key(|n| n.to_ascii_lowercase());
    Ok(names)
}

#[cfg(test)]
mod tests {
    /// The table the loader has is the one the install makes: rerun `uc textures floors -o
    /// crates/uc-world/src/refloor.rs` after changing how it's made.
    #[test]
    fn the_refloor_table_is_current() {
        let Some(game) = uc_formats::default_install() else { return };
        let made = super::floors(&game).unwrap();
        assert!(made == include_str!("../../uc-world/src/refloor.rs"), "refloor.rs is out of date:\n{made}");
    }

    /// The same for the rooftops: `uc textures roofs -o crates/uc-world/src/rooftiles.rs`.
    #[test]
    fn the_rooftiles_table_is_current() {
        let Some(game) = uc_formats::default_install() else { return };
        let made = super::roofs(&game, false).unwrap();
        assert!(made == include_str!("../../uc-world/src/rooftiles.rs"), "rooftiles.rs is out of date:\n{made}");
    }

    /// The same for the walls: `uc textures walls -o crates/uc-world/src/rewall.rs`.
    #[test]
    fn the_rewall_table_is_current() {
        let Some(game) = uc_formats::default_install() else { return };
        let made = super::walls(&game).unwrap();
        assert!(made == include_str!("../../uc-world/src/rewall.rs"), "rewall.rs is out of date:\n{made}");
    }
}
