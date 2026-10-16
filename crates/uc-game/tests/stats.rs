//! The stats (#132): what the player does is counted as it happens. Skipped without an
//! install.

use glam::{vec2, vec3};
use uc_game::game::stats::Stats;
use uc_game::{Controls, Game};
use uc_world::Level;

mod common;
use common::DT;

/// An endless run in RTA's park with the first of the wave in front of Darci, everyone else
/// out of it.
fn face_off() -> Option<(Level, Game, usize)> {
    let level = common::load("police1")?;
    let mut g = Game::new_endless(&level, 5, Some(vec2(8827.0, 10274.0)), 9.0);
    let me = g.player.unwrap();
    let mut foe = None;
    for _ in 0..(10.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        foe = g.people.iter().position(|p| !p.player && p.hostile() && !p.removed);
        if foe.is_some() {
            break;
        }
    }
    let foe = foe?;
    for (k, p) in g.people.iter_mut().enumerate() {
        if k != me && k != foe {
            p.removed = true;
        }
    }
    let at = g.people[me].fig.pos;
    let f = g.people[me].fig.facing();
    g.people[foe].fig.pos = at + vec3(f.x, 0.0, f.y) * 400.0;
    Some((level, g, foe))
}

#[test]
fn shots_hits_and_a_knockout_by_gun_are_counted() {
    let Some((level, mut g, foe)) = face_off() else { return };
    g.record = Stats::default();
    assert!(g.give("pistol"));
    g.people[foe].health = 1;
    for _ in 0..40 {
        g.update(DT, &Controls { punch: true, ..Default::default() }, &level);
        g.update(DT, &Controls::default(), &level);
        if g.people[foe].is_down() {
            break;
        }
    }
    assert!(g.people[foe].is_down(), "never shot him down");
    let s = &g.record;
    assert!(s.shots >= 1 && s.hits >= 1 && s.hits <= s.shots, "shots {} hits {}", s.shots, s.hits);
    assert_eq!((s.ko_guns, s.ko_fists), (1, 0));
}

#[test]
fn walking_counts_as_on_foot() {
    let Some((level, mut g, foe)) = face_off() else { return };
    g.people[foe].removed = true;
    g.record = Stats::default();
    let me = g.player().unwrap().fig.pos;
    for _ in 0..(3.0 / DT) as usize {
        g.update(DT, &Controls { wish: vec2(0.0, 1.0), run: true, ..Default::default() }, &level);
    }
    let gone = (g.player().unwrap().fig.pos - me).length() / 256.0;
    assert!(gone > 1.0, "she didn't go anywhere");
    let s = &g.record;
    assert!((s.on_foot - gone).abs() < gone * 0.5 + 0.5, "counted {:.1} on foot for {gone:.1}", s.on_foot);
    assert_eq!((s.driven, s.biked), (0.0, 0.0));
}

/// The totals add up, the longest ride the longest.
#[test]
fn stats_add_up() {
    let a = Stats { arrests: 2, longest_zip: 12.0, on_foot: 3.5, ..Default::default() };
    let b = Stats { arrests: 1, longest_zip: 7.0, on_foot: 1.0, ..Default::default() };
    let t = a.plus(&b);
    assert_eq!((t.arrests, t.longest_zip, t.on_foot), (3, 12.0, 4.5));
    let mut back = Stats::default();
    for (n, v) in t.fields() {
        back.set(n, v);
    }
    assert_eq!(back, t);
}
