//! Which missions have been completed: the best time for each, and the badges and side
//! objectives done in it (across all the runs that were won). Kept in a small text file in the
//! user's settings folder, a line per mission: `mission seconds b=<badge points> s=<side
//! objectives>`. (Older files have four stat counts in place of the lists.)

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
    /// The Beat Cop shift length chosen in the menu, in minutes (`None` open-ended).
    shift: Option<u32>,
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
pub fn beat_key(label: &str, minutes: Option<u32>) -> String {
    format!("beat{}:{label}", minutes.map_or("open".to_string(), |m| m.to_string()))
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

    pub fn load() -> Progress {
        let path = Progress::default_path();
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
        let shift = text.lines().find_map(|l| l.strip_prefix("@shift ")?.trim().parse::<u32>().ok())
            .map_or(uc_game::game::beat::DEFAULT_SHIFT, |m| (m > 0).then_some(m));
        let mut stats = uc_game::game::stats::Stats::default();
        for l in text.lines().filter_map(|l| l.strip_prefix("@stat ")) {
            let mut f = l.split_whitespace();
            if let (Some(name), Some(v)) = (f.next(), f.next().and_then(|v| v.parse().ok())) {
                stats.set(name, v);
            }
        }
        let achieved = text.lines().filter_map(|l| Some(l.strip_prefix("@ach ")?.trim().to_string())).collect();
        Progress { done, civ_warnings, endless, difficulty, shift, totals: stats, achieved, path }
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

    pub fn shift(&self) -> Option<u32> {
        self.shift
    }

    pub fn set_shift(&mut self, minutes: Option<u32>) {
        self.shift = minutes;
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
                format!("{m} {:.1} b={} s={} d={on}{legacy}\n", d.best, join(&d.badges), join(&d.sides))
            })
            .chain((self.civ_warnings > 0).then(|| format!("@civ_warnings {}\n", self.civ_warnings)))
            .chain(self.endless.iter().map(|(m, (s, w))| format!("@endless {m} {s} {w}\n")))
            .chain((self.shift != uc_game::game::beat::DEFAULT_SHIFT).then(|| format!("@shift {}\n", self.shift.unwrap_or(0))))
            .chain(self.totals.fields().into_iter().filter(|(_, v)| *v != 0.0).map(|(n, v)| format!("@stat {n} {v}\n")))
            .chain(self.achieved.iter().map(|a| format!("@ach {a}\n")))
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

    /// Beat Cop bests are kept per shift length, and a map label with spaces in is one key.
    #[test]
    fn shift_bests_are_kept_apart() {
        let mut p = Progress::default();
        let (five, open) = (beat_key("West District: the park", Some(5)), beat_key("West District: the park", None));
        p.record_endless(&five, 900, 4);
        p.record_endless(&open, 300, 2);
        assert_eq!(p.best_endless(&five), Some((900, 4)));
        assert_eq!(p.best_endless(&open), Some((300, 2)));
        assert!(!key(&five).contains(' '));
    }
}
