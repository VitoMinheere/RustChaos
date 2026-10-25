//! What footsteps sound like (#299, `footstep_wave`, Person.cpp:1195).
//!
//! A step in a warehouse is soft, one in a puddle wet, and anywhere else it's the floor
//! texture's own sound: `soundfx.ini` in the world's texture folder (for its own pages) and
//! in `shared` (for the shared ones, and the groups) give each page one of ten groups:
//! forest, glass, grass, gravel, metal, road, snow, tiles, water and wood
//! (`SOUND_InitFXGroups`, `SOUND_FXMapping`, texture.cpp:800). A page with no group is road.

use std::collections::HashMap;
use std::path::Path;

/// A run of sound ids to pick one of, first to last.
pub type Range = (usize, usize);

/// `S_FOOTS_ROAD_START` to `_END`.
pub const ROAD: Range = (54, 57);
/// `S_FOOTS_PUDDLE_START` to `_END`.
pub const PUDDLE: Range = (62, 65);
/// `S_SOFT_STEP_START` to `_END`.
pub const SOFT: Range = (510, 513);
/// The group a page with none takes (`if (page==255) page=5`).
const DEFAULT_GROUP: usize = 5;

#[derive(Debug, Clone, Default)]
pub struct Steps {
    /// Each group's run of sounds, as `[Groups]` lists them.
    groups: Vec<Range>,
    /// Each page's group.
    pages: HashMap<u32, usize>,
}

/// The lines of `section` in an ini file's text, as (key, value).
fn section<'a>(text: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line.eq_ignore_ascii_case(&format!("[{name}]"));
        } else if inside {
            if let Some((k, v)) = line.split_once('=') {
                out.push((k.trim(), v.trim()));
            }
        }
    }
    out
}

impl Steps {
    /// From the ini files' text: the shared one's groups and pages, then the world's pages.
    pub fn parse(shared: &str, world: &str) -> Steps {
        // "road (default)=30,31,-1,-1,54,57": the last two are the run of sounds.
        let groups = section(shared, "Groups").into_iter()
            .filter_map(|(_, v)| {
                let n: Vec<i64> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                (n.len() >= 6).then(|| (n[4].max(0) as usize, n[5].max(0) as usize))
            })
            .collect();
        let mut pages = HashMap::new();
        for (text, world_pages) in [(shared, false), (world, true)] {
            for (k, v) in section(text, "Textures") {
                let digits: String = k.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
                let (Ok(page), Ok(group)) = (digits.parse::<u32>(), v.parse::<usize>()) else { continue };
                // A world's file speaks for its own pages, the shared one for the rest.
                if world_pages == uc_world::level::world_page(page) {
                    pages.insert(page, group);
                }
            }
        }
        Steps { groups, pages }
    }

    /// The install's files for texture set `world`. Missing files give road everywhere.
    pub fn load(game: &Path, world: Option<i32>) -> Steps {
        let read = |rel: String| uc_formats::resolve(game, &rel).and_then(|p| std::fs::read(p).ok()).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
        let shared = read(r"server\textures\shared\soundfx.ini".to_string());
        let world = world.map(|w| read(format!(r"server\textures\world{w}\soundfx.ini"))).unwrap_or_default();
        Steps::parse(&shared, &world)
    }

    /// The sounds of a step on floor page `page`.
    pub fn on_page(&self, page: u32) -> Range {
        let group = self.pages.get(&page).copied().unwrap_or(DEFAULT_GROUP);
        self.groups.get(group).copied().unwrap_or(ROAD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHARED: &str = "[Groups]\nforest=183,186,-1,-1,284,287\nglass=178,181,-1,-1,279,282\ngrass=26,27,-1,-1,46,49\ngravel=28,29,-1,-1,50,53\nmetal=32,33,-1,-1,58,61\nroad (default)=30,31,-1,-1,54,57\n\n[Textures]\ntex264hi.tga=2\ntex010hi.tga=3\n";

    #[test]
    fn pages_take_their_group_and_the_rest_road() {
        let s = Steps::parse(SHARED, "[Textures]\ntex010hi.tga=4\ntex300hi.tga=0\n");
        assert_eq!(s.on_page(264), (46, 49), "a shared grass page");
        assert_eq!(s.on_page(10), (58, 61), "the world's own says metal");
        assert_eq!(s.on_page(300), ROAD, "a world file can't speak for a shared page");
        assert_eq!(s.on_page(77), ROAD);
    }
}
