//! Mission voices are said one at a time. Needs an install and a sound device; skipped
//! without either (which is what CI sees).

use std::path::Path;

/// How long `talk2\<mission>.ucm<n>.wav` runs for.
fn seconds(game: &Path, name: &str) -> f32 {
    let bytes = std::fs::read(game.join("talk2").join(name)).unwrap();
    uc_audio::wav::decode(&bytes).unwrap().seconds()
}

#[test]
fn spoken_lines_wait_their_turn() {
    let Some(game) = uc_formats::default_install() else { return };
    let Ok(mut audio) = uc_audio::Audio::new(&game) else { return };

    // Nitro Car opens with two lines back to back. Skipping the fly-by brings the second one
    // up while the first is still going.
    let first = seconds(&game, "Bankbomb1.ucm22.wav");
    assert!(first > 1.0, "the opening line is {first} seconds long?");
    assert!(audio.voice("Bankbomb1", 22), "the opening line should be recorded");
    assert_eq!(audio.speaking(), (true, 0));
    assert!(audio.voice("Bankbomb1", 23), "the line after it should be recorded");
    assert_eq!(audio.speaking(), (true, 1), "the second line should be waiting, not talking over the first");

    // Still the first line most of the way through it.
    let step = 1.0 / 60.0;
    for _ in 0..((first - 0.2) / step) as usize {
        audio.update(step);
    }
    assert_eq!(audio.speaking(), (true, 1), "the first line should still have the floor");

    // Once it's done, the one waiting takes over.
    for _ in 0..24 {
        audio.update(step);
    }
    assert_eq!(audio.speaking(), (true, 0), "the second line should start when the first is over");

    audio.hush();
    assert_eq!(audio.speaking(), (false, 0), "hushing should stop the lot");
}

/// Skipping through a run of them doesn't queue up minutes of talking.
#[test]
fn only_so_many_lines_wait() {
    let Some(game) = uc_formats::default_install() else { return };
    let Ok(mut audio) = uc_audio::Audio::new(&game) else { return };
    for n in [21, 22, 23, 24, 25, 26, 27, 29] {
        audio.voice("Bankbomb1", n);
    }
    let (talking, waiting) = audio.speaking();
    assert!(talking);
    assert!(waiting <= 3, "{waiting} lines queued up behind the one being said");
}

/// A line the install has no recording of is said from the voices folder, when there is one;
/// the install's own recordings come first.
#[test]
fn made_lines_fill_in_for_missing_ones() {
    let Some(game) = uc_formats::default_install() else { return };
    let Ok(mut audio) = uc_audio::Audio::new(&game) else { return };
    let dir = std::env::temp_dir().join(format!("uc-voices-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("talk2")).unwrap();
    // Half a second of 16-bit silence.
    let data = vec![0u8; 22050];
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + data.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    for n in [1u16, 1] { wav.extend(n.to_le_bytes()); }
    for n in [22050u32, 44100] { wav.extend(n.to_le_bytes()); }
    for n in [2u16, 16] { wav.extend(n.to_le_bytes()); }
    wav.extend(b"data");
    wav.extend((data.len() as u32).to_le_bytes());
    wav.extend(&data);
    std::fs::write(dir.join("talk2").join("Crimescene.ucm15B.wav"), wav).unwrap();

    assert!(!audio.conversation_line("Crimescene", 15, 1), "the install has no recording of 24th Street's lines");
    audio.set_voices(Some(dir.clone()));
    assert!(audio.conversation_line("Crimescene", 15, 1), "the made one should be said");
    assert!(audio.voice("Bankbomb1", 22), "the install's own lines still play");
    std::fs::remove_file(dir.join("talk2").join("Crimescene.ucm15B.wav")).unwrap();
}
