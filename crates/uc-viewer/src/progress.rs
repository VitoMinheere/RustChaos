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
    path: Option<PathBuf>,
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
                    } else if let (Ok(n), true) = (field.parse(), legacy < 4) {
                        d.legacy[legacy] = n;
                        legacy += 1;
                    }
                }
                Some((mission, d))
            })
            .collect();
        Progress { done, civ_warnings, endless, path }
    }

    /// Another mission ended with too many civilian casualties: the chief's warnings before
    /// this one (Game.cpp:2783). The fourth costs her badge.
    pub fn warn(&mut self) -> u8 {
        let before = self.civ_warnings;
        self.civ_warnings = before.saturating_add(1);
        self.save();
        before
    }

    /// The best endless run on `map`: (score, wave).
    pub fn best_endless(&self, map: &str) -> Option<(i64, u32)> {
        self.endless.get(&map.to_ascii_lowercase()).copied()
    }

    /// Records an endless run on `map`; whether it's a new best.
    pub fn record_endless(&mut self, map: &str, score: i64, wave: u32) -> bool {
        let e = self.endless.entry(map.to_ascii_lowercase()).or_insert((i64::MIN, 0));
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
    pub fn record(&mut self, mission: &str, secs: f32, badges: &BTreeSet<usize>, sides: &BTreeSet<usize>) {
        let d = self.done.entry(mission.to_ascii_lowercase()).or_insert(Done { best: secs, ..Default::default() });
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
                format!("{m} {:.1} b={} s={}{legacy}\n", d.best, join(&d.badges), join(&d.sides))
            })
            .chain((self.civ_warnings > 0).then(|| format!("@civ_warnings {}\n", self.civ_warnings)))
            .chain(self.endless.iter().map(|(m, (s, w))| format!("@endless {m} {s} {w}\n")))
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
        p.record("police1", 500.0, &found, &[149].into());
        p.record("police1", 550.0, &found, &BTreeSet::new());
        let d = p.get("police1").unwrap();
        assert_eq!((d.best, d.badges.len(), d.sides.len()), (500.0, 2, 1));
        let s = p.stats();
        assert!(s[0] >= 2 && s[3] >= 1 && s.iter().map(|&n| n as usize).sum::<usize>() <= 5);
    }
}
