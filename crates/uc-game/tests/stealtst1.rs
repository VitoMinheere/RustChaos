//! Seek, Sneak and Seize (`Stealtst1`): lift the disk and meet at the front gates. Skipped
//! without an install.

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
    let level = Level::load(&game, "Stealtst1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {
    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    fn play(&mut self, secs: f32) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            let me = self.g.player.unwrap();
            self.g.people[me].health = self.g.people[me].max_health;
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

    fn stand(&mut self, p: Vec3) {
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }

    fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
    }
}

/// The disk is lifted and taken to the gates.
#[test]
fn seek_sneak_and_seize_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(3.0);

    // The disk point 26 puts down is a file, which only became something Darci can carry when
    // evidence items did; `item_held` on point 27 is watching for it.
    let disk = r.g.items.iter().find(|it| it.way == 26).expect("the disk").clone();
    assert_eq!(disk.kind, 14, "the disk is a file");
    r.stand(disk.pos);
    r.play(2.0);
    assert!(r.said("You got the disk"), "it was never lifted: {:?}", r.seen);
    assert_eq!(r.g.outcome, None, "the disk alone doesn't finish it");

    // Then the front gates, which point 36 watches as a cuboid.
    r.stand(r.point(36));
    r.play(4.0);
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// Walking out to the gates without the disk proves the two halves are both needed.
#[test]
fn the_gates_alone_arent_enough() {
    let Some(mut r) = start() else { return };
    r.play(3.0);
    r.stand(r.point(36));
    r.play(5.0);
    assert_eq!(r.g.outcome, None, "the gates without the disk shouldn't win it");
}

/// Point 92 ends it if the man point 28 creates is killed.
#[test]
fn losing_the_contact_fails_it() {
    let Some(mut r) = start() else { return };
    r.play(3.0);
    let Some(i) = r.g.people.iter().position(|p| p.way == 28) else { return };
    let from = r.g.people[i].fig.pos;
    r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "that should lose it: {:?}", r.g.outcome);
}
