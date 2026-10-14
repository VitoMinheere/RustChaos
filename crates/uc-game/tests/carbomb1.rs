//! Auto-Destruct (`carbomb1`): two cars with bombs in them go to the bomb squad's garages, the
//! bombers are arrested, and the UCNN video they hid at their den is found and brought back.
//! Checks the script end to end, with the walking and driving done by putting people and cars
//! where they have to be and the fighting by knocking them down, as the other mission tests
//! do. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Search, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

fn start() -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "carbomb1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
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

    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    fn ground(&self, p: Vec3) -> Vec3 {
        vec3(p.x, self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0), p.z)
    }

    fn stand(&mut self, p: Vec3) {
        let at = self.ground(p);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = at;
    }

    /// Delivers the car point `car` made to point `to`.
    fn deliver(&mut self, car: usize, to: usize) {
        let at = self.ground(self.point(to));
        let v = self.g.vehicles.iter_mut().find(|v| v.way == car).unwrap_or_else(|| panic!("no car from point {car}"));
        v.pos = at;
        v.speed = 0.0;
        self.stand(at + vec3(400.0, 0.0, 0.0));
        self.play(3.0, Controls::default());
    }

    /// Knocks down the person point `way` made and cuffs them.
    fn arrest(&mut self, way: usize) {
        let i = self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}: {:?}", self.seen));
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(160.0, 0.0, 0.0));
        // A frame for her arrival to register: a bodyguard far from her is out of harm's way.
        self.play(DT, Controls::default());
        let at = self.g.people[i].fig.pos;
        // From behind: he fights, and a guard up front would stop it.
        let f = self.g.people[i].fig.facing();
        self.g.people[i].take_hit(at - vec3(f.x, 0.0, f.y) * 50.0, 0, ko::KNOCKDOWN, false, false, Weapon::None);
        // Let her get over any blow she took first: a flinch eats the button.
        for _ in 0..60 {
            let me = self.g.player.unwrap();
            if matches!(self.g.people[me].state, State::Move) {
                break;
            }
            self.play(DT * 1.01, Controls::default());
        }
        let body = self.g.people[i].body_pos();
        let beside = (0..8)
            .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
            .find(|p| (self.level.ground.surface(&self.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
            .expect("somewhere to stand");
        self.stand(beside);
        self.play(0.2, Controls { action: true, ..Default::default() });
        self.play(4.0, Controls::default());
        let p = &self.g.people[i];
        assert!(p.arrested || matches!(p.state, State::Cuffed), "{way} wasn't arrested: {:?}", p.state);
    }
}

/// Both bomb cars delivered, both bombers cuffed, the video found and brought back.
#[test]
fn auto_destruct_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());

    // The first car to the garage (point 8 watches it), which brings out the second.
    r.deliver(6, 8);
    assert!(r.g.point_active(8), "the first car didn't count: {:?}", r.seen);
    r.play(2.0, Controls::default());
    r.deliver(82, 92);
    assert!(r.g.point_active(92), "the second car didn't count: {:?}", r.seen);

    // The bombers show themselves.
    r.play(3.0, Controls::default());
    r.arrest(147);
    r.arrest(148);
    assert!(r.seen.iter().any(|m| m.contains("You got the bombers")), "{:?}", r.seen);

    // The video is stashed at their den; search the prop it's in and pick it up.
    r.play(6.0, Controls::default());
    let ob = r.g.hidden.iter().find(|(_, it)| it.way == 181).expect("the video, stashed").0;
    let o = r.level.map.objects[ob];
    let prop = vec3(o.x as f32, o.y as f32, o.z as f32);
    r.stand(prop + vec3(110.0, 0.0, 0.0));
    let me = r.g.player.unwrap();
    r.g.people[me].face_towards(prop);
    r.play(DT, Controls { action: true, sprint: true, ..Default::default() });
    r.play(Search::FOUND + 0.2, Controls { sprint: true, ..Default::default() });
    let video = r.g.items.iter().find(|it| it.way == 181).expect("the search turned up nothing").pos;
    r.play(Search::DONE, Controls::default());
    r.stand(video);
    r.play(1.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("UCNN video")), "the video was never lifted: {:?}", r.seen);

    // Back to base with it.
    r.stand(r.point(169));
    for _ in 0..90 {
        if r.g.outcome.is_some() {
            break;
        }
        r.play(1.0, Controls::default());
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won), "{:?}", r.seen);
}
