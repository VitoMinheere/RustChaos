//! Estate of Emergency's tripwires: walking through a beam sets off the alarm (guards and
//! explosions), crawling under a high one doesn't, and the security panel switches them
//! all off. Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::Controls;

mod common;
use common::{Run, DT};

/// A guard the alarm of the tripwire at point 46 (by the house's east side) brings out.
const GUARD: usize = 54;
/// The security panel (`player_uses_radius`, "Security systems off!").
const PANEL: usize = 88;

fn start() -> Option<Run> {
    let mut r = common::start("estate2")?;
    r.g.skip_cutscene();
    r.stop_at_outcome = false;
    Some(r)
}

impl Run {
    /// Puts the player on the ground at (x, z), facing along `dir`: the floor under
    /// height `y` (indoors, not on the roof).
    fn put(&mut self, at: Vec2, y: f32, dir: Vec2) {
        let p = self.g.player.unwrap();
        let y = self.level.ground.surface(&self.level.map, at.x, at.y, y + 64.0);
        self.g.people[p].fig.pos = vec3(at.x, y, at.y);
        self.g.people[p].fig.angle = uc_world::character::angle_of(dir);
    }

    fn pos(&self) -> Vec2 {
        let p = self.g.people[self.g.player.unwrap()].fig.pos;
        vec2(p.x, p.z)
    }

    fn spawned(&self, way: usize) -> bool {
        self.g.people.iter().any(|p| p.way == way)
    }

    /// The wire point `WIRE` watches: its middle, and the way across it. Stands the player
    /// `back` short of it, on the side away from the house.
    fn before_wire(&mut self, back: f32) -> (Vec2, Vec2) {
        let w = self.g.trips.wires.iter().find(|w| (w.a - vec2(22049.0, 15744.0)).length() < 64.0).expect("no wire at point 46");
        let y = w.y;
        let mid = (w.a + w.b) / 2.0;
        let across = (w.b - w.a).perp().normalize();
        self.put(mid - across * back, y, across);
        (mid, across)
    }
}

fn walk(dir: Vec2) -> Controls {
    Controls { wish: dir, ..Default::default() }
}

#[test]
fn walking_through_a_beam_sets_off_the_alarm() {
    let Some(mut r) = start() else { return };
    assert_eq!(r.g.trips.wires.len(), 9, "Estate of Emergency has nine wires");
    r.play(1.0, Controls::default());
    assert!(!r.spawned(GUARD), "the guard is out before the alarm");
    let (mid, across) = r.before_wire(150.0);
    r.play(2.0, walk(across));
    assert!((r.pos() - mid).dot(across) > 0.0, "never got past the wire");
    assert!(r.spawned(GUARD), "walking through the beam brought nobody out");
}

#[test]
fn crawling_under_a_high_beam_goes_unnoticed() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    let (mid, across) = r.before_wire(150.0);
    // Action standing still crouches; held, a direction crawls.
    r.play(DT, Controls { action: true, sprint: true, ..Default::default() });
    r.play(1.0, Controls { sprint: true, ..Default::default() });
    let p = r.g.player.unwrap();
    assert!(r.g.people[p].crouching(), "action didn't crouch: {:?}", r.g.people[p].state);
    r.play(2.5, Controls { wish: across, sprint: true, ..Default::default() });
    assert!(r.g.people[p].crouching(), "stopped crawling: {:?}", r.g.people[p].state);
    assert!((r.pos() - mid).dot(across) > 40.0, "never crawled past the wire: {:.0}", (r.pos() - mid).dot(across));
    assert!(!r.spawned(GUARD), "crawling under the beam set it off");
    // Letting go stands her up.
    r.play(1.5, Controls::default());
    assert!(!r.g.people[p].crouching(), "still down: {:?}", r.g.people[p].state);
}

#[test]
fn the_security_panel_switches_the_wires_off() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    let e = r.level.mission.event_points.iter().find(|e| e.index == PANEL).unwrap();
    r.put(vec2(e.x as f32, e.z as f32), e.y as f32, vec2(0.0, 1.0));
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(1.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("Security systems off")), "the panel did nothing: {:?}", r.seen);
    assert!(r.g.trips.wires.iter().all(|w| w.off), "a wire is still on");
    let (mid, across) = r.before_wire(150.0);
    r.play(2.0, walk(across));
    assert!((r.pos() - mid).dot(across) > 0.0, "never got past the wire");
    assert!(!r.spawned(GUARD), "a switched-off wire set off the alarm");
}
