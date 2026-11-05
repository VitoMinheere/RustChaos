//! What the mission tests share: loading a mission, playing it a turn at a time with the
//! cutscenes skipped and the player kept whole, the messages seen on the way, and putting
//! people where a test wants them. Each test file has `mod common;` and adds its own
//! helpers in an `impl Run` of its own.

// Not every test file uses every helper.
#![allow(dead_code)]

use glam::{vec2, vec3, Vec2, Vec3};
use uc_game::person::ko;
use uc_game::{Charge, Controls, Game, State, Weapon};
use uc_world::{Level, LoadOptions};

/// The test's frame: thirty a second.
pub const DT: f32 = 1.0 / 30.0;

/// A map square's middle.
pub fn mid(x: f32, z: f32) -> Vec2 {
    vec2(x, z) * 256.0
}

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

/// Loads `mission` from the install with `opts`, or None without an install.
pub fn load_with(mission: &str, opts: &LoadOptions) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, opts).unwrap())
}

/// Loads `mission` from the install, fullbright, with the characters, or None without an
/// install.
pub fn load(mission: &str) -> Option<Level> {
    load_with(mission, &LoadOptions { fullbright: true, character: true, overrides: None })
}

/// Loads `mission` as `load` does but with its baked lighting kept.
pub fn load_lit(mission: &str) -> Option<Level> {
    load_with(mission, &LoadOptions { fullbright: false, character: true, overrides: None })
}

/// Loads `mission` as `load` does but without people's models and animations.
pub fn load_without_people(mission: &str) -> Option<Level> {
    load_with(mission, &LoadOptions { fullbright: true, character: false, overrides: None })
}

/// Plays `g` from the start with no controls, skipping cutscenes, until a second's gone by
/// and no conversation is holding the player still, or `secs` have passed.
pub fn settle(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
}

/// Loads `mission` as `load` does and starts the game on it, `settle`d for up to a minute.
/// None without an install.
pub fn begin(mission: &str) -> Option<(Level, Game)> {
    let level = load(mission)?;
    let mut g = Game::new(&level, true);
    settle(&mut g, &level, 60.0);
    Some((level, g))
}

/// As `settle`, but until no cutscene is playing either.
pub fn settle_past_cutscenes(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
        if g.time > 1.0 && !g.in_conversation() && g.cutscene.is_none() {
            break;
        }
    }
}

/// Plays `n` frames of `g` with no controls, skipping cutscenes.
pub fn frames(g: &mut Game, level: &Level, n: usize) {
    for _ in 0..n {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
    }
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

    /// The ground under `p`.
    pub fn ground_at(&self, p: Vec3) -> Vec3 {
        vec3(p.x, self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0), p.z)
    }

    /// Delivers the car point `car` made to point `to`: puts it there, stopped, the player
    /// beside it.
    pub fn deliver_car(&mut self, car: usize, to: usize) {
        let at = self.ground_at(self.point(to));
        let v = self.g.vehicles.iter_mut().find(|v| v.way == car).unwrap_or_else(|| panic!("no car from point {car}"));
        v.pos = at;
        v.speed = 0.0;
        self.stand(at + vec3(400.0, 0.0, 0.0));
        self.play(3.0, Controls::default());
    }

    /// Knocks down the person point `way` made and cuffs them, from behind.
    pub fn cuff(&mut self, way: usize) {
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

    /// Presses action for one frame (holding it would get into a car and straight back out).
    pub fn tap_action(&mut self) {
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    /// Presses action once no filmed conversation is holding the player still.
    pub fn press_action(&mut self) {
        self.until(60.0, |r| !r.g.in_conversation());
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    /// Talks to the person made by `way` from `off` of them, facing them, then plays `secs`.
    pub fn talk_to(&mut self, way: usize, off: Vec3, secs: f32) {
        let at = self.g.people[self.who(way)].fig.pos;
        self.stand(at + off);
        let me = self.player();
        self.g.people[me].fig.angle = uc_world::character::angle_of(vec2(-off.x, -off.z));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }

    /// Walks up to the person event point `way` created, talks to them, and lets them finish.
    /// Unlike `talk_to`, she's left facing whichever way she was.
    pub fn walk_up_and_talk(&mut self, way: usize, secs: f32) {
        let at = self.g.people[self.who(way)].fig.pos;
        self.stand(at + vec3(130.0, 0.0, 0.0));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }

    /// The first person point `way` made who's still in the game.
    pub fn alive(&self, way: usize) -> Option<usize> {
        self.g.people.iter().position(|p| p.way == way && !p.removed)
    }

    /// Whether the player is a person of type `kind`.
    pub fn playing_as(&self, kind: u8) -> bool {
        self.g.people[self.player()].kind == kind
    }

    /// Where the player is on the map.
    pub fn at(&self) -> Vec2 {
        let p = self.g.people[self.player()].fig.pos;
        vec2(p.x, p.z)
    }

    /// The labels of the beacons showing.
    pub fn beacons(&self) -> Vec<String> {
        self.g.beacons().into_iter().map(|(l, _)| l).collect()
    }

    /// Puts person `i` on the ground (the floor under `at`), facing along `dir`.
    pub fn put(&mut self, i: usize, at: Vec3, dir: Vec2) {
        let y = self.level.ground.surface(&self.level.map, at.x, at.z, at.y + 64.0);
        self.g.people[i].fig.pos = vec3(at.x, y, at.z);
        self.g.people[i].fig.angle = uc_world::character::angle_of(dir);
    }

    /// Kills person `i` with a finishing blow from beside them.
    pub fn kill_person(&mut self, i: usize) {
        let from = self.g.people[i].fig.pos + vec3(50.0, 0.0, 0.0);
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }

    /// Kills the first person point `way` made who's still in the game.
    pub fn kill(&mut self, way: usize) {
        let i = self.alive(way).unwrap_or_else(|| panic!("nobody from point {way}"));
        self.kill_person(i);
    }

    /// Kills the first person the script made at point `way` (removed or not), the blow
    /// coming from where they stand.
    pub fn kill_point_blank(&mut self, way: usize) {
        let i = self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}"));
        let from = self.g.people[i].fig.pos;
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }

    /// Picks up the explosives point `way` puts down.
    pub fn fetch(&mut self, way: usize) {
        let at = self.g.items.iter().find(|it| it.way == way).expect("explosives lying about").pos;
        self.stand(at);
        self.play(0.5, Controls::default());
    }

    /// Takes the explosives out and sets one down where she stands.
    pub fn set_charge(&mut self) {
        self.take_out(Weapon::Explosives);
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    /// Sets a charge beside what point `way` watches and waits, well clear, for it to go up.
    pub fn blow_up(&mut self, way: usize) {
        self.stand(self.point(way) + vec3(150.0, 0.0, 0.0));
        self.set_charge();
        self.stand(self.point(way) + vec3(0.0, 0.0, 2500.0));
        self.play(Charge::FUSE + 1.5, Controls::default());
    }

    /// The car event point `way` made, unless it's been wrecked.
    pub fn car(&self, way: usize) -> Option<usize> {
        self.g.vehicles.iter().position(|v| v.way == way && !v.wrecked())
    }

    /// Gets into the car that event point `way` created, by standing at its door and pressing
    /// action. Returns false if it wouldn't open (a locked car).
    pub fn get_in(&mut self, way: usize) -> bool {
        let v = self.car(way).expect("the car is there");
        let door = self.g.vehicles[v].door();
        self.stand(vec3(door.x, self.g.vehicles[v].pos.y, door.y));
        self.tap_action();
        // Getting in takes as long as the animation (`ANIM_ENTER_CAR`).
        let i = self.g.player.unwrap();
        for _ in 0..150 {
            if !matches!(self.g.people[i].state, State::Enter { .. }) {
                break;
            }
            self.play(DT, Controls::default());
        }
        self.g.player_vehicle().is_some()
    }

    /// Walks straight at each of `pts` (squares) in turn.
    pub fn walk_by(&mut self, pts: &[(f32, f32)]) {
        for &(x, z) in pts {
            let to = mid(x, z);
            let mut done = false;
            for _ in 0..(8.0 / DT) as usize {
                let d = to - self.at();
                if d.length() < 40.0 || self.g.outcome.is_some() {
                    done = true;
                    break;
                }
                self.play(DT, Controls { wish: d.normalize(), ..Default::default() });
            }
            assert!(done, "Darci got stuck at {} on the way to ({x}, {z}) ({:?}; {:?})",
                    self.g.people[self.player()].fig.pos, self.g.people[self.player()].state, self.g.outcome);
        }
    }
}
