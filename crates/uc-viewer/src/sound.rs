//! Which of the original recordings each game sound plays, and the music for each mood.

use glam::Vec3;
use uc_audio::{Audio, Listener};
use uc_game::{Mood, Sfx, Theme};
use uc_world::character::person as ptype;

fn numbered(prefix: &str, n: std::ops::RangeInclusive<u32>, suffix: &str) -> Vec<String> {
    n.map(|i| format!("{prefix}{i}{suffix}")).collect()
}

/// The prefix of a character's lines in `talk2\misc\` (`GRED-PAIN01.wav`...), if they have
/// any. The original plays a few of them; the rest were recorded but nothing plays them, so
/// here every take goes into the pool for its line ([`Audio::takes`]).
fn voice(kind: u8) -> Option<&'static str> {
    Some(match kind {
        ptype::DARCI => "DARCI",
        ptype::ROPER => "ROPER",
        ptype::COP => "COP",
        ptype::CIV => "MCIV",
        ptype::THUG_RASTA => "GRASTA",
        ptype::THUG_GREY => "GGREY",
        ptype::THUG_RED => "GRED",
        ptype::HOSTAGE => "HOSTAGE",
        ptype::MECHANIC => "GWORK",
        ptype::TRAMP => "GTRAMP",
        _ => return None,
    })
}

/// Every take of `kind`'s `line` (`"PAIN"`, `"TAUNT"`...), besides `also`.
fn lines(audio: &mut Audio, kind: u8, line: &str, mut also: Vec<String>) -> Vec<String> {
    if let Some(v) = voice(kind) {
        also.extend(audio.takes(&format!("{v}-{line}")));
    }
    also
}

/// The folder of a gang's own sounds (`hey you.wav`).
fn gang_dir(kind: u8) -> Option<&'static str> {
    match kind {
        ptype::THUG_RASTA => Some("bthug1"),
        ptype::THUG_RED => Some("wthug2"),
        ptype::THUG_GREY => Some("wthug1"),
        _ => None,
    }
}

/// `names`, padded with silence so that one of them is said about one time in `every`.
fn sometimes(mut names: Vec<String>, every: usize) -> Vec<String> {
    names.extend(std::iter::repeat_n(String::new(), names.len() * (every - 1)));
    names
}

/// Plays the sound for one game event at `pos` (world units). A sound with several
/// recordings goes through them rather than repeating one ([`Audio::fresh`]).
pub fn play(audio: &mut Audio, sfx: Sfx, pos: Vec3, l: &Listener, mission: &str) {
    let at = |audio: &mut Audio, names: Vec<String>, vol: f32| {
        if let Some(n) = audio.fresh(&names).filter(|n| !n.is_empty()) {
            audio.play_at(&n, vol, pos, l);
        }
    };
    match sfx {
        Sfx::Swing => at(audio, numbered("f_knSWP", 1..=4, ".wav"), 0.3),
        Sfx::Hit { heavy } => at(audio, numbered("f_PUNCH", 1..=5, ".wav"), if heavy { 1.0 } else { 0.75 }),
        // `PainSound` (Sound.cpp:521): Darci's and Roper's hits, and everyone else's pain
        // from `talk2\misc\`, with all their takes. The civilians' are the port's own.
        Sfx::Hurt { kind } => {
            let own = match kind {
                ptype::DARCI => numbered("darci/hit ", 1..=5, ".WAV"),
                ptype::ROPER => numbered("roper/hit", 1..=7, ".WAV"),
                _ => Vec::new(),
            };
            let names = lines(audio, kind, "PAIN", own);
            at(audio, names, 0.8);
        }
        Sfx::MibExplode => at(audio, vec!["MIBexplo1.wav".into()], 1.0),
        // The Men in Black levitate as they die (`S_MIB_LEVITATE`, Person.cpp:1140).
        Sfx::KnockedOut { kind: ptype::MIB1 | ptype::MIB2 | ptype::MIB3 } => at(audio, vec!["MIBlevitate1.wav".into()], 1.0),
        Sfx::KnockedOut { kind } => {
            let mut names = match kind {
                ptype::DARCI => numbered("darci/die ", 1..=3, ".WAV"),
                // `DieSound`: the cops' own (`S_COP_DIE_START` to `_END`), and the others'
                // deaths recorded beside them.
                // With the one-off last words recorded beside them (#486).
                ptype::COP => lines(audio, kind, "DIE", vec!["COP-TAUNTDIE02.wav".into()]),
                ptype::THUG_RED => lines(audio, kind, "DIE", vec!["GRED-PAINDIE02.wav".into()]),
                _ => lines(audio, kind, "DIE", Vec::new()),
            };
            if names.is_empty() {
                names = match kind {
                    ptype::SLAG_TART | ptype::SLAG_FATUGLY | ptype::HOSTAGE => numbered("f_fDIE", 1..=2, ".wav"),
                    _ => numbered("f_mDIE", 1..=2, ".wav"),
                };
            }
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
        Sfx::CopArrest => {
            let names = lines(audio, ptype::COP, "ARREST", Vec::new());
            at(audio, names, 0.9);
        }
        // `SOUND_Curious`: the gangs' uncertain lines (`S_BTHUG_UNCERTAIN_START` and on), and
        // the others' questions, which nothing played.
        Sfx::Curious { kind } => {
            let own = match kind {
                ptype::THUG_RASTA => Some(210..=216),
                ptype::THUG_GREY => Some(223..=231),
                ptype::THUG_RED => Some(238..=245),
                _ => None,
            }
            .map_or(Vec::new(), |r| r.filter_map(uc_audio::sounds::name).map(|n| n.replace('\\', "/")).collect());
            let names = lines(audio, kind, "INQUIRE", own);
            at(audio, names, 0.9);
        }
        // `ScreamFallSound` (Sound.cpp:609): Darci's, and Roper's, which the civilians share.
        Sfx::ScreamFall { kind: ptype::DARCI } => at(audio, vec!["darci/scream fall.WAV".into(), "darci/scream fall 2.WAV".into()], 1.0),
        Sfx::ScreamFall { kind: ptype::ROPER | ptype::CIV } => at(audio, numbered("roper/roper fall", 1..=3, ".WAV"), 1.0),
        Sfx::ScreamFall { .. } => {}
        // Catching fire: the screams nothing played (`ScreamF1`, `ScreamF2`, `ScreamM1`), and
        // the screams recorded for the character, if any (`DARCI-SCREAM01`, `MCIV-SCREAM`...).
        Sfx::OnFire { kind, female } => {
            let own = if female { vec!["ScreamF1.wav".into(), "ScreamF2.wav".into()] } else { vec!["ScreamM1.wav".into()] };
            // Not a man's scream from a woman of the crowd.
            let names = if female && kind == ptype::CIV { own } else { lines(audio, kind, "SCREAM", own) };
            at(audio, names, 1.0);
        }
        Sfx::Land => at(audio, vec!["Footsteps/StreetLand1.wav".into()], 0.6),
        Sfx::Grab => at(audio, vec!["JudoChop1.wav".into()], 0.8),
        Sfx::Arrest => {
            let names = lines(audio, ptype::DARCI, "ARREST", vec!["darci/under arrest.WAV".into()]);
            at(audio, names, 1.0);
        }
        Sfx::Freeze(line) => at(audio, vec![["darci/freeze.wav", "darci/freeze tk2.wav", "darci/stop police tk2.wav"][line.min(2) as usize].to_string()], 1.0),
        // About one knockout in three, from every take of the line.
        Sfx::Taunt { kind } => {
            let names = lines(audio, kind, "TAUNT", Vec::new());
            if !names.is_empty() {
                at(audio, sometimes(names, 3), 1.0);
            }
        }
        // The gangs' "hey you", and everyone's alerts, which nothing played.
        Sfx::Alert { kind } => {
            let own = gang_dir(kind).map_or(Vec::new(), |d| vec![format!("{d}/hey you.wav"), format!("{d}/hey you 2.wav")]);
            let names = lines(audio, kind, "ALERT", own);
            at(audio, names, 0.9);
        }
        // About one item in three.
        Sfx::PickupLine { kind } => {
            let names = lines(audio, kind, "PICKUP", Vec::new());
            if !names.is_empty() {
                at(audio, sometimes(names, 3), 0.9);
            }
        }
        // The citizens' recorded chatter (#486), a take for their sex, if it was recorded for
        // it: the women's #20 stressed with a gun on them, calm otherwise.
        Sfx::CitizenLine { line, female, stressed } => {
            let names = citizen_takes(line, female, stressed).iter().map(|t| t.to_string()).collect();
            at(audio, names, 1.0);
        }
        // A frightened citizen's line, in a made voice for their sex (`talk2\misc\fcit-annoyed01.wav`
        // or `mcit-...`, from the made voices' folder): silent without them.
        Sfx::AnnoyedLine { line, female } => {
            let who = if female { "fcit" } else { "mcit" };
            at(audio, vec![format!("{who}-annoyed{line:02}.wav")], 1.0);
        }
        // What the crowd says when spoken to, in a made voice for their sex
        // (`fcit-chat1a2b3c4d.wav`, from the made voices' folder): silent without them.
        Sfx::ChatterLine { key, female } => {
            let who = if female { "fcit" } else { "mcit" };
            at(audio, vec![format!("{who}-chat{key:08x}.wav")], 1.0);
        }
        // The characters' lines that were recorded and never played (#486), every take.
        Sfx::Bark { kind, bark } => {
            use uc_game::Bark;
            let mut names = lines(audio, kind, bark_line(bark), Vec::new());
            // A panicking man screams as often as he cries out.
            if bark == Bark::Panic {
                names = lines(audio, kind, "SCREAM", names);
            }
            // Not every time for the small things: a grunt one pull-up in three, a moan one
            // empty clip in two, a cry one fright in two.
            let names = match bark {
                Bark::Grunt => sometimes(names, 3),
                Bark::Complain | Bark::Panic => sometimes(names, 2),
                _ => names,
            };
            if !names.is_empty() {
                at(audio, names, 1.0);
            }
        }
        // Over the radio, but not over a line being said.
        Sfx::RadioSays(said) => {
            if audio.speaking().0 {
                return;
            }
            let names: Vec<String> = match said {
                uc_game::RadioLine::Won => audio.takes("police-CONGRATS").into_iter().chain(audio.takes("police-CHEER")).collect(),
                uc_game::RadioLine::Lost => audio.takes("police-FAIL"),
            };
            if let Some(n) = audio.fresh(&names).filter(|n| !n.is_empty()) {
                audio.play(&n, 0.9);
            }
        }
        // A mission made out of another says its lines with that one's recordings
        // (`uc_world::level::PORT_MISSIONS`); those it says differently have numbers of their own.
        Sfx::Voice(point) => {
            audio.voice(uc_world::level::mission_file(mission), point);
        }
        Sfx::ConversationLine(point, line) => {
            audio.conversation_line(uc_world::level::mission_file(mission), point, line);
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
        // More sfx than music, over whatever's playing.
        Sfx::Danger { near } => audio.play(if near { "SFX/DangerRed.wav" } else { "SFX/DangerGreen.wav" }, 0.8),
        Sfx::Siren => at(audio, vec!["f_SIREN1.wav".into(), "f_SIREN2.wav".into(), "f_SIREN3.wav".into(), "General/CopSiren1.wav".into()], 1.0),
        Sfx::Crash => at(audio, vec!["MedCrash2.wav".into(), "Impact1.wav".into(), "Impact2.wav".into()], 1.0),
        Sfx::Cone => at(audio, vec!["Cone1.wav".into()], 0.8),
        Sfx::Barrel => at(audio, numbered("sfx080799+/Barrel", 1..=3, ".wav"), 0.8),
        Sfx::Elec => at(audio, numbered("f_ELEC", 1..=5, ".wav"), 0.9),
        Sfx::Squish => at(audio, vec!["thumpsquish.wav".into()], 1.0),
        Sfx::Scrape => at(audio, numbered("Scrape", 3..=6, ".wav"), 0.8),
        Sfx::Nuts => at(audio, vec!["General/Bollox1.wav".into(), "General/Bollox3.wav".into()], 1.0),
        // Not placed: a sting over the shot (`MFX_play_stereo`).
        Sfx::Sting => audio.play("6Sound.wav", 1.0),
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

/// The line a [`uc_game::Bark`] is said with: `<WHO>-<LINE>NN.wav` in `talk2\misc\`.
fn bark_line(bark: uc_game::Bark) -> &'static str {
    use uc_game::Bark;
    match bark {
        Bark::Startle => "STARTLE",
        Bark::Search => "SEARCH",
        Bark::Gross => "GROSS",
        Bark::Curse => "CURSE",
        Bark::Complain => "complain",
        Bark::Grunt => "GRUNT",
        Bark::Fail => "FAIL",
        Bark::Relax => "RELAX",
        Bark::Carry => "CARRY",
        Bark::Shout => "SHOUT",
        Bark::Command => "COMMAND",
        Bark::Inquire => "INQUIRE",
        Bark::Greet => "GREET",
        Bark::Panic => "PANIC",
        Bark::Innocent => "INNOCENT",
    }
}

/// The recordings of `citsez.txt`'s line `line` (`talk2\misc\`), for a woman or a man.
fn citizen_takes(line: u8, female: bool, stressed: bool) -> &'static [&'static str] {
    match (line, female) {
        (1, true) => &["fcit-sez04.wav", "fcit-sez04b.wav"],
        (1, false) => &["mcit-sez01.wav"],
        (3, false) => &["mcit-sez02.wav"],
        (6, true) => &["fcit-sez03.wav", "fcit-sez03b.wav"],
        (6, false) => &["mcit-sez03.wav"],
        (10, false) => &["mcit-sez04.wav"],
        (13, true) => &["fcit-sez01.wav", "fcit-sez01b.wav"],
        (20, true) if stressed => &["fcit-sez02.wav", "fcit-sez02b.wav"],
        (20, true) => &["fcit-sez02c.wav"],
        _ => &[],
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
    /// Every unplayed line (#486) has takes in the install for whoever says it, and the
    /// radio's too. Skipped without one.
    #[test]
    fn every_bark_has_takes() {
        use uc_game::Bark::*;
        use uc_world::character::person as ptype;
        let Some(game) = uc_formats::default_install() else { return };
        let said = [
            (ptype::DARCI, vec![Startle, Search, Gross, Curse, Complain, Grunt, Fail, Relax, Command]),
            (ptype::ROPER, vec![Startle, Search, Gross, Curse, Grunt, Fail, Relax, Carry, Shout, Command]),
            (ptype::COP, vec![Greet]),
            (ptype::CIV, vec![Panic, Innocent]),
            (ptype::HOSTAGE, vec![Startle, Inquire]),
        ];
        for (kind, barks) in said {
            for b in barks {
                let prefix = format!("{}-{}", super::voice(kind).unwrap(), super::bark_line(b));
                assert!(!uc_audio::takes(&game, &prefix).is_empty(), "no takes of {prefix}");
            }
        }
        for line in ["police-CONGRATS", "police-CHEER", "police-FAIL"] {
            assert!(!uc_audio::takes(&game, line).is_empty(), "no takes of {line}");
        }
        for last in ["COP-TAUNTDIE02.wav", "GRED-PAINDIE02.wav"] {
            assert!(uc_formats::resolve(&game, &format!(r"talk2\misc\{last}")).is_some(), "{last} isn't in the install");
        }
    }

    /// Every take of the citizens' recorded chatter is in the install. Skipped without one.
    #[test]
    fn the_citizens_takes_are_shipped() {
        let Some(game) = uc_formats::default_install() else { return };
        for line in [1, 3, 6, 10, 13, 20] {
            for (female, stressed) in [(true, true), (true, false), (false, false)] {
                for t in super::citizen_takes(line, female, stressed) {
                    assert!(uc_formats::resolve(&game, &format!(r"talk2\misc\{t}")).is_some(), "{t} isn't in the install");
                }
            }
        }
        assert!(super::citizen_takes(3, true, false).is_empty(), "no woman says line 3");
    }

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
