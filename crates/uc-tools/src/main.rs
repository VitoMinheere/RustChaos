//! `uc`: command-line tools for Urban Chaos data.
//!
//!   uc mission export <file.ucm> [-o out.json] [--lossless]
//!   uc mission import <file.json> -o <out.ucm>
//!   uc mission check <file.ucm|file.json>
//!   uc textures missing [--game DIR] [--overrides DIR]

mod textures;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use uc_formats::ucm::Mission;
use uc_formats::ucm_json::{from_json, to_json, MissionJson};

const USAGE: &str = "\
uc mission export <file.ucm> [-o out.json] [--lossless]
    Convert a mission to JSON (stdout unless -o). --lossless also keeps the bytes the
    game ignores, so importing reproduces the original file exactly.
uc mission import <file.json> -o <out.ucm>
    Convert JSON back into a mission the original game (and the viewer) can load.
uc mission check <file.ucm|file.json>
    Parse and validate a mission, and print a short summary.
uc textures missing [--game DIR] [--overrides DIR]
    For the missions with no clump of their own: the world pages drawn with another
    world's pictures, by world, as the world<N>/page_NNNN.png overrides that would fix
    them ([x] where --overrides already has one). The install is --game, else
    $UC_GAME_DIR, else the GOG location.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let mut pos: Vec<&str> = Vec::new();
    let (mut out, mut lossless) = (None, false);
    let (mut game, mut overrides) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--output" => out = Some(PathBuf::from(it.next().ok_or("-o needs a path")?)),
            "--lossless" => lossless = true,
            "--game" => game = Some(PathBuf::from(it.next().ok_or("--game needs a folder")?)),
            "--overrides" => overrides = Some(PathBuf::from(it.next().ok_or("--overrides needs a folder")?)),
            "-h" | "--help" => return Err(USAGE.into()),
            s => pos.push(s),
        }
    }
    match pos.as_slice() {
        ["mission", "export", input] => {
            let m = load_mission(Path::new(input))?;
            let text = serde_json::to_string_pretty(&to_json(&m, lossless)).map_err(|e| e.to_string())?;
            match out {
                Some(p) => write(&p, text.as_bytes()),
                None => {
                    println!("{text}");
                    Ok(())
                }
            }
        }
        ["mission", "import", input] => {
            let out = out.ok_or("import needs -o <out.ucm>")?;
            let m = load_mission(Path::new(input))?;
            write(&out, &m.write()?)
        }
        ["mission", "check", input] => {
            let m = load_mission(Path::new(input))?;
            m.validate()?;
            let kinds = m.event_points.iter().fold(std::collections::BTreeMap::new(), |mut acc, e| {
                let name = uc_formats::ucm::names::WAYPOINT.get(e.kind as usize).copied().unwrap_or("?");
                *acc.entry(name).or_insert(0) += 1;
                acc
            });
            println!("{input}: version {}, map {}, {} event points, {} messages, {} cutscenes",
                m.version, m.map, m.event_points.len(), m.messages().count(), m.cutscenes().count());
            for (k, n) in kinds {
                println!("  {n:4} {k}");
            }
            Ok(())
        }
        ["textures", "missing"] => {
            let game = game.or_else(uc_formats::default_install).ok_or("no install found: pass --game DIR")?;
            textures::missing(&game, overrides.as_deref())
        }
        _ => Err(USAGE.into()),
    }
}

/// Reads a `.ucm` or a JSON mission (decided by the `.json` extension).
fn load_mission(path: &Path) -> Result<Mission, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
        let j: MissionJson = serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        from_json(&j).map_err(|e| format!("{}: {e}", path.display()))
    } else {
        Mission::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    eprintln!("wrote {} ({} bytes)", path.display(), bytes.len());
    Ok(())
}
