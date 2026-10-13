//! Urban Chaos gameplay, renderer-independent: people, melee combat, arrests and the
//! mission script that drives them.
//!
//! ```text
//! input ──► Controls ─┐
//!                     ├─► Person::step (each person) ──► Events ──► Game::apply
//! AI ─► Person::think ┘                                     ▲
//! Script::update ◄── Query (Game) ──► Commands ─────────────┘
//! ```
//!
//! A frame: the mission script runs and its commands are applied (spawns, messages, win/lose).
//! Then every person steps from a snapshot of everyone taken at the start of the frame.
//! Finally the events they produced (hits, grabs, arrests) are applied, grappled pairs are
//! kept together and people are pushed apart.

pub mod goals;
pub mod mist;
pub mod person;
pub mod pyro;
pub mod script;
pub mod vehicle;

use glam::{vec2, Vec2, Vec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uc_world::character::person as ptype;
use uc_world::{Batch, Level};

pub use mist::Mists;
pub use person::{Controls, Person, Sfx, State, Weapon};
pub use pyro::Pyro;
pub use script::Shot;
pub use vehicle::{Drive, Vehicle};

/// `facet_rand`'s LCG (fallen/Source/facet.cpp): the one source of chance in the game, so a
/// run plays out the same way twice.
pub(crate) fn rand(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(69069).wrapping_add(1);
    *seed >> 16
}

/// A barrel, bin or traffic cone (`BT_*`): knocked about by cars.
#[derive(Debug, Clone)]
pub struct Barrel {
    pub kind: u8,
    pub pos: Vec3,
    pub vel: Vec2,
    pub yaw: f32,
    /// How far it has toppled over (0 upright .. pi/2 on its side).
    pub tilt: f32,
}

impl Barrel {
    /// Its prim (barrels and cones are modelled with z up).
    pub fn prim(&self) -> u8 {
        match self.kind {
            1 => 67,
            5 => 145,
            _ => 141,
        }
    }
}

/// Something lying in the world that can be picked up (or a barrel).
#[derive(Debug, Clone)]
pub struct Item {
    /// The event point that created it (0 = dropped by someone).
    pub way: usize,
    /// `IT_*`.
    pub kind: u8,
    pub pos: Vec3,
}

/// A camera fly-by in progress.
pub struct Cutscene {
    shot: Shot,
    /// Index of the waypoint being flown to.
    leg: usize,
    /// Camera position (world units).
    pub pos: Vec3,
    wait: f32,
}

impl Cutscene {
    /// Moves the camera on; returns the waypoints reached this frame, and whether it's over.
    fn update(&mut self, dt: f32) -> (Vec<usize>, bool) {
        let mut reached = Vec::new();
        if self.wait > 0.0 {
            self.wait -= dt;
            return (reached, false);
        }
        let Some(&(way, to, speed, delay)) = self.shot.waypoints.get(self.leg) else { return (reached, true) };
        let d = to - self.pos;
        let step = speed * dt;
        if d.length() <= step {
            self.pos = to;
            reached.push(way);
            self.wait = delay;
            self.leg += 1;
        } else {
            self.pos += d / d.length() * step;
        }
        (reached, false)
    }
}

/// A conversation being said, one line at a time.
struct Convo {
    way: usize,
    /// The recordings' number.
    speech: usize,
    lines: Vec<(usize, String)>,
    line: usize,
    left: f32,
}

/// What the music should be doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Calm,
    Fight,
    Sprint,
    Won,
    Lost,
}
pub use script::{Command, Script};

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Won,
    Lost(String),
}

/// Item kinds `BADGE..BADGE + 4` are the badges: stamina, strength, reflexes, constitution.
pub const BADGE: u8 = 100;
pub const STAT_NAMES: [&str; 4] = ["Stamina", "Strength", "Reflexes", "Constitution"];

/// The player's gun picks targets this close (world units).
const TARGET_RANGE: f32 = 14.0 * 256.0;

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    /// Seconds left on screen.
    pub left: f32,
}

pub struct Game {
    pub people: Vec<Person>,
    pub player: Option<usize>,
    by_way: HashMap<usize, usize>,
    script: Option<Script>,
    pub messages: Vec<Message>,
    pub outcome: Option<Outcome>,
    pub time: f32,
    rng: u32,
    /// Sounds not tied to a person this frame.
    sounds: Vec<(Sfx, Vec3)>,
    /// Seconds left of fight music after the fighting stops.
    fight_timer: f32,
    sprinting: bool,
    pub items: Vec<Item>,
    pub vehicles: Vec<Vehicle>,
    veh_by_way: HashMap<usize, usize>,
    pub barrels: Vec<Barrel>,
    /// A road sign flashed by the mission: (text, seconds left).
    pub sign: Option<(String, f32)>,
    pub cutscene: Option<Cutscene>,
    convo: Option<Convo>,
    /// The player pressed action this frame.
    used: bool,
    /// Who the player's gun is aimed at.
    pub target: Option<usize>,
    /// Darci's stats: stamina, strength, reflexes, constitution (badges from earlier
    /// missions, see `set_stats`, plus `badges`).
    pub stats: [u8; 4],
    /// Badges picked up in this mission.
    pub badges: [u8; 4],
    /// The points of the badges picked up in this mission.
    pub found: BTreeSet<usize>,
    /// The `create_item` points whose item the player has picked up (`EWAY_FLAG_GOTITEM`,
    /// which the `specific_item_held` trigger watches).
    taken: BTreeSet<usize>,
    /// Cops already counted towards the script's tally of them, so none is counted twice.
    dead_cops: BTreeSet<usize>,
    /// The player has fired a gun. `GF_PLAYER_FIRED_GUN` is set once and never cleared, so a
    /// mission that cares only ever asks whether it has happened at all.
    player_fired: bool,
    /// Explosions and fires burning.
    pub pyro: Pyro,
    /// Sheets of ground mist the mission has laid down.
    pub mists: Mists,
    /// What to call the player, for the HUD and for their lines in a conversation.
    player_name: &'static str,
}

/// What a mission calls the player. The original's panel carries no name at all (it just
/// picks Darci's face or Roper's), so these are this engine's own: the person's type, except
/// where the mission's own dialogue keeps calling them something, as Nitro Car's does Mako.
fn player_name(mission: &str, kind: u8) -> &'static str {
    match mission.to_ascii_lowercase().as_str() {
        "bankbomb1" | "gangorder2" => "Mako",
        _ => ptype::name(kind),
    }
}

impl Game {
    /// Starts a level. With `scripted`, the mission's event points run (spawning the player,
    /// enemies, messages...); otherwise Darci just stands at the player start.
    pub fn new(level: &Level, scripted: bool) -> Game {
        let script = scripted.then(|| Script::new(&level.mission));
        let mut g = Game {
            people: Vec::new(), player: None, by_way: HashMap::new(), script, messages: Vec::new(),
            outcome: None, time: 0.0, rng: 1, sounds: Vec::new(), fight_timer: 0.0, sprinting: false,
            items: Vec::new(), cutscene: None, convo: None, used: false, vehicles: Vec::new(), veh_by_way: HashMap::new(),
            barrels: Vec::new(), sign: None, target: None, stats: [0; 4], badges: [0; 4], found: BTreeSet::new(),
            taken: BTreeSet::new(), dead_cops: BTreeSet::new(), player_fired: false,
            pyro: Pyro::default(), mists: Mists::default(),
            player_name: "",
        };
        if !scripted {
            g.spawn_player(level, level.spawn.0, level.spawn.1, 0, ptype::DARCI, 0);
        }
        // Run the first tick now so the player exists straight away.
        g.run_script(0.0, level);
        if g.player.is_none() {
            g.spawn_player(level, level.spawn.0, level.spawn.1, 0, ptype::DARCI, 0);
        }
        g
    }

    /// Event points the script can't run yet.
    pub fn unsupported(&self) -> &[String] {
        self.script.as_ref().map_or(&[], |s| &s.unsupported)
    }

    fn rand(&mut self) -> u32 {
        rand(&mut self.rng)
    }

    /// A blast that knocks people about: damage falls off from `damage` at the middle to
    /// nothing at `radius`, and over 30 points knocks them down (`create_shockwave`,
    /// fallen/Source/collide.cpp). People in cars are shielded, as everywhere else here.
    fn shockwave(&mut self, at: Vec3, radius: f32, damage: i32) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.removed || p.is_dead() {
                continue;
            }
            // QDIST3: the largest axis plus a quarter of each of the others.
            let d = (p.fig.pos - at).abs();
            let dist = d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0;
            if dist >= radius {
                continue;
            }
            let hit = (damage as f32 * (radius - dist) / radius) as i32;
            let ko = if hit > 30 { person::ko::FINISHERS } else { 0 };
            self.people[i].take_hit(at, hit, ko, false, false, Weapon::None);
        }
    }

    fn spawn(&mut self, level: &Level, kind: u8, player: bool, way: usize, pos: Vec3, angle: f32) -> Option<usize> {
        let people = level.people.as_ref()?;
        // Points can sit a little under the surface they're on (The Jump's jumper is 64 below
        // his ledge): look from half a storey up.
        let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 128.0);
        let variant = self.rand();
        let fig = people.figure(kind, variant, Vec3::new(pos.x, y, pos.z), angle)?;
        self.people.push(Person::new(fig, kind, player, way));
        let i = self.people.len() - 1;
        if way != 0 {
            self.by_way.insert(way, i);
        }
        Some(i)
    }

    fn spawn_player(&mut self, level: &Level, pos: Vec3, angle: f32, way: usize, kind: u8, has: u8) {
        if self.player.is_none() {
            self.player = self.spawn(level, kind, true, way, pos, angle);
            self.player_name = player_name(&level.mission.mission_name, kind);
            // Carried, not drawn: `PCOM_create_player` only raises `FLAGS_HAS_GUN`, which puts
            // the pistol in the weapon list (`PANEL_ADDWEAPON`) for the player to pick.
            if let Some(i) = self.player {
                self.people[i].give(Weapon::from_has(has));
            }
        }
    }

    pub fn player(&self) -> Option<&Person> {
        self.player.map(|i| &self.people[i])
    }

    /// The side objectives of `goals` done so far: those whose reward the script has created.
    pub fn sides_done(&self, goals: &goals::Goals) -> BTreeSet<usize> {
        let Some(s) = &self.script else { return BTreeSet::new() };
        goals.sides.iter().filter(|g| g.rewards.iter().any(|&r| s.point_active(r))).map(|g| g.id).collect()
    }

    /// Starts with badge stats from earlier missions (constitution adds health).
    pub fn set_stats(&mut self, stats: [u8; 4]) {
        self.stats = stats;
        if let Some(p) = self.player {
            let p = &mut self.people[p];
            p.max_health += 10 * stats[3] as i32;
            p.health = p.max_health;
        }
    }

    /// Where each person is walking to when they aren't fighting: `PCOM_process_patrol`
    /// (move 2 or 3) heads for the nearest active point of their colour and group, or once
    /// there (within a square), the next one by number; `PCOM_MOVE_FOLLOW` (move 5) keeps up
    /// with the person their `follow` point created, the player by default.
    fn walk_goals(&mut self) {
        let Some(script) = &self.script else { return };
        // Where everyone stands this frame, so they all aim at the same thing.
        let at: Vec<Vec3> = self.people.iter().map(|p| p.fig.pos).collect();
        let mut goals: Vec<(Option<Vec3>, person::Pace)> = vec![(None, person::Pace::Walk); self.people.len()];
        for (i, p) in self.people.iter().enumerate() {
            if p.player || p.aggro() || p.is_dead() {
                continue;
            }
            // QDIST3: the largest axis plus a quarter of each of the other two.
            let near = |q: Vec3| {
                let d = (q - at[i]).abs();
                d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0
            };
            goals[i] = match p.mv {
                // Followers run after whoever they're following, as `PCOM_set_person_ai_follow`
                // asks (`PERSON_SPEED_RUN`), and break into a sprint to keep up with a player
                // who is sprinting. They stop a couple of squares short of treading on them.
                5 => {
                    let j = if p.follow == 0 { self.player } else { self.by_way.get(&p.follow).copied() };
                    let to = j
                        .filter(|&j| !self.people[j].removed && !self.people[j].is_dead())
                        .map(|j| at[j])
                        .filter(|&to| (to - at[i]).length() > 2.0 * 256.0);
                    let hurrying = j == self.player && self.sprinting;
                    (to, if hurrying { person::Pace::Sprint } else { person::Pace::Run })
                }
                // `PCOM_process_patrol`: a walk, unless they're `PCOM_BENT_DILIGENT`.
                2 | 3 => {
                    let points = script.patrol_points(p.colour, p.group);
                    let nearest = (0..points.len()).min_by(|&a, &b| near(points[a].1).total_cmp(&near(points[b].1)));
                    let to = nearest.map(|k| {
                        let k = if near(points[k].1) < 256.0 { (k + 1) % points.len() } else { k };
                        points[k].1
                    });
                    (to, if p.bent & 2 != 0 { person::Pace::Run } else { person::Pace::Walk })
                }
                _ => (None, person::Pace::Walk),
            };
        }
        for (p, (goal, pace)) in self.people.iter_mut().zip(goals) {
            (p.goal, p.pace) = (goal, pace);
        }
    }

    /// Where person `i` is: their car's position while driving.
    fn thing_pos(&self, i: usize) -> Vec3 {
        match self.people[i].state {
            State::InCar { vehicle } => self.vehicles[vehicle].pos,
            _ => self.people[i].fig.pos,
        }
    }

    fn run_script(&mut self, dt: f32, level: &Level) {
        let Some(mut script) = self.script.take() else { return };
        let commands = script.update(dt, &Scene { game: self, level });
        self.script = Some(script);
        for c in commands {
            match c {
                Command::SpawnPlayer { way, pos, angle, person, has } => {
                    self.spawn_player(level, pos, angle, way, person, has);
                    if let Some(p) = self.player {
                        self.by_way.insert(way, p);
                    }
                }
                Command::SpawnEnemy { way, pos, angle, spec } => {
                    if let Some(i) = self.spawn(level, spec.person, false, way, pos, angle) {
                        let p = &mut self.people[i];
                        (p.ai, p.bent, p.ko_moves) = (spec.ai, spec.bent, spec.ko_moves);
                        (p.skill, p.mv, p.weapon) = (spec.skill, spec.mv, Weapon::from_has(spec.has));
                        p.follow = spec.follow;
                        p.fig.clothes = spec.colour & 3;
                        p.fig.jacket = spec.skill / 4;
                        p.other = spec.other;
                        (p.colour, p.group) = (spec.colour, spec.group);
                        p.give(p.weapon);
                    }
                }
                Command::Adjust { way, ai, bent, ko_moves, other, mv, follow, colour, group } => {
                    if let Some(&i) = self.by_way.get(&way) {
                        let p = &mut self.people[i];
                        (p.ai, p.bent, p.ko_moves, p.other, p.mv, p.follow) = (ai, bent, ko_moves, other, mv, follow);
                        (p.colour, p.group) = (colour, group);
                    }
                }
                Command::Remove { way } => {
                    if let Some(&i) = self.by_way.get(&way) {
                        self.people[i].removed = true;
                    }
                    self.items.retain(|it| it.way != way);
                    // A car a kill_waypoint takes away blows up where it stands: the original
                    // wrecks it (1050 damage) and sets off a shockwave that catches the player
                    // too, then forgets it was ever created, so a "killed" trigger watching
                    // the car won't fire afterwards (`EWAY_DO_KILL_WAYPOINT`, eway.cpp).
                    if let Some(v) = self.veh_by_way.remove(&way) {
                        let at = self.vehicles[v].pos;
                        self.vehicles[v].health = 0;
                        if let Some(d) = self.vehicles[v].driver.take() {
                            self.people[d].state = State::Move;
                            self.people[d].fig.pos = at;
                        }
                        self.pyro.burst(at + Vec3::new(0.0, 48.0, 0.0), pyro::CAR_BOMB, 400.0);
                        self.sounds.push((Sfx::Explosion, at));
                        self.shockwave(at, 768.0, 350);
                    }
                }
                Command::Move { way, pos, angle } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    if let Some(&i) = self.by_way.get(&way) {
                        let p = &mut self.people[i];
                        if !p.is_dead() {
                            p.fig.pos = Vec3::new(pos.x, y, pos.z);
                            p.fig.angle = angle;
                        }
                    }
                    for it in self.items.iter_mut().filter(|it| it.way == way) {
                        it.pos = Vec3::new(pos.x, y, pos.z);
                    }
                }
                Command::Item { way, kind, pos } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    self.items.push(Item { way, kind, pos: Vec3::new(pos.x, y, pos.z) });
                }
                Command::Camera { way, shot } => {
                    let _ = way;
                    self.cutscene = Some(Cutscene { pos: shot.start, shot, leg: 0, wait: 0.0 });
                }
                Command::Conversation { way, speech, lines } => {
                    // A conversation still going is cut short but counts as said, as in the
                    // original (`EWAY_finish_conversation`): what waits on it still happens.
                    if let (Some(old), Some(s)) = (self.convo.take(), self.script.as_mut()) {
                        s.conversation_over(old.way);
                    }
                    self.convo = Some(Convo { way, speech, lines, line: usize::MAX, left: 0.0 });
                }
                Command::Message { way, speech, text, secs } => {
                    let _ = way;
                    let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                    self.sounds.push((Sfx::Voice(speech), at));
                    self.messages.retain(|m| m.text != text);
                    self.messages.push(Message { text, left: secs });
                }
                Command::Vehicle { way, kind, pos, angle, locked } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    // A vehicle's angle is its heading (its nose along (sin, cos)).
                    self.vehicles.push(Vehicle::new(way, kind, Vec3::new(pos.x, y, pos.z), angle, locked));
                    self.veh_by_way.insert(way, self.vehicles.len() - 1);
                }
                Command::Stall { way } => {
                    if let Some(&v) = self.veh_by_way.get(&way) {
                        self.vehicles[v].stalled = true;
                    }
                }
                Command::Lock { way, locked } => {
                    if let Some(&v) = self.veh_by_way.get(&way) {
                        self.vehicles[v].locked = locked;
                    }
                }
                // A `visual_effect` point's bang. The original's shockwave here never goes
                // off: `if (ew->ed.subtype=!16)` in `EWAY_DO_EXPLODE` (eway.cpp) assigns
                // instead of comparing, so the test is always false. Nobody is hurt.
                Command::Explode { way, pos, kinds, scale } => {
                    let _ = way;
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    let at = Vec3::new(pos.x, y.max(pos.y), pos.z);
                    self.pyro.burst(at, kinds, if scale > 0.0 { scale } else { 400.0 });
                    self.sounds.push((Sfx::Explosion, at));
                }
                Command::Barrel { way, kind, pos } => {
                    let _ = way;
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    let yaw = (self.rand() % 628) as f32 / 100.0;
                    self.barrels.push(Barrel { kind, pos: Vec3::new(pos.x, y, pos.z), vel: Vec2::ZERO, yaw, tilt: 0.0 });
                }
                Command::Sign { which, flip } => {
                    let text = match (which, flip) {
                        (0, _) => "U-TURN",
                        (1, false) => "TURN RIGHT",
                        (1, true) => "TURN LEFT",
                        (2, false) => "NO RIGHT TURN",
                        (2, true) => "NO LEFT TURN",
                        _ => "STOP",
                    };
                    self.sign = Some((text.to_string(), 2.5));
                }
                Command::Mist { pos } => self.mists.create(pos.x, pos.z),
                Command::Sound { id, pos, replace } => self.sounds.push((Sfx::ById { id, replace }, pos)),
                Command::OpenDoor { pos } => {
                    if !level.ground.open_door_near(pos.x, pos.z) {
                        // The original says so too, on its console.
                        eprintln!("mission: no door near {:.0},{:.0} to open", pos.x, pos.z);
                    }
                }
                Command::Win => self.finish(Outcome::Won),
                Command::Lose => self.finish(Outcome::Lost("Mission failed".into())),
            }
        }
    }

    /// Keeps the script's tally of cops killed up to date: the original bumps counter 7
    /// wherever a `PERSON_COP` dies, and a mission can ask for a number of them.
    fn count_dead_cops(&mut self) {
        let newly: Vec<usize> = self
            .people
            .iter()
            .enumerate()
            .filter(|(i, p)| p.kind == ptype::COP && p.knocked_out && !self.dead_cops.contains(i))
            .map(|(i, _)| i)
            .collect();
        for i in newly {
            self.dead_cops.insert(i);
            if let Some(s) = self.script.as_mut() {
                s.note_cop_killed();
            }
        }
    }

    fn finish(&mut self, o: Outcome) {
        if self.outcome.is_none() {
            let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
            self.sounds.push((if o == Outcome::Won { Sfx::Won } else { Sfx::Lost }, at));
            self.outcome = Some(o);
        }
    }

    /// The sounds made since the last call, with where they were made (world units).
    pub fn take_sounds(&mut self) -> Vec<(Sfx, Vec3)> {
        let mut out = std::mem::take(&mut self.sounds);
        for p in &mut self.people {
            let at = p.fig.pos + Vec3::new(0.0, 100.0, 0.0);
            out.extend(p.cues.drain(..).map(|s| (s, at)));
        }
        out
    }

    /// Whether mission event point `i` is active (for tests and debugging).
    pub fn point_active(&self, i: usize) -> bool {
        self.script.as_ref().is_some_and(|s| s.point_active(i))
    }

    /// The visible countdown, in seconds, while one runs.
    pub fn countdown(&self) -> Option<f32> {
        self.script.as_ref()?.countdown()
    }

    /// The fly-by camera, if one is running: (position, looking at), world units.
    pub fn camera(&self) -> Option<(Vec3, Vec3)> {
        self.cutscene.as_ref().map(|c| (c.pos, c.shot.target))
    }

    /// Skips the rest of a fly-by (everything it would have triggered still happens).
    pub fn skip_cutscene(&mut self) {
        let Some(c) = self.cutscene.take() else { return };
        if let Some(mut script) = self.script.take() {
            for &(way, ..) in &c.shot.waypoints[c.leg.min(c.shot.waypoints.len())..] {
                script.camera_reached(way, self);
            }
            self.script = Some(script);
        }
    }

    fn speaker(&self, way: usize) -> &'static str {
        let p = if way == 0 { self.player } else { self.by_way.get(&way).copied() };
        match p.map(|i| &self.people[i]) {
            Some(p) if p.player => self.player_name,
            Some(p) if matches!(p.kind, ptype::COP | ptype::ROPER) => ptype::name(p.kind),
            _ => "",
        }
    }

    /// What to call the player.
    pub fn player_name(&self) -> &'static str {
        self.player_name
    }

    /// Fly-bys and conversations move on.
    fn update_scenes(&mut self, dt: f32) {
        if let Some(c) = self.cutscene.as_mut() {
            let (reached, done) = c.update(dt);
            if done {
                self.cutscene = None;
            }
            if let Some(mut script) = self.script.take() {
                for way in reached {
                    script.camera_reached(way, self);
                }
                self.script = Some(script);
            }
        }
        if let Some(mut cv) = self.convo.take() {
            cv.left -= dt;
            if cv.left <= 0.0 {
                cv.line = cv.line.wrapping_add(1);
                match cv.lines.get(cv.line).cloned() {
                    Some((who, text)) => {
                        cv.left = (1.5 + text.len() as f32 * 0.06).min(6.0);
                        let name = self.speaker(who);
                        let shown = if name.is_empty() { text } else { format!("{name}: {text}") };
                        self.messages.push(Message { text: shown, left: cv.left });
                        let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                        self.sounds.push((Sfx::ConversationLine(cv.speech, cv.line), at));
                    }
                    None => {
                        if let Some(s) = &mut self.script {
                            s.conversation_over(cv.way);
                        }
                        return;
                    }
                }
            }
            self.convo = Some(cv);
        }
    }

    /// The player presses action: get out of the car, into a car next to her, or talk to the
    /// person next to her. Returns whether something used the key.
    fn use_something(&mut self, level: &Level) -> bool {
        let Some(pi) = self.player else { return false };
        match self.people[pi].state {
            State::InCar { vehicle } => {
                let v = &mut self.vehicles[vehicle];
                if v.speed.abs() > 250.0 {
                    return true;
                }
                v.driver = None;
                let door = v.door();
                let y = level.ground.surface(&level.map, door.x, door.y, v.pos.y + 64.0);
                let p = &mut self.people[pi];
                p.fig.pos = Vec3::new(door.x, y, door.y);
                p.state = State::Move;
                p.cues.push(Sfx::CarDoor);
                true
            }
            State::Move => {
                let pos = self.people[pi].fig.pos;
                let near = |v: &Vehicle| !v.wrecked() && !v.locked && v.driver.is_none()
                    && v.covers(Vec2::new(pos.x, pos.z), 140.0) && (v.pos.y - pos.y).abs() < 150.0;
                if let Some(vi) = self.vehicles.iter().position(near) {
                    self.vehicles[vi].driver = Some(pi);
                    let p = &mut self.people[pi];
                    p.state = State::InCar { vehicle: vi };
                    p.cues.push(Sfx::CarDoor);
                    return true;
                }
                // Talk to someone friendly close by.
                let talk = self.people.iter().enumerate().find(|(j, o)| {
                    *j != pi && o.way != 0 && !o.removed && !o.hostile() && !o.is_down()
                        && (o.fig.pos - pos).length() < 170.0
                });
                if let Some((_, o)) = talk {
                    let way = o.way;
                    if let Some(mut script) = self.script.take() {
                        script.person_used(way, self);
                        self.script = Some(script);
                    }
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Cars move; the player's car follows her controls. Cars knock people over and push
    /// cones and barrels around.
    fn drive(&mut self, dt: f32, input: &Controls, level: &Level) {
        let player = self.player;
        for vi in 0..self.vehicles.len() {
            let v = &mut self.vehicles[vi];
            let d = if v.driver.is_some() && v.driver == player { input.drive } else { Drive::default() };
            let sliding = v.slip.abs() > 300.0;
            if let Some(speed) = v.update(d, dt, &level.ground, &level.map) {
                if speed > 300.0 {
                    self.sounds.push((Sfx::Crash, v.pos));
                }
            }
            if !sliding && v.slip.abs() > 300.0 {
                self.sounds.push((Sfx::Skid, v.pos));
            }
            let v = self.vehicles[vi].clone();
            if let Some(d) = v.driver {
                let p = &mut self.people[d];
                p.fig.pos = v.pos;
                p.fig.angle = v.driver_angle();
            }
            let fwd = v.forward();
            let side = Vec2::new(fwd.y, -fwd.x);
            // People ahead of a moving car run out of its way, as in the original. Those who
            // don't make it are knocked flying when it's fast and nudged aside when it's slow.
            if v.speed.abs() > 150.0 {
                let heading = fwd * v.speed.signum();
                for (i, p) in self.people.iter_mut().enumerate() {
                    if p.player || Some(i) == v.driver || p.removed || p.is_down() || p.dodge.is_some()
                        || !matches!(p.state, State::Move) || (p.fig.pos.y - v.pos.y).abs() > 150.0
                    {
                        continue;
                    }
                    let r = Vec2::new(p.fig.pos.x - v.pos.x, p.fig.pos.z - v.pos.z);
                    let (ahead, across) = (r.dot(heading), r.dot(side));
                    // Within the next second or so of the car's path.
                    if ahead > 0.0 && ahead < 400.0 + v.speed.abs() && across.abs() < 300.0 {
                        let away = if across >= 0.0 { side } else { -side };
                        p.dodge = Some((away, 0.9));
                    }
                }
            }
            // People in the way.
            for (i, p) in self.people.iter_mut().enumerate() {
                if Some(i) == v.driver || p.removed || matches!(p.state, State::InCar { .. }) || (p.fig.pos.y - v.pos.y).abs() > 150.0 {
                    continue;
                }
                let at = Vec2::new(p.fig.pos.x, p.fig.pos.z);
                if !v.covers(at, 30.0) {
                    continue;
                }
                if v.speed.abs() > 300.0 && !p.is_down() {
                    p.take_hit(v.pos, (v.speed.abs() / 12.0) as i32, person::ko::KNOCKDOWN, false, true, Weapon::None);
                }
                let away = if (at - Vec2::new(v.pos.x, v.pos.z)).dot(side) >= 0.0 { side } else { -side };
                let to = level.ground.slide(at, p.fig.pos.y, away * 40.0, 36.0);
                p.fig.pos.x = to.x;
                p.fig.pos.z = to.y;
            }
            // Cones and barrels get knocked about.
            for b in &mut self.barrels {
                let at = Vec2::new(b.pos.x, b.pos.z);
                if v.speed.abs() > 60.0 && v.covers(at, 30.0) && b.vel.length() < v.speed.abs() * 0.5 {
                    let side = (at - Vec2::new(v.pos.x, v.pos.z)).normalize_or_zero();
                    b.vel = fwd * v.speed * 0.9 + side * v.speed.abs() * 0.3;
                    self.sounds.push((Sfx::Cone, b.pos));
                }
            }
        }
        for b in &mut self.barrels {
            if b.vel.length_squared() < 1.0 {
                continue;
            }
            let at = Vec2::new(b.pos.x, b.pos.z);
            let to = level.ground.slide(at, b.pos.y, b.vel * dt, 30.0);
            b.pos = Vec3::new(to.x, level.ground.surface(&level.map, to.x, to.y, b.pos.y + 32.0), to.y);
            b.vel *= (-2.5 * dt).exp();
            b.yaw += b.vel.length() * dt / 60.0;
            if b.kind == 1 {
                b.tilt = (b.tilt + 8.0 * dt).min(std::f32::consts::FRAC_PI_2);
            }
        }
    }

    /// The car the player is driving.
    pub fn player_vehicle(&self) -> Option<&Vehicle> {
        match self.player().map(|p| &p.state) {
            Some(State::InCar { vehicle }) => self.vehicles.get(*vehicle),
            _ => None,
        }
    }

    /// Keeps the player's target: the enemy picked stays picked while it can be shot,
    /// otherwise the nearest one (preferring those in front). `cycle` moves on to the next.
    fn update_target(&mut self, cycle: bool, level: &Level) {
        let Some(pi) = self.player else { return };
        let p = &self.people[pi];
        if !p.weapon.is_gun() || matches!(p.state, State::InCar { .. }) {
            self.target = None;
            return;
        }
        let (pos, f) = (p.fig.pos, p.facing_dir());
        let eye = Vec3::new(0.0, 140.0, 0.0);
        // (index, how far, how far round from straight ahead), nearest first.
        let mut options: Vec<(usize, f32, f32)> = self
            .people
            .iter()
            .enumerate()
            .filter(|&(i, q)| i != pi && !q.removed && !q.is_dead() && !q.is_down() && (q.hostile() || q.aggro()))
            .filter_map(|(i, q)| {
                let d = Vec2::new(q.fig.pos.x - pos.x, q.fig.pos.z - pos.z);
                let dist = d.length();
                (dist < TARGET_RANGE && (q.fig.pos.y - pos.y).abs() < 4.0 * 256.0
                    && level.ground.line_of_sight(pos + eye, q.fig.pos + eye))
                    .then(|| (i, dist, 1.0 - f.dot(d / dist.max(1.0))))
            })
            .collect();
        options.sort_by(|a, b| a.1.total_cmp(&b.1));
        let kept = self.target.filter(|t| options.iter().any(|o| o.0 == *t));
        self.target = if cycle && !options.is_empty() {
            let at = kept.and_then(|t| options.iter().position(|o| o.0 == t));
            Some(options[at.map_or(0, |k| (k + 1) % options.len())].0)
        } else if kept.is_some() {
            kept
        } else {
            // The nearest, favouring those ahead (someone right behind counts as 3x as far).
            options.iter().min_by(|a, b| (a.1 * (1.0 + a.2)).total_cmp(&(b.1 * (1.0 + b.2)))).map(|o| o.0)
        };
    }

    /// The player walks over items: health heals, weapons and ammo are picked up.
    fn pick_up(&mut self) {
        let Some(pi) = self.player else { return };
        let p = &self.people[pi];
        if !matches!(p.state, State::Move | State::Land { .. }) {
            return;
        }
        let pos = p.fig.pos;
        // Badges are collected within 160 units, summing the three axes (as the original does).
        let near = |it: &Item| if (BADGE..BADGE + 4).contains(&it.kind) {
            let d = (it.pos - pos).abs();
            d.x + d.y + d.z < 160.0
        } else {
            Vec2::new(it.pos.x - pos.x, it.pos.z - pos.z).length() < 80.0 && (it.pos.y - pos.y).abs() < 100.0
        };
        let Some(i) = self.items.iter().position(near) else { return };
        let (kind, way) = (self.items[i].kind, self.items[i].way);
        let p = &mut self.people[pi];
        let took = match kind {
            3 if p.health < p.max_health => {
                p.health = (p.health + 100).min(p.max_health);
                true
            }
            // A badge raises one of Darci's stats. Constitution adds health; strength makes
            // her blows land harder (see `apply`).
            k if (BADGE..BADGE + 4).contains(&k) => {
                let stat = (k - BADGE) as usize;
                self.stats[stat] += 1;
                self.badges[stat] += 1;
                self.found.insert(way);
                if stat == 3 {
                    p.max_health += 10;
                    p.health += 10;
                }
                self.messages.push(Message { text: format!("{} increased", STAT_NAMES[stat]), left: 4.0 });
                true
            }
            // A gun: carried (with its rounds), and drawn if the hands are empty.
            2 | 4 | 6 => {
                let w = Weapon::from_item(kind).unwrap();
                p.give(w);
                if p.weapon == Weapon::None {
                    p.weapon = w;
                }
                true
            }
            5 | 8 if p.melee == Weapon::None => {
                let w = Weapon::from_item(kind).unwrap();
                p.give(w);
                if p.weapon == Weapon::None {
                    p.weapon = w;
                }
                true
            }
            // The things a mission sends her to fetch: a key, a keycard, files, a floppy, a
            // crowbar, a gas mask, a wrench, a video, gloves, weedkiller, explosives. They do
            // nothing in the hand; what matters is that she has them, which a mission reads
            // with `specific_item_held` (The Fallen's five clues are picked up this way).
            1 | 13..=21 | 23 => true,
            // Ammo: pistol, shotgun, AK-47.
            10..=12 => {
                let g = Weapon::GUNS[(kind - 10) as usize];
                p.ammo[(kind - 10) as usize] += g.clip();
                true
            }
            _ => false,
        };
        if took {
            p.cues.push(Sfx::Pickup);
            self.items.remove(i);
            if way != 0 {
                self.taken.insert(way);
            }
        }
    }

    /// What the music should be doing: fight music while enemies are on the player, sprint
    /// music while sprinting, quiet otherwise.
    pub fn mood(&self) -> Mood {
        match &self.outcome {
            Some(Outcome::Won) => Mood::Won,
            Some(Outcome::Lost(_)) => Mood::Lost,
            None if self.fight_timer > 0.0 => Mood::Fight,
            None if self.sprinting => Mood::Sprint,
            None => Mood::Calm,
        }
    }

    /// Advances the game by `dt` seconds with the player's `input`.
    pub fn update(&mut self, dt: f32, input: &Controls, level: &Level) {
        self.time += dt;
        for m in &mut self.messages {
            m.left -= dt;
        }
        self.messages.retain(|m| m.left > 0.0);
        self.used = input.action;
        if self.outcome.is_none() {
            self.update_target(input.cycle_target, level);
        }
        if self.outcome.is_none() {
            self.run_script(dt, level);
        }
        self.update_scenes(dt);
        self.pyro.update(dt);
        let frozen = self.cutscene.as_ref().is_some_and(|c| c.shot.lock_player);

        if let Some((_, t)) = &mut self.sign {
            *t -= dt;
            if *t <= 0.0 {
                self.sign = None;
            }
        }
        // Getting in and out of cars and talking to people take the action key first.
        let mut input = *input;
        input.target = self.target;
        if self.outcome.is_none() && !frozen && input.action && self.use_something(level) {
            input.action = false;
        }
        // Fly-bys that hold the player still hold the car too (it rolls to a stop).
        if frozen || self.outcome.is_some() {
            input.drive = Drive::default();
        }
        self.drive(dt, &input, level);

        let snaps: Vec<_> = self.people.iter().map(|p| p.snap()).collect();
        let mut events = Vec::new();
        let player = self.player;
        // Who each of the AI fights: an assassin (`PCOM_AI_ASSASIN`, `_SHOOT_DEAD`) goes for
        // the person its `ai_other` point created, until they're dead; everyone else for the
        // player. A fight between other people waits until Darci is near enough to see it
        // (12 squares from the mark), then carries on: the original starts at once, so the
        // crash cops in RTA had shot the drunk long before she got there.
        self.walk_goals();
        let darci = player.map(|p| self.people[p].fig.pos);
        for i in 0..self.people.len() {
            let p = &self.people[i];
            let foe = match (p.ai, p.other) {
                (3 | 21, w) if w != 0 => self.by_way.get(&w).copied().filter(|&f| {
                    let mark = &self.people[f];
                    let seen = Some(f) == player || p.aggro()
                        || darci.is_some_and(|d| (d - mark.fig.pos).length() < 12.0 * 256.0);
                    !mark.is_dead() && seen
                }),
                // A bodyguard goes for whoever is going for the person they guard, or for
                // themselves: the nearest of them (`PCOM_find_bodyguard_victim`). Only someone
                // out to kill counts, as the original's "killing or navigating to kill" does.
                // Being hit makes anyone go `aggro`, the player included, and the player
                // counts as their own foe, so without this a bodyguard would turn on the very
                // person they are guarding the moment a cop landed a blow on them.
                // They have to be able to see them, too, and keep seeing them: without any
                // route-finding here, a bodyguard who sets off after someone round a corner
                // walks into the wall between them and stands there for the rest of the
                // mission. The original gives up as well when it can't work out how to get
                // there (`PCOM_AI_SUBSTATE_CANTFIND` in `PCOM_set_person_ai_follow`) and
                // waits until `can_a_see_b` says the way is clear again.
                (person::AI_BODYGUARD, w) => {
                    let client = if w == 0 { player } else { self.by_way.get(&w).copied() };
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    self.people
                        .iter()
                        .enumerate()
                        .filter(|(j, o)| {
                            *j != i && o.hostile() && o.aggro() && !o.is_dead()
                                && (o.foe == client || o.foe == Some(i))
                                && (o.fig.pos - p.fig.pos).length() < 8.0 * 256.0
                                && level.ground.line_of_sight(p.fig.pos + eye, o.fig.pos + eye)
                        })
                        .min_by(|a, b| {
                            let d = |o: &Person| (o.fig.pos - p.fig.pos).length();
                            d(a.1).total_cmp(&d(b.1))
                        })
                        .map(|(j, _)| j)
                }
                _ => player,
            };
            // Twenty blocks from the player, a bodyguard is out of harm's way.
            let away = p.ai == person::AI_BODYGUARD
                && darci.is_some_and(|d| (d - p.fig.pos).length() > 20.0 * 256.0);
            self.people[i].foe = foe;
            self.people[i].invulnerable = away;
        }
        for i in 0..self.people.len() {
            let c = if Some(i) == player {
                if self.outcome.is_some() || frozen { Controls::default() } else { input }
            } else {
                let mut rng = self.rng;
                let mut rand = || { rng = rng.wrapping_mul(69069).wrapping_add(1); rng >> 16 };
                let c = if frozen { Controls::default() } else { self.people[i].think(i, &snaps, &level.ground, dt, &mut rand) };
                self.rng = rng;
                c
            };
            self.people[i].step(i, &c, &snaps, &level.ground, &level.map, dt, &mut events);
        }
        self.apply(events, level);
        self.pick_up();
        for p in &mut self.people {
            p.advance(dt, &level.ground);
        }
        self.sync_grapples();
        self.separate(level);
        self.count_dead_cops();

        if let (Some(p), None) = (self.player, &self.outcome) {
            if self.people[p].knocked_out {
                let who = self.player_name();
                self.finish(Outcome::Lost(format!("{who} was knocked out")));
            }
        }
        if let Some(pl) = self.player() {
            let pos = pl.fig.pos;
            let fighting = matches!(pl.state, State::Attack { .. } | State::Grapple { .. } | State::Hurt)
                || self.people.iter().any(|p| p.aggro() && (p.fig.pos - pos).length() < 6.0 * 256.0);
            let sprinting = input.sprint && input.run && input.wish.length() > 0.5 && matches!(pl.state, State::Move);
            self.fight_timer = if fighting { 6.0 } else { (self.fight_timer - dt).max(0.0) };
            self.sprinting = sprinting;
        }
    }

    fn apply(&mut self, events: Vec<person::Event>, level: &Level) {
        use person::{Event, Grab};
        for e in events {
            // Whether the player has ever fired, hit or miss: the original raises
            // `GF_PLAYER_FIRED_GUN` where the shot leaves the gun, not where it lands.
            if let Event::Shot { attacker, .. } = &e {
                self.player_fired |= Some(*attacker) == self.player;
            }
            match e {
                Event::Hit { attacker, victim, mut damage, ko, high, low, weapon } => {
                    if Some(attacker) == self.player {
                        damage += damage * self.stats[1] as i32 / 20;
                    }
                    let from = self.people[attacker].fig.pos;
                    if let State::Held { .. } = self.people[victim].state {
                        // Only the grappler's finishing move lands on a held victim.
                        if ko != person::ko::GRAPPLE {
                            continue;
                        }
                        self.people[victim].state = State::Move;
                    }
                    let was_down = self.people[victim].is_down();
                    self.people[victim].take_hit(from, damage, ko, high, low, weapon);
                    if !was_down && self.people[victim].is_down() && Some(attacker) == self.player {
                        let kind = self.people[attacker].kind;
                        self.people[attacker].cues.push(Sfx::Taunt { kind });
                    }
                    self.drop_weapon(victim);
                }
                Event::Shot { attacker, victim: Some(victim), weapon } => {
                    let (a, v) = (&self.people[attacker], &self.people[victim]);
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    if !level.ground.line_of_sight(a.fig.pos + eye, v.fig.pos + eye) {
                        continue;
                    }
                    // The player's aim is good; the AI's gets better with skill and worse with
                    // distance.
                    let dist = (a.fig.pos - v.fig.pos).length();
                    let chance = if a.player {
                        (0.95 - dist / (40.0 * 256.0)).clamp(0.4, 0.95)
                    } else {
                        (0.35 + a.skill as f32 * 0.03 - dist / (24.0 * 256.0)).clamp(0.1, 0.85)
                    };
                    // Damage from `get_shot_damage` (fallen/Source/person.cpp): the shotgun falls
                    // off with distance, the AI's AK-47 is weaker, the player takes half.
                    let mut damage = match weapon {
                        Weapon::Pistol => 70,
                        Weapon::Ak47 if a.player => 100,
                        Weapon::Ak47 => 40,
                        Weapon::Shotgun => (250.0 * (1.0 - dist / (8.0 * 256.0))).max(30.0) as i32,
                        _ => 0,
                    };
                    if v.player {
                        damage /= 2;
                    }
                    let ko = if weapon == Weapon::Shotgun && damage >= 120 { person::ko::KNOCKDOWN } else { 0 };
                    let from = a.fig.pos;
                    if (self.rand() % 1000) as f32 / 1000.0 < chance {
                        let was_down = self.people[victim].is_down();
                        self.people[victim].take_hit(from, damage, ko, true, false, weapon);
                        if !was_down && self.people[victim].is_down() && Some(attacker) == self.player {
                            let kind = self.people[attacker].kind;
                            self.people[attacker].cues.push(Sfx::Taunt { kind });
                        }
                        self.drop_weapon(victim);
                    }
                }
                Event::Grab { attacker, victim } => {
                    let v = &mut self.people[victim];
                    v.state = State::Held { by: attacker };
                    v.fig.restart(Grab::Grab.anims().1, uc_world::Play::Once);
                }
                Event::GrabMove { victim, grab } => {
                    self.people[victim].fig.restart(grab.anims().1, uc_world::Play::Once);
                }
                Event::Release { victim } => {
                    let from = self.people[victim].fig.pos;
                    let v = &mut self.people[victim];
                    if matches!(v.state, State::Held { .. }) {
                        v.state = State::Move;
                        v.take_hit(from, 0, 0, false, false, Weapon::None);
                    }
                }
                Event::Arrest { attacker, victim } => {
                    let at = self.people[attacker].fig.pos;
                    let v = &mut self.people[victim];
                    v.state = State::Cuffed;
                    v.face_towards(at);
                    v.fig.restart(uc_world::character::anim::ARREST_BE_CUFFED, uc_world::Play::Once);
                }
                Event::Arrested { victim } => {
                    self.people[victim].arrested = true;
                    self.drop_weapon(victim);
                }
                Event::Punch { attacker } if Some(attacker) == self.player => {
                    if let Some(s) = &mut self.script {
                        s.note_punch();
                    }
                }
                Event::Kick { attacker } if Some(attacker) == self.player => {
                    if let Some(s) = &mut self.script {
                        s.note_kick();
                    }
                }
                _ => {}
            }
        }
    }

    /// Someone knocked out or arrested drops their weapon.
    fn drop_weapon(&mut self, i: usize) {
        let p = &mut self.people[i];
        if p.player || !(p.knocked_out || p.arrested) || p.weapon == Weapon::None {
            return;
        }
        if let Some(kind) = p.weapon.item() {
            let f = p.facing_dir();
            let pos = p.fig.pos + Vec3::new(f.x, 0.0, f.y) * 60.0;
            p.weapon = Weapon::None;
            self.items.push(Item { way: 0, kind, pos });
        }
    }

    /// A grappled victim stands in front of the attacker, facing him (`set_grapple_pos`,
    /// fallen/Source/combat.cpp: 90 units in front, turned 180 degrees).
    fn sync_grapples(&mut self) {
        for i in 0..self.people.len() {
            let State::Grapple { victim, .. } = self.people[i].state else {
                if let State::Held { by } = self.people[i].state {
                    if !matches!(self.people[by].state, State::Grapple { victim, .. } if victim == i) {
                        self.people[i].state = State::Move;
                    }
                }
                continue;
            };
            let (pos, angle, f) = (self.people[i].fig.pos, self.people[i].fig.angle, self.people[i].facing_dir());
            let v = &mut self.people[victim];
            if matches!(v.state, State::Held { .. }) {
                v.fig.pos = pos + Vec3::new(f.x, 0.0, f.y) * 90.0;
                v.fig.angle = angle + std::f32::consts::PI;
            }
        }
    }

    /// Pushes overlapping standing people apart.
    fn separate(&mut self, level: &Level) {
        const MIN: f32 = 64.0;
        let n = self.people.len();
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = (&self.people[i], &self.people[j]);
                let solid = |p: &Person| !p.removed && matches!(p.state, State::Move | State::Attack { .. } | State::Hurt | State::Land { .. });
                if !solid(a) || !solid(b) || (a.fig.pos.y - b.fig.pos.y).abs() > 100.0 {
                    continue;
                }
                let d = vec2(b.fig.pos.x - a.fig.pos.x, b.fig.pos.z - a.fig.pos.z);
                let len = d.length();
                if len >= MIN || len < 1e-3 {
                    continue;
                }
                let push = d / len * (MIN - len) * 0.5;
                for (k, s) in [(i, -1.0), (j, 1.0)] {
                    let p = &mut self.people[k].fig.pos;
                    let to = level.ground.slide(vec2(p.x, p.z), p.y, push * s, 36.0);
                    // Never shove anyone off an edge.
                    if (level.ground.surface(&level.map, to.x, to.y, p.y) - p.y).abs() <= uc_world::ground::STEP_HEIGHT {
                        p.x = to.x;
                        p.z = to.y;
                    }
                }
            }
        }
    }

    /// Navigation beacons: (label, position), hiding those whose target is down or gone.
    pub fn beacons(&self) -> Vec<(String, Vec3)> {
        let Some(s) = &self.script else { return Vec::new() };
        s.beacons()
            .into_iter()
            .filter_map(|b| match b.track {
                Some(way) => {
                    let p = &self.people[*self.by_way.get(&way)?];
                    (!p.removed && !p.is_down()).then_some((b.label, p.fig.pos))
                }
                None => Some((b.label, b.pos)),
            })
            .collect()
    }

    /// This frame's people meshes, cars, items and effects. `eye` is where the camera is
    /// (world units), which the puffs of an explosion turn to face.
    pub fn batches(&self, level: &Level, eye: Vec3) -> BTreeMap<u32, Batch> {
        let lighter = level.lighter();
        let mut out = BTreeMap::new();
        for p in self.people.iter().filter(|p| !p.removed && !matches!(p.state, State::InCar { .. })) {
            p.fig.append_batches(&mut out, lighter.as_ref());
            shadow(&mut out, p, level);
            // The weapon in hand, and a muzzle flash just after a shot.
            if let Some((held, part, flash)) = p.weapon.held().filter(|_| !p.weapon.is_gun() || p.gun_out()) {
                if let Some((rot, at)) = p.fig.part_world(part) {
                    let flash = flash.filter(|_| p.flash > 0.0);
                    for id in std::iter::once(held).chain(flash) {
                        if let Some(prim) = level.weapon_prims.get(&id) {
                            uc_world::level::prim_batches(prim, |v| at + rot * v, lighter.as_ref(), &mut out);
                        }
                    }
                }
            }
        }
        // Cars: body and wheels (front wheels turned by the steering), and a long shadow.
        for v in &self.vehicles {
            let (body, wheel, wx, fz, rz) = vehicle::TYPES[v.kind as usize];
            // The meshes are modelled nose towards -z: turn them round to face the way it drives.
            // They sit on the road as modelled, so the body goes down as it is.
            if let Some(prim) = level.item_prims.get(&body) {
                uc_world::level::prim_batches(prim, |p| v.place(Vec3::new(-p.x, p.y, -p.z)), lighter.as_ref(), &mut out);
            }
            if let Some(prim) = level.item_prims.get(&wheel) {
                let (ss, cs) = v.wheel_spin.sin_cos();
                // A wheel is modelled about its axle, so it rides on its own radius: the vans
                // and the jeep run on a bigger one than the cars do.
                let (lo, hi) = uc_world::level::prim_bounds(prim);
                let radius = (hi.y - lo.y) / 2.0;
                // Wheel positions mirrored like the body; the ones towards the nose steer.
                for (x, z, front) in [(wx, -rz, true), (-wx, -rz, true), (wx, -fz, false), (-wx, -fz, false)] {
                    let (st, ct) = if front { (v.steer * 0.5).sin_cos() } else { (0.0, 1.0) };
                    let place = |p: Vec3| {
                        // Spin about the axle (x), then steer about y.
                        let r = Vec3::new(p.x, cs * p.y - ss * p.z, ss * p.y + cs * p.z);
                        let r = Vec3::new(ct * r.x - st * r.z, r.y, st * r.x + ct * r.z);
                        v.place(r + Vec3::new(x, radius, z))
                    };
                    uc_world::level::prim_batches(prim, place, lighter.as_ref(), &mut out);
                }
            }
            let corner = |x: f32, z: f32| {
                let p = v.to_world(x, z);
                Vec3::new(p.x, level.ground.surface(&level.map, p.x, p.y, v.pos.y + 32.0) + 2.0, p.y) / uc_world::UNIT
            };
            out.entry(uc_world::SHADOW).or_default().quad(
                [corner(-170.0, -300.0), corner(170.0, -300.0), corner(-170.0, 310.0), corner(170.0, 310.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [[0, 0, 0, 170]; 4],
            );
        }
        // Barrels and cones: modelled with z up, they topple when knocked.
        for b in &self.barrels {
            let Some(prim) = level.item_prims.get(&b.prim()) else { continue };
            let lift = if b.kind == 1 { 28.0 } else { 51.0 };
            let (sy, cy) = b.yaw.sin_cos();
            let (st, ct) = b.tilt.sin_cos();
            uc_world::level::prim_batches(prim, |p| {
                // Model z is up; tip over about the model x axis.
                let up = Vec3::new(p.x, p.z + lift, -p.y);
                let t = Vec3::new(up.x, ct * up.y - st * up.z, st * up.y + ct * up.z);
                b.pos + Vec3::new(cy * t.x + sy * t.z, t.y, -sy * t.x + cy * t.z)
            }, lighter.as_ref(), &mut out);
        }
        // Items turn slowly where they lie (barrels stand still).
        let spin = self.time * 1.5;
        for it in &self.items {
            let Some(prim) = uc_world::level::item_prim(it.kind).and_then(|id| level.item_prims.get(&id)) else { continue };
            let (s, c) = if it.kind == 9 { (0.0, 1.0) } else { spin.sin_cos() };
            // Floating a little above the ground (some are modelled well above their origin).
            let lift = if it.kind == 9 { 0.0 } else { 24.0 - uc_world::level::prim_bounds(prim).0.y.max(0.0) };
            uc_world::level::prim_batches(prim, |v| it.pos + Vec3::new(c * v.x + s * v.z, v.y + lift, -s * v.x + c * v.z),
                                          lighter.as_ref(), &mut out);
        }
        // Ground mist: flat sheets blended over whatever is under them.
        let secs = self.time;
        self.mists.quads(secs, |p, uv| {
            out.entry(uc_world::MIST).or_default().quad(p, uv, [[255, 255, 255, 90]; 4]);
        });
        // Explosions and fire: round puffs turned to face the camera, the bright ones added
        // to the scene and the smoke blended over it.
        for puff in &self.pyro.puffs {
            let away = puff.pos - eye;
            let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * puff.radius();
            let up = right.cross(away).normalize_or(Vec3::Y) * puff.radius();
            let page = if puff.smoke { uc_world::SHADOW } else { uc_world::GLOW };
            let corner = |x: f32, y: f32| (puff.pos + right * x + up * y) / uc_world::UNIT;
            out.entry(page).or_default().quad(
                [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [puff.colour(); 4],
            );
        }
        out
    }

    /// Texture pages any person in this level might use.
    pub fn pages(level: &Level) -> Vec<u32> {
        let Some(people) = &level.people else { return Vec::new() };
        let mut out = Vec::new();
        for t in 0..=ptype::MIB3 {
            for v in 0..4 {
                if let Some(f) = people.figure(t, v, Vec3::ZERO, 0.0) {
                    out.extend(f.pages());
                }
            }
        }
        out.extend(level.item_prims.values().chain(level.weapon_prims.values()).flat_map(|p| p.faces.iter().map(|f| f.texture().0)));
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// A soft round shadow on the surface under a person, like the original's blob shadows:
/// fainter the higher they are above it, wider when lying down.
fn shadow(out: &mut BTreeMap<u32, Batch>, p: &Person, level: &Level) {
    // Under the body, which knockdowns carry away from the figure's origin.
    let pos = p.body_pos();
    let floor = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 8.0);
    let height = (pos.y - floor).max(0.0);
    let strength = (1.0 - height / 512.0).clamp(0.0, 1.0);
    if strength <= 0.0 {
        return;
    }
    let mut r = if p.is_down() { 110.0 } else { 80.0 } * (1.0 + height / 512.0);
    // Someone knocked out for good bleeds: the shadow turns into a spreading red pool.
    let blood = p.blood();
    if let Some(b) = blood {
        r *= 1.0 + 0.5 * b;
    }
    let corner = |dx: f32, dz: f32| {
        let (x, z) = (pos.x + dx * r, pos.z + dz * r);
        Vec3::new(x, level.ground.surface(&level.map, x, z, floor + 16.0) + 2.0, z) / uc_world::UNIT
    };
    let a = (200.0 * strength) as u8;
    out.entry(uc_world::SHADOW).or_default().quad(
        [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
        [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        [match blood {
            Some(b) => [(110.0 * b) as u8, 0, 0, a.max((200.0 * b) as u8)],
            None => [0, 0, 0, a],
        }; 4],
    );
}

/// The game with its level, for script conditions that need the walls.
struct Scene<'a> {
    game: &'a Game,
    level: &'a Level,
}

impl script::Query for Scene<'_> {
    fn player_pos(&self) -> Option<Vec3> {
        self.game.player_pos()
    }
    fn person(&self, way: usize) -> Option<script::PersonState> {
        self.game.person(way)
    }
    fn thing(&self, way: usize) -> Option<script::ThingState> {
        self.game.thing(way)
    }
    fn in_vehicle(&self, person: usize, vehicle: usize) -> bool {
        self.game.in_vehicle(person, vehicle)
    }
    fn player_grabbing(&self) -> bool {
        self.game.player_grabbing()
    }
    fn player_used(&self) -> bool {
        self.game.player_used()
    }
    fn got_item(&self, way: usize) -> bool {
        self.game.got_item(way)
    }
    fn player_fired(&self) -> bool {
        self.game.player_fired()
    }

    /// `can_a_see_b` (fallen/Source/Person.cpp): up to 8 squares, within about 74 degrees of
    /// straight ahead (much wider up close), with a clear line between their heads.
    fn sees(&self, a: usize, b: usize) -> bool {
        let g = self.game;
        let (Some(&a), Some(&b)) = (g.by_way.get(&a), g.by_way.get(&b)) else { return false };
        let (pa, pb) = (&g.people[a], &g.people[b]);
        if pa.removed || pb.removed || pa.is_dead() {
            return false;
        }
        let (from, to) = (pa.fig.pos, g.thing_pos(b));
        let d = Vec2::new(to.x - from.x, to.z - from.z);
        let dist = d.length();
        if dist > 8.0 * 256.0 {
            return false;
        }
        let ahead = pa.facing_dir().dot(d / dist.max(1.0));
        let cone = if dist < 192.0 { -0.54 } else { 0.28 };
        let head = Vec3::new(0.0, 96.0, 0.0);
        ahead > cone && self.level.ground.line_of_sight(from + head, to + head)
    }
}

impl script::Query for Game {
    fn player_pos(&self) -> Option<Vec3> {
        self.player().map(|p| p.fig.pos)
    }

    fn person(&self, way: usize) -> Option<script::PersonState> {
        if let Some(&v) = self.veh_by_way.get(&way) {
            let w = self.vehicles[v].wrecked();
            return Some(script::PersonState { dead: w, down: w, arrested: false, half_dead: self.vehicles[v].health < 150 });
        }
        let p = &self.people[*self.by_way.get(&way)?];
        Some(script::PersonState {
            // Someone a kill_waypoint took away isn't dead: the original just moves them off
            // the map (Driving Silver swaps its instructor this way).
            dead: p.is_dead() && !p.removed,
            down: p.is_down() || p.removed,
            // Being cuffed counts: otherwise "killed, not arrested" fires mid-arrest.
            arrested: p.arrested || matches!(p.state, State::Cuffed),
            half_dead: p.health * 2 < p.max_health,
        })
    }

    fn thing(&self, way: usize) -> Option<script::ThingState> {
        if let Some(&v) = self.veh_by_way.get(&way) {
            let v = &self.vehicles[v];
            return Some(script::ThingState { pos: v.pos, angle: v.script_angle(), speed: v.speed });
        }
        let p = &self.people[*self.by_way.get(&way)?];
        if let State::InCar { vehicle } = p.state {
            let v = &self.vehicles[vehicle];
            return Some(script::ThingState { pos: v.pos, angle: v.script_angle(), speed: v.speed });
        }
        Some(script::ThingState { pos: p.fig.pos, angle: p.fig.angle, speed: 0.0 })
    }

    fn in_vehicle(&self, person: usize, vehicle: usize) -> bool {
        let Some(&p) = self.by_way.get(&person) else { return false };
        match self.people[p].state {
            State::InCar { vehicle: v } => vehicle == 0 || self.vehicles[v].way == vehicle,
            _ => false,
        }
    }

    fn player_used(&self) -> bool {
        self.used
    }

    fn player_grabbing(&self) -> bool {
        self.player().is_some_and(|p| p.grabbing())
    }

    fn got_item(&self, way: usize) -> bool {
        self.taken.contains(&way)
    }

    fn player_fired(&self) -> bool {
        self.player_fired
    }
}
