// What each of the game's missions still needs: the event points the script runner can't run
// yet, mission by mission and totalled by type. Reads the mission list from `data\urban.sty`
// the way the menu does, so it's in the order they're played.
//
//   cargo run --release -p uc-game --example unsupported

use std::collections::BTreeMap;

/// The missions in the order the game lists them: (file name, title).
fn missions(game: &std::path::Path) -> Vec<(String, String)> {
    let text = uc_formats::resolve(game, "data\\urban.sty")
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| b.iter().map(|&c| c as char).collect::<String>())
        .unwrap_or_default();
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
        let f: Vec<&str> = line.split(" : ").map(str::trim).collect();
        if f.len() < 9 || !f[7].to_ascii_lowercase().ends_with(".ucm") {
            continue;
        }
        let name = f[7][..f[7].len() - 4].to_string();
        if out.iter().any(|e| e.0.eq_ignore_ascii_case(&name)) {
            continue;
        }
        out.push((name, f[8].to_string()));
    }
    out
}

fn main() {
    let Some(game) = uc_formats::default_install() else {
        eprintln!("no Urban Chaos install found");
        return;
    };
    let mut blocks: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (mut clear, mut total) = (0, 0);
    for (name, title) in missions(&game) {
        let Some(path) = uc_formats::resolve(&game, &format!("levels\\{name}.ucm")) else { continue };
        let Ok(mission) = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| uc_formats::ucm::Mission::parse(&b).map_err(|e| e.to_string()))
        else {
            println!("{title:28} {name:14} -- won't parse");
            continue;
        };
        total += 1;
        // Only the type matters here, not which point it was.
        let mut kinds: Vec<String> = uc_game::Script::new(&mission)
            .unsupported
            .iter()
            .filter_map(|u| u.split_once(' ').map(|(_, k)| k.to_string()))
            .collect();
        kinds.sort();
        let mut counts: BTreeMap<&String, usize> = BTreeMap::new();
        for k in &kinds {
            *counts.entry(k).or_default() += 1;
        }
        for k in counts.keys() {
            blocks.entry((*k).clone()).or_default().push(title.clone());
        }
        if counts.is_empty() {
            clear += 1;
            println!("{title:28} {name:14} -- every point runs");
        } else {
            let list: Vec<String> = counts.iter().map(|(k, n)| format!("{k} x{n}")).collect();
            println!("{title:28} {name:14} {}", list.join(", "));
        }
    }
    println!("\n{clear} of {total} missions have no unsupported points left.\n");
    println!("What's left, and how many missions each would unblock:");
    let mut by_reach: Vec<(&String, &Vec<String>)> = blocks.iter().collect();
    by_reach.sort_by_key(|(_, m)| std::cmp::Reverse(m.len()));
    for (kind, used_by) in by_reach {
        println!("  {:<16} {:2} missions: {}", kind, used_by.len(), used_by.join(", "));
    }
}
