//! Day Of Reckoning (`Finale1`), played through: on top of Gunston Tower, with no floor below,
//! Roper carries the three wounded survivors to safety; then Darci switches on both fuel
//! pumps, blows up the fuel reservoir and gets back to where she started. The fighting and
//! the travelling are done by cheating. Skipped without an install.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_game::{Charge, Controls, Outcome, State};
use uc_world::character::person as ptype;

mod common;
use common::{Run, DT};

fn start() -> Option<Run> {
    let mut r = common::start("Finale1")?;
    // Bane's men are taken off and Darci and Roper kept whole: the fighting isn't
    // what's tested.
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut() {
            if p.hostile() {
                p.removed = true;
            }
            if matches!(p.kind, ptype::DARCI | ptype::ROPER) {
                p.health = p.max_health;
            }
        }
    });
    Some(r)
}

impl Run {
    /// Puts person `i` on the floor under `at` (on the roof, as `at` is), facing along `dir`.
    fn put(&mut self, i: usize, at: Vec3, dir: Vec2) {
        let y = self.level.ground.surface(&self.level.map, at.x, at.z, at.y + 64.0);
        self.g.people[i].fig.pos = vec3(at.x, y, at.z);
        self.g.people[i].fig.angle = uc_world::character::angle_of(dir);
    }

    fn press_action(&mut self) {
        self.until(60.0, |r| !r.g.in_conversation());
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    /// The player at point `way`, pressing action there (a `player_uses_radius` switch).
    fn use_at(&mut self, way: usize) {
        let (p, at) = (self.player(), self.point(way));
        self.put(p, at, vec2(0.0, 1.0));
        self.play(0.5, Controls::default());
        self.press_action();
        self.play(1.0, Controls::default());
    }
}

#[test]
fn day_of_reckoning_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    assert!(r.g.no_floor, "point 92 should have taken the floor away");

    // Roper carries each survivor to the safe zone by where he starts.
    let roper = r.player();
    assert_eq!(r.g.people[roper].kind, ptype::ROPER);
    for (way, zone) in [(31, 30), (32, 67), (33, 66)] {
        let s = r.who(way);
        let body = r.g.people[s].body_pos();
        r.put(roper, body + vec3(60.0, 0.0, 0.0), vec2(-1.0, 0.0));
        r.play(0.5, Controls::default());
        r.press_action();
        assert_eq!(r.g.people[roper].carrying(), Some(s), "didn't pick up survivor {way}: {:?}", r.g.people[roper].state);
        r.play(2.5, Controls::default());
        let z = r.point(zone);
        r.put(roper, z + vec3(0.0, 0.0, 200.0), vec2(0.0, -1.0));
        r.play(0.5, Controls::default());
        if r.g.player != Some(roper) {
            break;
        }
        // Facing somewhere with room to put them down (192 clear ahead).
        let me = r.g.people[roper].fig.pos + vec3(0.0, 64.0, 0.0);
        let dir = [vec2(0.0, -1.0), vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0)].into_iter()
            .find(|d| r.level.ground.line_of_sight(me, me + vec3(d.x, 0.0, d.y) * 192.0))
            .expect("no room anywhere");
        r.g.people[roper].fig.angle = uc_world::character::angle_of(dir);
        r.press_action();
        r.play(3.0, Controls::default());
        assert!(matches!(r.g.people[s].state, State::Injured), "survivor {way} not put down: {:?}", r.g.people[s].state);
    }
    assert!(r.until(30.0, |r| r.said("I got them all in safely")), "{:?}", r.seen);
    assert!(r.until(30.0, |r| r.g.people[r.player()].kind == ptype::DARCI), "never handed over to Darci");

    // Darci: both fuel pumps, the reservoir (a prop that has to be damaged, point 62), and
    // back to where she started (cuboid 78).
    r.use_at(50);
    assert!(r.said("Fuel Pump 2 ACTIVE!"), "{:?}", r.seen);
    r.use_at(48);
    assert!(r.said("Fuel Pump 1 ACTIVE!"), "{:?}", r.seen);
    let reservoir = r.point(62);
    r.g.charges.push(Charge { pos: reservoir, fuse: 0.1 });
    r.play(1.0, Controls::default());
    let (p, home) = (r.player(), r.point(78));
    r.put(p, home, vec2(0.0, 1.0));
    r.play(3.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
