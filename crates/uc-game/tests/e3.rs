//! Media Trouble (`e3`): Darci frees the reporter, Ms. Gordansky, and walks her back to the
//! station. Checks the script end to end, with the walking done by putting people where they
//! have to be and the fighting by knocking them down, as the other mission tests do. Skipped
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
    let level = Level::load(&game, "e3", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
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

    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
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

/// Her captor (62) goes down, she follows Darci (point 85), and once she's at the station
/// (point 115) she says her thanks and the mission is won.
#[test]
fn media_trouble_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let captor = r.who(62);
    r.look_at(captor, 160.0);
    let from = r.g.people[captor].fig.pos;
    r.g.people[captor].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(3.0, Controls::default());
    let reporter = r.who(63);
    assert_eq!(r.g.people[reporter].mv, 5, "she should follow Darci now: {:?}", r.seen);

    // Back to the station, the two of them.
    let station = r.point(115);
    r.stand(station + vec3(200.0, 0.0, 0.0));
    let y = r.level.ground.surface(&r.level.map, station.x, station.z, station.y + 64.0);
    r.g.people[reporter].fig.pos = vec3(station.x, y, station.z);
    for _ in 0..90 {
        if r.g.outcome.is_some() {
            break;
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(!r.g.people[reporter].is_dead(), "the reporter mustn't come to harm");
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
