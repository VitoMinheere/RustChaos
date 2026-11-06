//! Followers (`PCOM_MOVE_FOLLOW`): Gordansky, freed in Media Trouble, keeps close behind
//! Darci, out through the beach house's door and down the road, walking or running, by the
//! way she went; and isn't someone Darci's gun picks out. Skipped without an install.

use glam::{Vec2, Vec3};
use uc_game::person::{Weapon, AI_GANG, MOVE_FOLLOW};
use uc_game::{Controls, Game, Sfx};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Media Trouble with only Darci and Gordansky, she following, Darci beside her:
/// (level, game, Darci, Gordansky).
fn freed() -> Option<(Level, Game, usize, usize)> {
    let install = uc_formats::default_install()?;
    let level = Level::load(&install, "e3", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..10 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let me = g.player?;
    let h = g.people.iter().position(|p| p.kind == uc_world::character::person::HOSTAGE)?;
    for (i, p) in g.people.iter_mut().enumerate() {
        if i != me && i != h {
            p.removed = true;
        }
    }
    (g.people[h].mv, g.people[h].follow) = (MOVE_FOLLOW, 0);
    let at = g.people[h].fig.pos;
    g.teleport(at + Vec3::new(100.0, 0.0, 0.0), &level);
    Some((level, g, me, h))
}

/// Darci goes out of the beach house and up the road for `secs`, by the navigation map,
/// walking or running: the furthest Gordansky falls behind after the first second (squares).
fn furthest_behind(run: bool, secs: usize) -> Option<f32> {
    let (level, mut g, me, h) = freed()?;
    let dest = (40u8, 80u8);
    let mut seed = 1u32;
    let mut worst: f32 = 0.0;
    for f in 0..secs * 30 {
        let at = g.people[me].fig.pos;
        let sq = ((at.x / 256.0) as u8, (at.z / 256.0) as u8);
        let mut c = Controls::default();
        if sq != dest {
            let (act, _) = g.nav().route(sq, dest, uc_world::mav::CAPS_DARCI, &mut || {
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                seed >> 16
            });
            let to = Vec2::new(act.dest.0 as f32 * 256.0 + 128.0, act.dest.1 as f32 * 256.0 + 128.0);
            c.wish = (to - Vec2::new(at.x, at.z)).normalize_or_zero();
            c.run = run;
        }
        g.update(DT, &c, &level);
        if f >= 30 {
            worst = worst.max((g.people[me].fig.pos - g.people[h].fig.pos).length() / 256.0);
        }
    }
    // Darci got somewhere.
    assert!(g.people[me].fig.pos.z < 95.0 * 256.0, "Darci didn't get out: {:?}", g.people[me].fig.pos / 256.0);
    Some(worst)
}

#[test]
fn she_keeps_close_behind_at_a_walk() {
    let Some(worst) = furthest_behind(false, 20) else { return };
    assert!(worst < 1.5, "fell {worst:.1} squares behind");
}

#[test]
fn she_keeps_close_behind_at_a_run() {
    let Some(worst) = furthest_behind(true, 10) else { return };
    assert!(worst < 1.5, "fell {worst:.1} squares behind");
}

#[test]
fn darcis_gun_doesnt_pick_her_out() {
    let Some((level, mut g, me, h)) = freed() else { return };
    g.people[me].give(Weapon::Pistol);
    g.people[me].weapon = Weapon::Pistol;
    // Gordansky straight ahead of Darci, nobody else about.
    let at = g.people[me].fig.pos;
    let ahead = Vec2::new(g.people[h].fig.pos.x - at.x, g.people[h].fig.pos.z - at.z).normalize_or_zero();
    for _ in 0..15 {
        g.update(DT, &Controls { wish: ahead * 0.01, ..Default::default() }, &level);
    }
    assert_eq!(g.target, None, "her gun picked out the hostage she's freed");
    // But someone still held hostage can be (`calc_target_score_new` scores them).
    g.people[h].mv = 1;
    for _ in 0..15 {
        g.update(DT, &Controls { wish: ahead * 0.01, ..Default::default() }, &level);
    }
    assert_eq!(g.target, Some(h));
}

#[test]
fn darci_drawing_her_gun_doesnt_send_her_running() {
    let Some((level, mut g, me, h)) = freed() else { return };
    for _ in 0..5 {
        g.update(DT, &Controls::default(), &level);
    }
    g.people[me].give(Weapon::Pistol);
    g.people[me].weapon = Weapon::Pistol;
    for _ in 0..30 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.people[h].scared.is_none(), "the hostage she's freed ran from Darci's gun");
    assert_eq!(g.people[h].mv, MOVE_FOLLOW);
}

#[test]
fn a_thug_shooting_at_darci_doesnt_send_her_running() {
    let Some((level, mut g, me, h)) = freed() else { return };
    // A thug with a pistol, 4 squares the other side of Darci from her, coming for Darci.
    let at = g.people[me].fig.pos;
    let away = (at - g.people[h].fig.pos).with_y(0.0).normalize_or_zero();
    let i = (0..g.people.len()).find(|&i| i != me && i != h && g.people[i].health > 0).expect("nobody");
    let q = &mut g.people[i];
    q.removed = false;
    (q.ai, q.kind, q.mv, q.fake, q.bent, q.weapon, q.skill) = (AI_GANG, ptype::THUG_RED, 1, false, 0, Weapon::Pistol, 10);
    q.give(Weapon::Pistol);
    q.fig.pos = at + away * 400.0;
    q.foe = Some(me);
    q.go_for();
    g.take_sounds();
    let mut shots = 0;
    for _ in 0..10 * 30 {
        g.people[me].health = g.people[me].max_health;
        g.people[i].fig.pos = at + away * 400.0;
        g.update(DT, &Controls::default(), &level);
        shots += g.take_sounds().iter().filter(|(s, _)| matches!(s, Sfx::Shot(_))).count();
        assert!(g.people[h].scared.is_none(), "the hostage she's freed ran from the thug's shot");
    }
    assert!(shots > 0, "the thug never fired");
    assert_eq!(g.people[h].mv, MOVE_FOLLOW);
}
