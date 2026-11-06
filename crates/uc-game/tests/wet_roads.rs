//! Wet roads (#522): the night missions are wet, as they're the ones it rains on
//! (`GF_RAINING`, elev.cpp:2404); the day ones, and those with no lighting file (which the
//! original lights as day, `NIGHT_init`), are dry. Skipped without an install.

use uc_game::Game;
use uc_world::{Level, LoadOptions};

use crate::common;

/// Every mission in the game's list, and whether it rains.
const MISSIONS: [(&str, bool); 33] = [
    ("FTutor1", false), ("Assault1", false), ("testdrive1a", true), ("fight1", false),
    ("police1", false), ("testdrive2", false), ("fight2", false), ("police2", true),
    ("testdrive3", false), ("Bankbomb1", true), ("police3", true), ("Gangorder2", true),
    ("police4", false), ("bball2", true), ("wstores1", false), ("e3", true),
    ("westcrime1", false), ("carbomb1", true), ("botanicc", false), ("Semtex", true),
    ("AWOL1", false), ("mission2", true), ("park2", false), ("MIB", true),
    ("skymiss2", true), ("factory1", false), ("estate2", false), ("Stealtst1", true),
    ("snow2", false), ("Gordout1", false), ("Baalrog3", true), ("Finale1", true),
    ("Gangorder1", false),
];

#[test]
fn the_night_missions_are_wet() {
    let Some(game) = uc_formats::default_install() else { return };
    let text = uc_formats::sty::read(&game);
    let listed: Vec<&str> = uc_formats::sty::lines(&text).filter_map(|l| l.mission()).collect();
    for (m, wet) in MISSIONS {
        assert!(listed.iter().any(|l| l.eq_ignore_ascii_case(m)), "{m} isn't in the game's list");
        let level = Level::load(&game, m, &LoadOptions { fullbright: false, character: false, overrides: None }).unwrap();
        assert_eq!(Game::raining(&level), wet, "{m}");
    }
}

#[test]
fn a_wet_mission_starts_soaked_and_stays_so() {
    let Some(level) = common::load_lit("police2") else { return };
    let mut g = Game::new(&level, true);
    assert_eq!(g.wet.amount, 1.0);
    common::frames(&mut g, &level, 60);
    assert_eq!(g.wet.amount, 1.0, "the rain never stops");
}

#[test]
fn a_dry_mission_stays_dry() {
    let Some(level) = common::load_lit("police1") else { return };
    let mut g = Game::new(&level, true);
    common::frames(&mut g, &level, 60);
    assert_eq!(g.wet.amount, 0.0);
    assert!(g.headlight_glints(&level).is_empty(), "by day the headlights are off");
}

