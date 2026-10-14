//! Gatecrasher (`bball2`): Darci goes to the Wildcat leaders' meet at the ballpark and brings
//! one of them in. Checks the script end to end, with the walking done by putting people where
//! they have to be and the fighting by knocking them down, as the other mission tests do.
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
    let level = Level::load(&game, "bball2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
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

    fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
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

/// One of the three leaders at the meet is knocked down and arrested: that is counter 2, and
/// the win follows. (Killing all three is the way to lose.)
#[test]
fn gatecrasher_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    let leader = r.who(73);
    r.look_at(leader, 160.0);
    r.play(1.0, Controls::default());
    let at = r.g.people[leader].fig.pos;
    r.g.people[leader].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, uc_game::Weapon::None);
    // Let her get over any blow she took first: a flinch eats the button.
    for _ in 0..60 {
        let me = r.g.player.unwrap();
        if matches!(r.g.people[me].state, uc_game::State::Move) {
            break;
        }
        r.play(DT * 1.01, Controls::default());
    }
    let body = r.g.people[leader].body_pos();
    let beside = (0..8)
        .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
        .find(|p| (r.level.ground.surface(&r.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
        .expect("somewhere to stand");
    r.stand(beside);
    r.play(0.2, Controls { action: true, ..Default::default() });
    r.play(4.0, Controls::default());
    let l = &r.g.people[leader];
    assert!(l.arrested || matches!(l.state, uc_game::State::Cuffed), "the leader wasn't arrested: {:?}", l.state);

    r.play(10.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("I GOT HIM"), "the arrest didn't count: {:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
