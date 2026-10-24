//! The sound while playing: effects, ambience, weather and music.

use crate::*;

/// A sound id's file name, for the weather's cues.
pub(crate) fn sound_name(id: usize) -> Option<&'static str> {
    uc_audio::sounds::name(id)
}

/// Sound and music state.
pub(crate) struct Sound {
    pub(crate) audio: Audio,
    /// The mission's music folder (1-9).
    pub(crate) music_world: u8,
    /// Fights so far (to alternate the two fight themes).
    pub(crate) fights: u32,
    pub(crate) mood: Mood,
    /// The player is in a warehouse, with this ambience.
    pub(crate) indoors: Option<u8>,
    /// The world's loops and far-off sounds out of doors.
    pub(crate) weather: Weather,
    /// Game turns owed to the weather (20 a second).
    pub(crate) turns: f32,
}

impl Sound {
    pub(crate) fn new(args: &Args) -> Option<Sound> {
        if args.mute {
            return None;
        }
        let audio = match Audio::new(&args.game) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("{e}; playing without sound");
                return None;
            }
        };
        Some(Sound { audio, music_world: 1, fights: 0, mood: Mood::Calm, indoors: None, weather: Weather::new(Wtype::BusyCity, false), turns: 0.0 })
    }

    /// A mission starts: its music world and the city's ambience.
    pub(crate) fn start(&mut self, world: &World) {
        self.music_world = uc_audio::music_world(&world.args.game, &world.level.name, world.level.mission.music_world);
        self.mood = Mood::Calm;
        self.indoors = None;
        self.audio.hush();
        self.audio.music(None);
        self.weather = Weather::new(Wtype::of(world.level.map.texture_set), Game::raining(&world.level));
        self.turns = 0.0;
        self.audio.ambience(self.weather.looped().and_then(sound_name), 0.35);
        self.audio.bed(self.weather.world.bed().and_then(sound_name), 0.35);
    }

    /// Back from the pause menu: the ambience and the music as they were, which the next
    /// update starts again (as if just come out of a warehouse).
    pub(crate) fn resume(&mut self) {
        self.indoors = Some(u8::MAX);
    }

    /// Back to the menu: quiet.
    pub(crate) fn stop(&mut self) {
        self.audio.hush();
        self.audio.music(None);
        self.audio.ambience(None, 0.0);
        self.audio.bed(None, 0.0);
        self.audio.engine(None);
        self.audio.rotor(None);
        self.audio.siren(None);
    }

    /// Plays this frame's game sounds and keeps the music in step with the action.
    pub(crate) fn update(&mut self, world: &mut World, cam: &Camera, dt: f32) {
        let fwd = (cam.target - cam.pos).normalize_or_zero();
        let listener = Listener { pos: cam.pos * uc_world::UNIT, right: fwd.cross(Vec3::Y).normalize_or_zero() };
        for (sfx, pos) in world.game.take_sounds() {
            sound::play(&mut self.audio, sfx, pos, &listener, &world.level.name);
        }
        let mood = world.game.mood();
        let indoors = world.game.indoors();
        let moved = indoors != self.indoors;
        if moved {
            // Inside a warehouse the world's sounds stop (`process_weather`).
            self.indoors = indoors;
            let out = indoors.is_none();
            self.audio.ambience(self.weather.looped().filter(|_| out).and_then(sound_name), 0.35);
            self.audio.bed(self.weather.world.bed().filter(|_| out).and_then(sound_name), 0.35);
        }
        if indoors.is_none() {
            // Quieter the higher the camera is over the ground (`rain_gain`, 255 at the
            // ground and nothing 16 squares up), for the city's ambience as for the rain's.
            let at = cam.pos * uc_world::UNIT;
            let above = at.y - world.level.map.ground_height(at.x, at.z);
            self.audio.ambience_volume(0.35 * Weather::loop_gain(above));
            // The world's far-off sounds, a game turn at a time (`new_outdoors_effects`).
            let height = at.y;
            let me = world.game.player().map_or(listener.pos, |p| p.fig.pos);
            self.turns = (self.turns + dt * 20.0).min(4.0);
            while self.turns >= 1.0 {
                self.turns -= 1.0;
                for cue in self.weather.turn(me, height) {
                    let Some(name) = sound_name(cue.id) else { continue };
                    match cue.place {
                        Place::Ambient => self.audio.play(name, cue.gain * 0.6),
                        Place::At(at) => self.audio.play_toward(name, cue.gain * 0.6, at, &listener),
                    }
                }
            }
        }
        if mood != self.mood || moved {
            if mood == Mood::Fight && mood != self.mood {
                self.fights += 1;
            }
            self.mood = mood;
            if !matches!(mood, Mood::Won | Mood::Lost) {
                // A warehouse with an ambience plays it in place of the music.
                let track = match indoors {
                    Some(a @ 1..=4) => Some(uc_game::ware::AMBIENCE[a as usize - 1].to_string()),
                    _ => sound::music(mood, self.music_world, self.fights),
                };
                self.audio.music(track.as_deref());
            }
        }
        // The engine of the car Darci drives.
        let revs = world.game.player_vehicle().map(|v| (v.speed.abs() / 3000.0).min(1.0));
        self.audio.engine(revs);
        // The nearest helicopter's rotors, fading out 24 squares off.
        let rotor = world.game.chopper_noise(listener.pos).map(|at| {
            let fade = (1.0 - at.distance(listener.pos) / (24.0 * 256.0)).clamp(0.0, 1.0);
            0.6 * fade * fade
        });
        self.audio.rotor(rotor);
        // The nearest siren going, fading out 32 squares off.
        let siren = world.game.siren_noise(listener.pos).map(|at| {
            let fade = (1.0 - at.distance(listener.pos) / (32.0 * 256.0)).clamp(0.0, 1.0);
            0.8 * fade * fade
        });
        self.audio.siren(siren);
        self.audio.update(dt);
    }
}
