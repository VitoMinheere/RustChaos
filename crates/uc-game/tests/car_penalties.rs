//! Cars (#256): the player blowing up a police car or a meat wagon earns a red mark, any other
//! car doesn't (`VEH_reduce_health`, vehicle.cpp:4804); and each type has its own engine
//! (`ENGINE_*`): a van tops out well below a police car. Skipped without an install.

use uc_game::vehicle::Key;
use uc_game::{Controls, Drive, Vehicle};

mod common;

/// Blows up a car of `kind` that the player damaged, in RTA: how many red marks it earned.
fn red_marks_for(kind: u8) -> Option<u8> {
    let mut r = common::start("police1")?;
    r.play(1.0, Controls::default());
    let pi = r.player();
    let at = r.g.people[pi].fig.pos + glam::vec3(2000.0, 0.0, 0.0);
    r.g.vehicles.push(Vehicle::new(0, kind, at, 0.0, Key::from_vk(0)));
    let v = r.g.vehicles.len() - 1;
    let before = r.g.red_marks;
    r.g.vehicles[v].hurt_by = Some(pi);
    r.g.vehicles[v].health = 0;
    r.play(0.5, Controls::default());
    Some(r.g.red_marks - before)
}

#[test]
fn blowing_up_a_police_car_earns_a_red_mark() {
    let (Some(police), Some(wagon), Some(van)) = (red_marks_for(3), red_marks_for(6), red_marks_for(0)) else { return };
    assert_eq!((police, wagon, van), (1, 1, 0));
}

#[test]
fn a_van_is_slower_than_a_police_car() {
    let Some(r) = common::start("testdrive2") else { return };
    let top = |kind: u8| {
        let start = r.level.spawn.0;
        let mut v = Vehicle::new(0, kind, start, r.level.spawn.1, Key::from_vk(0));
        let mut fastest: f32 = 0.0;
        for _ in 0..(3.0 / common::DT) as usize {
            v.update(Drive { throttle: 1.0, faster: true, ..Default::default() }, common::DT, &r.level.ground, &r.level.map);
            fastest = fastest.max(v.speed);
        }
        fastest
    };
    let (van, police) = (top(0), top(3));
    assert!(van < police * 0.9, "van {van} against police {police}");
}
