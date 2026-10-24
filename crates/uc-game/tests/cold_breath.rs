//! Breath in the cold (#307, Person.cpp:11791): on snow, people standing about breathe out a
//! puff from their head once a breath; elsewhere they don't. Skipped without an install.

mod common;

use uc_game::Controls;

/// Puffs seen by her head over `secs` standing still: how many breaths' worth.
fn breaths(r: &mut common::Run, secs: f32) -> usize {
    let me = r.player();
    let mut most = 0;
    for _ in 0..(secs / common::DT) as usize {
        r.play(common::DT, Controls::default());
        let head = r.g.people[me].fig.part_world(11).map(|(_, at)| at).unwrap();
        let near = r.g.pyro.puffs.iter().filter(|p| (p.pos - head).length() < 80.0).count();
        most = most.max(near);
    }
    most
}

#[test]
fn on_snow_her_breath_shows() {
    let Some(level) = common::load("snow2") else { return };
    let mut r = common::Run::new(level);
    r.heal = true;
    r.play(1.0, Controls::default());
    assert!(breaths(&mut r, 6.0) > 0, "no breath in the cold");
}

#[test]
fn in_the_city_it_doesnt() {
    let Some(mut r) = common::start("police1") else { return };
    r.heal = true;
    r.play(1.0, Controls::default());
    assert_eq!(breaths(&mut r, 6.0), 0, "breath where it isn't cold");
}
