//! Small things about how the AI behaves (#255): someone with no AI (`PCOM_AI_NONE`) does
//! nothing, whatever their move says; and a `PCOM_AI_SHOOT_DEAD` fires as fast as the gun
//! goes off, where an ordinary assassin bides his time. In RTA, with its crowd. Skipped
//! without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, MOVE_FOLLOW};
use uc_game::{Controls, Weapon};
use uc_world::character::person as ptype;

mod common;
use common::{Run, DT};

fn crowd(r: &Run, n: usize) -> usize {
    r.g.people.iter().enumerate().filter(|(_, q)| q.fake && q.ai == AI_CIV && !q.is_dead()).map(|(i, _)| i).nth(n).expect("crowd")
}

#[test]
fn no_ai_does_nothing() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let i = crowd(&r, 0);
    let me = r.g.people[r.player()].fig.pos;
    let at = me + vec3(600.0, 0.0, 0.0);
    let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    let q = &mut r.g.people[i];
    (q.ai, q.mv, q.follow, q.fake) = (0, MOVE_FOLLOW, 0, false);
    q.fig.pos = vec3(at.x, y, at.z);
    r.play(3.0, Controls::default());
    assert!((r.g.people[i].fig.pos - vec3(at.x, y, at.z)).length() < 16.0, "set to follow, but with no AI he went after her");
}

/// How many times an assassin of `ai`, standing still 4 squares from Darci and marked on
/// her, fires in `secs`.
fn shots(ai: u8, secs: f32) -> Option<usize> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let i = crowd(&r, 0);
    let start = r.g.people[pi].fig.pos;
    let eye = vec3(0.0, 96.0, 0.0);
    let dist = 4.0 * 256.0;
    // Somewhere near her start with a clear view that far along flat ground.
    let (me, f) = (-6..=6)
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
    let at = me + vec3(f.x, 0.0, f.y) * dist;
    let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    let way = r.g.people[pi].way;
    let q = &mut r.g.people[i];
    (q.ai, q.mv, q.fake, q.kind, q.bent, q.other, q.skill) = (ai, 1, false, ptype::THUG_RED, 0, way, 5);
    q.give(Weapon::Pistol);
    q.weapon = Weapon::Pistol;
    q.fig.pos = vec3(at.x, y, at.z);
    q.fig.angle = uc_world::character::angle_of(-f);
    q.home = Some((q.fig.pos, q.fig.angle));
    let (mut n, mut was) = (0, false);
    for _ in 0..(secs / DT) as usize {
        r.g.people[pi].health = r.g.people[pi].max_health;
        r.play(DT, Controls::default());
        let now = r.g.people[i].flash > 0.0;
        n += usize::from(now && !was);
        was = now;
    }
    Some(n)
}

#[test]
fn shoot_dead_doesnt_wait() {
    let (Some(fast), Some(slow)) = (shots(21, 6.0), shots(3, 6.0)) else { return };
    assert!(fast > slow * 2, "shoot-dead fired {fast} times to the assassin's {slow}");
}
