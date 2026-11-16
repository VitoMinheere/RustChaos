//! Leaving a mission and picking it up again, and checkpoints (#544). Neither is in the
//! original, whose pause menu offers resume, restart and abandon (gamemenu.cpp:97).
//!
//! A save is the run so far as a replay has it (`uc_net::replay`): how it started and every
//! turn's inputs, in `saves\<mission>.ucs` next to the progress file, one a mission. Resuming
//! plays them into a new game as fast as they go, checking the state hashes on the way, so
//! the run carried on is the run that was left, its replay and its ghost whole. A save from
//! another build may not play the same, and is refused.
//!
//! A checkpoint is a copy of the game kept in memory each time the script's somewhere it was
//! waiting for the player to get to (the points a ghost's splits are timed at), or a side
//! objective's done. Going back to one doesn't count for medals, the ghost or the Mucky code.

use crate::*;
use uc_game::goals::Goals;
use uc_net::replay::{Recorder, Replay};

/// The file's first bytes, with the format's version in the last.
const MAGIC: &[u8; 4] = b"UCS1";

/// No checkpoint is taken this early in a mission (seconds): it would be the start again.
const EARLIEST: f32 = 10.0;

/// A mission left to be picked up again.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Save {
    /// The run so far.
    pub(crate) replay: Replay,
    /// It went back to a checkpoint on the way.
    pub(crate) checkpointed: bool,
}

impl Save {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.push(self.checkpointed as u8);
        out.extend(self.replay.to_bytes());
        out
    }

    pub(crate) fn from_bytes(b: &[u8]) -> Result<Save, String> {
        let rest = b.strip_prefix(MAGIC).ok_or("not a saved mission")?;
        let (&flags, replay) = rest.split_first().ok_or("a saved mission cut short")?;
        let replay = Replay::from_bytes(replay).map_err(|e| e.to_string())?;
        Ok(Save { replay, checkpointed: flags & 1 != 0 })
    }
}

fn folder() -> Option<PathBuf> {
    Some(progress::Progress::folder()?.join("saves"))
}

fn file(dir: &Path, mission: &str) -> PathBuf {
    dir.join(format!("{}.ucs", mission.to_ascii_lowercase()))
}

fn write_in(dir: &Path, save: &Save) -> Result<PathBuf, String> {
    let path = file(dir, &save.replay.start.mission);
    std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, save.to_bytes())).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

fn load_in(dir: &Path, mission: &str) -> Option<Save> {
    let path = file(dir, mission);
    let bytes = std::fs::read(&path).ok()?;
    Save::from_bytes(&bytes).map_err(|e| eprintln!("{}: {e}", path.display())).ok()
}

/// Mission `mission`'s save, if it has one that reads.
pub(crate) fn load(mission: &str) -> Option<Save> {
    load_in(&folder()?, mission)
}

/// Mission `mission`'s save gone: resumed, or not wanted.
pub(crate) fn discard(mission: &str) {
    if let Some(dir) = folder() {
        let _ = std::fs::remove_file(file(&dir, mission));
    }
}

/// A saved mission, for the mission list.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Listed {
    /// The mission clock where it was left, and when that was (seconds since 1970).
    pub(crate) time: f32,
    pub(crate) date: u64,
    /// The build that saved it, and whether it's this one: only then can it be resumed.
    pub(crate) build: String,
    pub(crate) same_build: bool,
}

fn list_in(dir: &Path) -> BTreeMap<String, Listed> {
    let Ok(read) = std::fs::read_dir(dir) else { return BTreeMap::new() };
    read.filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ucs")))
        .filter_map(|p| {
            let s = Save::from_bytes(&std::fs::read(&p).ok()?).ok()?;
            let r = s.replay;
            Some((r.start.mission.to_ascii_lowercase(), Listed { time: r.time, date: r.date, same_build: r.same_build(), build: r.build }))
        })
        .collect()
}

/// The saved missions, by mission (in lower case).
pub(crate) fn list() -> BTreeMap<String, Listed> {
    folder().map_or(BTreeMap::new(), |d| list_in(&d))
}

/// The game as it was at a checkpoint.
pub(crate) struct Checkpoint {
    game: Game,
    /// The run kept up to then, if it was being kept.
    recorder: Option<Recorder>,
}

/// A run's checkpoints: the last one, kept to go back to.
#[derive(Default)]
pub(crate) struct Checkpoints {
    /// The script's places reached and the side objectives done, as last counted.
    places: usize,
    sides: usize,
    /// More has been reached since the last checkpoint: the next moment that will do is one.
    due: bool,
    last: Option<Checkpoint>,
    /// The run has gone back to one.
    pub(crate) used: bool,
}

impl Checkpoints {
    /// After a turn (or a frame's turns): a checkpoint of `game`, if the player has just got
    /// somewhere and is in a state to carry on from. `recorder` has every turn played.
    pub(crate) fn note(&mut self, game: &Game, goals: &Goals, recorder: Option<&Recorder>) {
        let (places, sides) = (game.checkpoints().len(), game.sides_done(goals).len());
        self.due |= places > self.places || sides > self.sides;
        (self.places, self.sides) = (places, sides);
        let fit = game.outcome.is_none() && game.cutscene.is_none() && game.player().is_some_and(|p| p.health > 0 && !p.removed);
        if self.due && fit && game.time >= EARLIEST {
            self.due = false;
            self.last = Some(Checkpoint { game: game.clone(), recorder: recorder.cloned() });
        }
    }

    /// The mission clock at the checkpoint there is to go back to.
    pub(crate) fn time(&self) -> Option<f32> {
        self.last.as_ref().map(|c| c.game.time)
    }
}

impl World {
    /// A mission played alone as itself: the kind of run that has checkpoints, and can be
    /// left and resumed.
    fn story_run(&self) -> bool {
        let a = &self.args;
        self.replay.is_none() && self.editor.is_none() && self.seat.is_none()
            && !(a.explore || a.endless || a.beat || a.spar || a.no_character) && a.trial.is_none()
    }

    /// Whether this run can be saved to carry on later: one still going and still kept turn
    /// for turn (the console hasn't changed it), since the turns are the save.
    pub(crate) fn can_save(&self) -> bool {
        self.story_run() && !self.edited && self.recorder.is_some() && self.game.outcome.is_none()
    }

    /// Each frame, after its turns have been taken into the recorder: the checkpoint, if
    /// there's a new one.
    pub(crate) fn note_checkpoint(&mut self) {
        if self.story_run() {
            self.checkpoints.note(&self.game, &self.goals, self.recorder.as_ref());
        }
    }

    /// Saves the run to be resumed from the mission list, and lets go of it: nothing of it
    /// goes into the replays or the stats until it's carried on and ended. Whether it saved.
    pub(crate) fn save_and_quit(&mut self) -> bool {
        self.save_in(folder().as_deref())
    }

    fn save_in(&mut self, dir: Option<&Path>) -> bool {
        if !self.can_save() {
            return false;
        }
        let Some(dir) = dir else { return false };
        let mut replay = self.recorder.as_mut().unwrap().finish(&mut self.game);
        replay.outcome = "saved".into();
        match write_in(dir, &Save { replay, checkpointed: self.checkpoints.used }) {
            Ok(path) => {
                println!("mission saved to {}", path.display());
                self.recorder = None;
                self.stats_saved = true;
                true
            }
            Err(e) => {
                eprintln!("couldn't save the mission: {e}");
                self.console.say("Couldn't save the mission");
                false
            }
        }
    }

    /// Back to the last checkpoint: the game as it was, the run's replay as it was then (so
    /// it's still whole when it ends), and the run marked as having had the help.
    pub(crate) fn restart_from_checkpoint(&mut self) {
        let Some(c) = self.checkpoints.last.as_ref() else { return };
        self.game = c.game.clone();
        self.recorder = c.recorder.clone();
        // The game lost had its stats counted already, up to the loss: what's counted from
        // here is only what comes after the checkpoint.
        if self.stats_saved {
            self.game.record = Default::default();
            self.stats_saved = false;
        }
        (self.checkpoints.places, self.checkpoints.sides) = (self.game.checkpoints().len(), self.game.sides_done(&self.goals).len());
        self.checkpoints.due = false;
        self.checkpoints.used = true;
        self.result = None;
        self.medals = None;
        self.civ_warning = None;
        self.replay_saved = None;
        self.race = None;
        if let Some(p) = self.game.player() {
            self.orbit = Orbit::behind(p.fig.angle);
        }
    }

    /// The saved mission `save`, carried on: loaded as it started, then played up to where
    /// it was left.
    pub(crate) fn load_save(args: Args, save: &Save) -> Result<World, String> {
        let r = &save.replay;
        if !r.same_build() {
            return Err(format!("saved on another build ({}): it can't be resumed on this one", r.build));
        }
        let mut w = World::load_started(args, r.start.clone())?;
        let mut rec = w.recorder.take().ok_or("the run isn't kept")?;
        let (checkpoints, goals) = (&mut w.checkpoints, &w.goals);
        rec.resume(&mut w.game, &w.level, r, |g, rec| checkpoints.note(g, goals, Some(rec)))
            .map_err(|t| format!("the save stops playing as it was left {} in", progress::clock(t as f32 / 30.0)))?;
        w.recorder = Some(rec);
        w.checkpoints.used = save.checkpointed;
        // What the turns sounded like isn't heard all at once.
        drop(w.game.take_sounds());
        match w.race.as_mut() {
            Some(_) if save.checkpointed => w.race = None,
            // The splits already passed aren't shown again.
            Some(race) => {
                race.update(&w.game, 0.0);
                race.split = None;
            }
            None => {}
        }
        if let Some(p) = w.game.player() {
            w.orbit = Orbit::behind(p.fig.angle);
        }
        w.help = false;
        Ok(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("uc-saves-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Frames of uneven length, as the viewer plays them, from frame `from`: the bot's input,
    /// the opening fly-by skipped, the turns taken, the checkpoints looked for.
    fn play(w: &mut World, frames: std::ops::Range<usize>) {
        for frame in frames {
            let dt = [0.007, 0.016, 0.040, 0.025][frame % 4];
            let c = uc_net::bot_input(0, frame / 40);
            if w.game.cutscene.is_some() && frame > 30 {
                w.game.skip_cutscene();
            }
            w.game.update(dt, &c, &w.level);
            if let Some(r) = w.recorder.as_mut() {
                r.take(&mut w.game);
            }
            w.note_checkpoint();
        }
    }

    /// A mission saved from the viewer and resumed stands as it was left, plays on the same,
    /// and its replay is the whole run's. Skipped without an install.
    #[test]
    fn a_mission_saved_and_resumed_carries_on_the_same() {
        let Some(game) = uc_formats::default_install() else { return };
        let args = Args { game, mission: "police1".into(), weapon: uc_game::Weapon::Pistol, ..Args::default() };
        let dir = scratch("resume");
        let mut w = World::load(args.clone()).unwrap();
        play(&mut w, 0..900);
        // An unbroken run beside it, to compare with.
        let mut whole = World::load(args.clone()).unwrap();
        play(&mut whole, 0..900);
        assert_eq!(whole.game.state_hash(), w.game.state_hash());

        assert!(w.can_save());
        assert!(w.save_in(Some(&dir)));
        assert!(w.recorder.is_none() && w.stats_saved, "the run left isn't kept as a replay or counted");
        let listed = list_in(&dir);
        assert_eq!(listed.len(), 1);
        assert!(listed["police1"].same_build);
        assert_eq!(listed["police1"].time, w.game.time);

        let save = load_in(&dir, "Police1").unwrap();
        assert_eq!(save.replay.outcome, "saved");
        let mut back = World::load_save(Args { mission: "other".into(), ..args.clone() }, &save).unwrap();
        assert_eq!(back.args.mission, "police1");
        assert_eq!(back.game.turns(), whole.game.turns());
        assert_eq!(back.game.state_hash(), whole.game.state_hash());
        assert_eq!(back.game.time, whole.game.time);
        assert!(back.can_save() && !back.checkpoints.used);
        // Uneven frames leave part of a turn over, which the save doesn't keep: the two are
        // compared turn for turn.
        for t in 0..600 {
            let c = uc_net::bot_input(0, 20 + t / 30);
            for w in [&mut back, &mut whole] {
                w.game.step(&[c], &w.level);
                w.recorder.as_mut().unwrap().take(&mut w.game);
            }
            assert_eq!(back.game.state_hash(), whole.game.state_hash(), "{t} turns after resuming");
        }
        let a = back.recorder.as_mut().unwrap().finish(&mut back.game);
        let b = whole.recorder.as_mut().unwrap().finish(&mut whole.game);
        assert_eq!((a.turns, a.hashes, a.events), (b.turns, b.hashes, b.events));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A save from another build is refused, and one whose turns don't make the game it was.
    #[test]
    fn a_save_that_wont_play_the_same_is_refused() {
        let Some(game) = uc_formats::default_install() else { return };
        let args = Args { game, mission: "police1".into(), ..Args::default() };
        let dir = scratch("refused");
        let mut w = World::load(args.clone()).unwrap();
        play(&mut w, 0..400);
        assert!(w.save_in(Some(&dir)));
        let save = load_in(&dir, "police1").unwrap();
        let mut other = save.clone();
        other.replay.build = "another".into();
        assert!(World::load_save(args.clone(), &other).err().unwrap().contains("another build"));
        let other = Save::from_bytes(&other.to_bytes()).unwrap();
        assert!(other.replay.build == "another" && !other.checkpointed);
        let mut drifted = save.clone();
        drifted.replay.hashes[3].1 ^= 1;
        assert!(World::load_save(args, &drifted).err().unwrap().contains("0:04"));
        assert!(Save::from_bytes(b"UCR6").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A checkpoint is taken as the player gets somewhere the script was waiting for, and
    /// going back to it puts the game and its replay back as they were: the run plays on
    /// from there as it did the first time, and no longer counts as fair. The mark survives a
    /// save.
    #[test]
    fn back_to_a_checkpoint() {
        let Some(game) = uc_formats::default_install() else { return };
        let args = Args { game, mission: "police1".into(), ..Args::default() };
        let mut w = World::load(args.clone()).unwrap();
        play(&mut w, 0..600);
        // Somewhere reached, as the script would say: a checkpoint on the next look.
        w.checkpoints.places = 0;
        w.checkpoints.due = true;
        w.note_checkpoint();
        let at = w.game.time;
        assert_eq!(w.checkpoints.time(), Some(at));
        let (hash, turns) = (w.game.state_hash(), w.recorder.as_ref().unwrap().turns());
        let on = |w: &mut World| -> Vec<u64> {
            (0..300).map(|t| {
                let c = uc_net::bot_input(0, 3 + t / 30);
                w.game.step(&[c], &w.level);
                w.recorder.as_mut().unwrap().take(&mut w.game);
                w.game.state_hash()
            }).collect()
        };
        let first = on(&mut w);
        assert!(w.game.time > at + 9.0);

        w.restart_from_checkpoint();
        assert_eq!(w.game.time, at);
        assert_eq!(w.game.state_hash(), hash);
        assert_eq!(w.recorder.as_ref().unwrap().turns(), turns);
        assert!(w.checkpoints.used && w.race.is_none());
        assert_eq!(w.checkpoints.time(), Some(at), "it's still there to go back to again");
        assert_eq!(on(&mut w), first);
        // The replay is one run from the start, through the checkpoint, to here.
        let replay = w.recorder.as_mut().unwrap().finish(&mut w.game);
        let mut back = World::load_replay(args.clone(), replay).unwrap();
        let v = back.replay.as_mut().unwrap();
        v.playback.skip(&mut back.game, &back.level, usize::MAX);
        assert_eq!(v.playback.drift, None);
        assert_eq!(back.game.state_hash(), w.game.state_hash());

        let dir = scratch("checkpoint");
        assert!(w.save_in(Some(&dir)));
        let resumed = World::load_save(args, &load_in(&dir, "police1").unwrap()).unwrap();
        assert!(resumed.checkpoints.used && resumed.race.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
