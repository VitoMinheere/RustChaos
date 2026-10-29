//! Creatures (`create_creature`): Insane Assault (`Baalrog3`) is won by killing the Balrog,
//! which only bullets and blasts can do; Day Of Reckoning (`Finale1`) has a gargoyle and Bane.
//! Psycho Park and Stern Revenge ask only for bats, which the shipped game never makes.
//! Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::creature::{self, State as Cs};
use uc_game::person::ko;
use uc_game::{Controls, Outcome, State, Weapon};

mod common;
use common::Run;

fn start(mission: &str) -> Option<Run> {
    Some(Run::new(common::load(mission)?))
}

impl Run {
    fn set_down(&mut self, p: Vec3) {
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
        self.g.people[i].state = State::Move;
    }

    /// Insane Assault's two gang members turn up a second after Darci reaches Brick Road
    /// (point 20); the Balrog waits on both of them going down.
    fn wake_the_balrog(&mut self) {
        let corner = self.level.mission.event_points.iter().find(|e| e.index == 20).unwrap();
        self.set_down(vec3(corner.x as f32, corner.y as f32, corner.z as f32));
        self.play(3.0, Controls::default());
        assert!(self.g.creatures.is_empty(), "he waits on point 42");
        self.kill(21);
        self.kill(22);
        self.play(2.0, Controls::default());
    }

    fn kill(&mut self, way: usize) {
        let i = self.g.people.iter().position(|p| p.way == way).expect("person");
        let from = self.g.people[i].fig.pos;
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }
}

#[test]
fn bats_make_nothing() {
    for mission in ["park2", "skymiss2"] {
        let Some(mut r) = start(mission) else { return };
        assert!(r.g.unsupported().is_empty(), "{mission}: unsupported {:?}", r.g.unsupported());
        r.play(1.0, Controls::default());
        assert!(r.g.creatures.is_empty(), "{mission}: bats are compiled out of the original");
    }
}

/// Insane Assault: the two gang members down, and the Balrog turns up; shoot him dead and
/// that's the mission.
#[test]
fn the_balrog_comes_and_goes_down_to_bullets() {
    let Some(mut r) = start("Baalrog3") else { return };
    assert!(r.g.unsupported().is_empty(), "unsupported: {:?}", r.g.unsupported());
    // He bullies whoever is nearest, the city's extra people as much as Darci: keep them out
    // of it (and always in sight, so they aren't brought back).
    r.g.view_dist = f32::MAX;
    for p in r.g.people.iter_mut().filter(|p| p.fake) {
        p.removed = true;
    }
    r.wake_the_balrog();
    assert_eq!(r.g.creatures.len(), 1, "point 39 should make the Balrog");
    assert_eq!(r.g.creatures[0].kind, creature::BALROG);
    let home = r.g.creatures[0].fig.pos;

    // Six squares off and in plain view: he's after Darci, and comes for her or throws fire.
    r.set_down(home + vec3(6.0 * 256.0, 0.0, 0.0));
    let start = r.g.creatures[0].fig.pos;
    let mut threw = false;
    let mut states = std::collections::BTreeSet::new();
    for _ in 0..300 {
        r.play(0.1, Controls::default());
        threw |= !r.g.fireballs.is_empty();
        states.insert(format!("{:?}", r.g.creatures[0].state));
    }
    println!("states seen: {states:?}");
    let me = r.g.player.unwrap();
    assert_eq!(r.g.creatures[0].target, Some(me), "he should have picked on the player");
    let moved = (r.g.creatures[0].fig.pos - start).length();
    assert!(moved > 64.0 || threw, "in thirty seconds he neither moved ({moved:.0}) nor threw anything");

    // Fists do nothing, guns do. Pistol in hand and plenty of rounds: keep firing.
    let health = r.g.creatures[0].health;
    r.g.people[me].give(Weapon::Pistol);
    r.g.people[me].weapon = Weapon::Pistol;
    r.g.people[me].ammo[0] = 999;
    // Somewhere five squares off with a clear view of him.
    let b = r.g.creatures[0].fig.pos;
    let eye = vec3(0.0, 140.0, 0.0);
    let spot = (0..32)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 32.0;
            let p = b + vec3(a.sin(), 0.0, a.cos()) * 5.0 * 256.0;
            vec3(p.x, r.level.ground.surface(&r.level.map, p.x, p.z, b.y + 64.0), p.z)
        })
        .find(|&p| (p.y - b.y).abs() < 128.0 && r.level.ground.line_of_sight(p + eye, b + eye))
        .expect("an open spot near him");
    let mut aimed = 0;
    for _ in 0..1200 {
        r.set_down(spot);
        // Knocked down, she drops the pistol (`drop_current_gun`): back in her hand, as
        // picking it up again would have it.
        if r.g.people[me].weapon != Weapon::Pistol {
            r.g.people[me].give(Weapon::Pistol);
            r.g.people[me].weapon = Weapon::Pistol;
        }
        r.play(0.25, Controls { punch: true, ..Default::default() });
        aimed += r.g.aiming.creature.is_some() as u32;
        r.play(0.25, Controls::default());
        if r.g.outcome.is_some() {
            break;
        }
    }
    println!("aimed {aimed} times; health {health} -> {}, state {:?}, outcome {:?}, aim {:?}, weapon {:?}, rounds {:?}, darci {:?}",
             r.g.creatures[0].health, r.g.creatures[0].state, r.g.outcome, r.g.aiming.creature,
             r.g.people[me].weapon, r.g.people[me].rounds(), r.g.people[me].state);
    assert!(r.g.creatures[0].dead, "he should be dead");
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}

/// Shots at him come through at an eighth: a pistol round is 70, halved, eighthed, plus one.
#[test]
fn the_balrog_is_hard() {
    let Some(mut r) = start("Baalrog3") else { return };
    r.wake_the_balrog();
    let mut acts = Vec::new();
    let b = &mut r.g.creatures[0];
    b.hit(Some((Vec3::ZERO, None)), 70, &mut acts);
    assert_eq!(b.health, 255 - (70 / 2 / 8 + 1));
}

/// Day Of Reckoning: the gargoyle sits until the player comes near, then takes off; Bane
/// draws up the bodies round him and can't be hurt.
#[test]
fn the_gargoyle_wakes_and_bane_summons() {
    let Some(mut r) = start("Finale1") else { return };
    r.play(1.0, Controls::default());
    let garg = r.g.creatures.iter().position(|c| c.kind == creature::GARGOYLE).expect("a gargoyle");
    let bane = r.g.creatures.iter().position(|c| c.kind == creature::BANE).expect("Bane");
    assert_eq!(r.g.creatures[garg].state, Cs::Ground);

    let at = r.g.creatures[garg].fig.pos;
    r.set_down(at + vec3(3.0 * 256.0, 0.0, 0.0));
    r.play(10.0, Controls::default());
    assert_ne!(r.g.creatures[garg].state, Cs::Ground, "the player is near: it should be up");
    assert!(r.g.creatures[garg].fig.pos.y > at.y + 32.0, "and in the air");

    let mut acts = Vec::new();
    r.g.creatures[bane].hit(Some((Vec3::ZERO, None)), 100_000, &mut acts);
    assert!(!r.g.creatures[bane].dead, "nothing hurts Bane");
    let floating = r.g.people.iter().filter(|p| matches!(p.state, State::Float { .. })).count();
    println!("bodies floating: {floating}, summoned {:?}", r.g.creatures[bane].summon);
    assert_eq!(floating, r.g.creatures[bane].summon.iter().flatten().count());
}
