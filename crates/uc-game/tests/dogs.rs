//! Dogs (#110): the original's canids (canid.cpp), never shipped, here an extra that's off
//! unless asked for. A dog walks to a near goal and runs to a far one, chases whoever comes
//! close or whom it sees ahead, and stops to bark at them. Skipped without an install.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_game::canid::{self, State as Ds};
use uc_game::{Controls, Sfx};

mod common;
use common::Run;

/// Psycho Park, with everyone but Darci taken off, so only she is about for the dogs.
fn start() -> Option<Run> {
    let mut r = Run::new(common::load("park2")?);
    r.stop_at_outcome = false;
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| !p.player) {
            p.removed = true;
        }
    });
    r.play(0.1, Controls::default());
    Some(r)
}

impl Run {
    fn me(&self) -> Vec3 {
        self.g.people[self.player()].fig.pos
    }

    /// A way from Darci with open, level ground for `squares`: (direction, the dog's facing
    /// angle so it looks back along it at her).
    fn open_way(&self, squares: f32) -> Vec2 {
        let me = self.me();
        let ground = &self.level.ground;
        (0..16)
            .map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                vec2(a.sin(), a.cos())
            })
            .find(|d| {
                let far = me + vec3(d.x, 0.0, d.y) * squares * 256.0;
                let level = (self.level.map.ground_height(far.x, far.z) - me.y).abs() < 16.0;
                level && ground.line_of_sight(me + Vec3::Y * 40.0, far + Vec3::Y * 40.0)
            })
            .expect("somewhere open round Darci")
    }

    /// A dog `dist` from Darci along `d`, looking towards her (`toward`) or away.
    fn dog_at(&mut self, d: Vec2, dist: f32, toward: bool) -> usize {
        let at = self.me() + vec3(d.x, 0.0, d.y) * dist;
        // Facing (-sin, -cos) of its angle: back along `d` is towards her.
        let f = if toward { d } else { -d };
        let angle = f.x.atan2(f.y);
        assert!(self.g.spawn_dog(&self.level, at, angle), "no DOG2.all");
        self.g.dogs.len() - 1
    }

    fn barked(&mut self) -> bool {
        self.g.take_sounds().iter().any(|(s, _)| matches!(s, Sfx::ById { id, .. } if (canid::sound::BARK..=canid::sound::BARK + 1).contains(id)))
    }
}

#[test]
fn no_dogs_unless_asked() {
    let Some(r) = start() else { return };
    assert!(r.g.dogs.is_empty());
}

/// `--dogs N` finds room for them, on the ground.
#[test]
fn dogs_let_loose_stand_on_the_ground() {
    let Some(mut r) = start() else { return };
    let n = r.g.let_dogs_out(&r.level, 3);
    assert_eq!(n, 3);
    for d in &r.g.dogs {
        let ground = r.level.map.ground_height(d.fig.pos.x, d.fig.pos.z);
        assert!((d.fig.pos.y - ground).abs() < 1.0, "a dog at {:?}, ground {ground}", d.fig.pos);
    }
}

/// `CANID_Homing`: it runs (anim 2, 16 a tick) while its goal is further than the square
/// root of 48000, and walks (anim 1, 4 a tick) once nearer; the frames go round either way.
#[test]
fn walks_near_and_runs_far() {
    let Some(mut r) = start() else { return };
    let d = r.open_way(8.0);
    // Darci well out of the way, behind a dog that faces away from her.
    let k = r.dog_at(d, 7.0 * 256.0, false);
    r.stand(r.me() - vec3(d.x, 0.0, d.y) * 30.0 * 256.0);
    let at = r.g.dogs[k].fig.pos;
    // To a goal six squares on, past Darci's old spot.
    let dest = at - vec3(d.x, 0.0, d.y) * 6.0 * 256.0;
    let square = ((dest.x as i32) >> 8, (dest.z as i32) >> 8);
    r.g.dogs[k].prowl_to(square);
    let goal = vec2(((dest.x as i32 >> 8) << 8) as f32 + 128.0, ((dest.z as i32 >> 8) << 8) as f32 + 128.0);
    let (mut ran, mut walked) = (false, false);
    let mut frames = std::collections::BTreeSet::new();
    for _ in 0..300 {
        let before = r.g.dogs[k].fig.pos;
        let far = (vec2(before.x, before.z) - goal).length_squared() > 48000.0;
        r.play(1.0 / 20.0, Controls::default());
        let dog = &r.g.dogs[k];
        if dog.state != Ds::Prowl || dog.dest != square {
            // There: it's on to something else.
            break;
        }
        let step = (vec2(dog.fig.pos.x, dog.fig.pos.z) - vec2(before.x, before.z)).length();
        if step == 0.0 {
            // A frame without one of the dogs' ticks (20 a second).
            continue;
        }
        if far {
            assert_eq!(dog.anim(), canid::RUN, "far off it runs");
            ran |= step > 8.0;
        } else {
            assert_eq!(dog.anim(), canid::WALK, "near it walks");
            walked |= step > 0.0 && step < 8.0;
        }
        frames.insert(dog.fig.frame());
    }
    assert!(ran && walked, "ran {ran}, walked {walked}");
    assert!(frames.len() > 3, "the legs move: {frames:?}");
}

/// `CANID_6sense`: anyone within 0xd0 is chased, whichever way the dog looks; then within
/// 64 it barks (`CANID_process_chase`).
#[test]
fn chases_whoever_comes_close_and_barks() {
    let Some(mut r) = start() else { return };
    let d = r.open_way(3.0);
    let k = r.dog_at(d, 150.0, false);
    r.g.dogs[k].nap(1000);
    r.play(0.1, Controls::default());
    let me = r.player();
    assert_eq!(r.g.dogs[k].state, Ds::Chase, "asleep, it still smells her");
    assert_eq!(r.g.dogs[k].target, Some(me));
    r.g.take_sounds();
    let mut barked = false;
    for _ in 0..60 {
        r.play(0.1, Controls::default());
        barked |= r.barked();
        if r.g.dogs[k].state == Ds::Bark && barked {
            break;
        }
    }
    assert_eq!(r.g.dogs[k].state, Ds::Bark);
    assert!(barked, "a bark to hear");
    let gap = r.g.dogs[k].fig.pos - r.me();
    assert!(gap.x * gap.x + gap.z * gap.z < 14000.0, "it stops close");
}

/// `CANID_LOS`: further off, it chases only someone in front of it that it can see, and
/// only while about (prowling or chasing), not asleep.
#[test]
fn chases_further_off_only_what_it_sees() {
    let Some(mut r) = start() else { return };
    let d = r.open_way(4.0);
    // Three squares off: past smelling, within seeing.
    let k = r.dog_at(d, 3.0 * 256.0, false);
    let home = r.g.dogs[k].home;
    r.g.dogs[k].prowl_to(home);
    r.play(0.1, Controls::default());
    assert_ne!(r.g.dogs[k].state, Ds::Chase, "looking away, it doesn't see her");

    r.g.dogs.clear();
    let k = r.dog_at(d, 3.0 * 256.0, true);
    r.g.dogs[k].nap(1000);
    r.play(0.1, Controls::default());
    assert_eq!(r.g.dogs[k].state, Ds::Sleep, "asleep, it doesn't look");

    let home = r.g.dogs[k].home;
    r.g.dogs[k].prowl_to(home);
    r.play(0.1, Controls::default());
    assert_eq!(r.g.dogs[k].state, Ds::Chase, "awake and facing her, it sees her");

    // Off she goes: it barks while she's near, and chases again once she's away.
    assert!(r.until(5.0, |r| r.g.dogs[k].state == Ds::Bark), "it catches up");
    let away = r.me() - vec3(d.x, 0.0, d.y) * 1.5 * 256.0;
    r.stand(away);
    r.play(0.1, Controls::default());
    assert_eq!(r.g.dogs[k].state, Ds::Chase, "she's off: after her");
}
