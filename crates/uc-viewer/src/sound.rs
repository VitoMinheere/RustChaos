//! Which of the original recordings each game sound plays, and the music for each mood.

use glam::Vec3;
use uc_audio::{Audio, Listener};
use uc_game::{Mood, Sfx};
use uc_world::character::person as ptype;

fn numbered(prefix: &str, n: std::ops::RangeInclusive<u32>, suffix: &str) -> Vec<String> {
    n.map(|i| format!("{prefix}{i}{suffix}")).collect()
}

fn pick(audio: &mut Audio, names: &[String]) -> String {
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    audio.pick(&refs).to_string()
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
        Sfx::Hurt { .. } => {}
        Sfx::KnockedOut { kind } => {
            let names = match kind {
                ptype::DARCI => numbered("darci/die ", 1..=3, ".WAV"),
                ptype::SLAG_TART | ptype::SLAG_FATUGLY | ptype::HOSTAGE => numbered("f_fDIE", 1..=2, ".wav"),
                _ => numbered("f_mDIE", 1..=2, ".wav"),
            };
            at(audio, names, 0.9);
        }
        Sfx::Footstep { player } => at(audio, numbered("Footsteps/HardStep", 1..=4, ".wav"), if player { 0.3 } else { 0.2 }),
        Sfx::Jump => at(audio, numbered("darci/effort ", 1..=3, ".WAV"), 0.6),
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
        Sfx::Taunt => {
            // Not every time: about one knockout in three.
            let mut names: Vec<String> = (1..=19).map(|i| format!("darci/DARCI-TAUNT{i:02}.wav")).collect();
            names.extend(std::iter::repeat_n(String::new(), 38));
            let n = pick(audio, &names);
            if !n.is_empty() {
                audio.play_at(&n, 1.0, pos, l);
            }
        }
        Sfx::Alert { kind } => {
            let voice = match kind {
                ptype::THUG_RASTA => "bthug1",
                ptype::THUG_RED => "wthug2",
                ptype::THUG_GREY => "wthug1",
                _ => return,
            };
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
        Sfx::Shot(uc_game::Weapon::Ak47) => at(audio, vec!["General/AK1.wav".into()], 0.9),
        Sfx::Shot(_) => at(audio, numbered("f_pSHOT", 1..=2, ".wav"), 1.0),
        Sfx::Click => at(audio, vec!["General/Click2.wav".into()], 0.8),
        Sfx::Zip => at(audio, vec!["ZipWire.wav".into()], 0.8),
        Sfx::Pickup => at(audio, vec!["General/PickupSwish2.wav".into()], 0.8),
        Sfx::Crash => at(audio, vec!["MedCrash2.wav".into(), "Impact1.wav".into(), "Impact2.wav".into()], 1.0),
        Sfx::Cone => at(audio, vec!["Cone1.wav".into()], 0.8),
        Sfx::Skid => at(audio, numbered("Skid", 1..=3, ".wav"), 0.8),
        Sfx::CarDoor => at(audio, vec!["DoorOpenCls1.wav".into()], 0.8),
        Sfx::Won => audio.sting("GeneralMusic/Complete.wav"),
        Sfx::Lost => audio.sting("GeneralMusic/Dead2.wav"),
    }
}

/// The music for a mood, from the mission's music world (`data\sfx\1622\musicNN\darci`).
pub fn music(mood: Mood, world: u8, fights: u32) -> Option<String> {
    let dir = format!("music{world:02}/darci");
    match mood {
        // Alternate the two fight themes from one fight to the next.
        Mood::Fight => Some(format!("{dir}/Fight{}.wav", 1 + fights % 2)),
        Mood::Sprint => Some(format!("{dir}/Sprint1.wav")),
        Mood::Calm | Mood::Won | Mood::Lost => None,
    }
}
