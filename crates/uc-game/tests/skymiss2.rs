//! Stern Revenge (`skymiss2`), played through: the two groups of hostage-takers, the Men In
//! Black who turn up after, and the three Wildcat vans come to take the hostages away, shot
//! until they blow up. The fighting and the travelling are done by cheating, but the vans are
//! shot with the player's own gun. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Whose deaths the script counts: the two groups holding hostages, then the Men In Black.
const TARGETS: [usize; 8] = [217, 218, 219, 186, 187, 188, 249, 250];
/// The Wildcat vans.
const VANS: [usize; 3] = [52, 275, 284];

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

fn start() -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "skymiss2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {
    fn run(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            // Everyone else hostile is taken off and the player kept whole: the fighting isn't
            // what's tested.
            for p in self.g.people.iter_mut() {
                if p.hostile() && !TARGETS.contains(&p.way) {
                    p.removed = true;
                }
                if p.player {
                    p.health = p.max_health;
                }
            }
            self.g.update(DT, &c, &self.level);
            for m in &self.g.messages {
                if !self.seen.contains(&m.text) {
                    println!("[{:6.1}s] {}", self.g.time, m.text);
                    self.seen.push(m.text.clone());
                }
            }
            if self.g.outcome.is_some() {
                return;
            }
        }
    }

    fn until(&mut self, secs: f32, done: impl Fn(&Run) -> bool) -> bool {
        for _ in 0..(secs / 0.25) as usize {
            if done(self) {
                return true;
            }
            self.run(0.25, Controls::default());
        }
        done(self)
    }

    fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
    }

    fn kill(&mut self, way: usize) {
        let i = self.g.people.iter().position(|p| p.way == way && !p.removed).unwrap_or_else(|| panic!("nobody from point {way}"));
        let from = self.g.people[i].fig.pos + vec3(50.0, 0.0, 0.0);
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }

    fn van(&self, way: usize) -> usize {
        self.g.vehicles.iter().position(|v| v.way == way).unwrap_or_else(|| panic!("no van from point {way}"))
    }

    /// Stands the player, shotgun out, a little way from van `k` with a clear shot at it.
    fn line_up(&mut self, k: usize) {
        let at = self.g.vehicles[k].pos;
        let eye = vec3(0.0, 140.0, 0.0);
        let spot: Vec3 = (0..32)
            .flat_map(|i| [400.0, 550.0].map(|d| at + vec3((i as f32 * 0.196).sin(), 0.0, (i as f32 * 0.196).cos()) * d))
            .find(|p| {
                (self.level.ground.surface(&self.level.map, p.x, p.z, at.y + 64.0) - at.y).abs() < 16.0
                    && self.level.ground.line_of_sight(*p + eye, at + vec3(0.0, 64.0, 0.0))
            })
            .expect("nowhere to shoot it from");
        let p = self.g.player.unwrap();
        let me = &mut self.g.people[p];
        me.fig.pos = spot;
        me.fig.angle = uc_world::character::angle_of(vec2(at.x - spot.x, at.z - spot.z));
        me.weapon = Weapon::Shotgun;
        // And a fresh box of shells for each van.
        me.give(Weapon::Shotgun);
    }
}

/// The original hands the player a shotgun in this mission alone (pcom.cpp:5601).
#[test]
fn stern_revenge_starts_with_a_shotgun() {
    let Some(r) = start() else { return };
    let p = &r.g.people[r.g.player.unwrap()];
    assert!(p.has(Weapon::Shotgun), "no shotgun: {:?}", p.weapon);
}

#[test]
fn stern_revenge_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.run(1.0, Controls::default());

    // The two groups holding hostages, and the Men In Black who come after them.
    for way in [217, 218, 219, 186, 187, 188] {
        r.kill(way);
    }
    assert!(r.until(30.0, |r| r.said("That's one bunch safe!")), "{:?}", r.seen);
    assert!(r.until(30.0, |r| r.said("that's the hostages safe")), "{:?}", r.seen);
    assert!(r.until(30.0, |r| r.g.people.iter().any(|p| p.way == 249) && r.g.people.iter().any(|p| p.way == 250)), "no Men In Black");
    for way in [249, 250] {
        r.kill(way);
    }
    assert!(r.until(30.0, |r| r.said("Hostages are all okay")), "{:?}", r.seen);

    // The vans: shot until each goes up.
    assert!(r.until(60.0, |r| VANS.iter().all(|&w| r.g.vehicles.iter().any(|v| v.way == w))), "the vans never came");
    for way in VANS {
        let k = r.van(way);
        r.line_up(k);
        r.run(0.5, Controls::default());
        assert_eq!(r.g.aim_vehicle, Some(k), "the gun isn't on van {way}");
        for _ in 0..20 {
            if r.g.vehicles[k].wrecked() {
                break;
            }
            r.run(DT, Controls { punch: true, ..Default::default() });
            r.run(0.6, Controls::default());
        }
        assert!(r.g.vehicles[k].wrecked(), "van {way} still has {} health", r.g.vehicles[k].health);
    }
    assert!(r.until(10.0, |r| r.said("That's blown THEM all to Hell")), "{:?}", r.seen);
    r.run(3.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
