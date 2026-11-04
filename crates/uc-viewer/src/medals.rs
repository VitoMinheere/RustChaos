//! Medals per story mission (#516), a reason to play the campaign again. Each mission has
//! three goals:
//!
//! - **Time**, tiered from the mission's Mucky Time ([`par`]): bronze for a win within 3x it,
//!   silver within 2x, gold within 1.25x, and gold with the "Mucky" mark for beating it (as
//!   the original's win screen gave a code for, `ScoresDraw`, fallen/Source/Attract.cpp:684).
//!   The training missions have no Mucky Time, so no time goal.
//! - **Clean**: no red marks (`Genus.Player->RedMarks`: innocents killed). The game doesn't
//!   count civilians hurt but not killed, so red marks alone decide it.
//! - **Thorough**: every badge and side objective in the mission done, as the `perfect` stat
//!   counts it (across winning runs). A mission with none has no such goal.
//!
//! The best of each is kept with the mission's progress. None are given for a run the
//! console has changed (as the Mucky code isn't), nor for Beat Cop, endless runs, replays or
//! edited missions.

use crate::par;
use crate::progress::clock;
use uc_render::Overlay;

/// The time goal's medals, worst to best.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    #[default]
    None,
    Bronze,
    Silver,
    Gold,
    /// The Mucky Time beaten: gold, and shown specially.
    Mucky,
}

/// Each tier's limit as a multiple of the Mucky Time, bronze to gold.
const LIMITS: [(Tier, f32); 3] = [(Tier::Bronze, 3.0), (Tier::Silver, 2.0), (Tier::Gold, 1.25)];

impl Tier {
    const ALL: [Tier; 5] = [Tier::None, Tier::Bronze, Tier::Silver, Tier::Gold, Tier::Mucky];

    /// The medal won for `mission` in `secs`; `None` if it has no Mucky Time.
    pub fn of(mission: &str, secs: f32) -> Tier {
        let Some(mucky) = par::mucky_secs(mission) else { return Tier::None };
        if par::beaten(secs, mucky) {
            return Tier::Mucky;
        }
        LIMITS.iter().rev().find(|&&(_, x)| secs <= mucky as f32 * x).map_or(Tier::None, |&(t, _)| t)
    }

    pub fn name(self) -> &'static str {
        match self {
            Tier::None => "no medal",
            Tier::Bronze => "Bronze",
            Tier::Silver => "Silver",
            Tier::Gold => "Gold",
            Tier::Mucky => "Gold + Mucky",
        }
    }

    pub fn colour(self) -> [u8; 4] {
        match self {
            Tier::None => DIM,
            Tier::Bronze => [205, 127, 50, 255],
            Tier::Silver => [205, 210, 220, 255],
            Tier::Gold => GOLD,
            Tier::Mucky => MUCKY,
        }
    }

    fn code(self) -> u8 {
        Tier::ALL.iter().position(|&t| t == self).unwrap_or(0) as u8
    }

    fn from_code(c: u8) -> Tier {
        Tier::ALL.get(c as usize).copied().unwrap_or_default()
    }
}

const DIM: [u8; 4] = [90, 90, 88, 255];
const GREY: [u8; 4] = [170, 170, 160, 255];
const GOLD: [u8; 4] = [255, 210, 70, 255];
/// The Mucky mark: Mucky Foot's green.
const MUCKY: [u8; 4] = [120, 235, 120, 255];

/// A mission's medals: the time tier, and the clean and thorough goals met.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Medals {
    pub time: Tier,
    pub clean: bool,
    pub thorough: bool,
}

impl Medals {
    /// The best of each goal of the two.
    pub fn best(self, other: Medals) -> Medals {
        Medals { time: self.time.max(other.time), clean: self.clean || other.clean, thorough: self.thorough || other.thorough }
    }

    /// Which goals are better here than in `before`: (time, clean, thorough).
    pub fn improved(self, before: Medals) -> (bool, bool, bool) {
        (self.time > before.time, self.clean && !before.clean, self.thorough && !before.thorough)
    }

    /// For the progress file: `<tier><clean><thorough>`, such as `410`.
    pub fn encode(self) -> String {
        format!("{}{}{}", self.time.code(), self.clean as u8, self.thorough as u8)
    }

    /// From [`Medals::encode`]'s; `None` if it isn't one.
    pub fn decode(s: &str) -> Option<Medals> {
        let d: Vec<u8> = s.bytes().map(|b| b.wrapping_sub(b'0')).collect();
        match d[..] {
            [t, c, p] if t < 5 && c < 2 && p < 2 => Some(Medals { time: Tier::from_code(t), clean: c == 1, thorough: p == 1 }),
            _ => None,
        }
    }
}

/// Medals are given for `mission`: one of the story's (those with a place in the Mucky Time
/// table).
pub fn story(mission: &str) -> bool {
    par::listed(mission)
}

/// The briefing's time targets, a win under each earning its medal: "Bronze 3:15", "Silver
/// 2:10", "Gold 1:21", "Mucky 1:05" (`short`: "B 3:15" and so on). Those already won are in
/// their colours, the rest grey. None for a mission without a Mucky Time.
pub fn targets(mission: &str, have: Tier, short: bool) -> Vec<(String, [u8; 4])> {
    let Some(mucky) = par::mucky_secs(mission) else { return Vec::new() };
    let tiers = LIMITS.iter().map(|&(t, x)| (t, mucky as f32 * x)).chain([(Tier::Mucky, mucky as f32)]);
    tiers.map(|(t, secs)| {
        let name = if t == Tier::Mucky { "Mucky" } else { t.name() };
        let name = if short { &name[..1] } else { name };
        (format!("{name} {}", clock(secs)), if have >= t { t.colour() } else { GREY })
    }).collect()
}

/// The briefing's other goals: "Clean: no red marks" and (with badges or side objectives to
/// do) "Thorough: 100%", gold once won.
pub fn goals_line(m: Medals, thorough_goal: bool) -> Vec<(String, [u8; 4])> {
    let col = |on: bool| if on { GOLD } else { GREY };
    let mut out = vec![("Clean: no red marks".to_string(), col(m.clean))];
    if thorough_goal {
        out.push(("Thorough: 100%".to_string(), col(m.thorough)));
    }
    out
}

/// Small markers for a mission's medals, at `x`, `y` (a text row's top): a dot for time, clean
/// and thorough, coloured by the medal (dim when not won), the time's ringed in green for
/// Mucky. `thorough_goal`: the mission has badges or side objectives. The width they take.
pub fn dots(o: &mut Overlay, x: f32, y: f32, s: f32, mission: &str, m: Medals, thorough_goal: bool) -> f32 {
    let time = par::mucky_secs(mission).is_some().then_some(m.time.colour());
    let met = |on: bool| if on { GOLD } else { DIM };
    let marks = [time, Some(met(m.clean)), thorough_goal.then(|| met(m.thorough))];
    let mut at = x;
    for (k, col) in marks.into_iter().enumerate() {
        let Some(col) = col else { continue };
        let ring = if k == 0 && m.time == Tier::Mucky { MUCKY } else { [0, 0, 0, 220] };
        o.rect(at, y + 1.0 * s, 6.0 * s, 6.0 * s, ring);
        o.rect(at + s, y + 2.0 * s, 4.0 * s, 4.0 * s, if k == 0 && m.time == Tier::Mucky { GOLD } else { col });
        at += 7.0 * s;
    }
    at - x
}

/// The win screen's line: each goal's medal, with "NEW" on those better than `before`.
pub fn won_line(mission: &str, now: Medals, before: Medals, thorough_goal: bool) -> Vec<(String, [u8; 4])> {
    let (t, c, p) = now.improved(before);
    let new = |b: bool| if b { " NEW" } else { "" };
    let yes = |on: bool| if on { ("yes", GOLD) } else { ("no", DIM) };
    let mut out = vec![("Medals:".to_string(), [235, 235, 225, 255])];
    if par::mucky_secs(mission).is_some() {
        out.push((format!("Time {}{}", now.time.name(), new(t)), now.time.colour()));
    }
    let (word, col) = yes(now.clean);
    out.push((format!("Clean {word}{}", new(c)), col));
    if thorough_goal {
        let (word, col) = yes(now.thorough);
        out.push((format!("Thorough {word}{}", new(p)), col));
    }
    out
}

/// Draws coloured pieces of text one after another, three spaces apart, from `x`.
pub fn draw_line(o: &mut Overlay, x: f32, y: f32, s: f32, parts: &[(String, [u8; 4])]) {
    let gap = Overlay::text_width("   ", s);
    let mut at = x;
    for (text, col) in parts {
        at += o.text(at, y, s, *col, text) + gap;
    }
}

/// How wide the pieces are when drawn with [`draw_line`].
pub fn line_width(parts: &[(String, [u8; 4])], s: f32) -> f32 {
    let gap = Overlay::text_width("   ", s);
    (parts.iter().map(|(t, _)| Overlay::text_width(t, s) + gap).sum::<f32>() - gap).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Police1 (Alex Hood's 1:05): within 3x bronze, 2x silver, 1.25x gold, under it Mucky.
    #[test]
    fn time_tiers() {
        assert_eq!(Tier::of("police1", 64.0), Tier::Mucky);
        assert_eq!(Tier::of("police1", 65.5), Tier::Gold, "65 isn't under 65");
        assert_eq!(Tier::of("police1", 81.25), Tier::Gold);
        assert_eq!(Tier::of("police1", 82.0), Tier::Silver);
        assert_eq!(Tier::of("police1", 130.0), Tier::Silver);
        assert_eq!(Tier::of("police1", 131.0), Tier::Bronze);
        assert_eq!(Tier::of("POLICE1", 195.0), Tier::Bronze);
        assert_eq!(Tier::of("police1", 196.0), Tier::None);
        // Training: no Mucky Time, no time medal.
        assert_eq!(Tier::of("ftutor1", 1.0), Tier::None);
        assert_eq!(Tier::of("jung3", 1.0), Tier::None);
    }

    #[test]
    fn story_missions() {
        assert!(story("Police1") && story("ftutor1"));
        assert!(!story("jung3") && !story("arena"));
    }

    #[test]
    fn best_and_improved() {
        let a = Medals { time: Tier::Silver, clean: true, thorough: false };
        let b = Medals { time: Tier::Gold, clean: false, thorough: false };
        assert_eq!(a.best(b), Medals { time: Tier::Gold, clean: true, thorough: false });
        assert_eq!(b.improved(a), (true, false, false));
        assert_eq!(a.improved(Medals::default()), (true, true, false));
    }

    #[test]
    fn encode_round_trips() {
        for t in Tier::ALL {
            for (clean, thorough) in [(false, false), (true, false), (false, true), (true, true)] {
                let m = Medals { time: t, clean, thorough };
                assert_eq!(Medals::decode(&m.encode()), Some(m));
            }
        }
        assert_eq!(Medals { time: Tier::Mucky, clean: true, thorough: false }.encode(), "410");
        for bad in ["", "41", "4100", "510", "a10", "120"] {
            assert_eq!(Medals::decode(bad), None, "{bad}");
        }
    }

    #[test]
    fn briefing_targets() {
        // Gun Hunt: 1:37. Silver won: bronze and silver in colour, the rest grey.
        let t = targets("police3", Tier::Silver, false);
        let names: Vec<&str> = t.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(names, ["Bronze 4:51", "Silver 3:14", "Gold 2:01", "Mucky 1:37"]);
        assert_eq!(t.iter().map(|(_, c)| *c).collect::<Vec<_>>(), [Tier::Bronze.colour(), Tier::Silver.colour(), GREY, GREY]);
        let short: Vec<String> = targets("police3", Tier::None, true).into_iter().map(|(t, _)| t).collect();
        assert_eq!(short, ["B 4:51", "S 3:14", "G 2:01", "M 1:37"]);
        assert!(targets("fight1", Tier::None, false).is_empty());
        assert_eq!(goals_line(Medals { clean: true, ..Default::default() }, false), [("Clean: no red marks".to_string(), GOLD)]);
    }

    #[test]
    fn the_win_line_marks_whats_new() {
        let now = Medals { time: Tier::Gold, clean: true, thorough: true };
        let before = Medals { time: Tier::Gold, clean: false, thorough: true };
        let t: Vec<String> = won_line("police1", now, before, true).into_iter().map(|(t, _)| t).collect();
        assert_eq!(t, ["Medals:", "Time Gold", "Clean yes NEW", "Thorough yes"]);
        let t: Vec<String> = won_line("fight1", Medals::default(), Medals::default(), false).into_iter().map(|(t, _)| t).collect();
        assert_eq!(t, ["Medals:", "Clean no"]);
    }
}
