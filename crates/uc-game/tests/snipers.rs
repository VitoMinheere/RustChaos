//! Snipers (#249, `PCOM_AI_STATE_SNIPE`): an assassin told to stand still stays where he is,
//! gun out, and shoots his mark whenever he can see them. One of RTA's crowd made an assassin
//! after Darci. Skipped without an install.

use glam::vec3;
use uc_game::person::AI_CIV;
use uc_game::{Controls, Weapon};
use uc_world::character::person as ptype;

mod common;
use common::Run;

/// RTA with a still assassin `dist` in front of Darci, facing her, marked on her, with a
/// pistol. Returns him.
fn sniper(dist: f32) -> Option<(Run, usize)> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let start = r.g.people[pi].fig.pos;
    let eye = vec3(0.0, 96.0, 0.0);
    // Somewhere near her start with a clear view `dist` along the ground one way.
    let (me, dir) = (-6..=6)
        .flat_map(|x| (-6..=6).map(move |z| start + vec3(x as f32, 0.0, z as f32) * 512.0))
        .flat_map(|c| (0..16).map(move |k| (c, k as f32 * std::f32::consts::TAU / 16.0)))
        .map(|(c, a)| {
            let y = r.level.ground.surface(&r.level.map, c.x, c.z, c.y + 64.0);
            (vec3(c.x, y, c.z), glam::vec2(-a.sin(), -a.cos()))
        })
        .find(|(c, f)| {
            let end = *c + vec3(f.x, 0.0, f.y) * dist;
            let ground = r.level.ground.surface(&r.level.map, end.x, end.z, c.y + 64.0);
            (ground - c.y).abs() < 8.0 && r.level.ground.line_of_sight(*c + eye, end + eye)
        })
        .expect("nowhere open");
    r.g.people[pi].fig.pos = me;
    let i = r.g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("crowd");
    let at = me + vec3(dir.x, 0.0, dir.y) * dist;
    let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    let way = r.g.people[pi].way;
    let q = &mut r.g.people[i];
    (q.ai, q.mv, q.fake, q.kind, q.bent, q.other) = (3, 1, false, ptype::THUG_RED, 0, way);
    q.give(Weapon::Pistol);
    q.weapon = Weapon::Pistol;
    q.fig.pos = vec3(at.x, y, at.z);
    q.fig.angle = uc_world::character::angle_of(-dir);
    q.home = Some((q.fig.pos, q.fig.angle));
    r.heal = false;
    Some((r, i))
}

/// Plays `secs`, and whether person `i`'s gun went off meanwhile (its muzzle flash).
fn fires(r: &mut Run, i: usize, secs: f32) -> bool {
    let mut fired = false;
    for _ in 0..(secs / common::DT) as usize {
        r.play(common::DT, Controls::default());
        fired |= r.g.people[i].flash > 0.0;
    }
    fired
}

#[test]
fn a_sniper_stays_put_and_shoots() {
    let Some((mut r, i)) = sniper(5.0 * 256.0) else { return };
    let from = r.g.people[i].fig.pos;
    let fired = fires(&mut r, i, 8.0);
    assert!((r.g.people[i].fig.pos - from).length() < 16.0, "he moved {}", (r.g.people[i].fig.pos - from).length());
    // Whether a shot hits is the AI's aim; that he shoots is what's tested.
    assert!(fired, "he never fired");
}

#[test]
fn out_of_his_sight_he_waits() {
    let Some((mut r, i)) = sniper(9.0 * 256.0) else { return };
    let from = r.g.people[i].fig.pos;
    let fired = fires(&mut r, i, 4.0);
    assert!((r.g.people[i].fig.pos - from).length() < 16.0, "he came closer");
    assert!(!fired, "he fired at her out of his sight");
}
