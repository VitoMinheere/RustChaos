//! Readers for Urban Chaos (1999) data files, ported from the Python reference tools in
//! `tools/uc_formats/` and checked against the GOG release. See docs/ARCHITECTURE.md §5.
//!
//! No game data ships with this crate: point it at your own install.

pub mod all;
pub mod facet;
pub mod iam;
pub mod lgt;
pub mod prm;
pub mod reader;
pub mod tma;
pub mod txc;
pub mod ucm;
#[cfg(feature = "json")]
pub mod ucm_json;

pub use reader::{ParseError, Result};

use std::path::{Path, PathBuf};

/// Finds a file inside an install directory, ignoring case, given a game-style relative path
/// such as `Data\disturb1.iam`. The game was written for case-insensitive Windows paths.
pub fn resolve(root: &Path, game_path: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for part in game_path.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let direct = cur.join(part);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let lower = part.to_ascii_lowercase();
        let found = std::fs::read_dir(&cur).ok()?.flatten().find(|e| {
            e.file_name().to_string_lossy().to_ascii_lowercase() == lower
        })?;
        cur = found.path();
    }
    Some(cur)
}

/// The install to use: `UC_GAME_DIR` if set (and present), else the default GOG location if present.
pub fn default_install() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("UC_GAME_DIR") {
        let dir = PathBuf::from(dir);
        return dir.exists().then_some(dir);
    }
    let gog = PathBuf::from(r"C:\Program Files (x86)\GOG.com\Urban Chaos");
    gog.exists().then_some(gog)
}
