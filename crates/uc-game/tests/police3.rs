//! Gun Hunt (`police3`): Deeks the snitch knows where a weapon is stashed, and once Darci has
//! it the man who used it comes looking. Checks the script end to end, with the walking done
//! by putting Darci where she has to be and the fighting by knocking the suspect out, as the
//! other mission tests do. Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    let mut r = common::start("police3")?;
    r.heal = false;
    Some(r)
}

impl Run {
    /// Presses action for one frame.
    fn tap_action(&mut self) {
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    /// Walks up to the person event point `way` created and talks to them.
    fn talk_to(&mut self, way: usize) {
        let i = self.g.people.iter().position(|p| p.way == way).expect("they should be here");
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(120.0, 0.0, 0.0));
        self.tap_action();
    }
}

/// Deeks points the way, the gun turns up, its owner comes for it, and the station takes it
/// back in.
#[test]
fn gun_hunt_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    assert!(r.said("Deeks is in the park"), "no briefing: {:?}", r.seen);

    // Deeks is a tramp in the park, dropped at one of a few spots (`move_thing` on `random`).
    r.talk_to(153);
    r.play(40.0, Controls::default());
    assert!(r.said("behind some skips"), "Deeks never said where it was: {:?}", r.seen);
    assert_eq!(r.g.outcome, None, "talking to Deeks shouldn't end the mission");

    // The weapon is put somewhere random too. Walk over it.
    let gun = r.g.items.iter().find(|it| it.way == 54).expect("the stashed weapon").clone();
    assert_eq!(gun.kind, 2, "the stash should be a pistol");
    r.stand(gun.pos);
    r.play(2.0, Controls::default());
    assert!(r.g.player().unwrap().has(uc_game::Weapon::Pistol), "Darci should have picked the weapon up");
    assert!(r.said("weapon secured"), "control was never told: {:?}", r.seen);

    // Its owner turns up looking for it.
    r.play(20.0, Controls::default());
    assert!(r.said("armed man in your area"), "no armed suspect called in: {:?}", r.seen);
    let suspect = r.g.people.iter().position(|p| p.way == 74).expect("the armed suspect");

    // Take him down.
    let from = r.g.player().unwrap().fig.pos;
    r.g.people[suspect].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    assert!(r.said("get that gun back here"), "the suspect never went down: {:?}", r.seen);

    // Back to the station with it.
    r.stand(r.point(90));
    r.play(5.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// Deeks is a snitch, not a collar: killing him or arresting him fails the mission.
#[test]
fn losing_deeks_fails_the_mission() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    let deeks = r.g.people.iter().position(|p| p.way == 153).expect("Deeks");
    let from = r.g.people[deeks].fig.pos;
    r.g.people[deeks].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    assert!(r.said("Deeks is dead"), "nobody noticed: {:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "losing Deeks should lose: {:?}", r.g.outcome);
}
