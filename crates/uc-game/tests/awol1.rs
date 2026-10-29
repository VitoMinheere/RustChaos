//! Cop Killers (`AWOL1`), played through: Darci frees Officer Johnson and gets him to the
//! safe zone; then Roper opens the gate (which electrifies the fences) and carries Officer
//! Beck there. The fighting and the travelling are done by cheating. Skipped without an install.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Outcome, State, Weapon};
use uc_world::character::person as ptype;

mod common;
use common::{Run, DT};

/// Johnson's four guards, whose deaths the rescue waits on.
const GUARDS: [usize; 4] = [89, 90, 12, 88];

fn start() -> Option<Run> {
    let mut r = common::start("AWOL1")?;
    // The Wildcats are taken off (but for the guards the script counts), and Darci and
    // Roper kept whole: the fighting isn't what's tested.
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut() {
            if p.hostile() && !GUARDS.contains(&p.way) {
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
    fn alive(&self, way: usize) -> Option<usize> {
        self.g.people.iter().position(|p| p.way == way && !p.removed)
    }

    fn playing_as(&self, kind: u8) -> bool {
        self.g.people[self.player()].kind == kind
    }

    /// Puts person `i` on the ground at `at`, facing along `dir`, and lets them settle.
    fn put(&mut self, i: usize, at: Vec3, dir: Vec2) {
        let y = self.level.ground.surface(&self.level.map, at.x, at.z, at.y + 64.0);
        self.g.people[i].fig.pos = vec3(at.x, y, at.z);
        self.g.people[i].fig.angle = uc_world::character::angle_of(dir);
    }

    /// Presses action once no filmed conversation is holding the player still.
    fn press_action(&mut self) {
        self.until(60.0, |r| !r.g.in_conversation());
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    fn kill(&mut self, way: usize) {
        let i = self.alive(way).unwrap_or_else(|| panic!("nobody from point {way}"));
        let from = self.g.people[i].fig.pos + vec3(50.0, 0.0, 0.0);
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }

    /// The gate (point 43): "Gate Open", and the fences by Beck come on.
    fn open_the_gate(&mut self) {
        let p = self.player();
        let at = self.point(43);
        self.put(p, at, vec2(0.0, 1.0));
        self.stand_at_switch(43);
        self.play(0.5, Controls::default());
        self.press_action();
        // Pressing the switch takes a couple of seconds.
        self.play(2.5, Controls::default());
        assert!(self.said("Gate Open"), "{:?}", self.seen);
    }
}

/// From the start to "You saved them both!".
#[test]
fn cop_killers_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());

    // Darci deals with Johnson's guards and reaches him (within 864 of point 215): they talk,
    // and he's let go (point 28), which starts the fly-by that hands over to Roper.
    for way in GUARDS {
        r.kill(way);
    }
    let darci = r.player();
    let at = r.point(215);
    r.put(darci, at, vec2(0.0, -1.0));
    assert!(r.until(60.0, |r| r.said("Johnson, you okay?")), "no talk with Johnson: {:?}", r.seen);
    r.until(60.0, |r| !r.g.in_conversation());
    // Johnson to the safe zone (point 93).
    let johnson = r.alive(5).unwrap();
    let safe = r.point(93);
    r.put(johnson, safe, vec2(0.0, 1.0));
    assert!(r.until(30.0, |r| r.said("Johnson, you're safe!")), "{:?}", r.seen);
    assert!(r.until(60.0, |r| r.playing_as(ptype::ROPER)), "never handed over to Roper: {:?}", r.seen);

    // Roper opens the gate, and brings Beck (lying injured, point 3) to the safe zone (point 40).
    r.open_the_gate();
    assert!(!r.g.electrified.is_empty(), "the gate didn't electrify the fences");
    let roper = r.player();
    let beck = r.alive(3).unwrap();
    assert!(matches!(r.g.people[beck].state, State::Injured), "{:?}", r.g.people[beck].state);
    let body = r.g.people[beck].body_pos();
    r.put(roper, body + vec3(60.0, 0.0, 0.0), vec2(-1.0, 0.0));
    r.play(0.5, Controls::default());
    r.press_action();
    assert_eq!(r.g.people[roper].carrying(), Some(beck), "Roper didn't pick Beck up: {:?}", r.g.people[roper].state);
    r.play(2.5, Controls::default());
    let safe = r.point(40);
    r.put(roper, safe + vec3(0.0, 0.0, 200.0), vec2(0.0, -1.0));
    assert!(r.until(10.0, |r| r.said("You got Beck through")), "{:?}", r.seen);

    // Both safe: won four seconds after "You saved them both!".
    assert!(r.until(10.0, |r| r.said("You saved them both")), "{:?}", r.seen);
    r.play(6.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// Climbing an electrified fence: thrown off it with sparks, knocked down.
#[test]
fn an_electrified_fence_throws_off_whoever_climbs_it() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    r.open_the_gate();
    let ground = &r.level.ground;
    let (k, w) = ground.fences.iter().enumerate()
        .find(|(k, w)| r.g.electrified.contains(&ground.fence_buildings[*k]) && w.y1 - w.y0 > 150.0)
        .expect("no electrified fence");
    let _ = k;
    let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
    let mid = (a + b) / 2.0;
    let across = (b - a).perp().normalize();
    let p = r.player();
    let start = mid - across * 80.0;
    r.put(p, vec3(start.x, w.y0, start.y), across);
    r.play(0.5, Controls::default());
    let sparks = r.g.sparks.len();
    let mut shocked = false;
    for _ in 0..90 {
        r.play(DT, Controls { wish: across, ..Default::default() });
        if r.g.people[p].is_down() {
            shocked = true;
            break;
        }
    }
    assert!(shocked, "climbed it unhurt: {:?}", r.g.people[p].state);
    assert!(r.g.sparks.len() > sparks, "no sparks");
}
