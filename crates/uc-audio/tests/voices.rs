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
