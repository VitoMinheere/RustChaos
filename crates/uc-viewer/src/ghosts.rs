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

fn path(mission: &str, d: uc_game::Difficulty) -> Option<PathBuf> {
    let name = format!("{}-{}.ucg", mission.to_ascii_lowercase(), d.name().to_ascii_lowercase());
    Some(progress::Progress::folder()?.join("ghosts").join(name))
}

/// The best run's ghost on `mission` at difficulty `d`, if there is one.
pub(crate) fn load(mission: &str, d: uc_game::Difficulty) -> Option<Track> {
    Track::from_bytes(&std::fs::read(path(mission, d)?).ok()?)
}

pub(crate) fn save(mission: &str, d: uc_game::Difficulty, track: &Track) {
    let Some(path) = path(mission, d) else { return };
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
