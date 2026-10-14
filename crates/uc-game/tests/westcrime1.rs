//! Urban Shakedown (`westcrime1`): the Wildcats come in waves, each one brought on by wiping
//! out the one before, against the clock. Checks the script end to end, with the fighting done
//! by knocking them down, as the other mission tests do. Skipped without an install.

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
    let level = Level::load(&game, "westcrime1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {


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

/// The waves are the colours 7, 8, 9, 10 and 12; when the last is gone, the district is safe.
#[test]
fn urban_shakedown_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let mut downed = 0;
    for _ in 0..120 {
        if r.g.outcome.is_some() {
            break;
        }
        let wave: Vec<usize> = (0..r.g.people.len())
            .filter(|&i| {
                let p = &r.g.people[i];
                !p.player && !p.removed && !p.is_dead() && p.way != 0 && [7, 8, 9, 10, 12].contains(&p.colour)
            })
            .collect();
        for i in wave {
            r.look_at(i, 160.0);
            let from = r.g.people[i].fig.pos;
            r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
            downed += 1;
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s after {downed} down", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
