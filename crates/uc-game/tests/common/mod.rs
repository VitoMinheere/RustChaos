//! What the mission tests share: loading a mission, playing it a turn at a time with the
//! cutscenes skipped and the player kept whole, the messages seen on the way, and putting
//! people where a test wants them. Each test file has `mod common;` and adds its own
//! helpers in an `impl Run` of its own.

// Not every test file uses every helper.
#![allow(dead_code)]

use glam::{vec2, vec3, Vec3};
use uc_game::{Controls, Game, State, Weapon};
use uc_world::{Level, LoadOptions};

/// The test's frame: thirty a second.
pub const DT: f32 = 1.0 / 30.0;

/// A mission being played.
pub struct Run {
    pub level: Level,
    pub g: Game,
    /// Every message shown so far, in order.
    pub seen: Vec<String>,
    /// Top the player's health up every turn: how well the player fights isn't what most
    /// tests check. On by default.
    pub heal: bool,
    /// Stop playing once the mission's won or lost. On by default.
    pub stop_at_outcome: bool,
    /// Done to the game every turn before it's updated, for example taking off the people
    /// who'd get in the way of what's tested.
    pub each_turn: Option<fn(&mut Game)>,
}

/// Loads `mission` from the install, with the characters, or None without an install.
pub fn load(mission: &str) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap())
}

/// Starts `mission`, which must use nothing the game doesn't support yet. None without an
/// install.
pub fn start(mission: &str) -> Option<Run> {
    let r = Run::new(load(mission)?);
    assert!(r.g.unsupported().is_empty(), "{mission}: unsupported {:?}", r.g.unsupported());
    Some(r)
}

impl Run {
    /// Starts the game on `level`.
    pub fn new(level: Level) -> Run {
        let g = Game::new(&level, true);
        Run { level, g, seen: Vec::new(), heal: true, stop_at_outcome: true, each_turn: None }
    }

    /// Notes (and prints) any message not seen before.
    pub fn log(&mut self) {
        for m in &self.g.messages {
            if !self.seen.contains(&m.text) {
                println!("[{:6.1}s] {}", self.g.time, m.text);
                self.seen.push(m.text.clone());
            }
        }
    }

    /// Plays `secs` (at least a frame) with the controls held, skipping cutscenes.
    pub fn play(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            if let Some(f) = self.each_turn {
                f(&mut self.g);
            }
            if self.heal {
                let p = self.player();
                self.g.people[p].health = self.g.people[p].max_health;
            }
            self.g.update(DT, &c, &self.level);
            self.log();
            if self.stop_at_outcome && self.g.outcome.is_some() {
                return;
            }
        }
    }

    /// Plays for up to `secs` until `done`, checking every quarter second. Whether it was.
    pub fn until(&mut self, secs: f32, done: impl Fn(&Run) -> bool) -> bool {
        for _ in 0..(secs / 0.25) as usize {
            if done(self) {
                return true;
            }
            self.play(0.25, Controls::default());
        }
        done(self)
    }

    /// Whether a message containing `text` has been shown.
    pub fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
    }

    /// Where event point `i` is.
    pub fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    /// If point `way` is set off by a switch or a valve, stands the player where it's
    /// worked from, facing it, and says so.
    pub fn stand_at_switch(&mut self, way: usize) -> bool {
        let Some(ob) = self.g.point_switch(way) else { return false };
        let (at, a) = uc_game::switch_spot(&self.level, ob);
        self.stand(at);
        let p = self.player();
        self.g.people[p].fig.angle = a;
        true
    }

    /// The person made by event point `way`.
    pub fn who(&self, way: usize) -> usize {
        self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}"))
    }

    pub fn player(&self) -> usize {
        self.g.player.unwrap()
    }

    /// Switches the player's weapons round to `w` and waits while she gets it out.
    pub fn take_out(&mut self, w: Weapon) {
        let i = self.player();
        while self.g.people[i].weapon != w {
            self.play(DT, Controls { switch: true, ..Default::default() });
        }
        while matches!(self.g.people[i].state, State::Draw) {
            self.play(DT, Controls::default());
        }
    }

    /// Stands the player on the ground at `p`.
    pub fn stand(&mut self, p: Vec3) {
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        let i = self.player();
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }

    /// Stands the player `dist` away from person `i`, looking at them: `can_a_see_b` wants
    /// the looker facing roughly the right way, not just standing near.
    pub fn look_at(&mut self, i: usize, dist: f32) {
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(dist, 0.0, 0.0));
        let me = self.player();
        let from = self.g.people[me].fig.pos;
        self.g.people[me].fig.angle = uc_world::character::angle_of(vec2(at.x - from.x, at.z - from.z));
    }
}
