//! `transfer_player` and filmed conversations. Cop Killers (`AWOL1`) hands the player from
//! Darci to Roper; in RTA (`police1`) the mugger on the roof
//! shouts down at Darci, and the camera films whoever has the line. Skipped without an install.

use uc_formats::ucm::trigger;
use uc_game::talk::face;
use uc_game::{Controls, Game};
use uc_world::character::person;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(mission: &str, edit: impl Fn(&mut Level)) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let mut level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    edit(&mut level);
    let g = Game::new(&level, true);
    Some((level, g))
}

#[test]
fn cop_killers_hands_the_player_to_roper() {
    // Point 22 waits on the second fly-by, well into the mission: here it goes off as soon as
    // point 96 has made Roper.
    let Some((level, mut g)) = load("AWOL1", |l| {
        if let Some(e) = l.mission.event_points.iter_mut().find(|e| e.index == 22) {
            (e.trigger, e.ep_ref) = (trigger::DEPENDENCY, 96);
        }
    }) else { return };
    let darci = g.player.unwrap();
    assert_eq!(g.people[darci].kind, person::DARCI);
    for f in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls { punch: f % 20 == 0, ..Default::default() }, &level);
        if g.player != Some(darci) {
            break;
        }
    }
    let roper = g.player.unwrap();
    assert_ne!(roper, darci, "point 22 should have moved the player on");
    assert_eq!(g.people[roper].kind, person::ROPER);
    assert!(g.people[roper].player && !g.people[darci].player);
    assert_eq!(g.player_name(), "Roper");
    assert!(g.people[darci].protected && g.people[darci].invulnerable, "Darci can't be hurt now");
    assert!(!g.people[darci].hostile(), "and minds her own business");
}

/// The mugger and his shout, straight away: point 233 makes him, and 235 (which waits on his
/// seeing Darci) goes off as soon as he's there.
fn rta_shout() -> Option<(Level, Game)> {
    load("police1", |l| {
        for e in &mut l.mission.event_points {
            match e.index {
                233 => e.trigger = trigger::NONE,
                235 => (e.trigger, e.ep_ref) = (trigger::DEPENDENCY, 233),
                _ => {}
            }
        }
    })
}

#[test]
fn the_shout_is_filmed_with_both_faces() {
    let Some((level, mut g)) = rta_shout() else { return };
    for _ in 0..5 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.in_conversation(), "the conversation should be filmed");
    let (cam, at) = g.camera().expect("the conversation's camera");
    assert!(cam.distance(at) > 100.0);
    let w = g.wide.clone().expect("the widescreen panel");
    let mut faces = [w.top.unwrap(), w.bottom.unwrap()];
    faces.sort();
    assert_eq!(faces, [face::DARCI, face::THUG_GREY], "Darci and the grey-gang mugger");
    assert!(w.text.contains("ASSHOLE"), "the first line: {}", w.text);

    // She's held still while it goes on.
    let me = g.player.unwrap();
    let at = g.people[me].fig.pos;
    for _ in 0..30 {
        g.update(DT, &Controls { wish: glam::Vec2::X, run: true, ..Default::default() }, &level);
    }
    assert_eq!(g.people[me].fig.pos, at, "the player should be held during it");

    // Punching through it skips line by line, and then she's free.
    for f in 0..600 {
        g.update(DT, &Controls { punch: f % 20 == 0, ..Default::default() }, &level);
        if !g.in_conversation() {
            break;
        }
    }
    assert!(!g.in_conversation() && g.wide.is_none(), "it should be over");
}

/// Messages carry their speaker's face: dispatch's are the radio's.
#[test]
fn messages_have_faces() {
    let Some((level, mut g)) = load("police1", |_| {}) else { return };
    for _ in 0..30 {
        g.update(DT, &Controls::default(), &level);
    }
    let m = g.messages.iter().find(|m| m.text.contains("vehicle impact")).expect("the first message");
    assert_eq!(m.face, Some(face::RADIO));
}
