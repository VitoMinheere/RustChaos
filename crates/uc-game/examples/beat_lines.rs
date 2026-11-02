//! Beat Cop's dispatch lines, for making their voices: `file<TAB>words`, one a line.
//!
//!   cargo run -p uc-game --example beat_lines > beat.tsv
//!   python tools/voices/make_voices.py - OUT --beat beat.tsv

fn main() {
    for (file, text) in uc_game::game::beat::lines() {
        println!("{file}\t{text}");
    }
}
