//! Trouble in the Park (`police4`): Darci clears the Wildcats out of the park and meets Roper.
//! Checks the script end to end, with the walking done by putting people where they have to
//! be and the fighting by knocking them down, as the other mission tests do. Skipped without
//! an install.

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
    let level = Level::load(&game, "police4", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {

    fn who(&self, way: usize) -> usize {
        self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}"))
    }

    fn play(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            let me = self.g.player.unwrap();
            self.g.people[me].health = self.g.people[me].max_health;
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

    fn stand(&mut self, p: Vec3) {
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }


    /// Stands Mako `dist` away from person `i` and has him look at them: `can_a_see_b` wants
    /// the looker facing roughly the right way, not just standing near.
    fn look_at(&mut self, i: usize, dist: f32) {
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(dist, 0.0, 0.0));
        let me = self.g.player.unwrap();
        let from = self.g.people[me].fig.pos;
        self.g.people[me].fig.angle = uc_world::character::angle_of(glam::vec2(at.x - from.x, at.z - from.z));
    }

}

impl Run {
    /// Puts down, for good, whoever the points in `ways` created and is still standing.
    fn put_down(&mut self, ways: &[usize]) -> usize {
        let mut n = 0;
        for &way in ways {
            if let Some(i) = self.g.people.iter().position(|p| p.way == way && !p.is_dead() && !p.removed) {
                self.look_at(i, 160.0);
                let from = self.g.people[i].fig.pos;
                self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
                n += 1;
                self.play(1.0, Controls::default());
            }
        }
        n
    }
}

/// The disturbance is put down, the second gang that turns up after it too, and Roper has his
/// word with Darci.
#[test]
fn trouble_in_the_park_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    // The three Wildcats making the disturbance.
    assert_eq!(r.put_down(&[54, 137, 138]), 3, "the disturbance wasn't there");
    r.play(8.0, Controls::default());
    assert!(r.g.people.iter().any(|p| p.way == 158), "Roper never showed: {:?}", r.seen);

    // The second lot.
    assert_eq!(r.put_down(&[143, 145, 147, 149]), 4, "the second gang never turned up");

    // Darci sees Roper, and he has his say.
    let roper = r.who(158);
    for _ in 0..90 {
        if r.g.outcome.is_some() {
            break;
        }
        if !r.g.in_conversation() {
            r.look_at(roper, 300.0);
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(!r.g.people[roper].is_dead(), "Roper mustn't come to harm");
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
