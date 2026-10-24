//! The mission menu: the game's mission list (`data\urban.sty`) over one of its title
//! screens.

use crate::checklist;
use crate::credits::Credits;
use crate::progress::{self, clock, Progress};
use crate::replays;
use uc_game::goals::Goals;
use std::collections::BTreeMap;
use std::path::Path;
use uc_formats::txc::Texture;
use uc_render::Overlay;

/// Missions that play from start to finish here; the rest load, run what they can and are
/// good for exploring.
pub(crate) const PLAYABLE: [&str; 35] = ["ftutor1", "assault1", "testdrive1a", "fight1", "fight2", "police1", "testdrive2",
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
    /// The district it's pinned to on the city map, by its place in `urban.sty`'s list (none
    /// for the unreleased missions).
    pub district: Option<usize>,
}

/// A pin on the city map (`FE_MAPSCREEN`, fallen/Source/frontend.cpp:4046): a district's
/// name, where it is on the 640 by 480 map, and the missions in it (places in the list).
pub struct Pin {
    pub name: String,
    pub at: (f32, f32),
    pub missions: Vec<usize>,
}

/// An endless arena in the menu: a mission on its map, its name, its middle in squares (the
/// mission's start if none) and its radius.
pub type Arena = (String, String, Option<(f32, f32)>, f32);

/// Endless arenas with names: (map, name, middle in squares, radius in squares).
const NAMED_ARENAS: [(&str, &str, f32, f32, f32); 1] = [
    // Where Gun Hunt sends Darci to find Deeks ("Park: Find Deeks", police3 point 202).
    ("disturb1", "West District: the park", 34.5, 40.1, 9.0),
];

/// Beat Cop's districts: (mission, name). Only city streets with a crowd and traffic; the
/// crowd and cars come with the mission (`fake_civs`, `fake_cars`), not the map, so each is
/// the mission on its map with the most of both. Central Park, Skyline, Banes Estate and the
/// like have no traffic, and the training grounds neither.
const BEAT_DISTRICTS: [(&str, &str); 8] = [
    ("police1", "West District"),       // 15 people, 7 cars
    ("Gangorder1", "Gangland"),         // 15, 7
    ("e3", "Southside Beach"),          // 13, 7
    ("wstores1", "Rio Canal"),          // 13, 6
    ("botanicc", "Botanical Gardens"),  // 10, 7
    ("bball2", "Baseball Ground"),      // 15, 6
    ("factory1", "Television Centre"),  // 14, 6
    ("Baalrog3", "West Block"),         // 6, 4
];

/// The front end's screens (fallen/Source/frontend.cpp): the title (`FE_TITLESCREEN`), the
/// main menu (`FE_MAINMENU`) and the missions (the original's map screen, `FE_MAPSCREEN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Main,
    Missions,
    /// Endless mode's maps.
    Endless,
    /// The stats and achievements (#132).
    Stats,
    /// Co-op (#182): host, or join by address.
    Coop,
    /// Co-op: in a game, waiting for the other player or the host's choice.
    Lobby,
    /// The saved replays (#184).
    Replays,
    /// The credits (#209), scrolling.
    Credits,
}

/// What the lobby shows: who's in, this player's seat, and where the host can be reached.
#[derive(Debug, Clone, Default)]
pub struct Lobby {
    pub seat: usize,
    pub names: Vec<String>,
    pub hosting: Vec<String>,
    pub notices: Vec<String>,
}

/// The co-op screen's choices.
pub const COOP: [&str; 2] = ["HOST", "JOIN"];

/// The main menu's items (`raw_menu_data`, frontend.cpp:376): the ones there's something
/// behind here.
pub const MAIN: [&str; 10] = ["START", "CO-OP", "ENDLESS", "BEAT COP", "CAPTURE THE FLAG", "DIFFICULTY", "STATS", "REPLAYS", "CREDITS", "EXIT"];

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
    /// Endless mode's arenas: a mission on the map (to load it by), what's shown for it,
    /// and the arena (its middle in squares, the mission's start if none, and radius).
    pub maps: Vec<Arena>,
    /// Beat Cop's districts, the same way.
    pub beats: Vec<Arena>,
    pub map_selected: usize,
    /// A map chosen for an endless run, to load on the next frame.
    pub loading_endless: Option<usize>,
    /// Capture the flag chosen, to load on the next frame.
    pub loading_ctf: bool,
    /// The arena list is for a Beat Cop shift, not an endless run.
    pub for_beat: bool,
    /// Missions completed so far.
    pub progress: Progress,
    /// The co-op screen's choice, the address to join, and what went wrong last.
    pub coop: usize,
    pub address: String,
    pub status: String,
    /// The mission list is for the host's choice in co-op, not for playing alone.
    pub for_coop: bool,
    /// The co-op lobby, while in one.
    pub lobby: Option<Lobby>,
    /// The saved replays, newest first, as they were when the list was shown.
    pub replays: Vec<replays::Listed>,
    pub replay_selected: usize,
    /// A replay chosen, to load on the next frame.
    pub loading_replay: Option<usize>,
    /// The mission to open in the editor (#185), by its place in the list.
    pub loading_edit: Option<usize>,
    /// Seconds on the title screen with nothing pressed, and a demo to start (#208).
    pub idle: f32,
    pub loading_demo: bool,
    /// The city map's pins (#207), left to right as the original orders them, and the one
    /// chosen; and whether the missions show as the map or as the list.
    pub pins: Vec<Pin>,
    pub pin: usize,
    pub on_map: bool,
    /// How far through the credits.
    pub credits: Credits,
}

impl Menu {
    /// Reads the mission list.
    pub fn new(game: &Path) -> Menu {
        let mut entries: Vec<Entry> = Vec::new();
        let mut maps: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let (named, district_of) = districts(game);
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
            let district = district_of.get(&mission.to_ascii_lowercase()).copied();
            entries.push(Entry { mission, title, briefing, playable, goals, district });
        }
        let progress = Progress::load();
        let theme = Theme::of(entries.iter().filter(|e| progress.completed(&e.mission)).count());
        // An endless arena round the start of each map the missions use (named after its
        // first mission), and the few named ones besides.
        let mut arenas = Vec::new();
        for (map, ms) in maps {
            let stem = map.rsplit(['\\', '/']).next().unwrap_or(&map).trim_end_matches(".iam").to_string();
            arenas.push((ms[0].0.clone(), format!("{}: round the start ({stem})", ms[0].1), None, 10.0));
            for &(m, name, x, z, r) in &NAMED_ARENAS {
                if m.eq_ignore_ascii_case(&stem) {
                    arenas.push((ms[0].0.clone(), name.to_string(), Some((x, z)), r));
                }
            }
        }
        let maps = arenas;
        // A pin for each district with missions, ordered by where it is across the map
        // (`crap_remap`, frontend.cpp:4094).
        let mut pins: Vec<Pin> = named.into_iter().enumerate()
            .filter_map(|(d, n)| n.map(|(name, at)| (d, name, at)))
            .map(|(d, name, at)| Pin { name, at, missions: (0..entries.len()).filter(|&i| entries[i].district == Some(d)).collect() })
            .filter(|p| !p.missions.is_empty())
            .collect();
        pins.sort_by(|a, b| a.at.0.total_cmp(&b.at.0));
        let beats = BEAT_DISTRICTS
            .iter()
            .filter(|(m, _)| entries.iter().any(|e| e.mission.eq_ignore_ascii_case(m)))
            .map(|&(m, name)| (m.to_string(), name.to_string(), None, 10.0))
            .collect();
        let mut m = Menu { entries, screen: Screen::Title, main: 0, theme, since: std::time::Instant::now(), selected: 0, loading: None, progress, maps, beats, map_selected: 0, loading_endless: None, loading_ctf: false, for_beat: false,
               coop: 0, address: "127.0.0.1".into(), status: String::new(), for_coop: false, lobby: None,
               replays: Vec::new(), replay_selected: 0, loading_replay: None, loading_edit: None, idle: 0.0, loading_demo: false,
               pins, pin: 0, on_map: true, credits: Credits::default() };
        m.show_map();
        m
    }

    /// The map list on show: Beat Cop's districts or endless mode's arenas.
    pub fn arenas(&self) -> &[Arena] {
        if self.for_beat { &self.beats } else { &self.maps }
    }

    /// The image behind the current screen, from the install's `data\` (`menu_back_names`,
    /// `menu_map_names`, frontend.cpp:600): the title for the title and main menu, the map
    /// for the missions.
    pub fn image(&self) -> String {
        let w = self.theme.word();
        match self.screen {
            Screen::Title | Screen::Main | Screen::Credits => format!("data\\TITLE {}1.TGA", w.to_ascii_uppercase()),
            Screen::Missions | Screen::Endless | Screen::Stats | Screen::Coop | Screen::Lobby | Screen::Replays => format!("data\\map {w} darci.tga"),
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
        // Under the title, closer together on a short screen; and on one too short even for
        // that, a window of them round the choice that scrolls with it.
        let top = h * 0.32;
        let room = h - 16.0 * s - top;
        let step = (room / MAIN.len() as f32).clamp(22.0 * s, 26.0 * s);
        let shown = ((room / step) as usize).clamp(1, MAIN.len());
        let first = self.main.saturating_sub(shown / 2).min(MAIN.len() - shown);
        let mut y = top;
        for (k, &item) in MAIN.iter().enumerate().skip(first).take(shown) {
            let label = if item == "DIFFICULTY" {
                format!("DIFFICULTY: {}", self.progress.difficulty().name().to_uppercase())
            } else {
                item.to_string()
            };
            let item = label.as_str();
            let tw = Overlay::text_width(item, s * 2.0);
            if k == self.main {
                o.rect(w / 2.0 - tw / 2.0 - 8.0 * s, y - 3.0 * s, tw + 16.0 * s, 20.0 * s, [200, 150, 40, 200]);
            }
            o.text(w / 2.0 - tw / 2.0, y, s * 2.0, white, item);
            y += step;
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
            if let Some(p) = self.pin_of(i) {
                self.pin = p;
            }
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

    /// The pin the chosen mission is at, if it's on the map.
    fn pin_of(&self, entry: usize) -> Option<usize> {
        self.pins.iter().position(|p| p.missions.contains(&entry))
    }

    /// Onto the map: at the chosen mission's pin, or the first pin's first mission if it
    /// has none.
    pub fn show_map(&mut self) {
        self.on_map = true;
        match self.pin_of(self.selected) {
            Some(p) => self.pin = p,
            None => {
                self.pin = self.pin.min(self.pins.len().saturating_sub(1));
                if let Some(&m) = self.pins.get(self.pin).and_then(|p| p.missions.first()) {
                    self.selected = m;
                }
            }
        }
    }

    /// The next pin across the map (`by` +1 right, -1 left), at its first mission not yet
    /// completed, or its first.
    pub fn pin_step(&mut self, by: isize) {
        let n = self.pins.len() as isize;
        if n == 0 {
            return;
        }
        self.pin = (self.pin as isize + by).rem_euclid(n) as usize;
        let ms = &self.pins[self.pin].missions;
        self.selected = ms.iter().copied().find(|&m| !self.progress.completed(&self.entries[m].mission)).unwrap_or(ms[0]);
    }

    /// The next mission at the chosen pin (`by` +1 down, -1 up).
    pub fn pin_mission(&mut self, by: isize) {
        let Some(p) = self.pins.get(self.pin) else { return };
        let at = p.missions.iter().position(|&m| m == self.selected).unwrap_or(0) as isize;
        self.selected = p.missions[(at + by).rem_euclid(p.missions.len() as isize) as usize];
    }

    /// The missions as the original's map of Union City (`FE_MAPSCREEN`, #207): a pin for
    /// each district with missions in it, the one chosen gold and flashing with its missions
    /// beside it, and the mission chosen along the bottom.
    fn map_overlay(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        self.kibble(o, w, h, s);
        let (white, gold, dim) = ([235, 235, 225, 255], [255, 210, 70, 255], [150, 150, 145, 255]);
        let at = |p: &Pin| (p.at.0 * w / 640.0, p.at.1 * h / 480.0);
        let flash = ((self.since.elapsed().as_secs_f32() * 3.0) as u32).is_multiple_of(2);
        // The heading, and how far through.
        o.rect(0.0, 0.0, w, 30.0 * s, [0, 0, 0, 150]);
        o.text(12.0 * s, 8.0 * s, s * 2.0, gold, "UNION CITY");
        let done = self.entries.iter().filter(|e| self.progress.completed(&e.mission)).count();
        let tally = format!("{done} of {} missions complete", self.entries.len());
        o.text(w - Overlay::text_width(&tally, s) - 12.0 * s, 12.0 * s, s, gold, &tally);
        // The pins: red with something left to do, blue once it's all done.
        for (k, p) in self.pins.iter().enumerate() {
            let (x, y) = at(p);
            let all = p.missions.iter().all(|&m| self.progress.completed(&self.entries[m].mission));
            let chosen = k == self.pin;
            let col = match (chosen, all) {
                (true, _) if flash => [255, 240, 150, 255],
                (true, _) => gold,
                (false, true) => [110, 170, 230, 255],
                (false, false) => [220, 60, 50, 255],
            };
            let r = if chosen { 5.0 * s } else { 3.5 * s };
            o.rect(x - 0.5 * s, y - 9.0 * s, s, 9.0 * s, [30, 30, 30, 230]);
            o.rect(x - r - s, y - 9.0 * s - 2.0 * r - s, 2.0 * r + 2.0 * s, 2.0 * r + 2.0 * s, [0, 0, 0, 220]);
            o.rect(x - r, y - 9.0 * s - 2.0 * r, 2.0 * r, 2.0 * r, col);
        }
        // The chosen pin's district and missions beside it.
        if let Some(p) = self.pins.get(self.pin) {
            let (x, y) = at(p);
            let line = 11.0 * s;
            let rows: Vec<(String, usize)> = p.missions.iter().map(|&m| (self.entries[m].title.clone(), m)).collect();
            let bw = rows.iter().map(|(t, _)| Overlay::text_width(t, s)).fold(Overlay::text_width(&p.name, s * 1.5), f32::max) + 30.0 * s;
            let bh = 22.0 * s + rows.len() as f32 * line;
            let bx = if x + 14.0 * s + bw < w { x + 14.0 * s } else { x - 14.0 * s - bw };
            let by = (y - bh / 2.0).clamp(36.0 * s, h - 60.0 * s - bh);
            o.rect(bx, by, bw, bh, [0, 0, 0, 190]);
            o.text(bx + 8.0 * s, by + 6.0 * s, s * 1.5, gold, &p.name.to_uppercase());
            for (k, (title, m)) in rows.iter().enumerate() {
                let ry = by + 20.0 * s + k as f32 * line;
                if *m == self.selected {
                    o.rect(bx + 4.0 * s, ry - 2.0 * s, bw - 8.0 * s, line, [200, 150, 40, 200]);
                }
                if let Some(d) = self.progress.get(&self.entries[*m].mission) {
                    medal(o, bx + 10.0 * s, ry, s, d.hardest, self.full(&self.entries[*m]));
                }
                let col = if self.entries[*m].playable { white } else { dim };
                o.text(bx + 20.0 * s, ry, s, col, title);
            }
        }
        // Along the bottom: the mission chosen, how it's gone, and the keys.
        o.rect(0.0, h - 44.0 * s, w, 44.0 * s, [0, 0, 0, 170]);
        if let Some(i) = self.loading.or(self.loading_edit) {
            let text = format!("LOADING {}...", self.entries[i].title.to_uppercase());
            o.text(w / 2.0 - Overlay::text_width(&text, s * 2.0) / 2.0, h - 34.0 * s, s * 2.0, white, &text);
            return;
        }
        if let Some(e) = self.entries.get(self.selected) {
            o.text(12.0 * s, h - 40.0 * s, s * 1.5, gold, &e.title);
            let best = match (self.progress.best_time(&e.mission), self.progress.get(&e.mission)) {
                (Some(t), Some(d)) => format!("Best time {}, completed on {}", clock(t), d.hardest.name()),
                _ => "Not completed yet".to_string(),
            };
            o.text(12.0 * s, h - 25.0 * s, s, white, &best);
        }
        let keys = if self.for_coop {
            "CO-OP: choose the mission to play together.   Left/Right district   Up/Down mission   Enter play   Tab list   Esc back"
        } else {
            "Left/Right district   Up/Down mission   Enter play   Tab list   E edit   Esc back"
        };
        o.text(12.0 * s, h - 13.0 * s, s, [200, 200, 190, 220], &fit(keys, w - 24.0 * s, s));
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
        let (title, what, pick) = if self.for_beat {
            ("BEAT COP", "A shift: deal with what dispatch calls in. Left/Right shift length", "district")
        } else {
            ("ENDLESS", "Waves of thugs until you drop.", "map")
        };
        o.text(x, 16.0 * s, s * 3.0, gold, title);
        o.text(x, 44.0 * s, s, [200, 200, 190, 220], &format!("{what}   Up/Down choose a {pick}   Enter play   Esc back"));
        if self.for_beat {
            let len = format!("SHIFT: < {} >", uc_game::game::beat::shift_name(self.progress.shift()));
            o.text(x + Overlay::text_width(title, s * 3.0) + 16.0 * s, 24.0 * s, s * 1.5, white, &len);
        }
        if let Some(i) = self.loading_endless {
            let text = format!("LOADING {}...", self.arenas()[i].1.to_uppercase());
            o.text(w / 2.0 - Overlay::text_width(&text, s * 2.0) / 2.0, h * 0.5, s * 2.0, white, &text);
            return;
        }
        let line = 11.0 * s;
        let top = 64.0 * s;
        let rows = (((h - top - 40.0 * s) / line) as usize).max(3);
        let first = self.map_selected.saturating_sub(rows / 2).min(self.arenas().len().saturating_sub(rows));
        for (k, (_, label, _, _)) in self.arenas().iter().enumerate().skip(first).take(rows) {
            let y = top + (k - first) as f32 * line;
            if k == self.map_selected {
                o.rect(x - 4.0 * s, y - 2.0 * s, w * 0.55 - x - 8.0 * s, line, [200, 150, 40, 200]);
            }
            o.text(x, y, s, white, label);
            let best = if self.for_beat {
                self.progress.best_endless(&progress::beat_key(label, self.progress.shift()))
            } else {
                self.progress.best_endless(label)
            };
            if let Some((score, wave)) = best {
                let tag = if self.for_beat { format!("BEST {score} ({wave} HANDLED)") } else { format!("BEST {score} (WAVE {wave})") };
                o.text(w * 0.55 - Overlay::text_width(&tag, s) - 16.0 * s, y, s, gold, &tag);
            }
        }
    }

    /// The stats over all play, and the achievements with how far along each is.
    fn stats_overlay(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        self.kibble(o, w, h, s);
        o.rect(0.0, 0.0, w, h, [0, 0, 0, 170]);
        let (white, gold, dim) = ([235, 235, 225, 255], [255, 210, 70, 255], [150, 150, 145, 255]);
        let x = 20.0 * s;
        o.text(x, 16.0 * s, s * 3.0, gold, "STATS");
        o.text(x, 44.0 * s, s, [200, 200, 190, 220], "Esc back");
        let p = &self.progress;
        let t = &p.totals;
        let hit_rate = (t.hits * 100).checked_div(t.shots).map_or("-".into(), |r| format!("{r}%"));
        let wave = p.best_wave();
        let shift = p.best_shift().map_or("-".into(), |b| b.to_string());
        let rows = [
            ("LAW AND ORDER", String::new()),
            ("Arrests", t.arrests.to_string()),
            ("Takedowns from behind", t.takedowns.to_string()),
            ("People searched", t.searched.to_string()),
            ("Red marks", t.red_marks.to_string()),
            ("FIGHTS AND WEAPONS", String::new()),
            ("Knockouts: fists", t.ko_fists.to_string()),
            ("Knockouts: knife and bat", t.ko_weapons.to_string()),
            ("Knockouts: guns", t.ko_guns.to_string()),
            ("Shots fired", format!("{} ({hit_rate} hit)", t.shots)),
            ("GETTING ABOUT", String::new()),
            ("On foot", format!("{:.0} squares", t.on_foot)),
            ("Driving", format!("{:.0} squares", t.driven)),
            ("On the bike", format!("{:.0} squares", t.biked)),
            ("Zip wires", format!("{:.0} squares, longest {:.0}", t.zipped, t.longest_zip)),
            ("Ladders climbed", t.ladders.to_string()),
            ("Falls survived", t.falls.to_string()),
            ("MISSIONS AND MODES", String::new()),
            ("Missions completed", p.completed_count().to_string()),
            ("Endless: furthest wave", if wave > 0 { wave.to_string() } else { "-".into() }),
            ("Beat Cop: best shift", shift),
        ];
        let line = 10.0 * s;
        let mut y = 64.0 * s;
        for (label, value) in rows {
            if value.is_empty() {
                y += line * 0.4;
                o.text(x, y, s, gold, label);
            } else {
                o.text(x + 8.0 * s, y, s, white, label);
                o.text(x + 170.0 * s, y, s, white, &value);
            }
            y += line;
        }
        // The achievements, on the right.
        let ax = w * 0.5;
        let mut y = 64.0 * s + line * 0.4;
        o.text(ax, y, s, gold, "ACHIEVEMENTS");
        y += line * 1.4;
        for a in &crate::achievements::ALL {
            let earned = a.earned(p, t);
            let (have, goal) = a.progress(p, t);
            let col = if earned { gold } else { white };
            o.text(ax, y, s, col, a.name);
            let tag = if earned { "EARNED".to_string() } else if goal > 1 { format!("{have}/{goal}") } else { String::new() };
            o.text(w - Overlay::text_width(&tag, s) - 20.0 * s, y, s, col, &tag);
            y += line;
            o.text(ax + 8.0 * s, y, s, dim, a.text);
            y += line * 1.4;
        }
    }

    /// Co-op: host or join, then the lobby.
    fn coop_overlay(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        self.kibble(o, w, h, s);
        o.rect(0.0, 0.0, w, h, [0, 0, 0, 170]);
        let (white, gold, dim) = ([235, 235, 225, 255], [255, 210, 70, 255], [170, 170, 165, 255]);
        let x = 20.0 * s;
        o.text(x, 16.0 * s, s * 3.0, gold, "CO-OP");
        let mut y = 50.0 * s;
        match &self.lobby {
            None => {
                o.text(x, y, s, dim, "Two players: the host is Darci, the other player Roper.   Up/Down choose   Enter go   Esc back");
                y += 24.0 * s;
                for (k, item) in COOP.iter().enumerate() {
                    let label = if *item == "JOIN" { format!("JOIN {}_", self.address) } else { format!("HOST (port {})", uc_net::PORT) };
                    let tw = Overlay::text_width(&label, s * 2.0);
                    if k == self.coop {
                        o.rect(x - 6.0 * s, y - 3.0 * s, tw + 12.0 * s, 20.0 * s, [200, 150, 40, 200]);
                    }
                    o.text(x, y, s * 2.0, white, &label);
                    y += 26.0 * s;
                }
                o.text(x, y, s, dim, "To join, type the host's address (and :port if it isn't 7777).");
                y += 20.0 * s;
            }
            Some(l) => {
                let you = if l.seat == 0 { "You're hosting, as seat 1." } else { "You're in, as seat 2." };
                o.text(x, y, s, white, you);
                y += 16.0 * s;
                for (k, n) in l.names.iter().enumerate() {
                    let who = if k == 0 { "DARCI" } else { "ROPER" };
                    let name = if n.is_empty() { "(waiting...)" } else { n.as_str() };
                    o.text(x, y, s * 2.0, if n.is_empty() { dim } else { gold }, &format!("{}  {who}: {name}", k + 1));
                    y += 22.0 * s;
                }
                y += 6.0 * s;
                if !l.hosting.is_empty() {
                    o.text(x, y, s, white, &format!("The other player joins at {}", l.hosting.join("   or   ")));
                    y += 12.0 * s;
                    o.text(x, y, s, dim, "From another network: forward the port to this machine, or use a VPN such as Tailscale.");
                    y += 16.0 * s;
                }
                let keys = if l.seat == 0 { "Enter: choose the mission   Esc: leave" } else { "Waiting for the host to choose a mission.   Esc: leave" };
                o.text(x, y, s, dim, keys);
                y += 16.0 * s;
                for n in &l.notices {
                    o.text(x, y, s, white, n);
                    y += 12.0 * s;
                }
            }
        }
        if !self.status.is_empty() {
            o.text(x, y + 8.0 * s, s, [255, 120, 100, 255], &self.status);
        }
    }

    /// The replays screen, with the list as it is now.
    pub fn show_replays(&mut self) {
        self.replays = replays::list();
        self.replay_selected = self.replay_selected.min(self.replays.len().saturating_sub(1));
        self.screen = Screen::Replays;
    }

    /// The credits, from the start.
    pub fn show_credits(&mut self) {
        self.credits = Credits::default();
        self.screen = Screen::Credits;
    }

    /// The saved replays: a list on the left (when, which mission, the time, PB for a
    /// personal best), and the one selected in a panel on the right, everything about it.
    fn replays_overlay(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
        self.kibble(o, w, h, s);
        o.rect(0.0, 0.0, w * 0.5, h, [0, 0, 0, 170]);
        let (white, gold, dim) = ([235, 235, 225, 255], [255, 210, 70, 255], [150, 150, 145, 255]);
        let x = 20.0 * s;
        o.text(x, 16.0 * s, s * 3.0, gold, "REPLAYS");
        o.text(x, 44.0 * s, s, [200, 200, 190, 220], "Up/Down choose   Enter watch   Esc back");
        if let Some(i) = self.loading_replay {
            let text = format!("LOADING {}...", self.title_of(&self.replays[i].start.mission).to_uppercase());
            o.text(w / 2.0 - Overlay::text_width(&text, s * 2.0) / 2.0, h * 0.5, s * 2.0, white, &text);
            return;
        }
        if !self.status.is_empty() {
            o.text(x, h - 20.0 * s, s, [255, 120, 100, 255], &self.status);
        }
        if self.replays.is_empty() {
            let lines = Overlay::wrap("No replays yet. A mission won, lost, or left after ten seconds is kept here.", w * 0.5 - 2.0 * x, s);
            for (k, l) in lines.iter().enumerate() {
                o.text(x, 64.0 * s + k as f32 * 11.0 * s, s, white, l);
            }
            return;
        }
        let line = 11.0 * s;
        let top = 64.0 * s;
        let rows = (((h - top - 30.0 * s) / line) as usize).max(3);
        let first = self.replay_selected.saturating_sub(rows / 2).min(self.replays.len().saturating_sub(rows));
        // Columns: when, the mission, then the time (right-aligned at the list's edge).
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let title_x = x + Overlay::text_width("59 minutes ago", s) + 12.0 * s;
        let right = w * 0.5 - 12.0 * s;
        for (k, r) in self.replays.iter().enumerate().skip(first).take(rows) {
            let y = top + (k - first) as f32 * line;
            if k == self.replay_selected {
                o.rect(x - 4.0 * s, y - 2.0 * s, w * 0.5 - x - 8.0 * s, line, [200, 150, 40, 200]);
            }
            o.text(x, y, s, dim, &replays::ago(r.date, now));
            let time = match (self.personal_best(r), r.won()) {
                (true, _) => format!("PB {}", clock(r.time)),
                (false, true) => clock(r.time),
                // Not won: how it ended, short.
                _ if r.outcome.starts_with("lost") => "lost".into(),
                _ => "left".into(),
            };
            let tw = Overlay::text_width(&time, s);
            let title = fit(&self.title_of(&r.start.mission), right - tw - 12.0 * s - title_x, s);
            o.text(title_x, y, s, white, &title);
            o.text(right - tw, y, s, if r.won() { gold } else { dim }, &time);
        }
        if let Some(r) = self.replays.get(self.replay_selected) {
            self.replay_panel(o, w, h, s, r, now);
        }
    }

    /// Whether replay `r` is the run that set this mission's best time.
    fn personal_best(&self, r: &replays::Listed) -> bool {
        use uc_net::replay::Mode;
        // The progress keeps times to a tenth of a second.
        r.won() && matches!(r.start.mode, Mode::Mission | Mode::Coop)
            && self.progress.best_time(&r.start.mission).is_some_and(|b| (r.time - b).abs() < 0.06)
    }

    /// Everything about replay `r`, in a panel on the right.
    fn replay_panel(&self, o: &mut Overlay, w: f32, h: f32, s: f32, r: &replays::Listed, now: u64) {
        use uc_net::replay::Mode;
        let (white, gold, dim, red) = ([235, 235, 225, 255], [255, 210, 70, 255], [170, 170, 160, 255], [255, 140, 120, 255]);
        let mut rows: Vec<(String, [u8; 4])> = Vec::new();
        let best = self.progress.best_time(&r.start.mission);
        let pb = self.personal_best(r);
        match r.outcome.as_str() {
            "won" if pb => rows.push((format!("Won in {}: the personal best", clock(r.time)), gold)),
            "won" => rows.push((format!("Won in {}", clock(r.time)), gold)),
            "abandoned" => rows.push((format!("Left at {}", clock(r.time)), white)),
            lost => rows.push((format!("{} at {}", capital(lost), clock(r.time)), red)),
        }
        if let (Some(b), false, Mode::Mission | Mode::Coop) = (best, pb, r.start.mode) {
            let slower = if r.won() { format!(" (this run {:.1} s slower)", r.time - b) } else { String::new() };
            rows.push((format!("Personal best {}{slower}", clock(b)), white));
        }
        let mode = match r.start.mode {
            Mode::Mission => "The mission".to_string(),
            Mode::Explore => "Exploring (no script)".to_string(),
            Mode::Coop => "Co-op, with Roper".to_string(),
            Mode::Endless { seed, .. } => format!("Endless (seed {seed})"),
            Mode::Beat { shift, .. } => format!("Beat Cop: {} shift", shift.map_or("open-ended".into(), |m| format!("{m}-minute"))),
        };
        rows.push((format!("{mode} on {}", r.start.difficulty().name()), white));
        let entry = self.entries.iter().find(|e| e.mission.eq_ignore_ascii_case(&r.start.mission));
        match (r.start.mode, entry) {
            (Mode::Endless { .. }, _) => rows.push((format!("Score {}, wave {}", r.tally.score, r.tally.count), white)),
            (Mode::Beat { .. }, _) => rows.push((format!("Score {}, {} handled", r.tally.score, r.tally.count), white)),
            (_, Some(e)) if !e.goals.badges.is_empty() || !e.goals.sides.is_empty() => rows.push((
                format!("Badges {}/{}, side objectives {}/{}", r.tally.badges, e.goals.badges.len(), r.tally.sides, e.goals.sides.len()), white)),
            _ => {}
        }
        let mut kit = Vec::new();
        if r.start.weapon != uc_game::Weapon::None {
            kit.push(format!("{:?}", r.start.weapon).to_lowercase());
        }
        if r.start.dogs > 0 {
            kit.push(format!("{} dogs", r.start.dogs));
        }
        if !kit.is_empty() {
            rows.push((format!("Started with: {}", kit.join(", ")), white));
        }
        rows.push((String::new(), white));
        rows.push((format!("Played {}. The replay is {} long.", replays::ago(r.date, now), clock(r.length)), dim));
        let kept = if r.kept { "Kept for good" } else { "One of the last 20 (K on the end screen keeps a run for good)" };
        rows.push((kept.to_string(), dim));
        if !r.same_build {
            rows.push((format!("Recorded on another build ({}): it may not play back the same", r.build), red));
        }
        if let Some(name) = r.path.file_name() {
            rows.push((name.to_string_lossy().into_owned(), dim));
        }
        let (px, pw) = (w * 0.55, w * 0.42);
        let bw = pw - 24.0 * s;
        let line = 11.0 * s;
        let mut lines: Vec<(String, [u8; 4])> = Vec::new();
        for (t, c) in rows {
            if t.is_empty() {
                lines.push((t, c));
                continue;
            }
            lines.extend(Overlay::wrap(&t, bw, s).into_iter().map(|l| (l, c)));
        }
        let ph = 40.0 * s + lines.len() as f32 * line;
        let py = (h - ph - 24.0 * s).max(74.0 * s);
        o.rect(px, py, pw, ph, [0, 0, 0, 180]);
        o.text(px + 12.0 * s, py + 10.0 * s, s * 2.0, gold, &fit(&self.title_of(&r.start.mission), bw, s * 2.0));
        for (k, (l, c)) in lines.iter().enumerate() {
            o.text(px + 12.0 * s, py + 32.0 * s + k as f32 * line, s, *c, l);
        }
    }

    /// Mission `name`'s title, as the list has it.
    fn title_of(&self, name: &str) -> String {
        self.entries.iter().find(|e| e.mission.eq_ignore_ascii_case(name)).map_or(name.to_string(), |e| e.title.clone())
    }

    pub fn map_up(&mut self) {
        let n = self.arenas().len().max(1);
        self.map_selected = (self.map_selected + n - 1) % n;
    }

    pub fn map_down(&mut self) {
        self.map_selected = (self.map_selected + 1) % self.arenas().len().max(1);
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
        if self.screen == Screen::Stats {
            self.stats_overlay(&mut o, w, h, s);
            return o;
        }
        if self.screen == Screen::Replays {
            self.replays_overlay(&mut o, w, h, s);
            return o;
        }
        if self.screen == Screen::Credits {
            // The title image, dark enough to read over, as the outro's own backdrop.
            o.backdrop(0.0, 0.0, w, h, [255, 255, 255, 255]);
            o.rect(0.0, 0.0, w, h, [0, 0, 0, 190]);
            self.credits.draw(&mut o, w, h, s);
            return o;
        }
        if matches!(self.screen, Screen::Coop | Screen::Lobby) {
            self.coop_overlay(&mut o, w, h, s);
            return o;
        }
        if self.screen != Screen::Missions {
            self.front(&mut o, w, h, s);
            return o;
        }
        if self.on_map && !self.pins.is_empty() {
            self.map_overlay(&mut o, w, h, s);
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
        let keys = if self.for_coop { "CO-OP: choose the mission to play together.   Up/Down choose   Enter play   Tab map   Esc back" } else { "Up/Down choose   Enter play   E edit   Tab map   Esc back" };
        o.text(x, 44.0 * s, s, [200, 200, 190, 220], keys);
        // How far through the missions.
        let done = self.entries.iter().filter(|e| self.progress.completed(&e.mission)).count();
        let [sta, str, rfx, con] = self.progress.stats();
        let full = self.entries.iter().filter(|e| self.full(e)).count();
        let tally = format!("{done} of {} missions complete, {full} at 100%    Badges: STA {sta}  STR {str}  REF {rfx}  CON {con}",
                            self.entries.len());
        o.text(x, 56.0 * s, s, gold, &tally);
        if let Some(i) = self.loading.or(self.loading_edit) {
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
            let col = if e.playable { white } else { [150, 150, 145, 255] };
            // A medal before missions completed, by the hardest difficulty won on.
            if let Some(d) = self.progress.get(&e.mission) {
                medal(&mut o, x, y, s, d.hardest, self.full(e));
            }
            // The best time at the list's right edge.
            let best = self.progress.best_time(&e.mission).map_or(String::new(), clock);
            let right = w * 0.5 - 12.0 * s;
            let bw = Overlay::text_width(&best, s);
            o.text(x + 10.0 * s, y, s, col, &fit(&e.title, right - bw - 12.0 * s - (x + 10.0 * s), s));
            o.text(right - bw, y, s, gold, &best);
        }
        // The selected mission in a panel on the right: its title, the best time (or not
        // completed yet), the checklist (objectives, side objectives and badges, ticked once
        // done in a win), then as much of the briefing as fits on the screen.
        if let Some(e) = self.entries.get(self.selected) {
            let (px, pw) = (w * 0.55, w * 0.42);
            let bw = pw - 24.0 * s;
            let (story, objectives) = checklist::split_briefing(&e.briefing);
            let kept = self.progress.get(&e.mission);
            let (badges, _) = self.tally(e);
            let items = checklist::items(&objectives, kept.is_some(), &e.goals, badges,
                                         |id| kept.is_some_and(|d| d.sides.contains(&id)));
            let (best, best_col) = match (self.progress.best_time(&e.mission), kept) {
                (Some(t), Some(d)) => {
                    let full = if self.full(e) { ", everything found" } else { "" };
                    (format!("Best time {}, completed on {}{full}", clock(t), d.hardest.name()), gold)
                }
                _ => ("Not completed yet".to_string(), [170, 170, 160, 255]),
            };
            let list_h = checklist::height(&items, bw, s);
            // The briefing gets what's left between the panel's top limit and the bottom.
            let head = 58.0 * s + list_h;
            let room = (((h - 24.0 * s - 74.0 * s - head) / line).floor() as usize).max(1);
            let mut lines = Overlay::wrap(&story, bw, s);
            if lines.len() > room {
                lines.truncate(room);
                if let Some(last) = lines.last_mut() {
                    *last = fit(&format!("{last}..."), bw, s);
                }
            }
            let ph = head + lines.len() as f32 * line;
            let py = (h - ph - 24.0 * s).max(74.0 * s);
            o.rect(px, py, pw, ph, [0, 0, 0, 180]);
            o.text(px + 12.0 * s, py + 10.0 * s, s * 2.0, gold, &fit(&e.title, bw, s * 2.0));
            o.text(px + 12.0 * s, py + 30.0 * s, s, best_col, &fit(&best, bw, s));
            let y = py + 46.0 * s;
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
const UNRELEASED: [(&str, &str, &str); 6] = [
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
    // Two early prototypes, also untitled. Takeover1 is on a map in the older layout
    // (`jumper1`) and never had an ending, which this project adds (see
    // `uc_world::level::finish_unfinished`).
    ("Takeover1", "Bank Sector Takeover",
     "Mr X's gang have overrun the banking sector and sealed off the surrounding area. Get into \
      that sector and rescue the two hostages.\r\
      - Find the two workers: they'll follow you once they see you.\r\
      - Get them out of the front gates and to the extraction point."),
    ("subdock1", "Dock Extraction",
     "Terminate the dock operator and get to the chopper for extraction. The extraction site is \
      on the most southern part of this sector.\r\
      - Take out the dock operator.\r- Meet the chopper at the rendezvous point."),
];

/// `text` cut short with "..." to fit `width` at scale `s`.
pub(crate) fn fit(text: &str, width: f32, s: f32) -> String {
    if Overlay::text_width(text, s) <= width {
        return text.to_string();
    }
    let mut out: String = text.to_string();
    while !out.is_empty() && Overlay::text_width(&format!("{out}..."), s) > width {
        out.pop();
    }
    format!("{}...", out.trim_end())
}

/// "lost: Darci died" as "Lost: Darci died".
fn capital(text: &str) -> String {
    let mut c = text.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

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

/// A district on the city map: its name and where it is on the 640 by 480 map.
pub type District = (String, (f32, f32));

/// The city map's districts and where each mission is (`FRONTEND_districts`,
/// fallen/Source/frontend.cpp:4046): `urban.sty`'s `[districts]` list, `name=x,y` on the 640
/// by 480 map, by place in the list (none for the blank ones), and each mission's district,
/// the seventh field of its line (keyed by the mission file's name, lower case).
pub fn districts(game: &Path) -> (Vec<Option<District>>, BTreeMap<String, usize>) {
    let text = uc_formats::resolve(game, "data\\urban.sty")
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| b.iter().map(|&c| c as char).collect::<String>())
        .unwrap_or_default();
    let (mut named, mut of) = (Vec::new(), BTreeMap::new());
    let mut in_list = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_list = true;
        } else if in_list {
            // A pin's point is the bottom middle of the button (`mapx - 16`, `mapy - 32`).
            let pin = line.split_once('=').and_then(|(name, at)| {
                let (x, y) = at.split_once(',')?;
                let (x, y) = (x.trim().parse::<f32>().ok()?, y.trim().parse::<f32>().ok()?);
                (!name.is_empty() && x > 0.0 && y > 0.0).then(|| (name.to_string(), (x, y)))
            });
            named.push(pin);
        } else if !line.starts_with("//") {
            let f: Vec<&str> = line.split(" : ").map(str::trim).collect();
            if let (Some(d), Some(file)) = (f.get(6).and_then(|d| d.parse().ok()), f.get(7)) {
                if let Some(m) = file.to_ascii_lowercase().strip_suffix(".ucm") {
                    of.insert(m.to_string(), d);
                }
            }
        }
    }
    (named, of)
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

/// The medal before a completed mission in the list (#136), at `x`, `y` (the row's top):
/// bronze won on Easy, silver on Normal, gold on Hard, on a ribbon; ringed when everything
/// in it has been found (100%).
fn medal(o: &mut Overlay, x: f32, y: f32, s: f32, on: uc_game::Difficulty, full: bool) {
    let col = match on {
        uc_game::Difficulty::Easy => [205, 127, 50, 255],
        uc_game::Difficulty::Normal => [205, 210, 220, 255],
        uc_game::Difficulty::Hard => [255, 210, 70, 255],
    };
    let dark = [0, 0, 0, 200];
    // The ring: an outline round the whole medal.
    if full {
        let ring = [255, 245, 200, 255];
        o.rect(x - 3.0 * s, y - 2.0 * s, 9.0 * s, 1.0 * s, ring);
        o.rect(x - 3.0 * s, y + 8.0 * s, 9.0 * s, 1.0 * s, ring);
        o.rect(x - 3.0 * s, y - 2.0 * s, 1.0 * s, 11.0 * s, ring);
        o.rect(x + 5.0 * s, y - 2.0 * s, 1.0 * s, 11.0 * s, ring);
    }
    // The ribbon, two tails up top.
    o.rect(x - 1.0 * s, y - 1.0 * s, 1.5 * s, 3.0 * s, [60, 90, 200, 255]);
    o.rect(x + 2.5 * s, y - 1.0 * s, 1.5 * s, 3.0 * s, [200, 50, 50, 255]);
    // The disc: a square with its corners cut, edged dark.
    o.rect(x - 1.5 * s, y + 2.0 * s, 6.0 * s, 6.0 * s, dark);
    o.rect(x - 1.0 * s, y + 2.5 * s, 5.0 * s, 5.0 * s, col);
    for (cx, cy) in [(-1.0, 2.5), (3.0, 2.5), (-1.0, 6.5), (3.0, 6.5)] {
        o.rect(x + cx * s, y + cy * s, 1.0 * s, 1.0 * s, dark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `urban.sty`'s districts: the training missions are in the Combat Centre and on the
    /// tracks, RTA in the West District; every playable street mission has a pin, and the pins
    /// go left to right. Skipped without an install.
    #[test]
    fn the_missions_are_pinned_to_their_districts() {
        let Some(game) = uc_formats::default_install() else { return };
        let (named, of) = districts(&game);
        let name = |m: &str| named[of[m]].as_ref().map(|d| d.0.clone());
        assert_eq!(name("ftutor1").as_deref(), Some("Combat Centre"));
        assert_eq!(named[of["ftutor1"]].as_ref().unwrap().1, (465.0, 134.0));
        assert_eq!(name("testdrive1a").as_deref(), Some("Oval Circuit"));
        assert_eq!(name("police1").as_deref(), Some("West District"));
        let m = Menu::new(&game);
        for e in m.entries.iter().filter(|e| e.playable && !UNRELEASED.iter().any(|u| u.0 == e.mission)) {
            assert!(m.pins.iter().any(|p| p.missions.iter().any(|&i| m.entries[i].mission == e.mission)), "{} has no pin", e.mission);
        }
        assert!(m.pins.windows(2).all(|w| w[0].at.0 <= w[1].at.0));
        // The unreleased missions are only in the list.
        assert!(m.entries.iter().filter(|e| UNRELEASED.iter().any(|u| u.0 == e.mission)).all(|e| e.district.is_none()));
    }

    #[test]
    fn left_and_right_go_from_pin_to_pin_and_up_and_down_through_one() {
        let Some(game) = uc_formats::default_install() else { return };
        let mut m = Menu::new(&game);
        let start = m.pin;
        assert!(m.pins[start].missions.contains(&m.selected));
        m.pin_step(1);
        assert_eq!(m.pin, (start + 1) % m.pins.len());
        assert!(m.pins[m.pin].missions.contains(&m.selected));
        m.pin_step(-1);
        assert_eq!(m.pin, start);
        let many = m.pins.iter().position(|p| p.missions.len() > 1).unwrap();
        m.pin = many;
        m.selected = m.pins[many].missions[0];
        m.pin_mission(1);
        assert_eq!(m.selected, m.pins[many].missions[1]);
        m.pin_mission(-1);
        assert_eq!(m.selected, m.pins[many].missions[0]);
    }
}
