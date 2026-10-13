//! Headline Hostage (`Gordout1`): Gordansky has to be brought in, not shot and not arrested.
//! Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

fn start() -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "Gordout1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {
    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    fn who(&self, way: usize) -> usize {
        self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}"))
    }

    fn play(&mut self, secs: f32) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            self.g.update(DT, &Controls::default(), &self.level);
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

    fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
    }

    /// Puts the person `way` created on the ground at `p`.
    fn put(&mut self, way: usize, p: Vec3) {
        let i = self.who(way);
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }
}

/// Gordansky delivered: the drop is a radius round point 15, watching where he is.
#[test]
fn headline_hostage_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(3.0);
    assert_eq!(r.g.outcome, None, "nothing should have been decided yet");

    r.put(3, r.point(15));
    r.play(5.0);
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("You got Gordansky"), "the drop never registered: {:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// He's wanted alive and at liberty: killing him or arresting him both fail it.
#[test]
fn gordansky_must_be_brought_in_whole() {
    let Some(mut r) = start() else { return };
    r.play(3.0);
    let g = r.who(3);
    let from = r.g.people[g].fig.pos;
    r.g.people[g].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0);
    assert!(r.said("Gordansky is dead"), "nobody noticed: {:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "killing him should lose: {:?}", r.g.outcome);
}
