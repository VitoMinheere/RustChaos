//! Urban Chaos sound, played from your install: sound effects placed in the world, music
//! that crossfades between tracks, and mission voice lines (`talk2\<mission>.ucm<point>.wav`).
//!
//! Sounds are named by their path under `data\sfx\1622` (e.g. `"f_PUNCH1.wav"`,
//! `"Footsteps/HardStep1.wav"`), decoded once and cached. Everything degrades quietly: with
//! no audio device, [`Audio::new`] fails and the game runs silent; a missing file is skipped.

pub mod sounds;
pub mod wav;

use glam::Vec3;
use rodio::{ChannelCount, SampleRate, Source};
use std::collections::{HashMap, VecDeque};
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
pub use wav::Clip;

/// Sounds further than this (world units; a map square is 256) are inaudible.
const HEARING_RANGE: f32 = 12.0 * 256.0;
/// Seconds for music to fade fully in or out.
const FADE_SECONDS: f32 = 1.5;

/// Live control of a playing sound.
struct Control {
    gain: AtomicU32,
    /// Playback rate (1 = as recorded): pitch for engines.
    rate: AtomicU32,
    stop: AtomicBool,
}

impl Control {
    fn new(gain: f32) -> Arc<Control> {
        Arc::new(Control { gain: AtomicU32::new(gain.to_bits()), rate: AtomicU32::new(1f32.to_bits()), stop: AtomicBool::new(false) })
    }
    fn set_gain(&self, g: f32) {
        self.gain.store(g.to_bits(), Ordering::Relaxed);
    }
    fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }
    fn set_rate(&self, r: f32) {
        self.rate.store(r.to_bits(), Ordering::Relaxed);
    }
    fn rate(&self) -> f32 {
        f32::from_bits(self.rate.load(Ordering::Relaxed))
    }
}

/// A clip played in stereo with left/right gains, optionally looping.
struct Voice {
    clip: Arc<Clip>,
    /// Position in frames (fractional when the rate isn't 1).
    frame: f64,
    left: f32,
    right: f32,
    looped: bool,
    control: Arc<Control>,
    out: [f32; 2],
    second: bool,
}

impl Iterator for Voice {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.second {
            self.second = false;
            return Some(self.out[1]);
        }
        if self.control.stop.load(Ordering::Relaxed) {
            return None;
        }
        let ch = self.clip.channels as usize;
        let frames = self.clip.samples.len() / ch.max(1);
        if self.frame as usize >= frames {
            if !self.looped || frames == 0 {
                return None;
            }
            self.frame %= frames as f64;
        }
        let s = &self.clip.samples[self.frame as usize * ch..];
        let (l, r) = if ch >= 2 { (s[0], s[1]) } else { (s[0], s[0]) };
        let g = self.control.gain();
        self.out = [l * self.left * g, r * self.right * g];
        self.frame += self.control.rate() as f64;
        self.second = true;
        Some(self.out[0])
    }
}

impl Source for Voice {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        NonZero::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        NonZero::new(self.clip.rate).unwrap_or(NonZero::new(22050).unwrap())
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Where sounds are heard from: the camera.
#[derive(Debug, Clone, Copy)]
pub struct Listener {
    /// World units.
    pub pos: Vec3,
    /// Unit vector to the listener's right.
    pub right: Vec3,
}

struct Track {
    name: String,
    control: Arc<Control>,
    level: f32,
    target: f32,
}

/// The music world (1-9, the `data\sfx\1622\musicNN` folder) for a mission: from
/// `levels\mworlds.txt` like the original (`ELEV_load_level`), else the mission file's own
/// value, at least 1.
pub fn music_world(game: &Path, mission: &str, from_mission: u8) -> u8 {
    let table = uc_formats::resolve(game, "levels\\mworlds.txt").and_then(|p| std::fs::read_to_string(p).ok());
    table
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case(mission))
        .and_then(|(_, w)| w.trim().parse::<u8>().ok())
        .unwrap_or(from_mission)
        .clamp(1, 9)
}

pub struct Audio {
    sink: rodio::MixerDeviceSink,
    game: PathBuf,
    cache: HashMap<String, Option<Arc<Clip>>>,
    music: Vec<Track>,
    ambience: Option<Arc<Control>>,
    engine: Option<Arc<Control>>,
    /// A helicopter's rotors (`S_HELI`), looped while one's in the air.
    rotor: Option<Arc<Control>>,
    rng: u32,
    /// The line being said, and the ones waiting behind it. A mission's voices are said one
    /// at a time: skipping a fly-by or a message brings the next one up before this one has
    /// finished, and two voices at once are just noise.
    speaking: Option<Said>,
    waiting: VecDeque<Arc<Clip>>,
    /// The last sound a mission asked for that can be taken over by the next one
    /// (`play_glue_wave`'s type 1, which passes `MFX_REPLACE`).
    effect: Option<Arc<Control>>,
    pub sfx_volume: f32,
    pub music_volume: f32,
}

/// How many spoken lines will wait their turn. Skipping through a run of them shouldn't leave
/// the mission talking about the last scene halfway through the next one.
const MAX_WAITING: usize = 3;

/// A line being said: its handle, and the seconds left of it.
struct Said {
    control: Arc<Control>,
    left: f32,
}

impl Audio {
    /// Opens the default audio device. `game` is the install, where the sounds are read from.
    pub fn new(game: &Path) -> Result<Audio, String> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| format!("no audio device: {e}"))?;
        sink.log_on_drop(false);
        Ok(Audio {
            sink, game: game.to_path_buf(), cache: HashMap::new(), music: Vec::new(), ambience: None, engine: None, rotor: None, rng: 12345,
            speaking: None, waiting: VecDeque::new(), effect: None,
            sfx_volume: 0.8, music_volume: 0.55,
        })
    }

    /// Loads (and caches) a file relative to the install.
    fn load(&mut self, rel: &str) -> Option<Arc<Clip>> {
        if let Some(c) = self.cache.get(rel) {
            return c.clone();
        }
        let clip = uc_formats::resolve(&self.game, rel)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| wav::decode(&b).ok())
            .map(Arc::new);
        self.cache.insert(rel.to_string(), clip.clone());
        clip
    }

    fn sfx(&mut self, name: &str) -> Option<Arc<Clip>> {
        self.load(&format!("data\\sfx\\1622\\{name}"))
    }

    fn start(&self, clip: Arc<Clip>, left: f32, right: f32, gain: f32, looped: bool) -> Arc<Control> {
        let control = Control::new(gain);
        let voice = Voice { clip, frame: 0.0, left, right, looped, control: control.clone(), out: [0.0; 2], second: false };
        self.sink.mixer().add(voice);
        control
    }

    /// Picks one of `names` at random.
    pub fn pick<'a>(&mut self, names: &[&'a str]) -> &'a str {
        self.rng = self.rng.wrapping_mul(69069).wrapping_add(1);
        names[(self.rng >> 16) as usize % names.len()]
    }

    /// Plays a sound effect without position.
    pub fn play(&mut self, name: &str, volume: f32) {
        if let Some(c) = self.sfx(name) {
            let g = volume * self.sfx_volume;
            self.start(c, 1.0, 1.0, g, false);
        }
    }

    /// Plays a sound effect at world position `pos`: quieter with distance, panned.
    pub fn play_at(&mut self, name: &str, volume: f32, pos: Vec3, listener: &Listener) {
        let d = pos - listener.pos;
        let fade = (1.0 - d.length() / HEARING_RANGE).clamp(0.0, 1.0);
        if fade <= 0.0 {
            return;
        }
        let Some(c) = self.sfx(name) else { return };
        let pan = d.normalize_or_zero().dot(listener.right).clamp(-1.0, 1.0) * 0.7;
        let (l, r) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
        let g = volume * self.sfx_volume * fade * fade;
        self.start(c, l, r, g, false);
    }

    /// Plays a sound effect from the direction of `pos`, at full volume however far off it is:
    /// for the world's own far-off sounds (`PlayAmbient3D`), which are put 16 squares away,
    /// past where [`Audio::play_at`] would have faded them out.
    pub fn play_toward(&mut self, name: &str, volume: f32, pos: Vec3, listener: &Listener) {
        let Some(c) = self.sfx(name) else { return };
        let d = pos - listener.pos;
        let pan = d.normalize_or_zero().dot(listener.right).clamp(-1.0, 1.0) * 0.7;
        let (l, r) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
        self.start(c, l, r, volume * self.sfx_volume, false);
    }

    /// Like [`Audio::play_at`], but this one takes over from the last sound played this way
    /// rather than sounding over it: `play_glue_wave`'s type 1 asks for `MFX_REPLACE`, which
    /// is how a mission keeps one long effect going without stacking them up.
    pub fn play_at_replacing(&mut self, name: &str, volume: f32, pos: Vec3, listener: &Listener) {
        if let Some(c) = self.effect.take() {
            c.stop.store(true, Ordering::Relaxed);
        }
        let d = pos - listener.pos;
        let fade = (1.0 - d.length() / HEARING_RANGE).clamp(0.0, 1.0);
        if fade <= 0.0 {
            return;
        }
        let Some(c) = self.sfx(name) else { return };
        let pan = d.normalize_or_zero().dot(listener.right).clamp(-1.0, 1.0) * 0.7;
        let (l, r) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
        let g = volume * self.sfx_volume * fade * fade;
        self.effect = Some(self.start(c, l, r, g, false));
    }

    /// Plays a mission's recorded line for event point `point`, if it has one. Returns
    /// whether it did.
    pub fn voice(&mut self, mission: &str, point: usize) -> bool {
        match self.load(&format!("talk2\\{mission}.ucm{point}.wav")) {
            Some(c) => {
                self.say(c);
                true
            }
            None => false,
        }
    }

    /// Plays line `line` (0 = A, 1 = B...) of a mission's recorded conversation at point
    /// `point` (`talk2\<mission>.ucm<point><letter>.wav`, as `EWAY_talk_conv`).
    pub fn conversation_line(&mut self, mission: &str, point: usize, line: usize) -> bool {
        let letter = (b'A' + (line as u8).min(25)) as char;
        match self.load(&format!("talk2\\{mission}.ucm{point}{letter}.wav")) {
            Some(c) => {
                self.say(c);
                true
            }
            None => false,
        }
    }

    /// Says a line, or stands it behind one already being said.
    fn say(&mut self, clip: Arc<Clip>) {
        if self.speaking.is_some() {
            if self.waiting.len() < MAX_WAITING {
                self.waiting.push_back(clip);
            }
            return;
        }
        self.speak(clip);
    }

    fn speak(&mut self, clip: Arc<Clip>) {
        let left = clip.seconds();
        let control = self.start(clip, 1.0, 1.0, self.sfx_volume, false);
        self.speaking = Some(Said { control, left });
    }

    /// Stops whatever is being said and drops the rest: a mission restarting or the player
    /// going back to the menu shouldn't be talked over by the one before.
    pub fn hush(&mut self) {
        if let Some(s) = self.speaking.take() {
            s.control.stop.store(true, Ordering::Relaxed);
        }
        if let Some(c) = self.effect.take() {
            c.stop.store(true, Ordering::Relaxed);
        }
        self.waiting.clear();
    }

    /// Whether a line is being said, and how many are waiting their turn.
    pub fn speaking(&self) -> (bool, usize) {
        (self.speaking.is_some(), self.waiting.len())
    }

    /// A looping background sound (`None` for silence).
    pub fn ambience(&mut self, name: Option<&str>, volume: f32) {
        if let Some(a) = self.ambience.take() {
            a.stop.store(true, Ordering::Relaxed);
        }
        if let Some(c) = name.and_then(|n| self.sfx(n)) {
            self.ambience = Some(self.start(c, 1.0, 1.0, volume * self.sfx_volume, true));
        }
    }

    /// Changes the volume of the ambience playing, if any.
    pub fn ambience_volume(&mut self, volume: f32) {
        if let Some(a) = &self.ambience {
            a.set_gain(volume * self.sfx_volume);
        }
    }

    /// A helicopter's rotors at `volume` (`Some`), or none in the air (`None`).
    pub fn rotor(&mut self, volume: Option<f32>) {
        match volume {
            Some(v) => {
                if self.rotor.is_none() {
                    if let Some(c) = self.sfx("heli.wav") {
                        self.rotor = Some(self.start(c, 1.0, 1.0, 0.0, true));
                    }
                }
                if let Some(r) = &self.rotor {
                    r.set_gain(self.sfx_volume * v);
                }
            }
            None => {
                if let Some(r) = self.rotor.take() {
                    r.stop.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    /// The car engine: running (`Some(revs)`, 0 idle .. 1 flat out) or off (`None`).
    pub fn engine(&mut self, revs: Option<f32>) {
        match revs {
            Some(r) => {
                if self.engine.is_none() {
                    if let Some(c) = self.sfx("CarXLoop.wav") {
                        self.engine = Some(self.start(c, 1.0, 1.0, 0.0, true));
                    }
                }
                if let Some(e) = &self.engine {
                    e.set_rate(0.7 + r.clamp(0.0, 1.0) * 1.0);
                    e.set_gain(self.sfx_volume * (0.35 + r.clamp(0.0, 1.0) * 0.3));
                }
            }
            None => {
                if let Some(e) = self.engine.take() {
                    e.stop.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    /// Crossfades to music `name` (looping), or to silence. Asking for the track already
    /// playing changes nothing.
    pub fn music(&mut self, name: Option<&str>) {
        let current = self.music.iter().find(|t| t.target > 0.0).map(|t| t.name.as_str());
        if current == name {
            return;
        }
        for t in &mut self.music {
            t.target = 0.0;
        }
        let Some(name) = name else { return };
        if let Some(t) = self.music.iter_mut().find(|t| t.name == name) {
            t.target = 1.0;
            return;
        }
        if let Some(c) = self.sfx(name) {
            let control = self.start(c, 1.0, 1.0, 0.0, true);
            self.music.push(Track { name: name.to_string(), control, level: 0.0, target: 1.0 });
        }
    }

    /// Plays a one-off music cue (a win or lose sting) over silence.
    pub fn sting(&mut self, name: &str) {
        self.music(None);
        if let Some(c) = self.sfx(name) {
            self.start(c, 1.0, 1.0, self.music_volume, false);
        }
    }

    /// Advances music fades and the queue of spoken lines.
    pub fn update(&mut self, dt: f32) {
        // The line being said runs down, and the next one waiting starts when it's over.
        if let Some(s) = &mut self.speaking {
            s.left -= dt;
            if s.left <= 0.0 {
                self.speaking = None;
                if let Some(next) = self.waiting.pop_front() {
                    self.speak(next);
                }
            }
        }
        let step = dt / FADE_SECONDS;
        for t in &mut self.music {
            t.level = if t.level < t.target { (t.level + step).min(t.target) } else { (t.level - step).max(t.target) };
            t.control.set_gain(t.level * self.music_volume);
        }
        self.music.retain(|t| {
            let done = t.level <= 0.0 && t.target <= 0.0;
            if done {
                t.control.stop.store(true, Ordering::Relaxed);
            }
            !done
        });
    }
}
