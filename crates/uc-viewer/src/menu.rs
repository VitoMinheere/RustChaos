//! The mission menu: the game's mission list (`data\urban.sty`) over one of its title
//! screens.

use crate::checklist;
use crate::progress::{clock, Progress};
use uc_game::goals::Goals;
use std::path::Path;
use uc_formats::txc::Texture;
use uc_render::Overlay;

/// Missions that play from start to finish here; the rest load, run what they can and are
/// good for exploring.
const PLAYABLE: [&str; 14] = ["ftutor1", "assault1", "testdrive1a", "fight1", "fight2", "police1", "testdrive2",
                              "testdrive3", "police2", "Bankbomb1", "police3", "Gangorder2", "Gordout1",
                              "Stealtst1"];

pub struct Entry {
    /// The mission file's name, without `.ucm`.
    pub mission: String,
    pub title: String,
    pub briefing: String,
    pub playable: bool,
    /// Its badges and side objectives.
    pub goals: Goals,
}

pub struct Menu {
    pub entries: Vec<Entry>,
    pub selected: usize,
    /// A mission chosen, to load on the next frame (after "Loading" is on screen).
    pub loading: Option<usize>,
    /// Missions completed so far.
    pub progress: Progress,
}

impl Menu {
    /// Reads the mission list.
    pub fn new(game: &Path) -> Menu {
        let mut entries: Vec<Entry> = Vec::new();
        for (mission, title, briefing) in missions(game) {
            let playable = PLAYABLE.iter().any(|p| p.eq_ignore_ascii_case(&mission));
            let goals = uc_formats::resolve(game, &format!("levels\\{mission}.ucm"))
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| uc_formats::ucm::Mission::parse(&b).ok())
                .map(|m| uc_game::goals::goals(&m))
                .unwrap_or_default();
            entries.push(Entry { mission, title, briefing, playable, goals });
        }
        Menu { entries, selected: 0, loading: None, progress: Progress::load() }
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

    /// The menu as an overlay for a window of `size`.
    pub fn overlay(&self, size: (u32, u32)) -> Overlay {
        let mut o = Overlay::new();
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).round().max(1.0);
        // The title screen, darkened on the left behind the list.
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        o.rect(0.0, 0.0, w * 0.5, h, [0, 0, 0, 170]);
        let white = [235, 235, 225, 255];
        let gold = [255, 210, 70, 255];
        let x = 20.0 * s;
        o.text(x, 16.0 * s, s * 3.0, gold, "URBAN CHAOS");
        o.text(x, 44.0 * s, s, [200, 200, 190, 220], "Up/Down choose   Enter play   Esc quit");
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

/// The game's mission list (`data\urban.sty`): (mission file without `.ucm`, title,
/// briefing), for the missions this install has. Lines are
/// `id : group : parent : ... : file.ucm : title : briefing`.
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
    out
}

/// One of the game's title screens (24/32-bit uncompressed TGA), for the menu's background.
pub fn backdrop(game: &Path) -> Option<Texture> {
    let b = std::fs::read(uc_formats::resolve(game, "data\\TITLE LEAVES1.TGA")?).ok()?;
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
