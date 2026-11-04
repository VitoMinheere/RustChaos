//! Endless mode's radio lines, for making their voices: `file<TAB>words<TAB>voice`, one a
//! line (dispatch's in the `despatch` voice, Darci's in `darci`).
//!
//!   cargo run -p uc-game --example endless_lines > endless.tsv
//!   python tools/voices/make_voices.py EXPORT OUT --beat endless.tsv

fn main() {
    for (file, text, darci) in uc_game::game::endless::lines() {
        println!("{file}\t{text}\t{}", if darci { "darci" } else { "despatch" });
    }
}
