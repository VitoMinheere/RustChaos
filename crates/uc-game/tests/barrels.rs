//! Barrels (fallen/Source/barrel.cpp): a shot blows up anything but a cone or a bin, with a
//! shockwave that knocks people down, and the point that made it counts it as killed.
//! Semtex is won by shooting its two drums. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::{Charge, Controls, Game, Outcome, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

fn start(mission: &str) -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "{mission}: unsupported {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {
    fn run(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            let p = self.g.player.unwrap();
            self.g.people[p].health = self.g.people[p].max_health;
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
            self.run(DT, Controls { punch: true, ..Default::default() });
            self.run(0.4, Controls::default());
        }
        self.g.barrels[k].gone
    }
}

/// "One shot oughta set the whole thing off": the semtex is in two oil drums on the dock
/// roofs (points 183 and 199), and blowing both up wins it.
#[test]
fn semtex_is_won_by_shooting_the_two_drums() {
    let Some(mut r) = start("Semtex") else { return };
    r.run(1.0, Controls::default());
    for way in [199, 183] {
        let k = r.barrel(way);
        assert!(r.g.barrels[k].explodes, "drum {way} doesn't explode");
        r.line_up(k);
        r.run(0.5, Controls::default());
        assert_eq!(r.g.aim_barrel, Some(k), "the gun isn't on drum {way}");
        assert!(r.shoot(k), "shooting drum {way} didn't blow it up");
    }
    r.run(10.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("blowing that Semtex to hell")), "{:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// The blast knocks down whoever is close, and it's the shooter's doing.
#[test]
fn a_barrel_going_up_knocks_people_down() {
    let Some(mut r) = start("Semtex") else { return };
    r.run(1.0, Controls::default());
    let k = r.barrel(324);
    let at = r.g.barrels[k].pos;
    let victim = r.g.people.iter().position(|p| p.hostile() && !p.removed && !p.is_dead()).unwrap();
    r.g.people[victim].fig.pos = at + vec3(100.0, 0.0, 0.0);
    let p = r.g.player.unwrap();
    r.g.shoot_barrel(k, p, &r.level);
    r.run(0.5, Controls::default());
    assert!(r.g.barrels[k].gone);
    assert!(r.g.people[victim].is_down() || r.g.people[victim].is_dead(), "{:?}", r.g.people[victim].state);
}

/// Cones and bins only get knocked about.
#[test]
fn cones_and_bins_dont_explode() {
    let Some(mut r) = start("police1") else { return };
    r.run(1.0, Controls::default());
    let p = r.g.player.unwrap();
    for way in [75, 95] {
        let k = r.barrel(way);
        assert!(!r.g.barrels[k].explodes);
        r.g.shoot_barrel(k, p, &r.level);
        assert!(!r.g.barrels[k].gone, "point {way} blew up");
        assert!(r.g.barrels[k].vel.length() > 0.0, "point {way} wasn't knocked");
    }
}

/// Only a shot sets a barrel off: a charge going off beside it leaves it standing.
#[test]
fn a_charge_leaves_barrels_alone() {
    let Some(mut r) = start("Semtex") else { return };
    r.run(1.0, Controls::default());
    let k = r.barrel(324);
    let at: Vec3 = r.g.barrels[k].pos;
    r.g.charges.push(Charge { pos: at + vec3(60.0, 0.0, 0.0), fuse: 0.1 });
    r.run(2.0, Controls::default());
    assert!(!r.g.barrels[k].gone, "the charge set the barrel off");
}
