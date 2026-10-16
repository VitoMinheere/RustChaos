//! Carrying the injured: in Day Of Reckoning Roper picks up the three wounded survivors on
//! the roof (points 31 to 33, `PCOM_AI_HYPOCHONDRIA`) and carries them to the safe zone by
//! where he starts. Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::person::ko;
use uc_game::{Controls, State, Weapon};

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    let mut r = Run::new(common::load("Finale1")?);
    // The floor giving way (#15) is the one thing it can't run yet; nothing here needs it.
    assert!(r.g.unsupported().iter().all(|u| u.ends_with("no_floor")), "unsupported: {:?}", r.g.unsupported());
    r.stop_at_outcome = false;
    // Bane's men on the roof shoot Roper, and every hit stops what he's doing: they're
    // taken off, as the fighting isn't what's tested here.
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| p.hostile()) {
            p.removed = true;
        }
    });
    Some(r)
}

impl Run {
    fn roper(&self) -> usize {
        self.g.player.unwrap()
    }

    /// Stands Roper `d` from survivor `s`'s middle, facing them.
    fn beside(&mut self, s: usize, d: f32) {
        let body = self.g.people[s].body_pos();
        let at = (0..8)
            .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * d)
            .find(|p| (self.level.ground.surface(&self.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
            .expect("nowhere to stand");
        let r = self.roper();
        self.g.people[r].fig.pos = at;
        self.g.people[r].fig.angle = uc_world::character::angle_of(vec2(body.x - at.x, body.z - at.z));
    }

    /// Presses action, once no filmed conversation is holding the player still.
    fn press_action(&mut self) {
        for _ in 0..(60.0 / DT) as usize {
            if !self.g.in_conversation() {
                break;
            }
            self.play(DT, Controls::default());
        }
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    fn carrying(&self) -> Option<usize> {
        self.g.people[self.roper()].carrying()
    }
}

#[test]
fn the_survivors_lie_injured() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    for way in [31, 32, 33] {
        let s = r.who(way);
        assert!(matches!(r.g.people[s].state, State::Injured), "survivor {way}: {:?}", r.g.people[s].state);
        assert!(!r.g.people[s].is_dead());
    }
    assert_eq!(r.g.people[r.roper()].kind, uc_world::character::person::ROPER);
}

/// Picked up, carried a few steps, then brought to the safe zone and put down, for all
/// three. Each counts as safe once near the zone (an `enemy_radius` trigger), and with the
/// third Roper hands over to Darci.
#[test]
fn roper_carries_the_survivors_to_safety() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    let roper = r.roper();
    for (way, zone) in [(31, 30), (32, 67), (33, 66)] {
        let s = r.who(way);
        r.beside(s, 60.0);
        r.play(0.5, Controls::default());
        r.press_action();
        assert_eq!(r.carrying(), Some(s), "didn't pick up survivor {way}: {:?}", r.g.people[roper].state);
        r.play(2.5, Controls::default());
        assert!(matches!(r.g.people[s].state, State::Carried { .. }), "{:?}", r.g.people[s].state);
        // Carried along: they stay just ahead of him.
        let before = r.g.people[roper].fig.pos;
        let f = r.g.people[roper].fig.facing();
        r.play(1.0, Controls { wish: f, ..Default::default() });
        let at = r.g.people[roper].fig.pos;
        assert!((at - before).length() > 100.0, "Roper didn't move while carrying");
        let gap = vec2(r.g.people[s].fig.pos.x - at.x, r.g.people[s].fig.pos.z - at.z).length();
        assert!((gap - 90.0).abs() < 5.0, "survivor {way} isn't in his arms: {gap}");
        // The rest of the way: to the safe zone, and down.
        let z = r.point(zone);
        let y = r.level.ground.surface(&r.level.map, z.x, z.z, z.y + 64.0);
        r.g.people[roper].fig.pos = vec3(z.x, y, z.z + 200.0);
        r.g.people[roper].fig.angle = uc_world::character::angle_of(Vec2::new(0.0, -1.0));
        r.play(0.5, Controls::default());
        if r.g.player != Some(roper) {
            // The third: safe on arrival, and the player is Darci now.
            break;
        }
        r.press_action();
        r.play(3.0, Controls::default());
        assert_eq!(r.carrying(), None, "never put survivor {way} down");
        assert!(matches!(r.g.people[s].state, State::Injured), "{:?}", r.g.people[s].state);
    }
    r.play(5.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("I got them all in safely")), "{:?}", r.seen);
    assert_eq!(r.g.people[r.g.player.unwrap()].kind, uc_world::character::person::DARCI, "no hand-over to Darci");
}

/// Hit while carrying, Roper drops them; they lie injured again where they fell.
#[test]
fn a_blow_makes_roper_drop_them() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    let s = r.who(31);
    r.beside(s, 60.0);
    r.play(0.5, Controls::default());
    r.press_action();
    r.play(2.5, Controls::default());
    assert_eq!(r.carrying(), Some(s));
    let ri = r.roper();
    let from = r.g.people[ri].fig.pos + vec3(50.0, 0.0, 0.0);
    r.g.people[ri].take_hit(from, 10, ko::KNOCKDOWN, false, false, Weapon::None);
    r.play(0.5, Controls::default());
    assert_eq!(r.carrying(), None);
    assert!(matches!(r.g.people[s].state, State::Injured), "{:?}", r.g.people[s].state);
}
