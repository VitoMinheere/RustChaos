//! Ghost races (#186, #545): on the training missions against the clock and on every story
//! mission, the best fair run plays alongside as a see-through Darci (or whoever the
//! mission's player is), in a car or on a bike when she was. The ghost is kept per mission
//! and difficulty in `ghosts\` next to the progress file, and replaced when a run beats it.
//!
//! A run is timed at its splits: each place the script was waiting for someone to get to,
//! and each objective done (bonus points given, a side objective's reward made). Each shows
//! for a few seconds with how far ahead or behind the ghost it was; the objectives' are in
//! the Tab checklist and on the win screen, with the best time each stretch between two
//! splits has ever been done in (`<mission>-<difficulty>.splits` beside the ghost), whose
//! sum is the time a run of all the best stretches would have.

use crate::*;
use uc_game::game::ghost::Track;

/// The training missions raced against a ghost: the assault course and the three driving
/// tests.
const MISSIONS: [&str; 4] = ["assault1", "testdrive1a", "testdrive2", "testdrive3"];

/// Seconds a split stays on screen.
pub(crate) const SPLIT_SHOWN: f32 = 3.0;

/// Ahead of the ghost, and behind it.
pub(crate) const AHEAD: [u8; 4] = [110, 230, 110, 255];
pub(crate) const BEHIND: [u8; 4] = [240, 90, 70, 255];

/// The colour of a split `d` seconds behind (ahead if negative).
pub(crate) fn split_colour(d: f32) -> [u8; 4] {
    if d <= 0.0 { AHEAD } else { BEHIND }
}

/// Whether `mission` is raced against a ghost: those four, and every story mission (#545).
pub(crate) fn raced(mission: &str) -> bool {
    MISSIONS.iter().any(|m| m.eq_ignore_ascii_case(mission)) || medals::story(mission)
}

/// The file called `name` in `ghosts\`.
fn file(name: &str) -> Option<PathBuf> {
    Some(progress::Progress::folder()?.join("ghosts").join(name))
}

fn stem(mission: &str, d: uc_game::Difficulty) -> String {
    format!("{}-{}", mission.to_ascii_lowercase(), d.name().to_ascii_lowercase())
}

fn path(mission: &str, d: uc_game::Difficulty) -> Option<PathBuf> {
    file(&format!("{}.ucg", stem(mission, d)))
}

/// The best run's ghost on `mission` at difficulty `d`, if there is one.
pub(crate) fn load(mission: &str, d: uc_game::Difficulty) -> Option<Track> {
    Track::from_bytes(&std::fs::read(path(mission, d)?).ok()?)
}

pub(crate) fn save(mission: &str, d: uc_game::Difficulty, track: &Track) {
    write(path(mission, d), &track.to_bytes());
}

fn write(path: Option<PathBuf>, bytes: &[u8]) {
    let Some(path) = path else { return };
    let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, bytes));
    if let Err(e) = saved {
        eprintln!("couldn't save {}: {e}", path.display());
    }
}

/// A split: `+1.24` behind, `-0.80` ahead.
pub(crate) fn split_text(secs: f32) -> String {
    format!("{secs:+.2}")
}

/// Where a stretch of a run ends that ends at the win.
const FINISH: u32 = u32::MAX;

/// A stretch of a run between two splits: the point it starts from (0 for the start of the
/// mission), the point it ends at ([`FINISH`] for the win), and which pass of that one.
type Stretch = (u32, u32, u32);

/// The best time each stretch of a mission has been done in, over every fair win.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Golds(BTreeMap<Stretch, f32>);

impl Golds {
    /// From the file's lines: `from to pass seconds`.
    fn parse(text: &str) -> Golds {
        Golds(text.lines().filter_map(|l| {
            let mut w = l.split_whitespace();
            Some(((w.next()?.parse().ok()?, w.next()?.parse().ok()?, w.next()?.parse().ok()?), w.next()?.parse().ok()?))
        }).collect())
    }

    fn text(&self) -> String {
        self.0.iter().map(|((from, to, pass), secs)| format!("{from} {to} {pass} {secs}\n")).collect()
    }

    fn path(mission: &str, d: uc_game::Difficulty) -> Option<PathBuf> {
        file(&format!("{}.splits", stem(mission, d)))
    }

    fn load(mission: &str, d: uc_game::Difficulty) -> Golds {
        Golds::parse(&Golds::path(mission, d).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default())
    }

    /// Takes in a won run's `stretches` (each with the seconds it took), keeping the best of
    /// each: the sum of the best there's been of every stretch of this run. Whether any
    /// got better.
    fn take(&mut self, stretches: &[(Stretch, f32)]) -> (f32, bool) {
        let mut better = false;
        let sum = stretches.iter().map(|&(k, secs)| {
            let best = self.0.entry(k).or_insert(f32::INFINITY);
            if secs < *best {
                (*best, better) = (secs, true);
            }
            *best
        }).sum();
        (sum, better)
    }
}

/// A split this run has passed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Passed {
    /// The script's point, and which time through it this was (0 the first).
    pub(crate) point: usize,
    pub(crate) pass: usize,
    /// The mission's turns played by then, in seconds.
    pub(crate) time: f32,
    /// Seconds behind the ghost there (ahead if negative), if the ghost passed it too.
    pub(crate) diff: Option<f32>,
}

/// A split on screen: the objective it's for, if it's one; the time; how far behind the ghost
/// (ahead if negative), if the ghost has it; and the seconds left of showing it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shown {
    pub(crate) name: Option<String>,
    pub(crate) time: f32,
    pub(crate) diff: Option<f32>,
    pub(crate) left: f32,
}

/// A row of the win screen's splits: what, this run's time, and (with a ghost that got
/// there) the ghost's and the difference.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Row {
    pub(crate) name: String,
    pub(crate) time: f32,
    pub(crate) best: Option<f32>,
}

/// The names of `mission`'s objectives that are splits, by the script's point: what's said
/// as bonus points are given, and each side objective by the first of its rewards.
pub(crate) fn objectives(mission: &uc_formats::ucm::Mission, goals: &uc_game::goals::Goals) -> BTreeMap<usize, String> {
    use uc_formats::ucm::waypoint;
    let texts = mission.point_texts();
    let mut out: BTreeMap<usize, String> = mission.event_points.iter()
        .filter(|e| e.used != 0 && e.kind == waypoint::BONUS_POINTS)
        .filter_map(|e| Some((e.index, texts.get(&e.index)?.split('|').next()?.trim().trim_end_matches('.').to_string())))
        .filter(|(_, t)| !t.is_empty())
        .collect();
    for g in &goals.sides {
        if let Some(&first) = g.rewards.first() {
            out.insert(first, g.label.clone());
        }
    }
    out
}

/// The ghost's race against this run.
#[derive(Default)]
pub(crate) struct Race {
    /// The run raced against (the best so far), if any.
    pub(crate) best: Option<Track>,
    /// The checkpoints of this run already compared with the ghost's.
    seen: usize,
    /// The last split worth showing, while it's shown.
    pub(crate) split: Option<Shown>,
    /// Every split passed, in order.
    pub(crate) passed: Vec<Passed>,
    /// The mission's objectives among the splits, named.
    names: BTreeMap<usize, String>,
    /// The best each stretch has been done in.
    golds: Golds,
    /// A fair win: the sum of the best there's been of each stretch of this run.
    pub(crate) sum_of_best: Option<f32>,
    /// A win that beat the ghost (or the first win): saved as the new one.
    pub(crate) saved: bool,
}

impl Race {
    /// A race on `level`'s mission: this run kept as it goes, against the best.
    pub(crate) fn start(game: &mut Game, level: &Level) -> Race {
        game.record_ghost();
        let d = game.difficulty();
        Race {
            best: load(&level.name, d), golds: Golds::load(&level.name, d),
            names: objectives(&level.mission, &uc_game::goals::goals(&level.mission)), ..Default::default()
        }
    }

    /// Each frame: the checkpoints passed since, against the ghost's, and the split shown
    /// counting down. A split is shown if the ghost has it to compare, or it's an objective.
    pub(crate) fn update(&mut self, game: &Game, dt: f32) {
        if let Some(s) = &mut self.split {
            s.left -= dt;
            if s.left <= 0.0 {
                self.split = None;
            }
        }
        let passed = game.checkpoints();
        for i in self.seen..passed.len() {
            let (point, turns) = passed[i];
            let pass = passed[..i].iter().filter(|c| c.0 == point).count();
            let diff = self.best.as_ref().and_then(|b| b.split(point, pass, turns));
            let time = turns as f32 * uc_game::TURN;
            let name = self.names.get(&point).filter(|_| pass == 0).cloned();
            if diff.is_some() || name.is_some() {
                self.split = Some(Shown { name, time, diff, left: SPLIT_SHOWN });
            }
            self.passed.push(Passed { point, pass, time, diff });
        }
        self.seen = passed.len();
    }

    /// The objective at `point`'s split in this run, if it's been done: its time, and how far
    /// behind the ghost.
    pub(crate) fn objective(&self, point: usize) -> Option<&Passed> {
        self.passed.iter().find(|p| p.point == point && p.pass == 0)
    }

    /// This run's stretches, to its end `finish` seconds in: each with how long it took.
    fn stretches(&self, finish: f32) -> Vec<(Stretch, f32)> {
        let mut from = (0, 0.0);
        let mut out = Vec::new();
        for p in &self.passed {
            out.push(((from.0, p.point as u32, p.pass as u32), p.time - from.1));
            from = (p.point as u32, p.time);
        }
        out.push(((from.0, FINISH, 0), finish - from.1));
        out
    }

    /// The run's won. A fair win's stretches are kept where they're the best yet, and it's
    /// kept as the ghost if it beat the one before (or there wasn't one). A run the console
    /// changed, or that went back to a checkpoint, leaves nothing (as it earns no medal).
    pub(crate) fn won(&mut self, game: &Game, mission: &str, fair: bool) {
        let Some(run) = game.recorded().filter(|_| fair) else { return };
        let (sum, better) = self.golds.take(&self.stretches(run.time));
        self.sum_of_best = Some(sum);
        if better {
            write(Golds::path(mission, game.difficulty()), self.golds.text().as_bytes());
        }
        if self.best.as_ref().is_none_or(|b| run.time < b.time) {
            save(mission, game.difficulty(), &run);
            self.saved = true;
        }
    }

    /// The win screen's splits, for a run won `finish` seconds in: the objectives done, in
    /// the order they were, then the finish.
    pub(crate) fn rows(&self, finish: f32) -> Vec<Row> {
        let mut rows: Vec<Row> = self.passed.iter().filter(|p| p.pass == 0)
            .filter_map(|p| Some(Row { name: self.names.get(&p.point)?.clone(), time: p.time, best: p.diff.map(|d| p.time - d) }))
            .collect();
        rows.push(Row { name: "Finish".into(), time: finish, best: self.best.as_ref().map(|b| b.time) });
        rows
    }
}

/// A time trial's race against its best lap (#519): the ghost is the fastest lap of the
/// course in the car, kept in `ghosts\` as `trial-<course>-<car>.ucg`, and replaced as soon as
/// a lap beats it, to race on the next lap. The best race and lap go in the progress.
pub(crate) struct LapRace {
    /// The progress's key for the course and car, and the ghost's file.
    pub(crate) key: String,
    name: String,
    /// The lap raced against (the best so far), if any.
    pub(crate) best: Option<Track>,
    /// The best race and lap before this run.
    pub(crate) before: (Option<f32>, Option<f32>),
    /// The laps, and this lap's checkpoints, already compared.
    laps: usize,
    splits: usize,
    /// The last split: seconds behind (positive) or ahead, and seconds left on screen.
    pub(crate) split: Option<(f32, f32)>,
    /// The laps of this run that were the best yet when they were run.
    pub(crate) new_laps: Vec<usize>,
    /// The race is over and recorded: whether it was the best.
    pub(crate) new_race: Option<bool>,
}

impl LapRace {
    /// The race for the time trial `game` is, if it is one; this run kept as it goes.
    pub(crate) fn start(game: &mut Game) -> Option<LapRace> {
        let t = game.trial()?;
        let id = t.course().id.to_ascii_lowercase();
        let key = progress::trial_key(&id, t.car);
        let name = format!("trial-{id}-{}.ucg", t.car.id());
        game.record_ghost();
        let best = file(&name).and_then(|p| std::fs::read(p).ok()).and_then(|b| Track::from_bytes(&b));
        let before = progress::Progress::load().best_trial(&key);
        Some(LapRace { key, name, best, before, laps: 0, splits: 0, split: None, new_laps: Vec::new(), new_race: None })
    }

    /// Each frame: the laps done since, kept if they're the best; this lap's checkpoints
    /// against the ghost's; and the race recorded once it's over.
    pub(crate) fn update(&mut self, game: &Game, dt: f32) {
        if let Some((_, left)) = &mut self.split {
            *left -= dt;
            if *left <= 0.0 {
                self.split = None;
            }
        }
        let Some(t) = game.trial() else { return };
        let times = t.lap_times();
        for (n, &time) in times.iter().enumerate().skip(self.laps) {
            if let Some(b) = &self.best {
                self.split = Some((time - b.time, SPLIT_SHOWN));
            }
            progress::Progress::load().record_lap(&self.key, time);
            if self.best.as_ref().is_none_or(|b| time < b.time) {
                if let Some(lap) = game.trial_lap(n) {
                    write(file(&self.name), &lap.to_bytes());
                    self.best = Some(lap);
                    self.new_laps.push(n);
                }
            }
        }
        if times.len() > self.laps {
            (self.laps, self.splits) = (times.len(), 0);
        }
        for &(k, at) in t.splits.iter().skip(self.splits) {
            if let Some(d) = self.best.as_ref().and_then(|b| b.split(k, 0, at)) {
                self.split = Some((d, SPLIT_SHOWN));
            }
        }
        self.splits = t.splits.len();
        if let (Some(total), None) = (t.total(), self.new_race) {
            self.new_race = Some(progress::Progress::load().record_race(&self.key, total));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passed(point: usize, pass: usize, time: f32, diff: Option<f32>) -> Passed {
        Passed { point, pass, time, diff }
    }

    /// The best of each stretch is kept over the runs, and read back as written; the sum of
    /// them is what a run of all the best stretches would take.
    #[test]
    fn the_best_of_each_stretch_is_kept() {
        let mut g = Golds::default();
        let first = [((0, 9, 0), 10.0), ((9, 20, 0), 30.0), ((20, FINISH, 0), 5.0)];
        assert_eq!(g.take(&first), (45.0, true));
        // Slower to the first split, faster after: 10 + 25 + 5.
        let second = [((0, 9, 0), 12.0), ((9, 20, 0), 25.0), ((20, FINISH, 0), 5.0)];
        assert_eq!(g.take(&second), (40.0, true));
        assert_eq!(g.take(&first), (40.0, false));
        // Another way round: only the stretches it shares count.
        let other = [((0, 20, 0), 20.0), ((20, FINISH, 0), 4.5)];
        assert_eq!(g.take(&other), (24.5, true));
        assert_eq!(Golds::parse(&g.text()), g);
        assert_eq!(Golds::parse("nonsense\n1 2"), Golds::default());
    }

    /// A run's stretches go from split to split and end at the win; the win screen lists the
    /// objectives among them, with the ghost's time at each, and the finish.
    #[test]
    fn a_runs_stretches_and_rows() {
        let best = Track { time: 50.0, ..Default::default() };
        let r = Race {
            best: Some(best), names: [(9, "Mugger terminated".to_string())].into(),
            passed: vec![passed(4, 0, 10.0, Some(1.5)), passed(9, 0, 30.0, Some(-2.0)), passed(4, 1, 41.0, None)],
            ..Default::default()
        };
        assert_eq!(r.stretches(48.0), [((0, 4, 0), 10.0), ((4, 9, 0), 20.0), ((9, 4, 1), 11.0), ((4, FINISH, 0), 7.0)]);
        assert_eq!(r.rows(48.0), [
            Row { name: "Mugger terminated".into(), time: 30.0, best: Some(32.0) },
            Row { name: "Finish".into(), time: 48.0, best: Some(50.0) },
        ]);
        assert_eq!(r.objective(9).map(|p| p.time), Some(30.0));
        assert_eq!(r.objective(5), None);
        assert_eq!(Race::default().rows(9.0), [Row { name: "Finish".into(), time: 9.0, best: None }]);
    }

    /// Every story mission is raced, with the training courses that were; and a mission's
    /// objectives are named for their splits. Skipped without an install.
    #[test]
    fn story_missions_are_raced_and_their_objectives_named() {
        assert!(raced("Assault1") && raced("police1") && raced("Finale1"));
        assert!(!raced("nowhere"));
        let Some(game) = uc_formats::default_install() else { return };
        let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
        let goals = uc_game::goals::goals(&level.mission);
        let names = objectives(&level.mission, &goals);
        assert_eq!(names.get(&9).map(String::as_str), Some("Mugger terminated"));
        for g in &goals.sides {
            assert_eq!(names.get(&g.rewards[0]), Some(&g.label));
        }
    }

    /// A race as the viewer runs one: the splits passed are kept with their times, and only
    /// a fair win is taken in (nothing is written here: the run isn't won). Skipped without
    /// an install.
    #[test]
    fn a_race_keeps_the_splits_passed() {
        let Some(game) = uc_formats::default_install() else { return };
        let args = Args { game, mission: "assault1".into(), ..Args::default() };
        let mut w = World::load(args).unwrap();
        assert!(w.race.is_some());
        for frame in 0..1200 {
            if w.game.cutscene.is_some() {
                w.game.skip_cutscene();
            }
            let c = Controls { wish: vec2(0.0, 1.0), sprint: true, ..Controls::default() };
            w.game.update(1.0 / 30.0, &c, &w.level);
            let _ = frame;
            if let Some(r) = w.race.as_mut() {
                r.update(&w.game, 1.0 / 30.0);
            }
        }
        let r = w.race.as_mut().unwrap();
        assert_eq!(r.passed.len(), w.game.checkpoints().len());
        for (p, c) in r.passed.iter().zip(w.game.checkpoints()) {
            assert_eq!((p.point, p.time), (c.0, c.1 as f32 * uc_game::TURN));
        }
        // An unfair win leaves no sum of best, and no ghost.
        r.won(&w.game, "assault1", false);
        assert!(r.sum_of_best.is_none() && !r.saved);
    }
}
