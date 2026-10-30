//! `uc textures missing`: the world pages the missions without a clump of their own draw
//! with another world's pictures (or with none), by world, to be painted as `world<N>/page_NNNN.png`
//! overrides (`uc_world::level::override_path`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use uc_formats::txc::entry_page;
use uc_world::level::{override_path, world_page};
use uc_world::{Level, LoadOptions};

pub fn missing(game: &Path, overrides: Option<&Path>) -> Result<(), String> {
    let dir = game.join("levels");
    let mut names: Vec<String> = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .filter_map(|e| {
            let f = e.file_name().to_string_lossy().into_owned();
            Some(f.strip_suffix(".ucm").or_else(|| f.strip_suffix(".UCM"))?.to_string())
        })
        .collect();
    names.sort_by_key(|n| n.to_ascii_lowercase());
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
        // The doors' too, which the game draws.
        used.extend(level.doors().into_iter().flatten().map(|q| q.page));
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
