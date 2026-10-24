//! People who stand still go back to their spot (#248, `PCOM_AI_STATE_HOMESICK`): pushed off
//! it, they walk back, all the way, and take up their pose again facing the way they did.
//! Headline Hostage's hostage (point 3), who stands with her hands up. Skipped without an
//! install.

use glam::vec3;
use uc_game::{Controls, State};

mod common;

#[test]
fn the_hostage_goes_back_to_her_spot() {
    let Some(mut r) = common::start("Gordout1") else { return };
    r.play(2.0, Controls::default());
    let i = r.who(3);
    let (home, angle) = r.g.people[i].home.expect("she was made somewhere");
    // Off her spot, in the open: try a few ways.
    let eye = vec3(0.0, 96.0, 0.0);
    let off = [vec3(500.0, 0.0, 0.0), vec3(-500.0, 0.0, 0.0), vec3(0.0, 0.0, 500.0), vec3(0.0, 0.0, -500.0)]
        .into_iter()
        .find(|d| r.level.ground.line_of_sight(home + eye, home + *d + eye))
        .expect("nowhere open round her");
    let to = home + off;
    let y = r.level.ground.surface(&r.level.map, to.x, to.z, to.y + 64.0);
    r.g.people[i].fig.pos = vec3(to.x, y, to.z);
    r.g.people[i].state = State::Move;
    r.play(8.0, Controls::default());
    let p = &r.g.people[i];
    let d = glam::vec2(p.fig.pos.x - home.x, p.fig.pos.z - home.z).length();
    assert!(d <= 0x40 as f32 + 8.0, "she's {d} from her spot");
    assert!(matches!(p.state, State::Pose { .. }), "hands up again: {:?}", p.state);
    assert!((p.fig.angle - angle).abs() < 1e-3, "facing the way she did");
}
