//! Transmission Terminated (`factory1`): Darci blows up the Wildcats' transmitter on the roof
//! and clears the factory of them without losing a hostage. Checks the script end to end, with
//! the walking done by putting Darci where she has to be and the fighting by knocking people
//! down, as the other mission tests do. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
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
    let mut r = Run { level, g, seen: Vec::new() };
    r.play(1.0, Controls::default());
    Some(r)
}

impl Run {
    fn play(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            let me = self.g.player.unwrap();
            self.g.people[me].health = self.g.people[me].max_health;
            self.g.update(DT, &c, &self.level);
            if self.g.outcome.is_some() {
                return;
            }
            for m in &self.g.messages {
                if !self.seen.contains(&m.text) {
                    println!("[{:6.1}s] {}", self.g.time, m.text);
                    self.seen.push(m.text.clone());
                }
            }
        }
    }

    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    fn stand(&mut self, p: Vec3) {
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }


    /// Picks up the explosives point `way` puts down.
    fn fetch(&mut self, way: usize) {
        let at = self.g.items.iter().find(|it| it.way == way).expect("explosives lying about").pos;
        self.stand(at);
        self.play(0.5, Controls::default());
    }

    /// Takes the explosives out and sets one down where she stands.
    fn set_charge(&mut self) {
        let i = self.g.player.unwrap();
        while self.g.people[i].weapon != Weapon::Explosives {
            self.play(DT, Controls { switch: true, ..Default::default() });
        }
        self.play(DT, Controls { action: true, ..Default::default() });
    }
}

impl Run {
    /// Sets a charge beside what point `way` watches and waits, well clear, for it to go up.
    fn blow_up(&mut self, way: usize) {
        self.stand(self.point(way) + vec3(150.0, 0.0, 0.0));
        self.set_charge();
        self.stand(self.point(way) + vec3(0.0, 0.0, 2500.0));
        self.play(Charge::FUSE + 1.5, Controls::default());
    }
}

/// The transmitter (point 288 watches it) goes up and colour 2 is wiped out; the hostages
/// (colour 3) must all come through.
#[test]
fn transmission_terminated_script_runs_to_a_win() {
    let Some(mut r) = start("factory1") else { return };
    assert!(r.g.unsupported().is_empty(), "unsupported: {:?}", r.g.unsupported());

    r.fetch(238);
    r.blow_up(288);
    assert!(r.g.point_active(288), "the transmitter didn't go up: {:?}", r.seen);

    for _ in 0..60 {
        if r.g.outcome.is_some() {
            break;
        }
        let gang: Vec<usize> = (0..r.g.people.len())
            .filter(|&i| {
                let p = &r.g.people[i];
                !p.player && !p.removed && !p.is_dead() && p.way != 0 && p.colour == 2
            })
            .collect();
        for i in gang {
            let from = r.g.people[i].fig.pos;
            r.stand(from + vec3(160.0, 0.0, 0.0));
            r.play(DT, Controls::default());
            let from = r.g.people[i].fig.pos;
            r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
