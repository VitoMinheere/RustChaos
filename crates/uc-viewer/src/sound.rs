//! Which of the original recordings each game sound plays, and the music for each mood.

use glam::Vec3;
use uc_audio::{Audio, Listener};
use uc_game::{Mood, Sfx, Theme};
use uc_world::character::person as ptype;

fn numbered(prefix: &str, n: std::ops::RangeInclusive<u32>, suffix: &str) -> Vec<String> {
    n.map(|i| format!("{prefix}{i}{suffix}")).collect()
}

fn pick(audio: &mut Audio, names: &[String]) -> String {
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    audio.pick(&refs).to_string()
}

/// The recordings for a gang member's voice: their folder, the prefix their lines use and
/// how many taunts they have.
fn gang_voice(kind: u8) -> Option<(&'static str, &'static str, u32)> {
    match kind {
        ptype::THUG_RASTA => Some(("bthug1", "GRASTA", 7)),
        ptype::THUG_RED => Some(("wthug2", "GRED", 5)),
        ptype::THUG_GREY => Some(("wthug1", "GGREY", 5)),
        _ => None,
    }
}

/// Plays the sound for one game event at `pos` (world units).
pub fn play(audio: &mut Audio, sfx: Sfx, pos: Vec3, l: &Listener, mission: &str) {
    let at = |audio: &mut Audio, names: Vec<String>, vol: f32| {
        let n = pick(audio, &names);
        audio.play_at(&n, vol, pos, l);
    };
    match sfx {
        Sfx::Swing => at(audio, numbered("f_knSWP", 1..=4, ".wav"), 0.3),
        Sfx::Hit { heavy } => at(audio, numbered("f_PUNCH", 1..=5, ".wav"), if heavy { 1.0 } else { 0.75 }),
        Sfx::Hurt { kind: ptype::DARCI } => at(audio, numbered("darci/hit ", 1..=5, ".WAV"), 0.8),
        // A cop's pain (`PainSound`, Sound.cpp:520): three times in four one of the first
        // two, else any of the five. The gangs', the hostage's, the workman's and the
        // tramp's are in the `misc` folder, and Roper's beside his taunts, neither of which
        // the release ships.
        Sfx::Hurt { kind: ptype::COP } => {
            let all = ["COP-PAIN01", "COP-PAIN02", "COP-PAIN03", "COP-PAIN03B", "COP-PAIN04"];
            let first = audio.pick(&["a", "b", "c", "d"]) != "d";
            let names = if first { &all[..2] } else { &all[..] };
            at(audio, names.iter().map(|n| format!("cop/{n}.wav")).collect(), 0.8);
        }
        Sfx::Hurt { .. } => {}
        Sfx::MibExplode => at(audio, vec!["MIBexplo1.wav".into()], 1.0),
        // The Men in Black levitate as they die (`S_MIB_LEVITATE`, Person.cpp:1140).
        Sfx::KnockedOut { kind: ptype::MIB1 | ptype::MIB2 | ptype::MIB3 } => at(audio, vec!["MIBlevitate1.wav".into()], 1.0),
        Sfx::KnockedOut { kind } => {
            let names = match kind {
                ptype::DARCI => numbered("darci/die ", 1..=3, ".WAV"),
                // `DieSound`: the cops' own (`S_COP_DIE_START` to `_END`).
                ptype::COP => ["COP-DIE01", "COP-DIE02B", "COP-DIE03", "COP-DIE04", "COP-DIE04B"].iter().map(|n| format!("cop/{n}.wav")).collect(),
                ptype::SLAG_TART | ptype::SLAG_FATUGLY | ptype::HOSTAGE => numbered("f_fDIE", 1..=2, ".wav"),
                _ => numbered("f_mDIE", 1..=2, ".wav"),
            };
            at(audio, names, 0.9);
        }
        Sfx::Footstep { player, sound } => {
            let names = match sound {
                Some((a, b)) => (a..=b).filter_map(uc_audio::sounds::name).map(|n| n.replace('\\', "/")).collect(),
                None => Vec::new(),
            };
            let names = if names.is_empty() { numbered("Footsteps/HardStep", 1..=4, ".wav") } else { names };
            at(audio, names, if player { 0.3 } else { 0.2 });
        }
        // Only Darci grunts with the effort: the gangs have no such recordings.
        Sfx::Jump { kind: ptype::DARCI } => at(audio, numbered("darci/effort ", 1..=3, ".WAV"), 0.6),
        Sfx::Jump { .. } => {}
        // `SOUND_Curious`: the gangs' uncertain lines (`S_BTHUG_UNCERTAIN_START` and on).
        Sfx::Curious { kind } => {
            let range = match kind {
                ptype::THUG_RASTA => Some(210..=216),
                ptype::THUG_GREY => Some(223..=231),
                ptype::THUG_RED => Some(238..=245),
                _ => None,
            };
            if let Some(r) = range {
                at(audio, r.filter_map(uc_audio::sounds::name).map(|n| n.replace('\\', "/")).collect(), 0.9);
            }
        }
        // Darci's; Roper's (which the original gives the civilians too) aren't shipped.
        Sfx::ScreamFall { kind: ptype::DARCI } => at(audio, vec!["darci/scream fall.WAV".into(), "darci/scream fall 2.WAV".into()], 1.0),
        Sfx::ScreamFall { .. } => {}
        Sfx::Land => at(audio, vec!["Footsteps/StreetLand1.wav".into()], 0.6),
        Sfx::Grab => at(audio, vec!["JudoChop1.wav".into()], 0.8),
        Sfx::Arrest => at(
            audio,
            ["DARCI-ARREST01", "DARCI-ARREST01B", "DARCI-ARREST02", "DARCI-ARREST02B", "under arrest"]
                .iter()
                .map(|n| format!("darci/{n}.wav"))
                .collect(),
            1.0,
        ),
        Sfx::Freeze(line) => at(audio, vec![["darci/freeze.wav", "darci/freeze tk2.wav", "darci/stop police tk2.wav"][line.min(2) as usize].to_string()], 1.0),
        Sfx::Taunt { kind } => {
            let mut names: Vec<String> = match kind {
                ptype::DARCI => (1..=19).map(|i| format!("darci/DARCI-TAUNT{i:02}.wav")).collect(),
                _ => match gang_voice(kind) {
                    // Each gang has its own taunts, five or seven of them with an alternative
                    // take of most: `pick` drops the names it can't find.
                    Some((dir, name, last)) => (1..=last)
                        .flat_map(|i| ["", "B"].map(|b| format!("{dir}/{name}-TAUNT{i:02}{b}.wav")))
                        .collect(),
                    None => return,
                },
            };
            // Not every time: about one knockout in three.
            names.extend(std::iter::repeat_n(String::new(), names.len() * 2));
            let n = pick(audio, &names);
            if !n.is_empty() {
                audio.play_at(&n, 1.0, pos, l);
            }
        }
        Sfx::Alert { kind } => {
            let Some((voice, ..)) = gang_voice(kind) else { return };
            at(audio, vec![format!("{voice}/hey you.wav"), format!("{voice}/hey you 2.wav")], 0.9);
        }
        Sfx::Voice(point) => {
            audio.voice(mission, point);
        }
        Sfx::ConversationLine(point, line) => {
            audio.conversation_line(mission, point, line);
        }
        Sfx::Block => at(audio, vec!["ImpactSml.wav".into()], 0.7),
        Sfx::Stab => at(audio, numbered("Stab", 1..=3, ".wav"), 0.9),
        Sfx::BatHit => at(audio, vec!["BatHit1.wav".into(), "DarciBatHit1.wav".into(), "DarciBatHit2.wav".into()], 1.0),
        Sfx::Shot(uc_game::Weapon::Shotgun) => at(audio, vec!["f_sSHOT1.wav".into()], 1.0),
        Sfx::Shot(uc_game::Weapon::Sniper) => at(audio, vec!["General/AK1.wav".into()], 1.0),
        Sfx::Shot(uc_game::Weapon::Ak47) => at(audio, vec!["General/AK1.wav".into()], 0.9),
        Sfx::Shot(_) => at(audio, numbered("f_pSHOT", 1..=2, ".wav"), 1.0),
        Sfx::Click => at(audio, vec!["General/Click2.wav".into()], 0.8),
        Sfx::Zip => at(audio, vec!["ZipWire.wav".into()], 0.8),
        Sfx::Pickup => at(audio, vec!["General/PickupSwish2.wav".into()], 0.8),
        Sfx::Found => at(audio, vec!["sfx080799+/HiddenItemRev2.wav".into()], 0.9),
        // The radio's in her ear, not out in the street.
        Sfx::Radio => audio.play("RadioMessage.wav", 0.9),
        Sfx::Siren => at(audio, vec!["f_SIREN1.wav".into(), "f_SIREN2.wav".into(), "f_SIREN3.wav".into(), "General/CopSiren1.wav".into()], 1.0),
        Sfx::Crash => at(audio, vec!["MedCrash2.wav".into(), "Impact1.wav".into(), "Impact2.wav".into()], 1.0),
        Sfx::Cone => at(audio, vec!["Cone1.wav".into()], 0.8),
        Sfx::Skid => at(audio, numbered("Skid", 1..=3, ".wav"), 0.8),
        Sfx::Horn => at(audio, numbered("Horn", 1..=3, ".wav"), 0.8),
        Sfx::CarDoor => at(audio, vec!["DoorOpenCls1.wav".into()], 0.8),
        // `S_EXPLODE_START + 0..2` (fallen/Headers/sound_id.h), as `PYRO_construct` plays
        // for a fireball.
        Sfx::Explosion => at(audio, numbered("f_EXPLD", 1..=3, ".wav"), 1.0),
        // A mission asking for one of the game's own sounds by id.
        Sfx::ById { id, replace } => {
            if let Some(name) = uc_audio::sounds::name(id) {
                if replace {
                    audio.play_at_replacing(name, 1.0, pos, l);
                } else {
                    audio.play_at(name, 1.0, pos, l);
                }
            }
        }
        Sfx::Won => audio.sting("GeneralMusic/Complete.wav"),
        Sfx::Lost => audio.sting("GeneralMusic/Dead2.wav"),
        Sfx::Chaos => audio.sting(CHAOS),
    }
}

/// The title screen's and the menus' loop (`MUSIC_MODE_FRONTEND`, `S_TUNE_FRONTEND`).
pub const FRONTEND: &str = "GeneralMusic/FrontLoop.wav";

/// The countdown's tune (`S_TUNE_TIMER1`), played once as it drops under 30 seconds.
pub const TIMER: &str = "GeneralMusic/Timer122.wav";

/// Losing with the player still standing (`S_TUNE_CHAOS`).
const CHAOS: &str = "GeneralMusic/ChaosTheEnd.wav";

/// The music for a mood, from the mission's music world (`data\sfx\1622\musicNN\darci`):
/// `MUSIC_play_the_mode`'s table (fallen/Source/music.cpp:166), under the names the tracks
/// shipped as. Where a mode has two tracks the original only ever picks the first
/// (`start + rand() % (end - start)`); here the fight themes take turns.
pub fn music(mood: Mood, world: u8, fights: u32) -> Option<String> {
    let dir = format!("music{world:02}/darci");
    let album = |t: &str| Some(format!("GeneralMusic/AlbumTracks/{t}.wav"));
    match mood {
        // Alternate the two fight themes from one fight to the next.
        Mood::Fight => Some(format!("{dir}/Fight{}.wav", 1 + fights % 2)),
        Mood::Sprint => Some(format!("{dir}/Sprint1.wav")),
        Mood::Crawl => Some(format!("{dir}/Crawl.wav")),
        Mood::Drive => Some(format!("{dir}/Driving1.wav")),
        Mood::Theme(Theme::CombatTraining) => album("BleepTune"),
        Mood::Theme(Theme::AssaultTraining) => album("TourDeForce"),
        Mood::Theme(Theme::DrivingTraining) => album("OomaGabba"),
        Mood::Theme(Theme::Reckoning) => album("ADFoundation"),
        // The timer's tune is a one-off, not a loop: see [`TIMER`].
        Mood::Calm | Mood::Timer | Mood::Won | Mood::Lost | Mood::Chaos => None,
    }
}

#[cfg(test)]
mod tests {
    /// The voices added for #298 are in the release: the cops' pain and death, and Darci's
    /// heart giving out. Skipped without an install.
    #[test]
    fn the_cops_voices_and_the_heart_are_shipped() {
        let Some(game) = uc_formats::default_install() else { return };
        let names = ["COP-PAIN01", "COP-PAIN02", "COP-PAIN03", "COP-PAIN03B", "COP-PAIN04", "COP-DIE01", "COP-DIE02B", "COP-DIE03", "COP-DIE04", "COP-DIE04B"]
            .iter().map(|n| format!("cop/{n}.wav"))
            .chain(std::iter::once("borrowed/heartfail.wav".to_string()));
        for n in names {
            let rel = format!(r"data\sfx\1622\{}", n.replace('/', "\\"));
            assert!(uc_formats::resolve(&game, &rel).is_some(), "{n} isn't in the install");
        }
    }
}
