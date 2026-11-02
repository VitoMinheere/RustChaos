//! The characters' lines in `talk2\misc\`: where they're read from, every take of a line,
//! and going through the takes instead of repeating one.

#[test]
fn a_take_is_the_line_then_a_number_and_letters() {
    for take in ["GRED-PAIN01.wav", "GRED-PAIN01B.wav", "gred-pain05b.WAV", "MCIV-PAIN.wav", "MCIV-PAIN2.wav", "MCIV-PAINb.wav"] {
        assert!(uc_audio::is_take(take, if take.starts_with('M') { "MCIV-PAIN" } else { "GRED-PAIN" }), "{take}");
    }
    assert!(uc_audio::is_take("DARCI-TAUNT06GB.wav", "DARCI-TAUNT"));
    // Other lines that start the same.
    assert!(!uc_audio::is_take("GRED-PAINDIE02.wav", "GRED-PAIN"));
    assert!(!uc_audio::is_take("COP-TAUNTDIE02.wav", "COP-TAUNT"));
    assert!(!uc_audio::is_take("GRED-ALERT01.wav", "GRED-PAIN"));
}

#[test]
fn a_name_with_a_dash_is_read_from_talk2_misc() {
    assert_eq!(uc_audio::misc_path(r"misc\GRED-PAIN01.wav").as_deref(), Some(r"talk2\misc\GRED-PAIN01.wav"));
    assert_eq!(uc_audio::misc_path("darci/DARCI-TAUNT01.wav").as_deref(), Some(r"talk2\misc\DARCI-TAUNT01.wav"));
    assert_eq!(uc_audio::misc_path("Footsteps/HardStep1.wav"), None);
}

/// Against the install: how many takes there are, and that they decode (they're IMA ADPCM).
#[test]
fn the_takes_in_the_install() {
    let Some(game) = uc_formats::default_install() else { return };
    for (line, n) in [("DARCI-TAUNT", 37), ("DARCI-PICKUP", 13), ("ROPER-TAUNT", 24), ("GRED-ALERT", 18), ("GRED-PAIN", 10), ("COP-TAUNT", 8), ("MCIV-PAIN", 3)] {
        let takes = uc_audio::takes(&game, line);
        assert_eq!(takes.len(), n, "{line}: {takes:?}");
        for t in &takes {
            let bytes = std::fs::read(uc_formats::resolve(&game, &format!(r"talk2\misc\{t}")).unwrap()).unwrap();
            assert!(uc_audio::wav::decode(&bytes).is_ok_and(|c| c.seconds() > 0.1), "{t} doesn't decode");
        }
    }
}

/// A voice goes through its takes before saying one again. Needs a sound device.
#[test]
fn takes_are_not_repeated() {
    let Some(game) = uc_formats::default_install() else { return };
    let Ok(mut audio) = uc_audio::Audio::new(&game) else { return };
    let takes = audio.takes("GRED-ALERT");
    let said: Vec<String> = (0..takes.len()).map(|_| audio.fresh(&takes).unwrap()).collect();
    // The last sixteen are remembered: no take twice in the first sixteen.
    let first: std::collections::BTreeSet<&String> = said[..16].iter().collect();
    assert_eq!(first.len(), 16, "{said:?}");
    // Not in the install: never picked.
    assert_eq!(audio.fresh(&["nothing-here01.wav".to_string()]), None);
}
