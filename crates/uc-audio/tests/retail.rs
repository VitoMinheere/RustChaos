//! Decodes every WAV the game ships (skipped without an install).

fn wavs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            wavs(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("wav")) {
            out.push(p);
        }
    }
}

#[test]
fn every_shipped_wav_decodes() {
    let Some(game) = uc_formats::default_install() else { return };
    let mut files = Vec::new();
    wavs(&game.join("data").join("sfx"), &mut files);
    wavs(&game.join("talk2"), &mut files);
    assert!(files.len() > 2500, "only {} WAVs found", files.len());
    let mut failed = Vec::new();
    let mut seconds = 0.0;
    for f in &files {
        match uc_audio::wav::decode(&std::fs::read(f).unwrap()) {
            Ok(c) => {
                assert!(c.samples.iter().all(|s| s.is_finite() && s.abs() <= 1.0), "{}", f.display());
                seconds += c.seconds();
            }
            Err(e) => failed.push(format!("{}: {e}", f.display())),
        }
    }
    println!("{} files, {:.0} minutes of sound; failed: {failed:#?}", files.len(), seconds / 60.0);
    // getouttheway.wav is not a WAV file; everything else must decode.
    assert!(failed.len() <= 1, "{failed:#?}");
}
