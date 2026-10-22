//! Replays (#184): every run kept as `uc_net::replay` has it, in `replays\` next to the
//! progress file once it's won, lost, or left after a while. The last [`KEEP`] stay; older
//! ones go, unless K moved them into `replays\kept\` at the end of their run. The REPLAYS
//! menu and `--replay FILE` play them back, with pause, speed, jumping forward and, in co-op,
//! the choice of which player to follow.

use crate::*;
use uc_net::replay::{Playback, Replay, Start, Tally};

/// Replays kept besides those kept for good.
const KEEP: usize = 20;

/// A run left before this many turns (10 s), without having been won or lost, isn't kept.
pub(crate) const SHORTEST: u64 = 10 * 30;

/// The playback speeds: Up and Down go through them.
const SPEEDS: [f32; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

/// Seconds Right jumps forward.
const JUMP: usize = 10;

fn folder() -> Option<PathBuf> {
    Some(progress::Progress::folder()?.join("replays"))
}

/// Seconds since 1970 as (year, month, day, hour, minute, second), UTC.
fn civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let (days, rem) = ((secs / 86400) as i64, secs % 86400);
    // Howard Hinnant's days-to-civil.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + (m <= 2) as i64;
    (y, m, d, (rem / 3600) as u32, (rem / 60 % 60) as u32, (rem % 60) as u32)
}

/// When a replay was played, as long ago as it was at `now` (both seconds since 1970): `5
/// minutes ago`, `yesterday`; after a week, the day (`2026-10-21`). The machine's time zone
/// doesn't come into it.
pub(crate) fn ago(secs: u64, now: u64) -> String {
    let d = now.saturating_sub(secs);
    let n = |n: u64, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match d {
        0..60 => "just now".into(),
        60..3600 => n(d / 60, "minute"),
        3600..86400 => n(d / 3600, "hour"),
        86400..172800 => "yesterday".into(),
        172800..604800 => n(d / 86400, "day"),
        _ => {
            let (y, mo, d, ..) = civil(secs);
            format!("{y}-{mo:02}-{d:02}")
        }
    }
}

/// Saves `r` as `replays\<mission>-<when>.ucr`, and lets the oldest go. Where it went.
pub(crate) fn save(r: &Replay) -> Option<PathBuf> {
    let dir = folder()?;
    let (y, mo, d, h, mi, s) = civil(r.date);
    let path = dir.join(format!("{}-{y}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.ucr", r.start.mission.to_ascii_lowercase()));
    let saved = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, r.to_bytes()));
    if let Err(e) = saved {
        eprintln!("couldn't save the replay to {}: {e}", path.display());
        return None;
    }
    println!("replay saved to {}", path.display());
    prune(&dir);
    Some(path)
}

/// All but the newest [`KEEP`] replays (not counting those kept for good) go.
fn prune(dir: &Path) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = read
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ucr")))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    for (_, p) in files.iter().skip(KEEP) {
        let _ = std::fs::remove_file(p);
    }
}

/// Moves the replay at `path` into `replays\kept\`, where it isn't let go. Where it is now.
pub(crate) fn keep(path: &Path) -> Option<PathBuf> {
    let dir = folder()?.join("kept");
    let to = dir.join(path.file_name()?);
    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::rename(path, &to)) {
        Ok(()) => Some(to),
        Err(e) => {
            eprintln!("couldn't keep the replay {}: {e}", path.display());
            None
        }
    }
}

pub(crate) fn load(path: &Path) -> Result<Replay, String> {
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Replay::from_bytes(&b).map_err(|e| format!("{}: {e}", path.display()))
}

/// A saved replay, for the menu.
#[derive(Debug, Clone)]
pub(crate) struct Listed {
    pub(crate) path: PathBuf,
    pub(crate) start: Start,
    pub(crate) date: u64,
    pub(crate) outcome: String,
    /// The mission clock at the end, and the replay's length in seconds (longer, when the run
    /// went on after it was won or lost).
    pub(crate) time: f32,
    pub(crate) length: f32,
    pub(crate) tally: Tally,
    pub(crate) kept: bool,
    pub(crate) build: String,
    pub(crate) same_build: bool,
}

impl Listed {
    pub(crate) fn won(&self) -> bool {
        self.outcome == "won"
    }
}

/// The saved replays, newest first.
pub(crate) fn list() -> Vec<Listed> {
    let Some(dir) = folder() else { return Vec::new() };
    let mut out = Vec::new();
    for (dir, kept) in [(dir.clone(), false), (dir.join("kept"), true)] {
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for path in read.filter_map(|e| e.ok()).map(|e| e.path()) {
            if !path.extension().is_some_and(|x| x.eq_ignore_ascii_case("ucr")) {
                continue;
            }
            let Ok(r) = load(&path) else { continue };
            out.push(Listed {
                date: r.date, outcome: r.outcome.clone(), time: r.time, length: r.turns.len() as f32 / 30.0, tally: r.tally,
                kept, same_build: r.same_build(), build: r.build.clone(), start: r.start, path,
            });
        }
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.date));
    out
}

/// A replay being watched.
pub(crate) struct Viewing {
    pub(crate) playback: Playback,
    /// Its place in [`SPEEDS`].
    speed: usize,
    pub(crate) paused: bool,
}

impl Viewing {
    pub(crate) fn new(replay: Replay) -> Viewing {
        if !replay.same_build() {
            println!("replay: recorded on build {}, this is {}: it may not play back the same", replay.build, uc_net::BUILD);
        }
        Viewing { playback: Playback::new(replay), speed: 2, paused: false }
    }

    fn speed(&self) -> f32 {
        SPEEDS[self.speed]
    }
}

impl World {
    /// This run kept as a replay, once: when it's over, or left once it's gone on long enough.
    pub(crate) fn save_replay(&mut self) {
        let Some(r) = self.recorder.as_mut() else { return };
        if self.game.outcome.is_none() && self.game.turns() < SHORTEST {
            return;
        }
        let mut replay = r.finish(&mut self.game);
        replay.tally.badges = self.goals.badges.iter().filter(|b| self.game.found.contains(b)).count() as u8;
        replay.tally.sides = self.game.sides_done(&self.goals).len() as u8;
        self.recorder = None;
        self.replay_saved = save(&replay).map(|p| (p, false));
    }

    /// The game's been changed some other way than by playing it (the console): this run
    /// can't be played back, so it isn't kept.
    pub(crate) fn meddled(&mut self) {
        if self.recorder.take().is_some() {
            self.console.say("This run won't be saved as a replay");
        }
    }

    /// K at the end of a run: its replay kept for good.
    pub(crate) fn keep_replay(&mut self) {
        if let Some((path, false)) = &self.replay_saved {
            if let Some(to) = keep(path) {
                self.replay_saved = Some((to, true));
            }
        }
    }

    /// A frame of a replay: Space pauses, Up and Down change the speed, Right jumps forward,
    /// F follows the other player; and the turns due are played. With the camera left alone
    /// (`idle`), it swings round behind whoever it follows on foot, as it does behind a car.
    pub(crate) fn replay_frame(&mut self, inp: &Input, dt: f32, idle: bool) {
        let Some(v) = self.replay.as_mut() else { return };
        if inp.hit(KeyCode::Space) || inp.pad.hit(Button::South) {
            v.paused = !v.paused;
        }
        if inp.hit(KeyCode::ArrowUp) || inp.pad.hit(Button::DPadUp) {
            v.speed = (v.speed + 1).min(SPEEDS.len() - 1);
        }
        if inp.hit(KeyCode::ArrowDown) || inp.pad.hit(Button::DPadDown) {
            v.speed = v.speed.saturating_sub(1);
        }
        if inp.hit(KeyCode::ArrowRight) || inp.pad.hit(Button::DPadRight) {
            v.playback.skip(&mut self.game, &self.level, JUMP * 30);
        }
        if inp.hit(KeyCode::KeyF) && self.game.seats() > 1 {
            self.game.focus((self.game.focused() + 1) % self.game.seats());
            if let Some(p) = self.game.player() {
                self.orbit = Orbit::behind(p.fig.angle);
            }
        }
        let dt = if v.paused { 0.0 } else { dt * v.speed() };
        v.playback.frame(&mut self.game, &self.level, dt);
        if let (true, true, None, Some(p)) = (idle, self.third_person, self.game.player_vehicle(), self.game.player()) {
            let behind = Orbit::behind(p.fig.angle).yaw;
            let d = (behind - self.orbit.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            self.orbit.yaw += d * (1.5 * dt).min(1.0);
        }
    }

    /// Over a replay: where it's got to, how fast, the keys, and whether it's drifted.
    pub(crate) fn replay_overlay(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        let Some(v) = &self.replay else { return };
        let p = &v.playback;
        let secs = |turns: usize| progress::clock(turns as f32 / 30.0);
        let state = if p.done() {
            "END".to_string()
        } else if v.paused {
            "PAUSED".to_string()
        } else {
            format!("x{}", v.speed())
        };
        let follow = if self.game.seats() > 1 { format!("   following {}   F: the other player", self.game.player_name()) } else { String::new() };
        let line = format!("REPLAY  {} / {}  {state}{follow}", secs(p.turn()), secs(p.len()));
        let keys = "Space pause   Up/Down speed   Right +10 s   R from the start   V free camera   Esc menu";
        let y = h - 34.0 * s;
        o.rect(0.0, y - 4.0 * s, w, 26.0 * s, [0, 0, 0, 160]);
        o.text(12.0 * s, y, s, [255, 210, 70, 255], &line);
        o.text(12.0 * s, y + 11.0 * s, s, [200, 200, 190, 220], keys);
        let warning = match (p.drift, p.replay.same_build()) {
            (Some(t), _) => Some(format!("This replay drifted at {}: from there it no longer plays as it was recorded", secs(t as usize))),
            (None, false) => Some(format!("Recorded on another build ({}): it may not play back the same", p.replay.build)),
            _ => None,
        };
        if let Some(text) = warning {
            let x = w / 2.0 - Overlay::text_width(&text, s) / 2.0;
            o.rect(x - 4.0 * s, y - 18.0 * s, Overlay::text_width(&text, s) + 8.0 * s, 12.0 * s, [60, 0, 0, 190]);
            o.text(x, y - 16.0 * s, s, [255, 220, 200, 255], &text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(1_792_591_380), (2026, 10, 21, 14, 3, 0));
        assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0, 0));
        let now = 1_792_591_380;
        assert_eq!(ago(now - 30, now), "just now");
        assert_eq!(ago(now - 60, now), "1 minute ago");
        assert_eq!(ago(now - 7300, now), "2 hours ago");
        assert_eq!(ago(now - 90_000, now), "yesterday");
        assert_eq!(ago(now - 3 * 86400, now), "3 days ago");
        assert_eq!(ago(now - 30 * 86400, now), "2026-09-21");
    }

    /// A run started and played as the viewer does (the game made from the command line and
    /// the progress, frames of uneven length, a fly-by skipped) plays back the same from its
    /// saved bytes, through the same loading a replay from the menu goes through: a mission
    /// with a weapon and dogs, and an endless run. Skipped without an install.
    #[test]
    fn a_run_in_the_viewer_plays_back_the_same() {
        let Some(game) = uc_formats::default_install() else { return };
        let mission = Args { game, mission: "police1".into(), weapon: uc_game::Weapon::Pistol, dogs: 2, ..Args::default() };
        let endless = Args { endless: true, seed: Some(5), weapon: uc_game::Weapon::None, dogs: 0, ..mission.clone() };
        for args in [mission, endless] {
            let mut w = World::load(args.clone()).unwrap();
            for frame in 0..900 {
                let dt = [0.007, 0.016, 0.040, 0.025][frame % 4];
                let c = uc_net::bot_input(0, frame / 40);
                if w.game.cutscene.is_some() && frame > 30 {
                    w.game.skip_cutscene();
                }
                w.game.update(dt, &c, &w.level);
                if let Some(r) = w.recorder.as_mut() {
                    r.take(&mut w.game);
                }
            }
            let replay = w.recorder.as_mut().expect("the run is kept").finish(&mut w.game);
            let replay = Replay::from_bytes(&replay.to_bytes()).unwrap();
            let mut back = World::load_replay(args, replay).unwrap();
            assert!(back.recorder.is_none() && back.race.is_none());
            let v = back.replay.as_mut().unwrap();
            v.playback.skip(&mut back.game, &back.level, usize::MAX);
            assert_eq!(v.playback.drift, None, "{}", w.args.mission);
            assert_eq!(back.game.turns(), w.game.turns());
            assert_eq!(back.game.state_hash(), w.game.state_hash());
        }
    }
}
