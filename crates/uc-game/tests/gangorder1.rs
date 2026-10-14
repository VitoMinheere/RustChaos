//! Assassin (`Gangorder1`): Mako takes out Corrola for the Boss. Checks the script end to end,
//! with the walking done by putting people where they have to be and the fighting by knocking
//! them down, as the other mission tests do. Skipped without an install.

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
    let level = Level::load(&game, "Gangorder1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
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

    fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
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

    /// Walks up to the person event point `way` created, talks to them, and lets them finish.
    fn talk_to(&mut self, way: usize, secs: f32) {
        let i = self.who(way);
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(130.0, 0.0, 0.0));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }
}

/// Radar gives the job, Corrola goes down, and Mako gets back to the hideout.
#[test]
fn assassin_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    // Radar, up on his roof, points Mako at Corrola.
    r.talk_to(177, 70.0);
    assert!(r.g.point_active(179), "Radar never gave the job: {:?}", r.seen);

    // Corrola and his bodyguards turn up; put him down for good.
    r.play(2.0, Controls::default());
    let corrola = r.who(113);
    r.look_at(corrola, 160.0);
    let from = r.g.people[corrola].fig.pos;
    r.g.people[corrola].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    assert!(r.said("SMOKED CORROLA"), "nobody noticed: {:?}", r.seen);
    assert_eq!(r.g.outcome, None);

    // Back to the hideout.
    r.stand(r.point(116));
    r.play(8.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
