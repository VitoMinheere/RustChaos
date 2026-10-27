//! The briefing read out at the start of an unreleased mission ([`uc_game::Game::brief`]):
//! one line at a time, after any opening fly-by and the script's own opening messages.
//! Skipped without an install.

use uc_game::Controls;

mod common;
use common::Run;

fn briefed(mission: &str) -> Option<Run> {
    let mut r = Run::new(common::load(mission)?);
    r.g.brief(Some("The story so far.".into()), "Objectives: Do the job.".into());
    Some(r)
}

/// Mr Bates (gang1) says nothing at the start: the story, then the objectives.
#[test]
fn the_story_then_the_objectives() {
    let Some(mut r) = briefed("gang1") else { return };
    // Not over a fly-by.
    while r.g.cutscene.is_some() {
        r.g.update(common::DT, &Controls::default(), &r.level);
        assert!(r.g.messages.is_empty(), "said over the fly-by: {:?}", r.g.messages[0].text);
    }
    assert!(r.until(5.0, |r| r.said("The story so far.")), "the story wasn't read out");
    assert!(!r.said("Objectives:"), "both at once");
    assert!(r.until(20.0, |r| r.said("Objectives: Do the job.")), "the objectives weren't read out");
}

/// Mr X's Getaway (Getaway1) opens with a message of its own: that, then just the
/// objectives.
#[test]
fn just_the_objectives_after_the_scripts_own_opening() {
    let Some(mut r) = briefed("Getaway1") else { return };
    assert!(r.until(30.0, |r| r.said("Objectives: Do the job.")), "the objectives weren't read out");
    assert!(r.said("Meet contact"), "{:?}", r.seen);
    assert!(!r.said("The story so far."));
}
