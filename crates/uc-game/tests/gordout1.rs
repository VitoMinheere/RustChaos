//! Headline Hostage (`Gordout1`): Gordansky has to be brought in, not shot and not arrested.
//! Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Outcome};

mod common;
use common::Run;

fn start() -> Option<Run> {
    let mut r = common::start("Gordout1")?;
    r.heal = false;
    Some(r)
}

impl Run {
    /// Puts the person `way` created on the ground at `p`.
    fn put(&mut self, way: usize, p: Vec3) {
        let i = self.who(way);
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }
}

/// Gordansky delivered: the drop is a radius round point 15, watching where he is.
#[test]
fn headline_hostage_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    assert_eq!(r.g.outcome, None, "nothing should have been decided yet");

    r.put(3, r.point(15));
    r.play(5.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("You got Gordansky"), "the drop never registered: {:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// He's wanted alive and at liberty: killing him or arresting him both fail it.
#[test]
fn gordansky_must_be_brought_in_whole() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());
    let g = r.who(3);
    let from = r.g.people[g].fig.pos;
    r.g.people[g].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    assert!(r.said("Gordansky is dead"), "nobody noticed: {:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "killing him should lose: {:?}", r.g.outcome);
}
