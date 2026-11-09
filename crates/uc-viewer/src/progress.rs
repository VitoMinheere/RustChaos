//! Which missions have been completed: the best time for each, and the badges and side
//! objectives done in it (across all the runs that were won). Kept in a small text file in the
//! user's settings folder, a line per mission: `mission seconds b=<badge points> s=<side
//! objectives> d=<hardest difficulty> m=<medals>`. (Older files have four stat counts in place
//! of the lists, and no difficulty or medals.)

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use uc_game::goals::badge_stat;

#[derive(Default, Clone)]
pub struct Done {
    pub best: f32,
    /// Badge points collected.
    pub badges: BTreeSet<usize>,
    /// Side objectives done (by their point).
    pub sides: BTreeSet<usize>,
    /// Stat counts from before badges were tracked one by one.
    legacy: [u8; 4],
    /// The hardest difficulty it's been won on (Normal for wins from before there was one).
    pub hardest: uc_game::Difficulty,
    /// The best medal for each goal (#516), over all its fair wins.
    pub medals: crate::medals::Medals,
}

impl Done {
    /// The stats this mission's badges give: stamina, strength, reflexes, constitution.
    fn stats(&self) -> [u8; 4] {
        let mut s = [0u8; 4];
        for &b in &self.badges {
            s[badge_stat(b)] += 1;
        }
        // Never fewer than an older save recorded.
        for (n, old) in s.iter_mut().zip(self.legacy) {
            *n = (*n).max(old);
        }
        s
    }
}

#[derive(Default)]
pub struct Progress {
    done: BTreeMap<String, Done>,
    /// The chief's warnings so far about civilian casualties (`DarciDeadCivWarnings`).
    civ_warnings: u8,
    /// The best endless run on each map: (score, wave reached).
    endless: BTreeMap<String, (i64, u32)>,
    /// The difficulty chosen in the menu.
    difficulty: uc_game::Difficulty,
    /// The Beat Cop shift length chosen in the menu.
    shift: uc_game::game::beat::Shift,
    /// The time of day chosen for Endless and Beat Cop in the menu (#515), none for the
    /// map's own lighting.
    time: Option<uc_world::TimeOfDay>,
    /// Racing the best run's ghost on the missions that have one (#186): on unless switched off.
    ghosts: bool,
    /// The time trials (#519), by [`trial_key`]: the best race and the best lap.
    trials: BTreeMap<String, (Option<f32>, Option<f32>)>,
    /// Traffic on the roads in a time trial: off unless switched on.
    traffic: bool,
    /// Everything done, over all play (#132).
    pub totals: uc_game::game::stats::Stats,
    /// The achievements earned, by id.
    pub achieved: BTreeSet<String>,
    path: Option<PathBuf>,
}

/// An endless or Beat Cop record's key: lower case, and one word (the file splits on spaces).
fn key(map: &str) -> String {
    map.to_ascii_lowercase().replace(' ', "_")
}

/// The key a Beat Cop shift's best score is kept under: one per map and shift length.
pub fn beat_key(label: &str, shift: uc_game::game::beat::Shift) -> String {
    use uc_game::game::beat::Shift;
    let len = match shift {
        Shift::Minutes(m) => m.to_string(),
        Shift::Open => "open".into(),
        Shift::Unlimited => "unlimited".into(),
    };
    format!("beat{len}:{label}")
}

/// A shift's length in the progress file: the minutes, 0 for open-ended, or "unlimited".
fn shift_code(s: uc_game::game::beat::Shift) -> String {
    use uc_game::game::beat::Shift;
    match s {
        Shift::Minutes(m) => m.to_string(),
        Shift::Open => "0".into(),
        Shift::Unlimited => "unlimited".into(),
    }
}

fn shift_named(s: &str) -> Option<uc_game::game::beat::Shift> {
    use uc_game::game::beat::Shift;
    match s {
        "unlimited" => Some(Shift::Unlimited),
        "0" => Some(Shift::Open),
        n => n.parse().ok().map(Shift::Minutes),
    }
}

/// The key a time trial's bests are kept under: one per course and car.
pub fn trial_key(course: &str, car: uc_game::game::trial::Car) -> String {
    format!("{}:{}", key(course), car.id())
}

fn list(s: &str) -> BTreeSet<usize> {
    s.split(',').filter_map(|n| n.parse().ok()).collect()
}

impl Progress {
    /// `%APPDATA%\urban-chaos-rs\progress.txt` on Windows, `~/.config/urban-chaos-rs/...`
    /// elsewhere.
    fn default_path() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("urban-chaos-rs").join("progress.txt"))
    }

    /// The folder the progress file is in, where other saved things go too.
    pub fn folder() -> Option<PathBuf> {
        Some(Progress::default_path()?.parent()?.to_path_buf())
    }

    pub fn load() -> Progress {
        Progress::load_from(Progress::default_path())
    }

    /// The progress kept at `path` (none if there's no file, and nothing saved without one).
    fn load_from(path: Option<PathBuf>) -> Progress {
        let text = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        let civ_warnings = text.lines().find_map(|l| l.strip_prefix("@civ_warnings ")?.trim().parse().ok()).unwrap_or(0);
        let endless = text.lines().filter_map(|l| {
            let mut f = l.strip_prefix("@endless ")?.split_whitespace();
            Some((f.next()?.to_string(), (f.next()?.parse().ok()?, f.next()?.parse().ok()?)))
        }).collect();
        let done = text
            .lines()
            .filter(|l| !l.starts_with('@'))
            .filter_map(|l| {
                let mut f = l.split_whitespace();
                let mission = f.next()?.to_ascii_lowercase();
                let mut d = Done { best: f.next()?.parse().ok()?, ..Default::default() };
                let mut legacy = 0;
                for field in f {
                    if let Some(b) = field.strip_prefix("b=") {
                        d.badges = list(b);
                    } else if let Some(s) = field.strip_prefix("s=") {
                        d.sides = list(s);
                    } else if let Some(n) = field.strip_prefix("d=").and_then(|n| n.parse::<usize>().ok()) {
                        d.hardest = uc_game::Difficulty::ALL.get(n).copied().unwrap_or_default();
                    } else if let Some(m) = field.strip_prefix("m=").and_then(crate::medals::Medals::decode) {
                        d.medals = m;
                    } else if let (Ok(n), true) = (field.parse(), legacy < 4) {
                        d.legacy[legacy] = n;
                        legacy += 1;
                    }
                }
                Some((mission, d))
            })
            .collect();
        let difficulty = text.lines().find_map(|l| l.strip_prefix("@difficulty ")?.trim().parse::<usize>().ok())
            .and_then(|n| uc_game::Difficulty::ALL.get(n).copied()).unwrap_or_default();
        let shift = text.lines().find_map(|l| shift_named(l.strip_prefix("@shift ")?.trim()))
            .unwrap_or(uc_game::game::beat::DEFAULT_SHIFT);
        let time = text.lines().find_map(|l| uc_world::TimeOfDay::named(l.strip_prefix("@time ")?.trim()));
        let mut stats = uc_game::game::stats::Stats::default();
        for l in text.lines().filter_map(|l| l.strip_prefix("@stat ")) {
            let mut f = l.split_whitespace();
            if let (Some(name), Some(v)) = (f.next(), f.next().and_then(|v| v.parse().ok())) {
                stats.set(name, v);
            }
        }
        let achieved = text.lines().filter_map(|l| Some(l.strip_prefix("@ach ")?.trim().to_string())).collect();
        let ghosts = !text.lines().any(|l| l.trim() == "@ghosts off");
        let secs = |f: Option<&str>| f.and_then(|t| t.parse::<f32>().ok());
        let trials = text.lines().filter_map(|l| {
            let mut f = l.strip_prefix("@trial ")?.split_whitespace();
            Some((f.next()?.to_string(), (secs(f.next()), secs(f.next()))))
        }).collect();
        let traffic = text.lines().any(|l| l.trim() == "@traffic on");
        Progress { done, civ_warnings, endless, difficulty, shift, time, ghosts, trials, traffic, totals: stats, achieved, path }
    }

    pub fn ghosts(&self) -> bool {
        self.ghosts
    }

    pub fn set_ghosts(&mut self, on: bool) {
        self.ghosts = on;
        self.save();
    }

    pub fn traffic(&self) -> bool {
        self.traffic
    }

    pub fn set_traffic(&mut self, on: bool) {
        self.traffic = on;
        self.save();
    }

    /// The best race and the best lap of time trial `key` ([`trial_key`]).
    pub fn best_trial(&self, key: &str) -> (Option<f32>, Option<f32>) {
        self.trials.get(key).copied().unwrap_or_default()
    }

    /// Records a lap of time trial `key`; whether it's the best.
    pub fn record_lap(&mut self, key: &str, secs: f32) -> bool {
        let e = self.trials.entry(key.to_string()).or_default();
        let best = e.1.is_none_or(|b| secs < b);
        if best {
            e.1 = Some(secs);
            self.save();
        }
        best
    }

    /// Records a whole race of time trial `key`; whether it's the best.
    pub fn record_race(&mut self, key: &str, secs: f32) -> bool {
        let e = self.trials.entry(key.to_string()).or_default();
        let best = e.0.is_none_or(|b| secs < b);
        if best {
            e.0 = Some(secs);
            self.save();
        }
        best
    }

    /// Another mission ended with too many civilian casualties: the chief's warnings before
    /// this one (Game.cpp:2783). The fourth costs her badge.
    pub fn warn(&mut self) -> u8 {
        let before = self.civ_warnings;
        self.civ_warnings = before.saturating_add(1);
        self.save();
        before
    }

    pub fn difficulty(&self) -> uc_game::Difficulty {
        self.difficulty
    }

    pub fn set_difficulty(&mut self, d: uc_game::Difficulty) {
        self.difficulty = d;
        self.save();
    }

    pub fn shift(&self) -> uc_game::game::beat::Shift {
        self.shift
    }

    pub fn set_shift(&mut self, shift: uc_game::game::beat::Shift) {
        self.shift = shift;
        self.save();
    }

    pub fn time(&self) -> Option<uc_world::TimeOfDay> {
        self.time
    }

    pub fn set_time(&mut self, time: Option<uc_world::TimeOfDay>) {
        self.time = time;
        self.save();
    }

    /// Adds a mission's or a run's stats to the totals.
    pub fn add_stats(&mut self, s: &uc_game::game::stats::Stats) {
        self.totals = self.totals.plus(s);
        self.save();
    }

    /// Notes achievement `id` earned.
    pub fn achieve(&mut self, id: &str) {
        self.achieved.insert(id.to_string());
        self.save();
    }

    /// The furthest wave reached in an endless run, on any map.
    pub fn best_wave(&self) -> u32 {
        self.endless.iter().filter(|(k, _)| !k.starts_with("beat")).map(|(_, &(_, w))| w).max().unwrap_or(0)
    }

    /// The best Beat Cop score, any map and shift.
    pub fn best_shift(&self) -> Option<i64> {
        self.endless.iter().filter(|(k, _)| k.starts_with("beat")).map(|(_, &(s, _))| s).max()
    }

    /// The missions completed.
    pub fn completed_count(&self) -> usize {
        self.done.len()
    }

    /// The best endless run on `map`: (score, wave).
    pub fn best_endless(&self, map: &str) -> Option<(i64, u32)> {
        self.endless.get(&key(map)).copied()
    }

    /// The records kept under arena `old`'s name go under `new`'s, an endless run's and every
    /// Beat Cop shift length's, unless there's one there already (an arena renamed). Whether
    /// any moved; saved if so.
    pub fn rename_endless(&mut self, old: &str, new: &str) -> bool {
        let (old, new) = (key(old), key(new));
        let moving: Vec<(String, String)> = self.endless.keys()
            .filter_map(|k| {
                let (shift, name) = k.split_once(':').filter(|(s, _)| s.starts_with("beat")).map_or(("", k.as_str()), |(s, n)| (s, n));
                (name == old).then(|| (k.clone(), if shift.is_empty() { new.clone() } else { format!("{shift}:{new}") }))
            })
            .filter(|(_, to)| !self.endless.contains_key(to))
            .collect();
        for (from, to) in &moving {
            let r = self.endless.remove(from).unwrap();
            self.endless.insert(to.clone(), r);
        }
        if !moving.is_empty() {
            self.save();
        }
        !moving.is_empty()
    }

    /// Records an endless run on `map`; whether it's a new best.
    pub fn record_endless(&mut self, map: &str, score: i64, wave: u32) -> bool {
        let e = self.endless.entry(key(map)).or_insert((i64::MIN, 0));
        let best = score > e.0;
        if best {
            *e = (score, wave);
        }
        self.save();
        best
    }

    pub fn get(&self, mission: &str) -> Option<&Done> {
        self.done.get(&mission.to_ascii_lowercase())
    }

    pub fn completed(&self, mission: &str) -> bool {
        self.get(mission).is_some()
    }

    pub fn best_time(&self, mission: &str) -> Option<f32> {
        self.get(mission).map(|d| d.best)
    }

    /// Darci's stats from the badges kept in completed missions: stamina, strength, reflexes,
    /// constitution.
    pub fn stats(&self) -> [u8; 4] {
        self.done.values().fold([0; 4], |mut s, d| {
            for (t, n) in s.iter_mut().zip(d.stats()) {
                *t = t.saturating_add(n);
            }
            s
        })
    }

    /// Records a win in `secs`, with the badges found and side objectives done on the way
    /// (added to those of earlier wins).
    pub fn record(&mut self, mission: &str, secs: f32, badges: &BTreeSet<usize>, sides: &BTreeSet<usize>, on: uc_game::Difficulty) {
        let fresh = !self.done.contains_key(&mission.to_ascii_lowercase());
        let d = self.done.entry(mission.to_ascii_lowercase()).or_insert(Done { best: secs, hardest: on, ..Default::default() });
        if fresh || on > d.hardest {
            d.hardest = on;
        }
        d.best = d.best.min(secs);
        d.badges.extend(badges);
        d.sides.extend(sides);
        self.save();
    }

    /// Keeps the best of each of `mission`'s medals with those of `won` (a win already
    /// [`record`](Progress::record)ed); the medals before.
    pub fn record_medals(&mut self, mission: &str, won: crate::medals::Medals) -> crate::medals::Medals {
        let Some(d) = self.done.get_mut(&mission.to_ascii_lowercase()) else { return Default::default() };
        let before = d.medals;
        d.medals = before.best(won);
        self.save();
        before
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let join = |s: &BTreeSet<usize>| s.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",");
        let text: String = self
            .done
            .iter()
            .map(|(m, d)| {
                let [a, b, c, e] = d.legacy;
                let legacy = if d.legacy == [0; 4] { String::new() } else { format!(" {a} {b} {c} {e}") };
                let on = uc_game::Difficulty::ALL.iter().position(|&x| x == d.hardest).unwrap_or(1);
                let medals = if d.medals == Default::default() { String::new() } else { format!(" m={}", d.medals.encode()) };
                format!("{m} {:.1} b={} s={} d={on}{medals}{legacy}\n", d.best, join(&d.badges), join(&d.sides))
            })
            .chain((self.civ_warnings > 0).then(|| format!("@civ_warnings {}\n", self.civ_warnings)))
            .chain(self.endless.iter().map(|(m, (s, w))| format!("@endless {m} {s} {w}\n")))
            .chain((self.shift != uc_game::game::beat::DEFAULT_SHIFT).then(|| format!("@shift {}\n", shift_code(self.shift))))
            .chain(self.time.map(|t| format!("@time {}\n", t.name())))
            .chain(self.totals.fields().into_iter().filter(|(_, v)| *v != 0.0).map(|(n, v)| format!("@stat {n} {v}\n")))
            .chain(self.achieved.iter().map(|a| format!("@ach {a}\n")))
            .chain(self.trials.iter().map(|(k, (race, lap))| {
                let t = |x: &Option<f32>| x.map_or("-".to_string(), |s| format!("{s:.2}"));
                format!("@trial {k} {} {}\n", t(race), t(lap))
            }))
            .chain(self.traffic.then(|| "@traffic on\n".to_string()))
            .chain((!self.ghosts).then(|| "@ghosts off\n".to_string()))
            .chain((self.difficulty != uc_game::Difficulty::Normal).then(|| {
                let n = uc_game::Difficulty::ALL.iter().position(|&d| d == self.difficulty).unwrap_or(1);
                format!("@difficulty {n}\n")
            }))
            .collect();
        let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(path, text));
        if let Err(e) = saved {
            eprintln!("couldn't save progress to {}: {e}", path.display());
        }
    }
}

/// `m:ss`.
pub fn clock(secs: f32) -> String {
    let s = secs.round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// A time to the hundredth, for the time trials: `1:02.35`.
pub fn fine_clock(secs: f32) -> String {
    let h = (secs.max(0.0) * 100.0).round() as u32;
    format!("{}:{:02}.{:02}", h / 6000, h / 100 % 60, h % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chiefs_warnings_add_up() {
        let mut p = Progress::default();
        assert_eq!((p.warn(), p.warn(), p.warn()), (0, 1, 2));
        assert_eq!(p.civ_warnings, 3);
    }

    #[test]
    fn badges_count_once_and_old_saves_still_count() {
        let mut p = Progress::default();
        p.done.insert("police1".into(), Done { best: 600.0, legacy: [2, 0, 0, 1], ..Default::default() });
        // Two badges this time: the stats are at least what the old save had.
        let found: BTreeSet<usize> = [23, 82].into();
        p.record("police1", 500.0, &found, &[149].into(), uc_game::Difficulty::Normal);
        p.record("police1", 550.0, &found, &BTreeSet::new(), uc_game::Difficulty::Normal);
        let d = p.get("police1").unwrap();
        assert_eq!((d.best, d.badges.len(), d.sides.len()), (500.0, 2, 1));
        let s = p.stats();
        assert!(s[0] >= 2 && s[3] >= 1 && s.iter().map(|&n| n as usize).sum::<usize>() <= 5);
    }

    /// The hardest difficulty a mission's been won on is kept: a later, easier win doesn't
    /// lower it.
    #[test]
    fn the_hardest_win_is_kept() {
        use uc_game::Difficulty::*;
        let mut p = Progress::default();
        let none = BTreeSet::new();
        p.record("police1", 500.0, &none, &none, Easy);
        assert_eq!(p.get("police1").unwrap().hardest, Easy);
        p.record("police1", 500.0, &none, &none, Hard);
        p.record("police1", 400.0, &none, &none, Normal);
        assert_eq!(p.get("police1").unwrap().hardest, Hard);
    }

    /// A progress file of its own in the temp folder, never the player's.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("uc-progress-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.txt");
        let _ = std::fs::remove_file(&path);
        path
    }

    /// Medals are saved, the best of each goal kept, and loaded back as they were.
    #[test]
    fn medals_round_trip() {
        use crate::medals::{Medals, Tier};
        let path = scratch("medals");
        let mut p = Progress::load_from(Some(path.clone()));
        let none = BTreeSet::new();
        assert_eq!(p.record_medals("police1", Medals { time: Tier::Gold, ..Default::default() }), Medals::default(), "not won yet");
        p.record("police1", 70.0, &[23].into(), &none, uc_game::Difficulty::Normal);
        p.record_medals("police1", Medals { time: Tier::Gold, clean: false, thorough: true });
        let before = p.record_medals("police1", Medals { time: Tier::Bronze, clean: true, thorough: false });
        assert_eq!(before, Medals { time: Tier::Gold, clean: false, thorough: true });
        p.record("fight1", 30.0, &none, &none, uc_game::Difficulty::Easy);
        let q = Progress::load_from(Some(path.clone()));
        assert_eq!(q.get("police1").unwrap().medals, Medals { time: Tier::Gold, clean: true, thorough: true });
        assert_eq!(q.get("police1").unwrap().badges, [23].into());
        assert_eq!(q.get("fight1").unwrap().medals, Medals::default());
        // Missions without medals are saved as they were before there were any.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("police1 70.0 b=23 s= d=1 m=311\n") && text.contains("fight1 30.0 b= s= d=0\n"), "{text}");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// A time trial's best race and lap are kept for each course and car, only ever lowered,
    /// and loaded back; so is traffic switched on.
    #[test]
    fn trial_bests_round_trip() {
        use uc_game::game::trial::Car;
        let path = scratch("trials");
        let mut p = Progress::load_from(Some(path.clone()));
        let (oval, van) = (trial_key("oval", Car::Police), trial_key("oval", Car::Van));
        assert!(p.record_lap(&oval, 21.5));
        assert!(!p.record_lap(&oval, 22.0));
        assert!(p.record_lap(&oval, 20.25));
        assert!(p.record_race(&oval, 64.0));
        assert!(p.record_lap(&van, 30.0));
        p.set_traffic(true);
        let q = Progress::load_from(Some(path.clone()));
        assert_eq!(q.best_trial(&oval), (Some(64.0), Some(20.25)));
        assert_eq!(q.best_trial(&van), (None, Some(30.0)));
        assert_eq!(q.best_trial(&trial_key("oval", Car::Sports)), (None, None));
        assert!(q.traffic());
        assert_eq!(fine_clock(62.346), "1:02.35");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// A file from before medals (and before difficulties and badge lists) loads as it did,
    /// with none.
    #[test]
    fn old_files_load_without_medals() {
        let path = scratch("old");
        std::fs::write(&path, "police1 600.0 2 0 0 1\npolice2 95.5 b=23,82 s=149 d=2\n@civ_warnings 1\n@ach rookie\n").unwrap();
        let p = Progress::load_from(Some(path.clone()));
        let (a, b) = (p.get("police1").unwrap(), p.get("police2").unwrap());
        assert_eq!((a.best, a.legacy, a.medals), (600.0, [2, 0, 0, 1], Default::default()));
        assert_eq!((b.best, b.badges.len(), b.sides.len(), b.hardest), (95.5, 2, 1, uc_game::Difficulty::Hard));
        assert_eq!(b.medals, Default::default());
        assert_eq!((p.civ_warnings, p.achieved.len()), (1, 1));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// A file from before the counts added for #517 loads its stats and achievements as
    /// they were, the new counts at nought; saved again, the new counts go in with the old,
    /// and an unknown count (from a later version) is skipped.
    #[test]
    fn old_stats_load_and_new_ones_save() {
        let path = scratch("stats");
        std::fs::write(&path, "police1 600.0 b= s= d=1\n@stat arrests 12\n@stat longest_zip 21.5\n@stat perfect 1\n@stat from_the_future 3\n@ach rookie\n@ach high_wire\n").unwrap();
        let mut p = Progress::load_from(Some(path.clone()));
        let t = &p.totals;
        assert_eq!((t.arrests, t.longest_zip, t.perfect), (12, 21.5, 1));
        assert_eq!((t.stomps, t.nad_kicks, t.grenades, t.cars_stolen, t.marksman, t.boss_unhurt, t.quiet_shifts, t.furthest_wave, t.best_shift), (0, 0, 0, 0, 0, 0, 0, 0, 0));
        assert_eq!(p.achieved, ["high_wire".to_string(), "rookie".to_string()].into());
        // Those the old counts reach are there to be earned: Lawman's bronze is new.
        assert_eq!(crate::achievements::newly(&p, &p.totals).iter().map(|a| a.id).collect::<Vec<_>>(), ["thorough", "lawman_1"]);
        p.add_stats(&uc_game::game::stats::Stats { stomps: 3, furthest_wave: 11, best_shift: 2600, ..Default::default() });
        p.achieve("lawman_1");
        let q = Progress::load_from(Some(path.clone()));
        assert_eq!((q.totals.arrests, q.totals.stomps, q.totals.furthest_wave, q.totals.best_shift), (12, 3, 11, 2600));
        assert!(q.achieved.contains("lawman_1") && q.achieved.contains("rookie"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Beat Cop bests are kept per shift length, and a map label with spaces in is one key.
    #[test]
    fn shift_bests_are_kept_apart() {
        let mut p = Progress::default();
        use uc_game::game::beat::Shift;
        let (five, open) = (beat_key("West District: the park", Shift::Minutes(5)), beat_key("West District: the park", Shift::Open));
        p.record_endless(&five, 900, 4);
        p.record_endless(&open, 300, 2);
        assert_eq!(p.best_endless(&five), Some((900, 4)));
        assert_eq!(p.best_endless(&open), Some((300, 2)));
        assert!(!key(&five).contains(' '));
        assert_ne!(beat_key("Gangland", Shift::Unlimited), beat_key("Gangland", Shift::Open));
    }

    /// Every shift length goes into the progress file and comes back the same (#520).
    #[test]
    fn shift_lengths_round_trip() {
        for s in uc_game::game::beat::SHIFTS {
            assert_eq!(shift_named(&shift_code(s)), Some(s));
        }
    }

    /// A renamed arena keeps its records: the endless run's and each shift length's, not
    /// over one already kept under the new name.
    #[test]
    fn a_renamed_arena_keeps_its_records() {
        let mut p = Progress::default();
        let old = "Combat Tutorial: round the start (fight1)";
        p.record_endless(old, 500, 3);
        p.record_endless(&beat_key(old, uc_game::game::beat::Shift::Minutes(10)), 950, 4);
        p.record_endless(&beat_key("Gangland", uc_game::game::beat::Shift::Minutes(10)), 3450, 15);
        assert!(p.rename_endless(old, "Combat Centre"));
        assert_eq!(p.best_endless("Combat Centre"), Some((500, 3)));
        assert_eq!(p.best_endless(&beat_key("Combat Centre", uc_game::game::beat::Shift::Minutes(10))), Some((950, 4)));
        assert_eq!(p.best_endless(old), None);
        assert_eq!(p.best_endless(&beat_key("Gangland", uc_game::game::beat::Shift::Minutes(10))), Some((3450, 15)));
        p.record_endless("West Block", 100, 1);
        p.record_endless("Insane Assault: round the start (balbash1)", 900, 9);
        assert!(!p.rename_endless("Insane Assault: round the start (balbash1)", "West Block"));
        assert_eq!(p.best_endless("West Block"), Some((100, 1)));
    }
}
