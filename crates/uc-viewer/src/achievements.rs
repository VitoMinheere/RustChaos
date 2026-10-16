//! Achievements (#132): long-term goals over everything played, each a condition on the
//! saved progress and the stats (with the mission being played added in). Earned ones pop
//! up on screen and are kept in the progress file.

use crate::progress::Progress;
use uc_game::game::stats::Stats;

/// One achievement: how far along it is out of its goal.
pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub text: &'static str,
    pub goal: u32,
    have: fn(&Progress, &Stats) -> u32,
}

/// The training missions.
const TRAINING: [&str; 5] = ["ftutor1", "assault1", "testdrive1a", "fight1", "fight2"];

fn completed(p: &Progress, missions: &[&str]) -> u32 {
    missions.iter().filter(|m| p.completed(m)).count() as u32
}

fn on_hard(p: &Progress, missions: &[&str]) -> u32 {
    missions.iter().filter(|m| p.get(m).is_some_and(|d| d.hardest == uc_game::Difficulty::Hard)).count() as u32
}

pub const ALL: [Achievement; 10] = [
    Achievement { id: "rookie", name: "Rookie", text: "Finish the training", goal: TRAINING.len() as u32, have: |p, _| completed(p, &TRAINING) },
    Achievement { id: "by_the_book", name: "By the Book", text: "Win a mission with an arrest, no shots and no red marks", goal: 1, have: |_, s| s.by_the_book },
    Achievement { id: "clean_record", name: "Clean Record", text: "Finish the campaign without a red mark", goal: crate::menu::PLAYABLE.len() as u32,
        have: |p, s| if s.red_marks == 0 { completed(p, &crate::menu::PLAYABLE) } else { 0 } },
    Achievement { id: "silent_partner", name: "Silent Partner", text: "25 takedowns from behind", goal: 25, have: |_, s| s.takedowns },
    Achievement { id: "frisk", name: "Frisk", text: "Search 100 people", goal: 100, have: |_, s| s.searched },
    Achievement { id: "high_wire", name: "High Wire", text: "A zip-wire ride of 20 squares", goal: 20, have: |_, s| s.longest_zip as u32 },
    Achievement { id: "roof_runner", name: "Roof Runner", text: "Pull yourself up onto a roof", goal: 1, have: |_, s| s.roofs },
    Achievement { id: "riot_control", name: "Riot Control", text: "Reach wave 20 in endless mode", goal: 20, have: |p, _| p.best_wave() },
    Achievement { id: "thorough", name: "Thorough", text: "Win a mission with every badge and side objective done", goal: 1, have: |_, s| s.perfect },
    Achievement { id: "hard_boiled", name: "Hard Boiled", text: "Finish the campaign on Hard", goal: crate::menu::PLAYABLE.len() as u32, have: |p, _| on_hard(p, &crate::menu::PLAYABLE) },
];

impl Achievement {
    /// How far along: (so far, out of), with `stats` the totals (and the mission on).
    pub fn progress(&self, p: &Progress, stats: &Stats) -> (u32, u32) {
        ((self.have)(p, stats).min(self.goal), self.goal)
    }

    pub fn earned(&self, p: &Progress, stats: &Stats) -> bool {
        p.achieved.contains(self.id) || (self.have)(p, stats) >= self.goal
    }
}

/// The achievements earned with `stats` that `p` hasn't noted yet.
pub fn newly(p: &Progress, stats: &Stats) -> Vec<&'static Achievement> {
    ALL.iter().filter(|a| !p.achieved.contains(a.id) && (a.have)(p, stats) >= a.goal).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earned_once_the_stats_get_there() {
        let p = Progress::default();
        let mut s = Stats::default();
        assert!(newly(&p, &s).is_empty());
        s.takedowns = 24;
        s.longest_zip = 21.5;
        let ids: Vec<_> = newly(&p, &s).iter().map(|a| a.id).collect();
        assert_eq!(ids, ["high_wire"]);
        s.takedowns = 25;
        assert!(newly(&p, &s).iter().any(|a| a.id == "silent_partner"));
        let silent = ALL.iter().find(|a| a.id == "silent_partner").unwrap();
        assert_eq!(silent.progress(&p, &Stats { takedowns: 40, ..Default::default() }), (25, 25));
    }

    #[test]
    fn noted_ones_are_not_new_again() {
        let mut p = Progress::default();
        let s = Stats { roofs: 1, ..Default::default() };
        assert_eq!(newly(&p, &s).len(), 1);
        p.achieved.insert("roof_runner".into());
        assert!(newly(&p, &s).is_empty());
    }
}
