//! Guards look into what they hear (#293, `PCOM_process_investigating`): turned to the
//! sound for a moment, they walk over to it if they can't see where it came from, look
//! round, and give up or go on to somewhere someone might be hiding; one who can see it from
//! where he stands doesn't go. One of RTA's crowd made a guard. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::{Investigate, AI_CIV, AI_GUARD};
use uc_game::{Controls, Sfx};
use uc_world::character::person as ptype;

mod common;

fn guard() -> Option<(common::Run, usize)> {
    let mut r = common::start("police1")?;
    r.heal = true;
    r.play(1.0, Controls::default());
    let i = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("crowd");
    let q = &mut r.g.people[i];
    (q.ai, q.kind, q.mv, q.fake, q.home) = (AI_GUARD, ptype::THUG_RED, 1, false, None);
    // Far from the player, so he isn't seen.
    let me = r.g.people[r.player()].fig.pos;
    let spot = (0..200).map(|k| me + vec3((k % 20) as f32 * 256.0 - 2560.0, 0.0, (k / 20) as f32 * 256.0 + 3000.0))
        .find(|p| {
            let y = r.level.ground.surface(&r.level.map, p.x, p.z, 200.0);
            (y - me.y).abs() < 8.0
        })
        .expect("open ground away from her");
    r.g.people[i].fig.pos = spot;
    Some((r, i))
}

#[test]
fn he_walks_over_to_a_sound_out_of_sight() {
    let Some((mut r, i)) = guard() else { return };
    let at = r.g.people[i].fig.pos;
    let eye = vec3(0.0, 0x60 as f32, 0.0);
    // Somewhere a few squares off he can't see.
    let hidden = (1..40).flat_map(|k| [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)].map(move |(x, z)| (k, x, z)))
        .map(|(k, x, z)| at + vec3(x, 0.0, z) * (k as f32 * 64.0))
        .find(|p| (*p - at).length() > 300.0 && (*p - at).length() < 1200.0 && !r.level.ground.line_of_sight(at + eye, *p + vec3(0.0, 0xe0 as f32, 0.0))
            && (r.level.ground.surface(&r.level.map, p.x, p.z, p.y + 64.0) - p.y).abs() < 8.0);
    let sound = hidden.expect("somewhere out of his sight");
    r.g.people[i].cues.clear();
    r.g.people[i].hear(sound);
    assert!(r.g.people[i].cues.iter().any(|s| matches!(s, Sfx::Curious { .. })), "he said nothing");
    let mut walked = false;
    let mut looked = false;
    // After the look round he goes home, or on to a hiding place (a walk again).
    let mut done = false;
    for _ in 0..(25.0 / common::DT) as usize {
        r.play(common::DT, Controls::default());
        match r.g.people[i].heard.map(|h| h.phase) {
            Some(Investigate::WalkOver) if looked => done = true,
            Some(Investigate::WalkOver) => walked = true,
            Some(Investigate::Look) => looked = true,
            None if looked => done = true,
            _ => {}
        }
        if done {
            break;
        }
    }
    assert!(walked, "never set off");
    assert!(looked, "never looked round");
    assert!(done, "still looking round");
    let moved: Vec3 = r.g.people[i].fig.pos - at;
    assert!(moved.length() > 150.0, "only went {}", moved.length());
}

/// The look round over, half the time he goes on to somewhere someone might be hiding
/// (`PCOM_find_hiding_place`, pcom.cpp:1589): a square two to five away (x plus z) he can't
/// see a head on from his. Otherwise home.
#[test]
fn he_goes_on_to_a_likely_hiding_place() {
    let Some((mut r, i)) = guard() else { return };
    let eye = vec3(0.0, 0x60 as f32, 0.0);
    let mut hid = None;
    for _ in 0..30 {
        let at = r.g.people[i].fig.pos;
        r.g.people[i].heard = Some(uc_game::person::Heard { at, phase: Investigate::Look, time: 4.95 });
        r.play(common::DT * 3.0, Controls::default());
        if let Some(h) = r.g.people[i].heard.filter(|h| h.phase == Investigate::WalkOver) {
            hid = Some((at, h.at));
            break;
        }
    }
    let (from, spot) = hid.expect("never went looking for a hiding place");
    let squares = |v: f32| (v / 256.0).floor() as i32;
    let dist = (squares(spot.x) - squares(from.x)).abs() + (squares(spot.z) - squares(from.z)).abs();
    assert!((2..=10).contains(&dist), "{dist} squares off");
    assert!(!r.level.ground.line_of_sight(from + eye, spot + eye), "he could see it");
}
