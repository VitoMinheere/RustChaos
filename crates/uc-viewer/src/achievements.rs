//! Achievements (#132): long-term goals over everything played, each a condition on the
//! saved progress and the stats (with the mission being played added in). Earned ones pop
//! up on screen and are kept in the progress file. Not in the original.
//!
//! Some come in bronze, silver and gold (#517): three entries in [`ALL`], one after the
//! other, with the same name and rising goals.

use crate::medals::Tier;
use crate::progress::Progress;
use uc_game::game::stats::Stats;

/// One achievement: how far along it is out of its goal.
pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub text: &'static str,
    pub goal: u32,
    /// Bronze, silver or gold of a tiered one; `Tier::None` for one on its own.
    pub tier: Tier,
    have: fn(&Progress, &Stats) -> u32,
}

type Have = fn(&Progress, &Stats) -> u32;

/// One on its own.
const fn one(id: &'static str, name: &'static str, text: &'static str, goal: u32, have: Have) -> Achievement {
    Achievement { id, name, text, goal, tier: Tier::None, have }
}

/// One tier of a tiered one.
const fn tier(id: &'static str, name: &'static str, tier: Tier, text: &'static str, goal: u32, have: Have) -> Achievement {
    Achievement { id, name, text, goal, tier, have }
}

/// The training missions.
const TRAINING: [&str; 5] = ["ftutor1", "assault1", "testdrive1a", "fight1", "fight2"];

/// The playable missions with a Mucky Time, so with a time medal to win.
const TIMED: u32 = 27;

fn completed(p: &Progress, missions: &[&str]) -> u32 {
    missions.iter().filter(|m| p.completed(m)).count() as u32
}

fn on_hard(p: &Progress, missions: &[&str]) -> u32 {
    missions.iter().filter(|m| p.get(m).is_some_and(|d| d.hardest == uc_game::Difficulty::Hard)).count() as u32
}

/// The playable missions won with a gold time medal (or the Mucky mark).
fn gold(p: &Progress, _: &Stats) -> u32 {
    crate::menu::PLAYABLE.iter().filter(|m| p.get(m).is_some_and(|d| d.medals.time >= Tier::Gold)).count() as u32
}

/// The furthest endless wave: the saved best, or this run's.
fn wave(p: &Progress, s: &Stats) -> u32 {
    p.best_wave().max(s.furthest_wave)
}

/// The best Beat Cop score: the saved best, or this shift's.
fn shift(p: &Progress, s: &Stats) -> u32 {
    (p.best_shift().unwrap_or(0).max(0) as u32).max(s.best_shift)
}

use Tier::{Bronze, Gold, Silver};

pub static ALL: [Achievement; 41] = [
    one("rookie", "Rookie", "Finish the training", TRAINING.len() as u32, |p, _| completed(p, &TRAINING)),
    one("by_the_book", "By the Book", "Win a mission with an arrest, no shots and no red marks", 1, |_, s| s.by_the_book),
    one("clean_record", "Clean Record", "Finish the campaign without a red mark", crate::menu::PLAYABLE.len() as u32,
        |p, s| if s.red_marks == 0 { completed(p, &crate::menu::PLAYABLE) } else { 0 }),
    one("silent_partner", "Silent Partner", "25 takedowns from behind", 25, |_, s| s.takedowns),
    one("frisk", "Frisk", "Search 100 people", 100, |_, s| s.searched),
    one("high_wire", "High Wire", "A zip-wire ride of 20 squares", 20, |_, s| s.longest_zip as u32),
    one("roof_runner", "Roof Runner", "Pull yourself up onto a roof", 1, |_, s| s.roofs),
    one("riot_control", "Riot Control", "Reach wave 20 in endless mode", 20, wave),
    one("thorough", "Thorough", "Win a mission with every badge and side objective done", 1, |_, s| s.perfect),
    one("hard_boiled", "Hard Boiled", "Finish the campaign on Hard", crate::menu::PLAYABLE.len() as u32, |p, _| on_hard(p, &crate::menu::PLAYABLE)),
    // From the counts already kept (#517).
    tier("lawman_1", "Lawman", Bronze, "Make 10 arrests", 10, |_, s| s.arrests),
    tier("lawman_2", "Lawman", Silver, "Make 50 arrests", 50, |_, s| s.arrests),
    tier("lawman_3", "Lawman", Gold, "Make 100 arrests", 100, |_, s| s.arrests),
    one("bare_knuckles", "Bare Knuckles", "Knock out 50 people with fists and feet", 50, |_, s| s.ko_fists),
    one("blunt_instrument", "Blunt Instrument", "Knock out 25 people with a knife or bat", 25, |_, s| s.ko_weapons),
    one("sharpshooter", "Sharpshooter", "Win a mission with 20 shots or more, 3 in 4 of them hits", 1, |_, s| s.marksman),
    one("wheelman", "Wheelman", "Drive 100 squares", 100, |_, s| s.driven as u32),
    one("easy_rider", "Easy Rider", "Ride 50 squares on the bike", 50, |_, s| s.biked as u32),
    one("shoe_leather", "Shoe Leather", "Go 1000 squares on foot", 1000, |_, s| s.on_foot as u32),
    one("ladder_rat", "Ladder Rat", "Climb 50 ladders", 50, |_, s| s.ladders),
    one("bounce_back", "Bounce Back", "Get up from 10 falls of two storeys or more", 10, |_, s| s.falls),
    tier("last_stand_1", "Last Stand", Bronze, "Reach wave 10 in endless mode", 10, wave),
    tier("last_stand_2", "Last Stand", Silver, "Reach wave 30 in endless mode", 30, wave),
    tier("last_stand_3", "Last Stand", Gold, "Reach wave 50 in endless mode", 50, wave),
    one("untouchable", "Untouchable", "Clear an endless boss wave without losing any health", 1, |_, s| s.boss_unhurt),
    tier("on_the_beat_1", "On the Beat", Bronze, "Score 2500 in a Beat Cop shift", 2500, shift),
    tier("on_the_beat_2", "On the Beat", Silver, "Score 5000 in a Beat Cop shift", 5000, shift),
    tier("on_the_beat_3", "On the Beat", Gold, "Score 10000 in a Beat Cop shift", 10000, shift),
    one("quiet_streets", "Quiet Streets", "Finish a 20-minute shift with the crime rate under 20", 1, |_, s| s.quiet_shifts),
    // From counts added for them.
    tier("boot_party_1", "Boot Party", Bronze, "Stamp on 10 people lying down", 10, |_, s| s.stomps),
    tier("boot_party_2", "Boot Party", Silver, "Stamp on 50 people lying down", 50, |_, s| s.stomps),
    tier("boot_party_3", "Boot Party", Gold, "Stamp on 100 people lying down", 100, |_, s| s.stomps),
    one("low_blow", "Low Blow", "Double 10 people over with a kick between the legs", 10, |_, s| s.nad_kicks),
    tier("fire_in_the_hole_1", "Fire in the Hole", Bronze, "Throw 5 grenades", 5, |_, s| s.grenades),
    tier("fire_in_the_hole_2", "Fire in the Hole", Silver, "Throw 25 grenades", 25, |_, s| s.grenades),
    tier("fire_in_the_hole_3", "Fire in the Hole", Gold, "Throw 100 grenades", 100, |_, s| s.grenades),
    tier("joyrider_1", "Joyrider", Bronze, "Steal 5 cars", 5, |_, s| s.cars_stolen),
    tier("joyrider_2", "Joyrider", Silver, "Steal 25 cars", 25, |_, s| s.cars_stolen),
    tier("joyrider_3", "Joyrider", Gold, "Steal 100 cars", 100, |_, s| s.cars_stolen),
    // From the medals (#516).
    one("gold_rush", "Gold Rush", "Win a gold time medal on 10 missions", 10, gold),
    one("solid_gold", "Solid Gold", "Win a gold time medal on every mission that has one", TIMED, gold),
];

impl Achievement {
    /// How far along: (so far, out of), with `stats` the totals (and the mission on).
    pub fn progress(&self, p: &Progress, stats: &Stats) -> (u32, u32) {
        ((self.have)(p, stats).min(self.goal), self.goal)
    }

    pub fn earned(&self, p: &Progress, stats: &Stats) -> bool {
        p.achieved.contains(self.id) || (self.have)(p, stats) >= self.goal
    }

    /// The name, with the tier if it has one: "Lawman (Silver)".
    pub fn title(&self) -> String {
        match self.tier {
            Tier::None => self.name.to_string(),
            t => format!("{} ({})", self.name, t.name()),
        }
    }
}

/// The achievements earned with `stats` that `p` hasn't noted yet.
pub fn newly(p: &Progress, stats: &Stats) -> Vec<&'static Achievement> {
    ALL.iter().filter(|a| !p.achieved.contains(a.id) && (a.have)(p, stats) >= a.goal).collect()
}

/// [`ALL`] in groups, for the menu: a tiered one's tiers together, bronze first; the rest
/// one to a group.
pub fn groups() -> Vec<&'static [Achievement]> {
    let mut out: Vec<&'static [Achievement]> = Vec::new();
    let mut from = 0;
    for k in 1..=ALL.len() {
        if k == ALL.len() || ALL[k].tier == Tier::None || ALL[k].name != ALL[from].name {
            out.push(&ALL[from..k]);
            from = k;
        }
    }
    out
}

/// A group's tier to show: the first not earned yet, or the last (all earned).
pub fn showing<'a>(group: &'a [Achievement], p: &Progress, stats: &Stats) -> &'a Achievement {
    group.iter().find(|a| !a.earned(p, stats)).unwrap_or(&group[group.len() - 1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn ids(p: &Progress, s: &Stats) -> Vec<&'static str> {
        newly(p, s).iter().map(|a| a.id).collect()
    }

    #[test]
    fn earned_once_the_stats_get_there() {
        let p = Progress::default();
        let mut s = Stats::default();
        assert!(newly(&p, &s).is_empty());
        s.takedowns = 24;
        s.longest_zip = 21.5;
        assert_eq!(ids(&p, &s), ["high_wire"]);
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

    /// Ids are unique, and a tiered one is three together, bronze to gold, the goals rising.
    #[test]
    fn tiers_go_bronze_to_gold() {
        let unique: BTreeSet<_> = ALL.iter().map(|a| a.id).collect();
        assert_eq!(unique.len(), ALL.len());
        let groups = groups();
        assert_eq!(groups.iter().map(|g| g.len()).sum::<usize>(), ALL.len());
        for g in &groups {
            if g[0].tier == Tier::None {
                assert_eq!(g.len(), 1, "{}", g[0].id);
                continue;
            }
            assert_eq!(g.iter().map(|a| a.tier).collect::<Vec<_>>(), [Tier::Bronze, Tier::Silver, Tier::Gold], "{}", g[0].name);
            assert!(g.windows(2).all(|w| w[0].goal < w[1].goal && w[0].name == w[1].name), "{}", g[0].name);
        }
        assert_eq!(groups.iter().filter(|g| g.len() == 3).count(), 6);
    }

    /// Each tier comes as its count passes its goal, and the menu shows the next one.
    #[test]
    fn arrests_earn_each_tier_in_turn() {
        let mut p = Progress::default();
        let lawman = groups().into_iter().find(|g| g[0].name == "Lawman").unwrap();
        let s = Stats { arrests: 9, ..Default::default() };
        assert!(ids(&p, &s).is_empty());
        assert_eq!(showing(lawman, &p, &s).progress(&p, &s), (9, 10));
        let s = Stats { arrests: 55, ..Default::default() };
        assert_eq!(ids(&p, &s), ["lawman_1", "lawman_2"]);
        p.achieved.extend(["lawman_1".to_string(), "lawman_2".to_string()]);
        let shown = showing(lawman, &p, &s);
        assert_eq!((shown.title().as_str(), shown.progress(&p, &s)), ("Lawman (Gold)", (55, 100)));
        let s = Stats { arrests: 100, ..Default::default() };
        assert_eq!(ids(&p, &s), ["lawman_3"]);
        p.achieved.insert("lawman_3".into());
        assert_eq!(showing(lawman, &p, &s).id, "lawman_3");
    }

    /// The new counts each earn theirs.
    #[test]
    fn the_new_counts_earn_theirs() {
        let p = Progress::default();
        let s = Stats { stomps: 10, nad_kicks: 10, grenades: 5, cars_stolen: 5, marksman: 1, boss_unhurt: 1, quiet_shifts: 1, ..Default::default() };
        let got = ids(&p, &s);
        for id in ["boot_party_1", "low_blow", "fire_in_the_hole_1", "joyrider_1", "sharpshooter", "untouchable", "quiet_streets"] {
            assert!(got.contains(&id), "{id} in {got:?}");
        }
        assert!(!got.contains(&"boot_party_2"));
        let s = Stats { ko_fists: 50, ko_weapons: 25, driven: 100.5, biked: 50.0, on_foot: 1000.0, ladders: 50, falls: 10, ..Default::default() };
        assert_eq!(ids(&p, &s), ["bare_knuckles", "blunt_instrument", "wheelman", "easy_rider", "shoe_leather", "ladder_rat", "bounce_back"]);
    }

    /// An endless wave or a shift's score counts as soon as it's reached, before the run's
    /// over and saved.
    #[test]
    fn waves_and_shifts_count_mid_run() {
        let p = Progress::default();
        let s = Stats { furthest_wave: 20, best_shift: 5000, ..Default::default() };
        assert_eq!(ids(&p, &s), ["riot_control", "last_stand_1", "on_the_beat_1", "on_the_beat_2"]);
        // And from the saved bests, with nothing in the stats.
        let mut p = Progress::default();
        p.record_endless("Gangland", 900, 31);
        p.record_endless(&crate::progress::beat_key("Gangland", uc_game::game::beat::Shift::Minutes(20)), 10_500, 40);
        assert_eq!(ids(&p, &Stats::default()), ["riot_control", "last_stand_1", "last_stand_2", "on_the_beat_1", "on_the_beat_2", "on_the_beat_3"]);
    }

    /// Gold time medals (the Mucky mark too) count; silver doesn't.
    #[test]
    fn gold_medals_count() {
        use crate::medals::Medals;
        let mut p = Progress::default();
        let none = BTreeSet::new();
        let missions = ["police1", "police2", "police3", "Bankbomb1", "gangorder2", "police4", "bball2", "wstores1", "e3", "westcrime1", "carbomb1"];
        for (k, m) in missions.iter().enumerate() {
            p.record(m, 60.0, &none, &none, uc_game::Difficulty::Normal);
            let time = match k {
                0 => Tier::Silver,
                1 => Tier::Mucky,
                _ => Tier::Gold,
            };
            p.record_medals(m, Medals { time, ..Default::default() });
        }
        let rush = ALL.iter().find(|a| a.id == "gold_rush").unwrap();
        assert_eq!(rush.progress(&p, &Stats::default()), (10, 10));
        let all = ALL.iter().find(|a| a.id == "solid_gold").unwrap();
        assert_eq!(all.progress(&p, &Stats::default()), (10, TIMED));
    }

    /// Solid Gold's goal is every playable mission with a Mucky Time.
    #[test]
    fn every_timed_mission() {
        let timed = crate::menu::PLAYABLE.iter().filter(|m| crate::par::mucky_secs(m).is_some()).count();
        assert_eq!(timed as u32, TIMED);
    }
}
