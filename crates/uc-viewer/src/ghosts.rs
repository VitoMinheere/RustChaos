//! Ghost races (#186): on the training missions against the clock, the best run plays
//! alongside as a see-through Darci or squad car. The ghost is kept per mission and
//! difficulty in `ghosts\` next to the progress file, and replaced when a run beats it.

use crate::*;
use uc_game::game::ghost::Track;

/// The missions raced against a ghost: the assault course and the three driving tests.
const MISSIONS: [&str; 4] = ["assault1", "testdrive1a", "testdrive2", "testdrive3"];

/// Seconds a split stays on screen.
pub(crate) const SPLIT_SHOWN: f32 = 3.0;

/// Whether `mission` is raced against a ghost.
pub(crate) fn raced(mission: &str) -> bool {
    MISSIONS.iter().any(|m| m.eq_ignore_ascii_case(mission))
}

/// The ghost file called `name` in `ghosts\`.
fn file(name: &str) -> Option<PathBuf> {
    Some(progress::Progress::folder()?.join("ghosts").join(format!("{name}.ucg")))
}

fn path(mission: &str, d: uc_game::Difficulty) -> Option<PathBuf> {
    file(&format!("{}-{}", mission.to_ascii_lowercase(), d.name().to_ascii_lowercase()))
}

/// The best run's ghost on `mission` at difficulty `d`, if there is one.
pub(crate) fn load(mission: &str, d: uc_game::Difficulty) -> Option<Track> {
    Track::from_bytes(&std::fs::read(path(mission, d)?).ok()?)
}

pub(crate) fn save(mission: &str, d: uc_game::Difficulty, track: &Track) {
    write(path(mission, d), track);
}

fn write(path: Option<PathBuf>, track: &Track) {
    let Some(path) = path else { return };
    let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, track.to_bytes()));
    if let Err(e) = saved {
        eprintln!("couldn't save the ghost to {}: {e}", path.display());
    }
}

/// A split: `+1.24` behind, `-0.80` ahead.
pub(crate) fn split_text(secs: f32) -> String {
    format!("{secs:+.2}")
}

/// The ghost's race against this run.
#[derive(Default)]
pub(crate) struct Race {
    /// The run raced against (the best so far), if any.
    pub(crate) best: Option<Track>,
    /// The checkpoints of this run already compared with the ghost's.
    seen: usize,
    /// The last split: seconds behind (positive) or ahead, and seconds left on screen.
    pub(crate) split: Option<(f32, f32)>,
    /// A win that beat the ghost (or the first win): saved as the new one.
    pub(crate) saved: bool,
}

impl Race {
    /// A race on `mission`, if it has one: this run kept as it goes, against the best.
    pub(crate) fn start(game: &mut Game, mission: &str) -> Race {
        game.record_ghost();
        Race { best: load(mission, game.difficulty()), ..Default::default() }
    }

    /// Each frame: the checkpoints passed since, against the ghost's, and the split shown
    /// counting down.
    pub(crate) fn update(&mut self, game: &Game, dt: f32) {
        if let Some((_, left)) = &mut self.split {
            *left -= dt;
            if *left <= 0.0 {
                self.split = None;
            }
        }
        let passed = game.checkpoints();
        for i in self.seen..passed.len() {
            let (point, turns) = passed[i];
            let nth = passed[..i].iter().filter(|c| c.0 == point).count();
            if let Some(d) = self.best.as_ref().and_then(|b| b.split(point, nth, turns)) {
                self.split = Some((d, SPLIT_SHOWN));
            }
        }
        self.seen = passed.len();
    }

    /// The run's won: kept as the ghost if it beat the one before (or there wasn't one).
    pub(crate) fn won(&mut self, game: &Game, mission: &str) {
        let Some(run) = game.recorded() else { return };
        if self.best.as_ref().is_none_or(|b| run.time < b.time) {
            save(mission, game.difficulty(), &run);
            self.saved = true;
        }
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
        let name = format!("trial-{id}-{}", t.car.id());
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
                    write(file(&self.name), &lap);
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
