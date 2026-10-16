//! Gatecrasher (`bball2`): Darci goes to the Wildcat leaders' meet at the ballpark and brings
//! one of them in. Checks the script end to end, with the walking done by putting people where
//! they have to be and the fighting by knocking them down, as the other mission tests do.
//! Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    common::start("bball2")
}

/// One of the three leaders at the meet is knocked down and arrested: that is counter 2, and
/// the win follows. (Killing all three is the way to lose.)
#[test]
fn gatecrasher_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let leader = r.who(73);
    r.look_at(leader, 160.0);
    r.play(1.0, Controls::default());
    let at = r.g.people[leader].fig.pos;
    r.g.people[leader].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, uc_game::Weapon::None);
    // Let her get over any blow she took first: a flinch eats the button.
    for _ in 0..60 {
        let me = r.g.player.unwrap();
        if matches!(r.g.people[me].state, uc_game::State::Move) {
            break;
        }
        r.play(DT * 1.01, Controls::default());
    }
    let body = r.g.people[leader].body_pos();
    let beside = (0..8)
        .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
        .find(|p| (r.level.ground.surface(&r.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
        .expect("somewhere to stand");
    r.stand(beside);
    r.play(0.2, Controls { action: true, ..Default::default() });
    r.play(4.0, Controls::default());
    let l = &r.g.people[leader];
    assert!(l.arrested || matches!(l.state, uc_game::State::Cuffed), "the leader wasn't arrested: {:?}", l.state);

    r.play(10.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("I GOT HIM"), "the arrest didn't count: {:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
