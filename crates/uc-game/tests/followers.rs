//! Followers (`PCOM_MOVE_FOLLOW`): Gordansky, freed in Media Trouble, keeps close behind
//! Darci, out through the beach house's door and down the road, walking or running, by the
//! way she went; and isn't someone Darci's gun picks out. Skipped without an install.

use glam::{Vec2, Vec3};
use uc_game::person::{Weapon, MOVE_FOLLOW};
use uc_game::{Controls, Game};
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
