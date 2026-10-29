//! The store on Gangland's map, in Gang order3 (Getaway Van) and gang3 (Trash the Place):
//! Mako walks in through its door, and the cops who turn up as he gets there come for him.
//! Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::Controls;

mod common;

/// The store's door, a square of its front wall at the back of the porch (`open_the_store`
/// in uc_world::level): from (110,65) to (110,66), 256 up.
const DOOR_X: f32 = 110.0 * 256.0;

/// Up the steps, across the porch and in at the door: Mako ends up on the store's floor, 256
/// up, inside its walls and under its roof. In the map as shipped the door is a wall.
#[test]
fn mako_walks_into_the_store() {
    for mission in ["Gang order3", "gang3"] {
        let Some(mut r) = common::start(mission) else { return };
        r.play(1.0, Controls::default());
        // Nobody about to get in the way.
        r.each_turn = Some(|g| {
            for p in g.people.iter_mut().filter(|p| !p.player) {
                p.fig.pos = vec3(1000.0, 0.0, 1000.0);
            }
        });
        // On the street west of the steps, level with the door.
        r.stand(vec3(26700.0, 0.0, 65.5 * 256.0));
        r.play(10.0, Controls { wish: vec2(1.0, 0.0), ..Default::default() });
        let me = r.player();
        let at = r.g.people[me].fig.pos;
        assert!(at.x > DOOR_X + 128.0, "{mission}: Mako stopped at {at}, short of the door at x {DOOR_X}");
        assert!((at.y - 256.0).abs() < 8.0, "{mission}: Mako is at {at}, not on the store's floor");
        assert!(r.g.wares.under_roof(&r.level.map, &r.level.ground, at).is_some(), "{mission}: at {at}, not indoors");
    }
}

/// Getting to the store (point 142) brings the cops: four guards round the front (143, 144,
/// 148 and 150). A guard who sees Mako calls in every guard near him
/// (`PCOM_alert_nearby_mib_to_attack`), and they all come for him, not only the one who's
/// nearest: once each has noticed him, they make their way to him from across the street.
#[test]
fn the_cops_at_the_store_come_for_mako() {
    let Some(mut r) = common::start("Gang order3") else { return };
    r.play(1.0, Controls::default());
    r.stand(r.point(142));
    r.play(1.0, Controls::default());
    // On the steps up to the porch.
    let steps = vec3(27300.0, 0.0, 16500.0);
    r.stand(steps);
    let me = r.player();
    let guards = [143, 144, 148, 150];
    let dist = |r: &common::Run, w: usize| {
        let d: Vec3 = r.g.people[r.who(w)].fig.pos - r.g.people[me].fig.pos;
        vec2(d.x, d.z).length()
    };
    let from: Vec<f32> = guards.iter().map(|&w| dist(&r, w)).collect();
    r.play(8.0, Controls::default());
    for (&w, &d0) in guards.iter().zip(&from) {
        let p = &r.g.people[r.who(w)];
        assert!(p.aggro(), "cop {w} at {} never came for Mako", p.fig.pos);
        let d = dist(&r, w);
        assert!(d < d0 - 256.0 || d < 300.0, "cop {w} stood where he was: {d0:.0} from Mako, now {d:.0}");
    }
}
