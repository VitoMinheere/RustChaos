//! Barrels (fallen/Source/barrel.cpp): a shot blows up anything but a cone or a bin, with a
//! shockwave that knocks people down, and the point that made it counts it as killed.
//! Semtex is won by shooting its two drums. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::{Charge, Controls, Outcome, Weapon};

mod common;
use common::{Run, DT};

fn start(mission: &str) -> Option<Run> {
    common::start(mission)
}

impl Run {
    fn barrel(&self, way: usize) -> usize {
        self.g.barrels.iter().position(|b| b.way == way).unwrap_or_else(|| panic!("no barrel from point {way}"))
    }

    /// Stands the player, pistol drawn, a few squares from barrel `k` with a clear shot.
    fn line_up(&mut self, k: usize) {
        let at = self.g.barrels[k].pos;
        let eye = vec3(0.0, 140.0, 0.0);
        let spot = (0..32)
            .flat_map(|i| [300.0, 450.0, 600.0].map(|d| at + vec3((i as f32 * 0.196).sin(), 0.0, (i as f32 * 0.196).cos()) * d))
            .find(|p| {
                (self.level.ground.surface(&self.level.map, p.x, p.z, at.y + 64.0) - at.y).abs() < 16.0
                    && self.level.ground.line_of_sight(*p + eye, at + vec3(0.0, 50.0, 0.0))
            })
            .expect("nowhere to shoot it from");
        let p = self.g.player.unwrap();
        let me = &mut self.g.people[p];
        me.fig.pos = spot;
        me.give(Weapon::Pistol);
        me.weapon = Weapon::Pistol;
    }

    /// Fires until barrel `k` goes up (or a few seconds pass).
    fn shoot(&mut self, k: usize) -> bool {
        for _ in 0..12 {
            if self.g.barrels[k].gone {
                break;
            }
            self.play(DT, Controls { punch: true, ..Default::default() });
            self.play(0.4, Controls::default());
        }
        self.g.barrels[k].gone
    }
}

/// "One shot oughta set the whole thing off": the semtex is in two oil drums on the dock
/// roofs (points 183 and 199), and blowing both up wins it.
#[test]
fn semtex_is_won_by_shooting_the_two_drums() {
    let Some(mut r) = start("Semtex") else { return };
    r.play(1.0, Controls::default());
    for way in [199, 183] {
        let k = r.barrel(way);
        assert!(r.g.barrels[k].explodes, "drum {way} doesn't explode");
        r.line_up(k);
        r.play(0.5, Controls::default());
        assert_eq!(r.g.aiming.barrel, Some(k), "the gun isn't on drum {way}");
        assert!(r.shoot(k), "shooting drum {way} didn't blow it up");
    }
    r.play(10.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("blowing that Semtex to hell")), "{:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// The blast knocks down whoever is close, and it's the shooter's doing.
#[test]
fn a_barrel_going_up_knocks_people_down() {
    let Some(mut r) = start("Semtex") else { return };
    r.play(1.0, Controls::default());
    let k = r.barrel(324);
    let at = r.g.barrels[k].pos;
    let victim = r.g.people.iter().position(|p| p.hostile() && !p.removed && !p.is_dead()).unwrap();
    r.g.people[victim].fig.pos = at + vec3(100.0, 0.0, 0.0);
    let p = r.g.player.unwrap();
    r.g.shoot_barrel(k, p, &r.level);
    r.play(0.5, Controls::default());
    assert!(r.g.barrels[k].gone);
    assert!(r.g.people[victim].is_down() || r.g.people[victim].is_dead(), "{:?}", r.g.people[victim].state);
}

/// Cones and bins only get knocked about.
#[test]
fn cones_and_bins_dont_explode() {
    let Some(mut r) = start("police1") else { return };
    r.play(1.0, Controls::default());
    let p = r.g.player.unwrap();
    for way in [75, 95] {
        let k = r.barrel(way);
        assert!(!r.g.barrels[k].explodes);
        r.g.shoot_barrel(k, p, &r.level);
        assert!(!r.g.barrels[k].gone, "point {way} blew up");
        assert!(r.g.barrels[k].spheres.is_some(), "point {way} wasn't knocked");
    }
}

/// Only a shot sets a barrel off: a charge going off beside it leaves it standing.
#[test]
fn a_charge_leaves_barrels_alone() {
    let Some(mut r) = start("Semtex") else { return };
    r.play(1.0, Controls::default());
    let k = r.barrel(324);
    let at: Vec3 = r.g.barrels[k].pos;
    r.g.charges.push(Charge { pos: at + vec3(60.0, 0.0, 0.0), fuse: 0.1 });
    r.play(2.0, Controls::default());
    assert!(!r.g.barrels[k].gone, "the charge set the barrel off");
}

/// Barrels put down where there's one already stand on top of it (`BARREL_alloc`, 93 up),
/// and fall when the one beneath goes up (`BARREL_process_normal`).
#[test]
fn stacked_barrels_fall_when_the_one_under_goes() {
    let Some(mut r) = start("Semtex") else { return };
    r.play(1.0, Controls::default());
    let top = r.g.barrels.iter().position(|b| b.on.is_some()).expect("nothing stacked in Semtex");
    let under = r.g.barrels[top].on.unwrap();
    assert_eq!(r.g.barrels[top].pos.y, r.g.barrels[under].pos.y + 93.0);
    // Stays put while the one under it does.
    r.play(1.0, Controls::default());
    assert_eq!(r.g.barrels[top].on, Some(under));
    // Gone from under it: down it comes, onto the ground, and settles (standing or on its
    // side).
    let floor = r.g.barrels[under].pos.y;
    r.g.barrels[under].gone = true;
    r.play(6.0, Controls::default());
    let b = &r.g.barrels[top];
    assert!(b.on.is_none() && b.spheres.is_none(), "still moving");
    let mid = b.middle().y - floor;
    assert!(mid > 10.0 && mid < 60.0, "came to rest with its middle {mid} over the ground");
}
