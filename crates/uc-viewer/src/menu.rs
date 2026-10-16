//! The mission menu: the game's mission list (`data\urban.sty`) over one of its title
//! screens.

use crate::checklist;
use crate::progress::{clock, Progress};
use uc_game::goals::Goals;
use std::collections::BTreeMap;
use std::path::Path;
use uc_formats::txc::Texture;
use uc_render::Overlay;

/// Missions that play from start to finish here; the rest load, run what they can and are
/// good for exploring.
const PLAYABLE: [&str; 35] = ["ftutor1", "assault1", "testdrive1a", "fight1", "fight2", "police1", "testdrive2",
                              "testdrive3", "police2", "Bankbomb1", "police3", "Gangorder2", "Gordout1",
                              "Stealtst1", "Baalrog3", "Gangorder1", "police4", "bball2",
                              "wstores1", "e3", "westcrime1", "carbomb1", "botanicc",
                              "park2", "factory1", "snow2", "MIB", "Semtex", "estate2", "mission2", "skymiss2", "AWOL1", "Finale1",
                              "FreeCD1", "jung3"];

pub struct Entry {
    /// The mission file's name, without `.ucm`.
    pub mission: String,
    pub title: String,
    pub briefing: String,
    pub playable: bool,
    /// Its badges and side objectives.
    pub goals: Goals,
}

/// The front end's screens (fallen/Source/frontend.cpp): the title (`FE_TITLESCREEN`), the
/// main menu (`FE_MAINMENU`) and the missions (the original's map screen, `FE_MAPSCREEN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Main,
    Missions,
    /// Endless mode's maps.
    Endless,
}

/// The main menu's items (`raw_menu_data`, frontend.cpp:376): the ones there's something
/// behind here.
pub const MAIN: [&str; 3] = ["START", "ENDLESS", "EXIT"];

/// The menus' look, by how far through the game (`menu_theme`, set in
/// `FRONTEND_MissionHierarchy`, frontend.cpp:3337): leaves for the first eight missions done,
/// then rain, snow, and blood from 24 on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Leaves,
    Rain,
    Snow,
    Blood,
}

impl Theme {
    pub fn of(done: usize) -> Theme {
        match done {
            0..=7 => Theme::Leaves,
            8..=15 => Theme::Rain,
            16..=23 => Theme::Snow,
            _ => Theme::Blood,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Theme::Leaves => "leaves",
            Theme::Rain => "rain",
            Theme::Snow => "snow",
            Theme::Blood => "blood",
        }
    }
}

pub struct Menu {
    pub entries: Vec<Entry>,
    pub screen: Screen,
    /// The main menu's choice.
    pub main: usize,
    pub theme: Theme,
    /// When the menu came up: the falling leaves (or rain, snow, blood) move with it.
    since: std::time::Instant,
    pub selected: usize,
    /// A mission chosen, to load on the next frame (after "Loading" is on screen).
    pub loading: Option<usize>,
    /// Endless mode's maps: a mission on each (to load it by), and what's shown for it.
    pub maps: Vec<(String, String)>,
    pub map_selected: usize,
    /// A map chosen for an endless run, to load on the next frame.
    pub loading_endless: Option<usize>,
    /// Missions completed so far.
    pub progress: Progress,
}

impl Menu {
    /// Reads the mission list.
    pub fn new(game: &Path) -> Menu {
        let mut entries: Vec<Entry> = Vec::new();
        let mut maps: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for (mission, title, briefing) in missions(game) {
            let playable = PLAYABLE.iter().any(|p| p.eq_ignore_ascii_case(&mission));
            let goals = uc_formats::resolve(game, &format!("levels\\{mission}.ucm"))
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| uc_formats::ucm::Mission::parse(&b).ok())
                .map(|m| uc_game::goals::goals(&m))
                .unwrap_or_default();
            let map = uc_formats::resolve(game, &format!("levels\\{mission}.ucm"))
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| uc_formats::ucm::Mission::parse(&b).ok())
                .map(|m| m.map.to_ascii_lowercase());
            if let Some(map) = map.filter(|m| !m.is_empty()) {
                maps.entry(map).or_default().push((mission.clone(), title.clone()));
            }
            entries.push(Entry { mission, title, briefing, playable, goals });
        }
        let progress = Progress::load();
        let theme = Theme::of(entries.iter().filter(|e| progress.completed(&e.mission)).count());
        // One endless map for each map the missions use, called after its missions.
        let maps = maps.into_iter().map(|(map, ms)| {
            let names: Vec<&str> = ms.iter().take(3).map(|m| m.1.as_str()).collect();
            let more = if ms.len() > 3 { ", ..." } else { "" };
            let stem = map.rsplit(['\\', '/']).next().unwrap_or(&map).trim_end_matches(".iam").to_string();
            (ms[0].0.clone(), format!("{stem}: {}{more}", names.join(", ")))
        }).collect();
        Menu { entries, screen: Screen::Title, main: 0, theme, since: std::time::Instant::now(), selected: 0, loading: None, progress, maps, map_selected: 0, loading_endless: None }
    }

    /// The image behind the current screen, from the install's `data\` (`menu_back_names`,
    /// `menu_map_names`, frontend.cpp:600): the title for the title and main menu, the map
    /// for the missions.
    pub fn image(&self) -> String {
        let w = self.theme.word();
        match self.screen {
            Screen::Title | Screen::Main => format!("data\\TITLE {}1.TGA", w.to_ascii_uppercase()),
            Screen::Missions | Screen::Endless => format!("data\\map {w} darci.tga"),
        }
    }

    /// The falling kibble (`FRONTEND_kibble_init_one`, frontend.cpp:2289): leaves, rain, snow
    /// or blood drifting down over the menus, by the theme.
    fn kibble(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        let t = self.since.elapsed().as_secs_f32();
        let (n, speed, size, colour): (usize, f32, (f32, f32), [u8; 4]) = match self.theme {
            Theme::Leaves => (40, 40.0, (4.0, 3.0), [190, 120, 40, 220]),
            Theme::Rain => (120, 420.0, (1.0, 9.0), [170, 190, 215, 150]),
            Theme::Snow => (90, 30.0, (2.0, 2.0), [240, 240, 250, 230]),
            Theme::Blood => (50, 120.0, (2.0, 4.0), [150, 10, 10, 230]),
        };
        let mut seed = 0x1234_5678u32;
        let mut r = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        for _ in 0..n {
            let (x0, y0, v, sway) = (r(), r(), 0.6 + r() * 0.8, r() * std::f32::consts::TAU);
            let y = (y0 * h + t * speed * v * s / 2.0) % (h + 20.0) - 10.0;
            let drift = if self.theme == Theme::Rain { t * 30.0 * s } else { (t * 1.3 + sway).sin() * 14.0 * s };
            let x = (x0 * w + drift).rem_euclid(w);
            o.rect(x, y, size.0 * s, size.1 * s, colour);
        }
    }

    /// The title, or the main menu, over its image.
    fn front(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        self.kibble(o, w, h, s);
        let gold = [255, 210, 70, 255];
        let white = [235, 235, 225, 255];
        let big = s * 4.0;
        let title = "URBAN CHAOS";
        o.text(w / 2.0 - Overlay::text_width(title, big) / 2.0, h * 0.18, big, gold, title);
        if self.screen == Screen::Title {
            let any = "PRESS ANY KEY";
            if ((self.since.elapsed().as_secs_f32() * 2.0) as u32).is_multiple_of(2) {
                o.text(w / 2.0 - Overlay::text_width(any, s * 2.0) / 2.0, h * 0.8, s * 2.0, white, any);
            }
            return;
        }
        let mut y = h * 0.55;
        for (k, item) in MAIN.iter().enumerate() {
            let tw = Overlay::text_width(item, s * 2.0);
            if k == self.main {
                o.rect(w / 2.0 - tw / 2.0 - 8.0 * s, y - 3.0 * s, tw + 16.0 * s, 20.0 * s, [200, 150, 40, 200]);
            }
            o.text(w / 2.0 - tw / 2.0, y, s * 2.0, white, item);
            y += 26.0 * s;
        }
    }

    /// The entry after mission `name` (for "next mission").
    pub fn after(&self, name: &str) -> Option<usize> {
        let i = self.entries.iter().position(|e| e.mission.eq_ignore_ascii_case(name))?;
        (i + 1 < self.entries.len()).then_some(i + 1)
    }

    pub fn select(&mut self, name: &str) {
        if let Some(i) = self.entries.iter().position(|e| e.mission.eq_ignore_ascii_case(name)) {
            self.selected = i;
        }
    }

    /// Badges and side objectives of entry `e` done in winning runs: (badges, sides).
    fn tally(&self, e: &Entry) -> (usize, usize) {
        let Some(d) = self.progress.get(&e.mission) else { return (0, 0) };
        let badges = e.goals.badges.iter().filter(|b| d.badges.contains(b)).count();
        let sides = e.goals.sides.iter().filter(|g| d.sides.contains(&g.id)).count();
        (badges, sides)
    }

    /// Won, with every badge and side objective.
    fn full(&self, e: &Entry) -> bool {
        self.progress.completed(&e.mission) && self.tally(e) == (e.goals.badges.len(), e.goals.sides.len())
    }

    pub fn up(&mut self) {
        self.selected = (self.selected + self.entries.len().max(1) - 1) % self.entries.len().max(1);
    }

    pub fn down(&mut self) {
        self.selected = (self.selected + 1) % self.entries.len().max(1);
    }

    /// Endless mode's maps: the list, and the best run on the one chosen.
    fn endless_overlay(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        self.kibble(o, w, h, s);
        o.rect(0.0, 0.0, w * 0.55, h, [0, 0, 0, 170]);
        let (white, gold) = ([235, 235, 225, 255], [255, 210, 70, 255]);
        let x = 20.0 * s;
        o.text(x, 16.0 * s, s * 3.0, gold, "ENDLESS");
        o.text(x, 44.0 * s, s, [200, 200, 190, 220], "Waves of thugs until you drop. Up/Down choose a map   Enter play   Esc back");
        if let Some(i) = self.loading_endless {
            let text = format!("LOADING {}...", self.maps[i].1.to_uppercase());
            o.text(w / 2.0 - Overlay::text_width(&text, s * 2.0) / 2.0, h * 0.5, s * 2.0, white, &text);
            return;
        }
        let line = 11.0 * s;
        let top = 64.0 * s;
        let rows = (((h - top - 40.0 * s) / line) as usize).max(3);
        let first = self.map_selected.saturating_sub(rows / 2).min(self.maps.len().saturating_sub(rows));
        for (k, (mission, label)) in self.maps.iter().enumerate().skip(first).take(rows) {
            let y = top + (k - first) as f32 * line;
            if k == self.map_selected {
                o.rect(x - 4.0 * s, y - 2.0 * s, w * 0.55 - x - 8.0 * s, line, [200, 150, 40, 200]);
            }
            o.text(x, y, s, white, label);
            if let Some((score, wave)) = self.progress.best_endless(mission) {
                let tag = format!("BEST {score} (WAVE {wave})");
                o.text(w * 0.55 - Overlay::text_width(&tag, s) - 16.0 * s, y, s, gold, &tag);
            }
        }
    }

    pub fn map_up(&mut self) {
        self.map_selected = (self.map_selected + self.maps.len().max(1) - 1) % self.maps.len().max(1);
    }

    pub fn map_down(&mut self) {
        self.map_selected = (self.map_selected + 1) % self.maps.len().max(1);
    }

    /// The menu as an overlay for a window of `size`.
    pub fn overlay(&self, size: (u32, u32)) -> Overlay {
        let mut o = Overlay::new();
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).round().max(1.0);
        if self.screen == Screen::Endless {
            self.endless_overlay(&mut o, w, h, s);
            return o;
        }
        if self.screen != Screen::Missions {
            self.front(&mut o, w, h, s);
            return o;
        }
        // The map, darkened on the left behind the list, with the kibble falling.
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        self.kibble(&mut o, w, h, s);
        o.rect(0.0, 0.0, w * 0.5, h, [0, 0, 0, 170]);
        let white = [235, 235, 225, 255];
        let gold = [255, 210, 70, 255];
        let x = 20.0 * s;
        o.text(x, 16.0 * s, s * 3.0, gold, "URBAN CHAOS");
        o.text(x, 44.0 * s, s, [200, 200, 190, 220], "Up/Down choose   Enter play   Esc back");
        // How far through the missions.
        let done = self.entries.iter().filter(|e| self.progress.completed(&e.mission)).count();
        let [sta, str, rfx, con] = self.progress.stats();
        let full = self.entries.iter().filter(|e| self.full(e)).count();
        let tally = format!("{done} of {} missions complete, {full} at 100%    Badges: STA {sta}  STR {str}  REF {rfx}  CON {con}",
                            self.entries.len());
        o.text(x, 56.0 * s, s, gold, &tally);
        if let Some(i) = self.loading {
            let text = format!("LOADING {}...", self.entries[i].title.to_uppercase());
            o.text(w / 2.0 - Overlay::text_width(&text, s * 2.0) / 2.0, h * 0.5, s * 2.0, white, &text);
            return o;
        }
        // A window of the list around the selection.
        let line = 11.0 * s;
        let top = 74.0 * s;
        let rows = (((h - top - 70.0 * s) / line) as usize).max(3);
        let first = self.selected.saturating_sub(rows / 2).min(self.entries.len().saturating_sub(rows));
        for (k, e) in self.entries.iter().enumerate().skip(first).take(rows) {
            let y = top + (k - first) as f32 * line;
            if k == self.selected {
                o.rect(x - 4.0 * s, y - 2.0 * s, w * 0.5 - x - 8.0 * s, line, [200, 150, 40, 200]);
            }
            let done = self.progress.completed(&e.mission);
            let col = if e.playable { white } else { [150, 150, 145, 255] };
            // A tick before missions completed.
            if done {
                o.rect(x - 2.0 * s, y + 4.0 * s, 2.0 * s, 2.0 * s, gold);
                o.rect(x, y + 6.0 * s, 2.0 * s, 2.0 * s, gold);
                o.rect(x + 2.0 * s, y + 2.0 * s, 2.0 * s, 4.0 * s, gold);
                o.rect(x + 4.0 * s, y, 2.0 * s, 2.0 * s, gold);
            }
            o.text(x + 10.0 * s, y, s, col, &e.title);
            let (tag, tc) = match (done, e.playable) {
                (true, _) if self.full(e) => ("100%", gold),
                (true, _) => ("COMPLETE", gold),
                (false, true) => ("PLAYABLE", [120, 230, 120, 255]),
                (false, false) => ("EXPLORE", [150, 150, 145, 255]),
            };
            o.text(w * 0.5 - Overlay::text_width(tag, s) - 16.0 * s, y, s, tc, tag);
        }
        // The selected mission in a panel on the right: its title, the checklist (objectives,
        // side objectives and badges, ticked once done in a win), then the briefing.
        if let Some(e) = self.entries.get(self.selected) {
            let (px, pw) = (w * 0.55, w * 0.42);
            let bw = pw - 24.0 * s;
            let (story, objectives) = checklist::split_briefing(&e.briefing);
            let kept = self.progress.get(&e.mission);
            let (badges, _) = self.tally(e);
            let items = checklist::items(&objectives, kept.is_some(), &e.goals, badges,
                                         |id| kept.is_some_and(|d| d.sides.contains(&id)));
            let mut lines = Overlay::wrap(&story, bw, s);
            if let Some(t) = self.progress.best_time(&e.mission) {
                lines.push(String::new());
                lines.push(format!("Completed. Best time {}", clock(t)));
            }
            let list_h = checklist::height(&items, bw, s);
            let ph = 44.0 * s + list_h + lines.len() as f32 * line;
            let py = (h - ph - 24.0 * s).max(74.0 * s);
            o.rect(px, py, pw, ph, [0, 0, 0, 180]);
            o.text(px + 12.0 * s, py + 10.0 * s, s * 2.0, gold, &e.title);
            let y = py + 32.0 * s;
            checklist::draw(&mut o, px + 12.0 * s, y, bw, s, &items);
            for (k, l) in lines.iter().enumerate() {
                o.text(px + 12.0 * s, y + list_h + 6.0 * s + k as f32 * line, s, [215, 215, 205, 255], l);
            }
        }
        o
    }
}

/// Finished missions the game never listed, after the story as in the original's suggested
/// order (`suggest_order`, fallen/Source/frontend.cpp:259): (mission, title, briefing).
/// `urban.sty` has neither, so the titles are this project's and the briefings are written
/// from the missions' opening messages.
const UNRELEASED: [(&str, &str, &str); 4] = [
    ("FreeCD1", "Construction Site Siege",
     "The local Wildcat chapter have stormed the construction site and taken some workers hostage, \
      and they've spread out all over this section of town. We want those hostages ALIVE, Stern!\r\
      - Take out ALL the Wildcats within four minutes.\r- Keep the construction workers safe."),
    ("jung3", "El Cossa Island",
     "That nut Bane an' his men have retreated to El Cossa Island. We got sightings of a construction \
      site, possibly building MORE silos. We want those sites LEVELLED, Stern! Island residents have \
      already been killed: expect severe resistance.\r\
      - Get into the compound and take down Bane's workmen.\r- Ensure no more islanders come to harm."),
    // These two have no title or briefing anywhere in the game: both are made up from their
    // scripts. Officer Training is the later of the training course's two versions (`train1`
    // is the other), and both have a motorbike, as do the Missile Silos.
    ("train11", "Officer Training",
     "Welcome to officer training, Stern. Learn to jump, climb, drive and shoot like a Union City \
      officer: the assault course, the shooting range (with a sniper rifle to try out), the \
      driving school with its motorbike, and the hand to hand arena.\r\
      - Talk to the training officer.\r- Don't break the lasers, and don't hit a fellow officer."),
    ("snow1", "Missile Silos",
     "The Wildcats have three missiles up on the mountain, and they'll launch them the moment \
      they see you coming. Take a bike and get there first.\r\
      - Destroy all three missiles before they launch."),
];

/// The game's mission list (`data\urban.sty`): (mission file without `.ucm`, title,
/// briefing), for the missions this install has, then the unreleased ones that play. Lines
/// are `id : group : parent : ... : file.ucm : title : briefing`.
pub fn missions(game: &Path) -> Vec<(String, String, String)> {
    let text = uc_formats::resolve(game, "data\\urban.sty")
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| b.iter().map(|&c| c as char).collect::<String>())
        .unwrap_or_default();
    let mut out: Vec<(String, String, String)> = Vec::new();
    for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
        let f: Vec<&str> = line.split(" : ").map(str::trim).collect();
        if f.len() < 9 || !f[7].to_ascii_lowercase().ends_with(".ucm") {
            continue;
        }
        let mission = f[7][..f[7].len() - 4].to_string();
        if out.iter().any(|e| e.0.eq_ignore_ascii_case(&mission))
            || uc_formats::resolve(game, &format!("levels\\{mission}.ucm")).is_none()
        {
            continue;
        }
        out.push((mission, f[8].to_string(), f.get(9).copied().unwrap_or("").to_string()));
    }
    for (mission, title, briefing) in UNRELEASED {
        if uc_formats::resolve(game, &format!("levels\\{mission}.ucm")).is_some() {
            out.push((mission.to_string(), title.to_string(), briefing.to_string()));
        }
    }
    out
}

/// One of the game's front-end images (24/32-bit uncompressed TGA, `file` under the
/// install), for the menu's background.
pub fn backdrop(game: &Path, file: &str) -> Option<Texture> {
    let b = std::fs::read(uc_formats::resolve(game, file)?).ok()?;
    if b.len() < 18 || b[2] != 2 {
        return None;
    }
    let (w, h) = (u16::from_le_bytes([b[12], b[13]]) as usize, u16::from_le_bytes([b[14], b[15]]) as usize);
    let bpp = b[16] as usize / 8;
    let top_down = b[17] & 0x20 != 0;
    let data = b.get(18 + b[0] as usize..)?;
    if bpp < 3 || data.len() < w * h * bpp {
        return None;
    }
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        let src = if top_down { y } else { h - 1 - y };
        for x in 0..w {
            let p = &data[(src * w + x) * bpp..];
            let o = (y * w + x) * 4;
            rgba[o..o + 4].copy_from_slice(&[p[2], p[1], p[0], 255]);
        }
    }
    Some(Texture { width: w as u32, height: h as u32, has_alpha: false, rgba })
}
