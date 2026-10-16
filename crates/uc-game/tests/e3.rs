//! Media Trouble (`e3`): Darci frees the reporter, Ms. Gordansky, and walks her back to the
//! station. Checks the script end to end, with the walking done by putting people where they
//! have to be and the fighting by knocking them down, as the other mission tests do. Skipped
//! without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start() -> Option<Run> {
    common::start("e3")
}

/// Her captor (62) goes down, she follows Darci (point 85), and once she's at the station
/// (point 115) she says her thanks and the mission is won.
#[test]
fn media_trouble_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let captor = r.who(62);
    r.look_at(captor, 160.0);
    let from = r.g.people[captor].fig.pos;
    r.g.people[captor].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    let reporter = r.who(63);
    assert_eq!(r.g.people[reporter].mv, 5, "she should follow Darci now: {:?}", r.seen);

    // Back to the station, the two of them.
    let station = r.point(115);
    r.stand(station + vec3(200.0, 0.0, 0.0));
    let y = r.level.ground.surface(&r.level.map, station.x, station.z, station.y + 64.0);
    r.g.people[reporter].fig.pos = vec3(station.x, y, station.z);
    for _ in 0..90 {
        if r.g.outcome.is_some() {
            break;
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(!r.g.people[reporter].is_dead(), "the reporter mustn't come to harm");
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
