//! Auto-Destruct (`carbomb1`): two cars with bombs in them go to the bomb squad's garages, the
//! bombers are arrested, and the UCNN video they hid at their den is found and brought back.
//! Checks the script end to end, with the walking and driving done by putting people and cars
//! where they have to be and the fighting by knocking them down, as the other mission tests
//! do. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Outcome, Search};

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    common::start("carbomb1")
}

/// Both bomb cars delivered, both bombers cuffed, the video found and brought back.
#[test]
fn auto_destruct_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    // The first car to the garage (point 8 watches it), which brings out the second.
    r.deliver_car(6, 8);
    assert!(r.g.point_active(8), "the first car didn't count: {:?}", r.seen);
    r.play(2.0, Controls::default());
    r.deliver_car(82, 92);
    assert!(r.g.point_active(92), "the second car didn't count: {:?}", r.seen);

    // The bombers show themselves.
    r.play(3.0, Controls::default());
    r.cuff(147);
    r.cuff(148);
    assert!(r.seen.iter().any(|m| m.contains("You got the bombers")), "{:?}", r.seen);

    // The video is stashed at their den; search the prop it's in and pick it up.
    r.play(6.0, Controls::default());
    let ob = r.g.hidden.iter().find(|(_, it)| it.way == 181).expect("the video, stashed").0;
    let o = r.level.map.objects[ob];
    let prop = vec3(o.x as f32, o.y as f32, o.z as f32);
    r.stand(prop + vec3(110.0, 0.0, 0.0));
    let me = r.g.player.unwrap();
    r.g.people[me].face_towards(prop);
    r.play(DT, Controls { action: true, sprint: true, ..Default::default() });
    r.play(Search::FOUND + 0.2, Controls { sprint: true, ..Default::default() });
    let video = r.g.items.iter().find(|it| it.way == 181).expect("the search turned up nothing").pos;
    r.play(Search::DONE, Controls::default());
    r.stand(video);
    r.play(1.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("UCNN video")), "the video was never lifted: {:?}", r.seen);

    // Back to base with it.
    r.stand(r.point(169));
    for _ in 0..90 {
        if r.g.outcome.is_some() {
            break;
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
