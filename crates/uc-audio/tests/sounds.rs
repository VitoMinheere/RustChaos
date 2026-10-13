//! The game's sound ids. The table is ported as it stands, so the useful check is that the
//! files it names are the ones the install ships. Skipped without an install.

use uc_audio::sounds;

#[test]
fn the_table_reads_as_the_original_wrote_it() {
    // `sound_list[]` without its "!" sentinel.
    assert_eq!(sounds::SOUNDS.len(), 548);
    // Id 0 is the table's own placeholder and means silence.
    assert_eq!(sounds::name(0), None);
    assert_eq!(sounds::name(sounds::SOUNDS.len()), None, "past the end of the table");
    // The marker some names carry is skipped, as `music.cpp` does by starting six in.
    let marked = sounds::SOUNDS.iter().position(|n| n.starts_with(' ')).expect("a marked name");
    assert!(!sounds::name(marked).unwrap().starts_with(' '), "the marker should be skipped");
}

/// The three ids the missions ask for, so a rename or an off-by-one in the table would show.
#[test]
fn the_missions_sounds_are_what_they_should_be() {
    assert_eq!(sounds::name(17), Some("f_siren3.wav"), "The Fallen's siren");
    assert_eq!(sounds::name(174), Some(r"darci\no scream 2.WAV"), "Stern Revenge's fall");
    // The Jump asks for id 0, which is the table's placeholder: it was never going to be heard.
    assert_eq!(sounds::name(0), None);
}

/// Most of the table should name a file the install actually has: if it were mis-indexed, far
/// more than a handful would miss. The ones that do miss are names the retail release renamed
/// or dropped after this table was generated, which is worth knowing but isn't a fault here.
#[test]
fn the_names_point_at_files_that_exist() {
    let Some(game) = uc_formats::default_install() else { return };
    let mut missing = Vec::new();
    for id in 0..sounds::SOUNDS.len() {
        let Some(name) = sounds::name(id) else { continue };
        if uc_formats::resolve(&game, &format!(r"data\sfx\1622\{name}")).is_none() {
            missing.push(format!("{id}: {name}"));
        }
    }
    let found = sounds::SOUNDS.len() - missing.len();
    println!("{found} of {} named files are in the install", sounds::SOUNDS.len());
    assert!(found * 100 / sounds::SOUNDS.len() >= 85, "only {found} names resolved: {missing:?}");

    // What the release did to the rest: the music moved under the world that uses it, the
    // ambience lost its working names (`BirdSong2Shortenough` is `BirdSong2` now), and the
    // `misc\` folder of pain grunts isn't shipped at all.
    let odd: Vec<&String> = missing
        .iter()
        .filter(|m| {
            let l = m.to_ascii_lowercase();
            !["music", "notused", "wind", "ambience", r"misc\"].iter().any(|k| l.contains(k))
        })
        .collect();
    assert!(odd.is_empty(), "names missing for no reason we know of: {odd:?}");
}
