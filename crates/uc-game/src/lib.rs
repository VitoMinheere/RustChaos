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

pub mod city;
pub mod creature;
pub mod dirt;
pub mod goals;
pub mod mist;
pub mod person;
pub mod plat;
pub mod pyro;
pub mod road;
pub mod script;
pub mod steam;
pub mod talk;
pub mod vehicle;
pub mod wand;
pub mod ware;

use glam::{vec2, Vec2, Vec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uc_world::character::person as ptype;
use uc_world::{Batch, Figure, Level};

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

/// The player going through a prop for whatever is hidden in it (`STATE_SEARCH`,
/// `SUB_STATE_SEARCH_PRIM`).
#[derive(Debug, Clone, Copy)]
pub struct Search {
    /// The prop, as an index into the map's objects.
    pub ob: usize,
    /// Seconds spent at it so far.
    pub time: f32,
}

impl Search {
    /// `fn_person_search` adds 400 a tick (20 a second) to a timer that releases what's hidden
    /// at 100 << 8 and lets go at 125 << 8, so the bar stays full for a moment at the end.
    pub const FOUND: f32 = (100 << 8) as f32 / (400.0 * 20.0);
    pub const DONE: f32 = (125 << 8) as f32 / (400.0 * 20.0);

    /// How far through: 0 to 1, as the panel's bar shows it.
    pub fn progress(&self) -> f32 {
        (self.time / Self::FOUND).clamp(0.0, 1.0)
    }
}

/// The prop nearest `pos` within `range`, the three axes summed (`OB_find_index`,
/// fallen/Source/ob.cpp), as an index into the map's objects. `only` limits it to those.
fn find_ob(level: &Level, pos: Vec3, range: f32, only: Option<&BTreeSet<usize>>) -> Option<usize> {
    level.map.objects.iter().enumerate()
        .filter(|(i, _)| only.is_none_or(|s| s.contains(i)))
        .map(|(i, o)| (i, (o.x as f32 - pos.x).abs() + (o.y as f32 - pos.y).abs() + (o.z as f32 - pos.z).abs()))
        .filter(|&(_, d)| d < range)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// `PCOM_AI_DRIVER`: finds a car, gets in and drives it as their move says.
const AI_DRIVER: u8 = 9;
/// How fast a driver goes down a clear road (`VEH_SPEED_LIMIT`, "35 mph"), world units a second.
const CRUISE: f32 = 1100.0;
/// A driver looks for a car this far away (`PCOM_set_person_ai_findcar`: 0xc00, x plus z).
const FIND_CAR: f32 = 3072.0;

/// Someone at the wheel who isn't the player (`pcom_move_state` and its arguments).
#[derive(Debug, Clone, Copy, Default)]
struct Driving {
    /// The road being driven down, from node and to node (`PCOM_MOVE_STATE_DRIVE_DOWN`).
    road: Option<(usize, usize)>,
    /// The road picked for after this one, once the junction is near (`InsideRoom`).
    next: usize,
    /// Seconds left of backing off something the car ran into (`PCOM_MOVE_SUBSTATE_3PTURN`).
    reversing: f32,
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
    /// The two people, as indices: the one with the lower number goes at the top.
    pair: Option<(usize, usize)>,
    /// Overheard rather than filmed (`EWAY_DO_AMBIENT_CONV`): no camera, no widescreen.
    ambient: bool,
    /// Seconds until a press may cut the line short (`EWAY_conv_skip`).
    skip: f32,
    /// Where the camera is and what it looks at.
    cam: Option<(Vec3, Vec3)>,
}

impl Convo {
    /// Filmed: the camera on whoever is talking, and the player held still.
    fn filmed(&self) -> bool {
        !self.ambient && self.pair.is_some()
    }
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

/// `IT_EXPLOSIVES`.
pub const EXPLOSIVES: u8 = 23;

/// `PRIM_DAMAGE_DAMAGABLE`, `PRIM_DAMAGE_EXPLODES` and `PRIM_FLAG_LAMPOST`
/// (fallen/Editor/Headers/Prim.h).
const DAMAGABLE: u8 = 1;
const EXPLODES: u8 = 2;
const LAMPOST: u8 = 1;

/// Whose blast it is: a creature isn't hurt by its own, and the Balrog goes after whoever
/// hurt him.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blame {
    Nobody,
    Person(usize),
    Creature(usize),
}

/// What the player's gun is pointed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Aim {
    Person(usize),
    Creature(usize),
}

/// A bolt of lightning from a creature to someone (`SPARK_create`): from, to, seconds left.
#[derive(Debug, Clone, Copy)]
pub struct Spark {
    pub from: Vec3,
    pub to: Vec3,
    pub left: f32,
}

/// A charge set down and ticking (`SPECIAL_set_explosives`).
#[derive(Debug, Clone, Copy)]
pub struct Charge {
    pub pos: Vec3,
    /// Seconds to go: `16 * 20 * 5`, counted down by `0x10` a tick at 20 ticks a second.
    pub fuse: f32,
}

impl Charge {
    pub const FUSE: f32 = (16 * 20 * 5) as f32 / (0x10 as f32 * 20.0);
}

/// The player's gun picks targets this close (world units).
const TARGET_RANGE: f32 = 14.0 * 256.0;

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    /// Seconds left on screen.
    pub left: f32,
    /// The speaker's face (`talk::face`), shown beside it; none for signs and tutorial text.
    pub face: Option<u8>,
    /// A line of a filmed conversation, which the widescreen panel shows ([`Wide`]) rather
    /// than the messages.
    pub filmed: bool,
}

/// A conversation being filmed, as the panel shows it in widescreen (`PANEL_new_widescreen`):
/// one of the two at the top with their face on the right, the other at the bottom with
/// theirs on the left, and the line being said beside whoever is saying it.
#[derive(Debug, Clone)]
pub struct Wide {
    pub top: Option<u8>,
    pub bottom: Option<u8>,
    pub text: String,
    pub top_talking: bool,
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
    /// A conversation being filmed, for the widescreen panel.
    pub wide: Option<Wide>,
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
    /// Props damaged (`OB_FLAG_DAMAGED`), as indices into the map's objects.
    pub damaged: BTreeSet<usize>,
    /// Explosives set down and waiting to go off.
    pub charges: Vec<Charge>,
    /// The mission's moving platforms, made or still waiting to be.
    pub platforms: Vec<plat::Platform>,
    /// Their prims, loaded once.
    plat_prims: Vec<Option<uc_formats::prm::Prim>>,
    plat_clock: f32,
    /// The map's warehouses, with the ambience the mission has given each.
    pub wares: ware::Wares,
    /// The warehouse the player is in, and the ambience it had as they came in.
    indoors: Option<(usize, u8)>,
    /// The mission's creatures (`CLASS_BAT`).
    pub creatures: Vec<creature::Creature>,
    creature_by_way: HashMap<usize, usize>,
    pub fireballs: Vec<creature::Fireball>,
    pub sparks: Vec<Spark>,
    /// The creature the player's gun is pointed at, when it isn't a person.
    pub aim_creature: Option<usize>,
    /// Seconds into the creatures' next tick.
    creature_clock: f32,
    creature_rng: u32,
    /// Props that can be searched (`OB_FLAG_SEARCHABLE`), as indices into the map's objects.
    pub searchable: BTreeSet<usize>,
    /// Items stashed in a prop, waiting to be found: (the prop, the item).
    pub hidden: Vec<(usize, Item)>,
    /// The player's search, while one is going on.
    pub search: Option<Search>,
    /// Explosions and fires burning.
    pub pyro: Pyro,
    /// Sheets of ground mist the mission has laid down.
    pub mists: Mists,
    /// Drops in the air.
    pub drops: dirt::Drops,
    /// Steam from the mission's vents.
    pub steam: steam::Steam,
    /// Estate of Emergency, where the tramp's face is the boss's (`estate`).
    estate: bool,
    /// Grim Gardens, where jumping onto a platform puts you on it (`playing_level`).
    botanicc: bool,
    /// The roads cars wander along.
    roads: road::Roads,
    /// What each person at the wheel is doing, by person.
    drivers: HashMap<usize, Driving>,
    /// The pavements the city's extra people walk.
    wand: wand::Wander,
    /// How far the player can see (world units): the city's extra people further away than
    /// this are out of sight, and may be moved. The original's draw distance by default; a
    /// renderer that sees further should say so.
    pub view_dist: f32,
    /// Chance for the city's extra people, kept apart from the mission's.
    city_rng: u32,
    /// What the extra people say when spoken to.
    chatter: city::Chatter,
    /// The player had a gun out last frame.
    gun_drawn: bool,
    /// What to call the player, for the HUD.
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
            items: Vec::new(), cutscene: None, convo: None, wide: None, used: false, vehicles: Vec::new(), veh_by_way: HashMap::new(),
            barrels: Vec::new(), sign: None, target: None, stats: [0; 4], badges: [0; 4], found: BTreeSet::new(),
            taken: BTreeSet::new(), dead_cops: BTreeSet::new(), player_fired: false,
            searchable: BTreeSet::new(), hidden: Vec::new(), search: None, damaged: BTreeSet::new(), charges: Vec::new(),
            wares: ware::Wares::new(&level.map), indoors: None,
            platforms: level.platforms.iter().map(|&(way, ob)| {
                let o = &level.map.objects[ob];
                let yaw = o.yaw as f32 * std::f32::consts::TAU / 2048.0;
                plat::Platform::new(way, ob, o.prim, Vec3::new(o.x as f32, o.y as f32, o.z as f32), yaw)
            }).collect(),
            plat_prims: level.platforms.iter().map(|&(_, ob)| uc_world::level::load_prim(&level.game, level.map.objects[ob].prim)).collect(),
            plat_clock: 0.0,
            creatures: Vec::new(), creature_by_way: HashMap::new(), fireballs: Vec::new(), sparks: Vec::new(),
            aim_creature: None, creature_clock: 0.0, creature_rng: 0xba7,
            pyro: Pyro::default(), mists: Mists::default(), drops: dirt::Drops::default(), steam: steam::Steam::default(),
            player_name: "",
            estate: level.mission.mission_name.eq_ignore_ascii_case("estate2"),
            botanicc: level.name.eq_ignore_ascii_case("botanicc"),
            roads: road::Roads::new(&level.map, &level.mission.map),
            drivers: HashMap::new(),
            wand: wand::Wander::new(&level.map, &level.game),
            view_dist: city::DRAW_DIST,
            city_rng: 0xc17,
            chatter: city::Chatter::load(level),
            gun_drawn: false,
        };
        if !scripted {
            g.spawn_player(level, level.spawn.0, level.spawn.1, 0, ptype::DARCI, 0);
        }
        g.sync_platforms(level);
        // Run the first tick now so the player exists straight away.
        g.run_script(0.0, level);
        if g.player.is_none() {
            g.spawn_player(level, level.spawn.0, level.spawn.1, 0, ptype::DARCI, 0);
        }
        g.populate(level);
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
    fn shockwave(&mut self, level: &Level, at: Vec3, radius: f32, damage: i32, by: Blame) {
        self.shock_props(level, at, radius, damage);
        // Creatures: a little more than people get, as far out as the edge, and four times
        // that for the Balrog, who is "so hard that we'd better hurt them more here".
        let mut acts = Vec::new();
        for (k, c) in self.creatures.iter_mut().enumerate() {
            let d = (c.fig.pos - at).abs();
            let dist = d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0;
            if dist >= radius || by == Blame::Creature(k) {
                continue;
            }
            let mut hit = (damage as f32 * (radius - dist + 0x80 as f32) / radius) as i32;
            if c.kind == creature::BALROG {
                hit <<= 2;
            }
            let who = match by {
                Blame::Person(p) => Some(p),
                _ => None,
            };
            c.hit(Some((at, who)), hit, &mut acts);
        }
        self.sounds.extend(acts.into_iter().filter_map(|a| match a {
            creature::Act::Sound { id, at } => Some((Sfx::ById { id, replace: false }, at)),
            _ => None,
        }));
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

    /// The props a shockwave catches: those that can be damaged, within `radius` summing the
    /// three axes, that take more than 50 of its `damage` (`create_shockwave`). The original
    /// steps through each map cell's props two at a time (`oi += 1` on top of the loop's own
    /// `oi++`), so only every other one is ever looked at; that is kept. Where a cell has an
    /// odd number it also runs on past the end of the list into whatever was left in the
    /// buffer from before, which isn't.
    fn shock_props(&mut self, level: &Level, at: Vec3, radius: f32, damage: i32) {
        let cell = |v: f32| (v as i32) >> 10;
        let (lo, hi) = ((cell(at.x - radius), cell(at.z - radius)), (cell(at.x + radius), cell(at.z + radius)));
        let mut hit = Vec::new();
        let mut last = None;
        let mut nth = 0;
        for (i, o) in level.map.objects.iter().enumerate() {
            let c = (o.x >> 10, o.z >> 10);
            if last != Some(c) {
                (last, nth) = (Some(c), 0);
            }
            nth += 1;
            if nth % 2 == 0 || c.0 < lo.0 || c.0 > hi.0 || c.1 < lo.1 || c.1 > hi.1 {
                continue;
            }
            if level.ground.prop_damage.get(i).is_none_or(|d| d.0 & DAMAGABLE == 0) {
                continue;
            }
            let dist = (o.x as f32 - at.x).abs() + (o.y as f32 - at.y).abs() + (o.z as f32 - at.z).abs();
            if dist < radius && damage as f32 * (radius - dist) / radius > 50.0 {
                hit.push(i);
            }
        }
        for i in hit {
            self.damage_prop(level, i);
        }
    }

    /// `OB_damage`: the prop is marked damaged, and one that explodes (or a lamppost) goes up
    /// in a firebomb the first time, with a shockwave of its own.
    fn damage_prop(&mut self, level: &Level, i: usize) {
        let (damage, flag) = level.ground.prop_damage.get(i).copied().unwrap_or_default();
        if damage & DAMAGABLE == 0 {
            return;
        }
        // Marked before the blast, so the shockwave it sets off doesn't set it off again.
        let first = self.damaged.insert(i);
        if first && (damage & EXPLODES != 0 || flag & LAMPOST != 0) {
            let o = &level.map.objects[i];
            let at = Vec3::new(o.x as f32, o.y as f32, o.z as f32);
            self.pyro.burst(at, pyro::FIREBOMB, 256.0);
            self.sounds.push((Sfx::Explosion, at));
            self.shockwave(level, at, 0x300 as f32, 150, Blame::Nobody);
        }
    }

    /// Whether the prop a `prim_damaged` point at `pos` watches has been damaged. It watches
    /// the nearest prop in its own map cell, damaged or not (`EWAY_create_cond`).
    fn prim_damaged(&self, level: &Level, pos: Vec3) -> bool {
        let cell = ((pos.x as i32) >> 10, (pos.z as i32) >> 10);
        level.map.objects.iter().enumerate()
            .filter(|(_, o)| (o.x >> 10, o.z >> 10) == cell)
            .map(|(i, o)| (i, (o.x as f32 - pos.x).abs() + (o.y as f32 - pos.y).abs() + (o.z as f32 - pos.z).abs()))
            .filter(|&(_, d)| d < 0x7800 as f32)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .is_some_and(|(i, _)| self.damaged.contains(&i))
    }

    /// Charges burn down and go off: a flash, dust, streamers and a firebomb, and a shockwave
    /// five squares across that does 500 at the middle (`SPECIAL_EXPLOSIVES` in
    /// `special_normal`).
    fn charges(&mut self, dt: f32, level: &Level) {
        for c in &mut self.charges {
            c.fuse -= dt;
        }
        let (gone, left): (Vec<Charge>, Vec<Charge>) = self.charges.iter().partition(|c| c.fuse <= 0.0);
        self.charges = left;
        for c in gone {
            let at = c.pos + Vec3::new(0.0, 16.0, 0.0);
            self.pyro.burst(at, pyro::FLASH | pyro::DUST | pyro::STREAMERS | pyro::FIREBOMB, 0xa0 as f32);
            self.sounds.push((Sfx::Explosion, at));
            let by = self.player.map_or(Blame::Nobody, Blame::Person);
            self.shockwave(level, at, 0x500 as f32, 500, by);
        }
    }

    /// The creatures go about their business on the original's own 20 ticks a second, and
    /// their fireballs fly.
    fn run_creatures(&mut self, dt: f32, level: &Level, frozen: bool) {
        if self.creatures.is_empty() && self.fireballs.is_empty() {
            return;
        }
        for s in &mut self.sparks {
            s.left -= dt;
        }
        self.sparks.retain(|s| s.left > 0.0);
        // The Balrog burns as he goes (`PYRO_IMMOLATE` on him).
        for c in self.creatures.iter().filter(|c| c.kind == creature::BALROG && !c.dead) {
            for _ in 0..2 {
                let r = |rng: &mut u32| rand(rng) as f32 / 32768.0 - 1.0;
                let (dx, dz, up) = (r(&mut self.creature_rng), r(&mut self.creature_rng), r(&mut self.creature_rng));
                let at = c.fig.pos + Vec3::new(dx * 90.0, 160.0 + up * 150.0, dz * 90.0);
                self.pyro.lick(at, 50.0);
            }
        }
        self.creature_clock += dt;
        while self.creature_clock >= creature::TICK {
            self.creature_clock -= creature::TICK;
            self.creature_tick(level, frozen);
        }
    }

    fn creature_tick(&mut self, level: &Level, frozen: bool) {
        let seen: Vec<creature::Seen> = self.people.iter().enumerate().map(|(i, p)| {
            let pos = self.thing_pos(i);
            let hips = p.fig.hips();
            creature::Seen {
                pos,
                head: Vec3::new(hips.x, pos.y + 2.0 * (hips.y - pos.y).max(40.0), hips.z),
                alive: !p.removed && !p.is_dead() && !p.is_down(),
                dead: p.is_dead() || p.removed,
                player: p.player,
                ai: p.ai,
                up_high: pos.y - level.map.ground_height(pos.x, pos.z) > 128.0,
            }
        }).collect();
        let floor = |x: f32, z: f32| level.ground.surface(&level.map, x, z, 0x7fff as f32);
        let terrain = |x: f32, z: f32| level.map.ground_height(x, z);
        let clear = |a: Vec3, b: Vec3| level.ground.line_of_sight(a, b);
        let slide = |at: Vec2, y: f32, by: Vec2| level.ground.slide(at, y + 64.0, by, 48.0);
        let around = creature::Around {
            people: &seen, player: self.player, floor: &floor, terrain: &terrain, clear: &clear, slide: &slide, frozen,
        };
        let mut acts = Vec::new();
        let mut rng = self.creature_rng;
        for (k, c) in self.creatures.iter_mut().enumerate() {
            let before = acts.len();
            c.tick(k, &around, &mut rng, &mut acts);
            // Bolts start from the creature: tag them with where it is.
            for a in &mut acts[before..] {
                if let creature::Act::Spark { .. } = a {
                    self.sparks.push(Spark { from: c.middle() + Vec3::new(0.0, 40.0, 0.0), to: Vec3::ZERO, left: 0.0 });
                }
            }
        }
        self.creature_rng = rng;
        // Fireballs fall and burst on whatever they meet.
        let mut bursts = Vec::new();
        for f in &mut self.fireballs {
            f.vel.y -= creature::Fireball::GRAVITY;
            f.pos += f.vel;
            f.age += creature::TICK;
            if f.pos.y < level.ground.surface(&level.map, f.pos.x, f.pos.z, f.pos.y + 64.0) {
                bursts.push(f.pos);
                f.age = creature::Fireball::LIFE;
            }
        }
        self.fireballs.retain(|f| f.age < creature::Fireball::LIFE);
        for at in bursts {
            // `PYRO_WHOOMPH`, and a small shockwave that blames nobody.
            self.pyro.burst(at, pyro::FIREBALL | pyro::DUST, 0x140 as f32);
            self.sounds.push((Sfx::Explosion, at));
            self.shockwave(level, at, 0x140 as f32, 80, Blame::Nobody);
        }
        let mut spark = self.sparks.len() - acts.iter().filter(|a| matches!(a, creature::Act::Spark { .. })).count();
        for a in acts {
            match a {
                creature::Act::Shockwave { at, radius, damage, from } => self.shockwave(level, at, radius, damage, Blame::Creature(from)),
                creature::Act::Fireball { at, vel } => self.fireballs.push(creature::Fireball { pos: at, vel, age: 0.0 }),
                creature::Act::Hurt { person, damage, recoil, from } => self.hurt_directly(person, damage, recoil, from),
                creature::Act::Sound { id, at } => self.sounds.push((Sfx::ById { id, replace: false }, at)),
                creature::Act::Float { person } => {
                    let p = &mut self.people[person];
                    if !p.removed && !matches!(p.state, State::InCar { .. }) {
                        let floor = level.ground.surface(&level.map, p.fig.pos.x, p.fig.pos.z, p.fig.pos.y + 32.0);
                        p.state = State::Float { floor };
                    }
                }
                creature::Act::Spark { person, head } => {
                    let p = &self.people[person];
                    let hips = p.fig.hips();
                    let to = if head { Vec3::new(hips.x, p.fig.pos.y + 2.0 * (hips.y - p.fig.pos.y), hips.z) } else { hips };
                    let s = &mut self.sparks[spark];
                    (s.to, s.left) = (to, if head { 0.4 } else { 1.0 });
                    spark += 1;
                }
                creature::Act::Dust { at } => self.pyro.burst(at, pyro::DUST, 300.0),
                creature::Act::BalrogKill => {
                    if let Some(s) = &mut self.script {
                        s.note_balrog_kill();
                    }
                }
            }
        }
    }

    /// Damage straight off someone's health with a cry of pain, as a gargoyle's swoop and
    /// Bane's bolt do: it kills them when there's none left, and otherwise maybe sets them
    /// reeling (`set_person_recoil`).
    fn hurt_directly(&mut self, i: usize, damage: i32, recoil: bool, from: Vec3) {
        let p = &mut self.people[i];
        if p.removed || p.knocked_out || p.invulnerable || matches!(p.state, State::InCar { .. }) {
            return;
        }
        p.health -= damage;
        if p.health <= 0 {
            if p.is_down() {
                p.health = 0;
                p.knocked_out = true;
                p.cues.push(Sfx::KnockedOut { kind: p.kind });
            } else {
                p.take_hit(from, 0, person::ko::FINISHERS, false, false, Weapon::None);
            }
        } else if recoil && matches!(p.state, State::Move | State::Block | State::Attack { .. } | State::Shoot { .. }) {
            p.face_towards(from);
            p.take_hit(from, 0, 0, false, false, Weapon::None);
        } else {
            p.cues.push(Sfx::Hurt { kind: p.kind });
        }
    }

    /// Platforms move on the original's ticks, carrying whoever stands on them and killing
    /// whoever they come down on; then the ground learns where their tops are.
    fn move_platforms(&mut self, dt: f32, level: &Level) {
        if self.platforms.is_empty() {
            return;
        }
        let tops: Vec<Option<(Vec2, Vec2, f32, f32)>> = self.plat_prims.iter()
            .map(|prim| prim.as_ref().map(|prim| plat::top_box(&prim.points)))
            .collect();
        self.plat_clock += dt;
        while self.plat_clock >= plat::TICK {
            self.plat_clock -= plat::TICK;
            let Some(script) = self.script.take() else { break };
            for (k, t) in tops.iter().enumerate() {
                let Some((_, _, bot, top)) = *t else { continue };
                let before = self.platforms[k].pos;
                // Who is standing on it, before it goes.
                let riders: Vec<usize> = self.people.iter().enumerate()
                    .filter(|(_, p)| !p.removed && !matches!(p.state, State::InCar { .. }))
                    .filter(|(_, p)| (p.fig.pos.y - (before.y + top)).abs() < 16.0 && level.ground.over_platform(k, p.fig.pos.x, p.fig.pos.z, 0.0))
                    .map(|(i, _)| i)
                    .collect();
                let by = self.platforms[k].tick(&Route(&script));
                if by == Vec3::ZERO {
                    continue;
                }
                for &i in &riders {
                    self.people[i].fig.pos += by;
                }
                if self.platforms[k].rocket() && self.rand() & 1 == 0 {
                    self.pyro.lick(self.platforms[k].pos + Vec3::new(0.0, bot, 0.0), 70.0);
                }
                // Underneath it as it comes down: dead (`set_person_dead`).
                let under = (self.platforms[k].pos.y + bot - 0x80 as f32, self.platforms[k].pos.y + bot - 0x40 as f32);
                for i in 0..self.people.len() {
                    let p = &self.people[i];
                    if p.removed || p.is_dead() || riders.contains(&i) || !level.ground.over_platform(k, p.fig.pos.x, p.fig.pos.z, 10.0) {
                        continue;
                    }
                    let y = p.fig.pos.y;
                    if self.botanicc && matches!(p.state, State::Fall { .. }) && y >= before.y + bot && y <= before.y + top {
                        // Grim Gardens: jumping onto it puts you on it.
                        self.people[i].fig.pos.y = self.platforms[k].pos.y + top;
                    } else if by.y < 0.0 && y >= under.0 && y <= under.1 {
                        let from = self.platforms[k].pos;
                        let p = &mut self.people[i];
                        p.health = 0;
                        p.take_hit(from, 10_000, person::ko::FINISHERS, false, false, Weapon::None);
                    }
                }
            }
            self.script = Some(script);
            self.sync_platforms(level);
        }
    }

    /// Tells the ground where the platforms' tops are now.
    fn sync_platforms(&self, level: &Level) {
        level.ground.set_platforms(self.platforms.iter().zip(&self.plat_prims).filter_map(|(p, prim)| {
            let (lo, hi, _, top) = plat::top_box(&prim.as_ref()?.points);
            Some((Vec2::new(p.pos.x, p.pos.z), p.yaw, lo, hi, p.pos.y + top))
        }));
    }

    /// Whether the player has gone into a warehouse or come out of one. The ambience is the
    /// one it had as they came in, as `process_weather` picks it on the way in.
    fn go_indoors(&mut self, level: &Level) {
        let Some(p) = self.player() else { return };
        let at = p.fig.pos;
        let ware = self.wares.which_contains(&level.map, (at.x as i32) >> 8, (at.z as i32) >> 8).filter(|_| {
            // Under its roof, not on it.
            let roof = level.ground.surface(&level.map, at.x, at.z, 1.0e5);
            roof - at.y > 128.0
        });
        self.indoors = match (ware, self.indoors) {
            (Some(w), Some((was, amb))) if w == was => Some((was, amb)),
            (Some(w), _) => Some((w, self.wares.wares[w].ambience)),
            (None, _) => None,
        };
    }

    /// While the player is inside a warehouse, its ambience (0 for none): the city goes
    /// quiet, and any ambience it has plays instead of the music.
    pub fn indoors(&self) -> Option<u8> {
        self.indoors.map(|(_, amb)| amb)
    }

    /// The vents blow, and their steam scalds whoever stands in it: no stagger, just the
    /// health and a cry of pain, until it's too much (`PFLAG_HURTPEOPLE`). Nobody in a car is
    /// touched, nor anyone only the player can hurt (`PCOM_BENT_PLAYERKILL`).
    fn vent_steam(&mut self, dt: f32, level: &Level) {
        let Some(script) = &self.script else { return };
        let (vents, secs) = (script.vents(), script.time());
        let scald = self.steam.update(dt, secs, &vents, |at| level.ground.surface(&level.map, at.x, at.z, at.y + 8.0));
        if !scald || self.outcome.is_some() {
            return;
        }
        for p in &mut self.people {
            if p.removed || p.knocked_out || p.invulnerable || p.bent & 0x80 != 0 || matches!(p.state, State::InCar { .. }) {
                continue;
            }
            // From the feet to the top of the head.
            let height = 2.0 * (p.fig.hips().y - p.fig.pos.y).max(60.0);
            let hurt = self.steam.scalding(p.fig.pos, height);
            if hurt == 0 {
                continue;
            }
            p.health -= hurt;
            if p.health > 0 {
                p.cues.push(Sfx::Hurt { kind: p.kind });
            } else if p.is_down() {
                p.health = 0;
                p.knocked_out = true;
                p.cues.push(Sfx::KnockedOut { kind: p.kind });
            } else {
                let from = p.fig.pos;
                p.take_hit(from, 0, person::ko::FINISHERS, false, false, Weapon::None);
            }
        }
    }

    fn spawn(&mut self, level: &Level, kind: u8, player: bool, way: usize, pos: Vec3, angle: f32) -> Option<usize> {
        let variant = self.rand();
        self.spawn_variant(level, kind, player, way, pos, angle, variant)
    }

    /// [`Game::spawn`], with the look picked by `variant`.
    #[allow(clippy::too_many_arguments)]
    fn spawn_variant(&mut self, level: &Level, kind: u8, player: bool, way: usize, pos: Vec3, angle: f32, variant: u32) -> Option<usize> {
        let people = level.people.as_ref()?;
        // Points can sit a little under the surface they're on (The Jump's jumper is 64 below
        // his ledge): look from half a storey up.
        let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 128.0);
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
            // A driver out of a car walks to the door of the nearest one free.
            if p.ai == AI_DRIVER {
                if let (true, Some(vi)) = (matches!(p.state, State::Move), self.car_for(i)) {
                    let door = self.vehicles[vi].door();
                    goals[i] = (Some(Vec3::new(door.x, at[i].y, door.y)), person::Pace::Walk);
                }
                continue;
            }
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

    /// The car a driver out of one goes for (`PCOM_set_person_ai_findcar`): the nearest that
    /// nobody is driving, isn't wrecked and isn't locked, measured x plus z.
    fn car_for(&self, i: usize) -> Option<usize> {
        let at = self.people[i].fig.pos;
        self.vehicles.iter().enumerate()
            .filter(|(_, v)| v.driver.is_none() && !v.wrecked() && !v.locked)
            .map(|(k, v)| (k, (v.pos.x - at.x).abs() + (v.pos.z - at.z).abs()))
            .filter(|&(_, d)| d < FIND_CAR)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k)
    }

    /// Drivers who have walked up to their car's door get in (`PCOM_process_findcar`).
    fn board_cars(&mut self) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.ai != AI_DRIVER || p.player || p.removed || p.is_dead() || p.is_down() || p.aggro() || !matches!(p.state, State::Move) {
                continue;
            }
            let Some(vi) = self.car_for(i) else { continue };
            let door = self.vehicles[vi].door();
            if Vec2::new(p.fig.pos.x - door.x, p.fig.pos.z - door.y).length() < 150.0 {
                self.vehicles[vi].driver = Some(i);
                let p = &mut self.people[i];
                p.state = State::InCar { vehicle: vi };
                p.cues.push(Sfx::CarDoor);
                self.drivers.insert(i, Driving::default());
            }
        }
    }

    /// The controls of a car someone other than the player is driving. Wandering
    /// (`PCOM_MOVE_WANDER`) they drive the roads, picking a way at random at each junction
    /// (`PCOM_process_driving_wander`) and keeping to their lane (`DriveCar`, both in
    /// fallen/Source/pcom.cpp); otherwise they pull up and stay put
    /// (`PCOM_process_driving_still`).
    fn npc_drive(&mut self, vi: usize, pi: usize, dt: f32) -> Drive {
        const TURN: f32 = 896.0; // `LEFT_TURN_DIST`: how near the junction before turning
        let v = &self.vehicles[vi];
        let (at, speed, yaw, y) = (Vec2::new(v.pos.x, v.pos.z), v.speed, v.yaw, v.pos.y);
        let stop = if speed.abs() > 30.0 { Drive { throttle: -speed.signum(), ..Default::default() } } else { Drive::default() };
        let (mv, bent) = (self.people[pi].mv, self.people[pi].bent);
        let mut st = self.drivers.get(&pi).copied().unwrap_or_default();
        if mv != 4 || self.roads.nodes.len() < 2 {
            st.road = None;
            self.drivers.insert(pi, st);
            return stop;
        }
        // Onto the nearest road, going the way of the lane it's in.
        let (n1, n2) = match st.road {
            Some(r) => r,
            None => {
                let Some((mut a, mut b)) = self.roads.find(at) else { return stop };
                if self.roads.signed_dist(a, b, at) < 0.0 {
                    std::mem::swap(&mut a, &mut b);
                }
                st.next = 0;
                (a, b)
            }
        };
        // Near the junction: pick the next road, and turn into it (a left turn a little later).
        let mut road = (n1, n2);
        let to_end = creature::qdist2(self.roads.dest(n1, n2) - at);
        if to_end < TURN {
            if st.next == 0 {
                let mut rng = self.rng;
                st.next = self.roads.whereto_now(n1, n2, &mut || rand(&mut rng));
                self.rng = rng;
            }
            let late = if st.next != 0 && self.roads.bend(n1, n2, st.next) < 0 { 256.0 } else { 0.0 };
            if st.next != 0 && to_end + late < TURN {
                road = (n2, st.next);
                st.next = 0;
            }
        } else {
            st.next = 0;
        }
        st.road = Some(road);
        // Head for the far end of the road, or back into the lane first if well out of it.
        let (n1, n2) = road;
        let mut dest = self.roads.dest(n1, n2);
        let reversing = st.reversing > 0.0;
        if !reversing && (384.0 - self.roads.signed_dist(n1, n2, at)).abs() > 128.0 {
            let (r1, r2) = (self.roads.pos(n1), self.roads.pos(n2));
            let ab = r2 - r1;
            let t = ((at - r1).dot(ab) / ab.length_squared().max(1.0)).clamp(0.0, 1.0);
            let sign = |v: f32| if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
            let dr = Vec2::new(sign(ab.x), sign(ab.y)) * 256.0;
            dest = r1 + ab * t + dr * 2.0 + Vec2::new(-dr.y, dr.x);
        }
        let to = dest - at;
        let diff = (to.x.atan2(to.y) - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        // Slow for a bend, down to a third of the limit; diligent drivers go twice as fast.
        let dangle = diff.abs() * 2048.0 / std::f32::consts::TAU;
        let mut want = if dangle > 100.0 { (1.0 - (dangle - 100.0) / 2.0 / 750.0).max(250.0 / 750.0) } else { 1.0 } * CRUISE;
        if bent & 2 != 0 {
            want *= 2.0;
        }
        // (The car turns away from its steering: steer left for a larger yaw.)
        let mut steer = (-diff * 2.0).clamp(-1.0, 1.0);
        if reversing {
            st.reversing -= dt;
            want = -CRUISE * 0.4;
            steer = -steer;
        }
        // Stop for anyone in the road ahead (`PCOM_find_runover_thing`).
        let fwd = Vec2::new(yaw.sin(), yaw.cos());
        let side = Vec2::new(fwd.y, -fwd.x);
        let blocked = self.people.iter().enumerate().any(|(j, o)| {
            let r = Vec2::new(o.fig.pos.x - at.x, o.fig.pos.z - at.y);
            j != pi && !o.removed && !matches!(o.state, State::InCar { .. }) && (o.fig.pos.y - y).abs() < 150.0
                && r.dot(fwd) > 0.0 && r.dot(fwd) < 250.0 + speed.max(0.0) * 0.5 && r.dot(side).abs() < 170.0
        });
        self.drivers.insert(pi, st);
        if blocked && want > 0.0 {
            return stop;
        }
        Drive { throttle: Vehicle::throttle_for(want), steer, handbrake: false }
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
                    if let Some(k) = self.creature_by_way.remove(&way) {
                        self.creatures[k].state = creature::State::Dead;
                        self.creatures[k].fig.pos.y -= 1.0e5;
                    }
                    self.items.retain(|it| it.way != way);
                    self.hidden.retain(|(_, it)| it.way != way);
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
                        self.shockwave(level, at, 768.0, 350, Blame::Nobody);
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
                    let hidden = self.hidden.iter_mut().map(|(_, it)| it);
                    for it in self.items.iter_mut().chain(hidden).filter(|it| it.way == way) {
                        it.pos = Vec3::new(pos.x, y, pos.z);
                    }
                }
                Command::Item { way, kind, pos, stashed } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    let item = Item { way, kind, pos: Vec3::new(pos.x, y, pos.z) };
                    // A stashed item goes into the nearest prop within eight squares, which
                    // becomes searchable; with none that near it is left lying in the open
                    // (`EWAY_create_item`).
                    match find_ob(level, pos, 2048.0, None).filter(|_| stashed) {
                        Some(ob) => {
                            self.searchable.insert(ob);
                            self.hidden.push((ob, item));
                        }
                        None => self.items.push(item),
                    }
                }
                Command::Creature { way, kind, pos, angle } => {
                    let Some(chunk) = creature_chunk(level, kind) else {
                        eprintln!("mission: no anim prim for creature {kind}");
                        continue;
                    };
                    if let Some(fig) = Figure::anim_prim(chunk, pos, angle, 1) {
                        let floor = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 128.0);
                        self.creatures.push(creature::Creature::new(way, kind, fig, pos, angle, floor));
                        self.creature_by_way.insert(way, self.creatures.len() - 1);
                    }
                }
                Command::WareFx { pos, ambience } => {
                    match self.wares.which_contains(&level.map, (pos.x as i32) >> 8, (pos.z as i32) >> 8) {
                        Some(w) => self.wares.wares[w].ambience = ambience,
                        // The original says so too, in its debug output.
                        None => eprintln!("mission: warehouse ambience point at {:.0},{:.0} is not inside a warehouse", pos.x, pos.z),
                    }
                }
                Command::Pee { way, on } => {
                    if let Some(&i) = self.by_way.get(&way) {
                        self.people[i].peeing = on;
                    }
                }
                Command::Searchable { pos } => {
                    if let Some(ob) = find_ob(level, pos, 256.0, None) {
                        self.searchable.insert(ob);
                    }
                }
                Command::Camera { way, shot } => {
                    let _ = way;
                    self.cutscene = Some(Cutscene { pos: shot.start, shot, leg: 0, wait: 0.0 });
                }
                Command::Conversation { way, speech, lines, a, b, ambient } => {
                    // A conversation still going is cut short but counts as said, as in the
                    // original (`EWAY_finish_conversation`): what waits on it still happens.
                    if let (Some(old), Some(s)) = (self.convo.take(), self.script.as_mut()) {
                        s.conversation_over(old.way);
                    }
                    self.wide = None;
                    // It's filmed only if both of them are there and fit to talk
                    // (`person_ok_for_conversation`); otherwise it's just said.
                    let who = |w: usize| self.by_way.get(&w).copied().filter(|&i| self.can_talk(i));
                    let pair = who(a).zip(who(b)).filter(|(i, j)| i != j);
                    if let (Some((i, j)), false) = (pair, ambient) {
                        // `PCOM_make_people_talk_to_eachother`: they turn to each other.
                        let (pi, pj) = (self.people[i].fig.pos, self.people[j].fig.pos);
                        self.people[i].face_towards(pj);
                        self.people[j].face_towards(pi);
                    }
                    self.convo = Some(Convo { way, speech, lines, line: usize::MAX, left: 0.0, pair, ambient, skip: 1.0, cam: None });
                }
                Command::Message { way, speech, text, secs, speaker } => {
                    let _ = way;
                    let face = match speaker {
                        script::Speaker::Street | script::Speaker::Tutorial => None,
                        script::Speaker::Radio => Some(talk::face::RADIO),
                        script::Speaker::Person(w) => match self.by_way.get(&w).copied() {
                            // Dead people can't talk.
                            Some(i) if self.people[i].is_dead() => continue,
                            Some(i) => {
                                // Someone the player is talking to turns to them.
                                if let (Some(p), true) = (self.player, self.can_talk(i)) {
                                    let at = self.people[p].fig.pos;
                                    self.people[i].face_towards(at);
                                }
                                Some(self.face(i))
                            }
                            None => Some(talk::face::RADIO),
                        },
                    };
                    let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                    self.sounds.push((Sfx::Voice(speech), at));
                    self.messages.retain(|m| m.text != text);
                    self.messages.push(Message { text, left: secs, face, filmed: false });
                }
                Command::Transfer { way } => self.transfer(way, level),
                Command::Platform { way, colour, group, speed, flags } => {
                    if let Some(p) = self.platforms.iter_mut().find(|p| p.way == way) {
                        p.start(colour, group, speed, flags);
                    }
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

    /// A filmed conversation is going on: the player is held still until it's over.
    pub fn in_conversation(&self) -> bool {
        self.convo.as_ref().is_some_and(|c| c.filmed())
    }

    /// The fly-by camera, or a filmed conversation's, if one is running: (position, looking
    /// at), world units.
    pub fn camera(&self) -> Option<(Vec3, Vec3)> {
        self.cutscene.as_ref().map(|c| (c.pos, self.look_at(c.shot.target)))
            .or_else(|| self.convo.as_ref().filter(|c| c.filmed()).and_then(|c| c.cam))
    }

    /// Where a fly-by's target is now (`EWAY_process_camera`, fallen/Source/eway.cpp).
    fn look_at(&self, look: script::Look) -> Vec3 {
        use script::{Look, Query};
        match look {
            Look::Place(at) => at,
            // A person's chest rather than their feet.
            Look::Thing { way, at } => self.thing(way).map_or(at, |t| t.pos + Vec3::new(0.0, 160.0, 0.0)),
            Look::Near(at) => self.people.iter()
                .filter(|p| !p.removed && !p.is_dead() && (p.fig.pos - at).length() < 1280.0)
                .min_by(|a, b| (a.fig.pos - at).length().total_cmp(&(b.fig.pos - at).length()))
                .map_or(at, |p| p.fig.pos),
        }
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

    /// What to call the player.
    pub fn player_name(&self) -> &'static str {
        self.player_name
    }

    /// Fly-bys and conversations move on.
    /// `person_ok_for_conversation`: on their feet, not driving, climbing or in the air.
    fn can_talk(&self, i: usize) -> bool {
        let p = &self.people[i];
        !p.removed && !p.is_dead() && !p.is_down()
            && !matches!(p.state, State::InCar { .. } | State::Climb { .. } | State::Fall { .. } | State::Zip { .. } | State::Fence { .. } | State::PullUp { .. })
    }

    /// Person `i`'s face (`PANEL_new_face`).
    fn face(&self, i: usize) -> u8 {
        let p = &self.people[i];
        talk::face_of(p.kind, p.fig.model.person.saturating_sub(6), self.estate)
    }

    /// `EWAY_DO_TRANSFER_PLAYER`: the player becomes someone else. Whoever they were stays
    /// behind as a civilian who minds their own business, fights back if set on and can't be
    /// hurt; the new one loses any protection and is the camera's from now on.
    fn transfer(&mut self, way: usize, level: &Level) {
        // The original says nothing when there's nobody (its message is commented out).
        let Some(&new) = self.by_way.get(&way) else { return };
        if let Some(old) = self.player.filter(|&o| o != new) {
            let p = &mut self.people[old];
            p.player = false;
            (p.ai, p.bent, p.skill) = (1, person::BENT_ROBOT | person::BENT_FIGHT_BACK, 15);
            (p.mv, p.goal) = (1, None);
            p.protected = true;
            if matches!(p.state, State::Attack { .. } | State::Block | State::Shoot { .. }) {
                p.state = State::Move;
            }
        }
        let p = &mut self.people[new];
        p.player = true;
        (p.ai, p.skill) = (0, 0);
        p.bent &= !person::BENT_PLAYERKILL;
        p.protected = false;
        (p.goal, p.foe) = (None, None);
        self.player = Some(new);
        self.player_name = player_name(&level.mission.mission_name, p.kind);
        (self.target, self.aim_creature) = (None, None);
    }

    fn update_scenes(&mut self, dt: f32, level: &Level, pressed: bool) {
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
            // Anything happening to either of them breaks it off; it counts as said.
            if let (Some((i, j)), true) = (cv.pair, cv.filmed()) {
                let busy = |k: usize| !matches!(self.people[k].state, State::Move) || !self.can_talk(k);
                if busy(i) || busy(j) {
                    self.wide = None;
                    if let Some(s) = &mut self.script {
                        s.conversation_over(cv.way);
                    }
                    return;
                }
            }
            cv.left -= dt;
            cv.skip -= dt;
            // A press cuts the line short, once the line has been up for half a second (a
            // second for the first); a press before that means the next one does.
            if pressed && cv.filmed() {
                if cv.skip <= 0.0 {
                    cv.left = 0.0;
                } else {
                    cv.skip = 0.0;
                }
            }
            if cv.left <= 0.0 {
                cv.line = cv.line.wrapping_add(1);
                match cv.lines.get(cv.line).cloned() {
                    Some((who, text)) => {
                        // Two seconds, and a twenty-fifth for every letter (`EWAY_conv_timer`).
                        cv.left = 2.0 + text.chars().count() as f32 * 0.04;
                        cv.skip = 0.5;
                        let speaker = if who == 0 { self.player } else { self.by_way.get(&who).copied() };
                        let face = speaker.map_or(talk::face::RADIO, |i| self.face(i));
                        let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                        self.sounds.push((Sfx::ConversationLine(cv.speech, cv.line), at));
                        match (cv.pair, cv.filmed()) {
                            (Some((i, j)), true) => {
                                let (top, bottom) = (i.min(j), i.max(j));
                                let top_talking = speaker == Some(top);
                                self.messages.push(Message { text: text.clone(), left: cv.left, face: Some(face), filmed: true });
                                self.wide = Some(Wide { top: Some(self.face(top)), bottom: Some(self.face(bottom)), text, top_talking });
                                // The camera on whoever has the line, over the other one.
                                let (s, l) = if speaker == Some(j) { (j, i) } else { (i, j) };
                                let (sp, sa, lp) = (self.people[s].fig.pos, self.people[s].fig.angle, self.people[l].fig.pos);
                                let mut rng = self.rng;
                                let mut rand = || rand(&mut rng);
                                let eye = Vec3::new(0.0, 0.0, 0.0);
                                cv.cam = Some(talk::converse_camera(sp, sa, lp, |a, b| level.ground.line_of_sight(a + eye, b), &mut rand));
                                self.rng = rng;
                            }
                            _ => self.messages.push(Message { text, left: cv.left, face: Some(face), filmed: false }),
                        }
                    }
                    None => {
                        self.wide = None;
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
                // Explosives in hand: put one down at her feet with its fuse burning
                // (`SPECIAL_set_explosives`). The last one leaves her hands empty.
                let p = &mut self.people[pi];
                if p.weapon == Weapon::Explosives && p.explosives > 0 {
                    p.explosives -= 1;
                    if p.explosives == 0 {
                        p.weapon = Weapon::None;
                    }
                    self.charges.push(Charge { pos, fuse: Charge::FUSE });
                    self.messages.push(Message { text: "Five second fuse set".into(), left: 3.0, face: None, filmed: false });
                    return true;
                }
                let near = |v: &Vehicle| !v.wrecked() && !v.locked && v.driver.is_none()
                    && v.covers(Vec2::new(pos.x, pos.z), 140.0) && (v.pos.y - pos.y).abs() < 150.0;
                if let Some(vi) = self.vehicles.iter().position(near) {
                    self.vehicles[vi].driver = Some(pi);
                    let p = &mut self.people[pi];
                    p.state = State::InCar { vehicle: vi };
                    p.cues.push(Sfx::CarDoor);
                    return true;
                }
                // Search a prop in front of her (`set_person_search`: the nearest searchable
                // one within a square, no more than an eighth of a turn off her facing).
                if let Some(ob) = find_ob(level, pos, 256.0, Some(&self.searchable)) {
                    let o = &level.map.objects[ob];
                    let at = Vec3::new(o.x as f32, pos.y, o.z as f32);
                    let to = Vec2::new(at.x - pos.x, at.z - pos.z).normalize_or_zero();
                    let p = &mut self.people[pi];
                    if to == Vec2::ZERO || p.facing_dir().dot(to) > std::f32::consts::FRAC_1_SQRT_2 {
                        if to != Vec2::ZERO {
                            p.face_towards(at);
                        }
                        self.search = Some(Search { ob, time: 0.0 });
                        return true;
                    }
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
                self.chat(pi)
            }
            _ => false,
        }
    }

    /// Cars move; the player's car follows her controls. Cars knock people over and push
    /// cones and barrels around.
    fn drive(&mut self, dt: f32, input: &Controls, level: &Level) {
        let player = self.player;
        // Anyone else at the wheel decides first. A driver who has gone (a kill_waypoint)
        // leaves the car where it is.
        let mut npc = vec![None; self.vehicles.len()];
        for (vi, d) in npc.iter_mut().enumerate() {
            if let Some(pi) = self.vehicles[vi].driver.filter(|&pi| Some(pi) != player) {
                if self.people[pi].removed {
                    self.vehicles[vi].driver = None;
                } else {
                    *d = Some((pi, self.npc_drive(vi, pi, dt)));
                }
            }
        }
        for (vi, npc) in npc.into_iter().enumerate() {
            let v = &mut self.vehicles[vi];
            let d = if v.driver.is_some() && v.driver == player { input.drive } else { npc.map_or(Drive::default(), |(_, d)| d) };
            let sliding = v.slip.abs() > 300.0;
            if let Some(speed) = v.update(d, dt, &level.ground, &level.map) {
                if speed > 300.0 {
                    self.sounds.push((Sfx::Crash, v.pos));
                }
                // Ran into something: back off and come round (`PCOM_MOVE_SUBSTATE_3PTURN`).
                if let Some(dr) = npc.and_then(|(pi, _)| self.drivers.get_mut(&pi)) {
                    if dr.reversing <= 0.0 {
                        dr.reversing = 1.5;
                    }
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
            self.aim_creature = None;
            return;
        }
        let (pos, f) = (p.fig.pos, p.facing_dir());
        let eye = Vec3::new(0.0, 140.0, 0.0);
        let seen = |at: Vec3, aim: Aim| {
            let d = Vec2::new(at.x - pos.x, at.z - pos.z);
            let dist = d.length();
            (dist < TARGET_RANGE && (at.y - pos.y).abs() < 4.0 * 256.0 && level.ground.line_of_sight(pos + eye, at + eye))
                .then(|| (aim, dist, 1.0 - f.dot(d / dist.max(1.0))))
        };
        // (who, how far, how far round from straight ahead), nearest first. Creatures can be
        // shot as well as people (`BAT_apply_hit` from `shoot_get_ammo_sound_and_damage`).
        let people = self.people.iter().enumerate()
            .filter(|&(i, q)| i != pi && !q.removed && !q.is_dead() && !q.is_down() && (q.hostile() || q.aggro()))
            .filter_map(|(i, q)| seen(q.fig.pos, Aim::Person(i)));
        let creatures = self.creatures.iter().enumerate()
            .filter(|(_, c)| c.targetable())
            .filter_map(|(k, c)| seen(c.fig.pos, Aim::Creature(k)));
        let mut options: Vec<(Aim, f32, f32)> = people.chain(creatures).collect();
        options.sort_by(|a, b| a.1.total_cmp(&b.1));
        let current = match (self.target, self.aim_creature) {
            (Some(t), _) => Some(Aim::Person(t)),
            (None, Some(k)) => Some(Aim::Creature(k)),
            _ => None,
        };
        let kept = current.filter(|t| options.iter().any(|o| o.0 == *t));
        let aim = if cycle && !options.is_empty() {
            let at = kept.and_then(|t| options.iter().position(|o| o.0 == t));
            Some(options[at.map_or(0, |k| (k + 1) % options.len())].0)
        } else if kept.is_some() {
            kept
        } else {
            // The nearest, favouring those ahead (someone right behind counts as 3x as far).
            options.iter().min_by(|a, b| (a.1 * (1.0 + a.2)).total_cmp(&(b.1 * (1.0 + b.2)))).map(|o| o.0)
        };
        (self.target, self.aim_creature) = match aim {
            Some(Aim::Person(t)) => (Some(t), None),
            Some(Aim::Creature(k)) => (None, Some(k)),
            None => (None, None),
        };
    }

    /// The search goes on while action is held (`continue_pressing_action`); let go and it's
    /// abandoned. Part way through, whatever is stashed in the prop drops out where it was
    /// put, and the prop is empty from then on, though still searchable.
    fn searching(&mut self, dt: f32, held: bool) {
        let Some(mut s) = self.search.take() else { return };
        let standing = self.player.is_some_and(|p| matches!(self.people[p].state, State::Move));
        if !held || !standing || self.outcome.is_some() {
            return;
        }
        let before = s.time;
        s.time += dt;
        if before < Search::FOUND && s.time >= Search::FOUND {
            let (found, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.hidden).into_iter().partition(|(ob, _)| *ob == s.ob);
            self.hidden = kept;
            if !found.is_empty() {
                let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                self.sounds.push((Sfx::Found, at));
            }
            self.items.extend(found.into_iter().map(|(_, it)| it));
        }
        if s.time < Search::DONE {
            self.search = Some(s);
        }
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
                self.messages.push(Message { text: format!("{} increased", STAT_NAMES[stat]), left: 4.0, face: None, filmed: false });
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
            // crowbar, a gas mask, a wrench, a video, gloves, weedkiller. They do nothing in
            // the hand; what matters is that she has them, which a mission reads with
            // `specific_item_held` (The Fallen's five clues are picked up this way).
            1 | 13..=21 => true,
            // Explosives are carried, four at most, and left lying once she has four
            // (`should_person_get_item`).
            EXPLOSIVES if p.explosives < 4 => {
                p.explosives += 1;
                true
            }
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
        let pressed = input.punch || input.kick || input.jump || input.action;
        self.update_scenes(dt, level, pressed);
        self.pyro.update(dt);
        if self.outcome.is_none() {
            self.charges(dt, level);
        }
        self.vent_steam(dt, level);
        self.go_indoors(level);
        self.move_platforms(dt, level);
        self.run_creatures(dt, level, self.cutscene.as_ref().is_some_and(|c| c.shot.lock_player));
        // From just under the pelvis, for as long as they're on their feet (the original asks
        // only that they're peeing, but its pelvis is never anywhere sensible once they're down).
        let streams: Vec<(Vec3, Vec2)> = self.people.iter()
            .filter(|p| p.peeing && !p.removed && !p.is_down() && !p.is_dead())
            .map(|p| (p.fig.hips() - Vec3::new(0.0, 16.0, 0.0), p.facing_dir()))
            .collect();
        self.drops.update(dt, &streams, |at| level.ground.surface(&level.map, at.x, at.z, at.y + 8.0));
        let frozen = self.cutscene.as_ref().is_some_and(|c| c.shot.lock_player);
        // A filmed conversation holds the player still too (`EWAY_cam_freeze`), but not
        // everyone else as a fly-by does.
        let held = frozen || self.convo.as_ref().is_some_and(|c| c.filmed());

        if let Some((_, t)) = &mut self.sign {
            *t -= dt;
            if *t <= 0.0 {
                self.sign = None;
            }
        }
        // Getting in and out of cars and talking to people take the action key first.
        let mut input = *input;
        input.target = self.target;
        input.aim = self.aim_creature.map(|k| self.creatures[k].middle());
        if self.outcome.is_none() && !held && input.action && self.use_something(level) {
            input.action = false;
        }
        // Fly-bys that hold the player still hold the car too (it rolls to a stop).
        if held || self.outcome.is_some() {
            input.drive = Drive::default();
        }
        self.drive(dt, &input, level);
        // Action held: the viewer passes it on as `sprint`.
        self.searching(dt, input.sprint);

        let snaps: Vec<_> = self.people.iter().map(|p| p.snap()).collect();
        let mut events = Vec::new();
        let player = self.player;
        // Who each of the AI fights: an assassin (`PCOM_AI_ASSASIN`, `_SHOOT_DEAD`) goes for
        // the person its `ai_other` point created, until they're dead; everyone else for the
        // player. A fight between other people waits until Darci is near enough to see it
        // (12 squares from the mark), then carries on: the original starts at once, so the
        // crash cops in RTA had shot the drunk long before she got there.
        self.walk_goals();
        self.city_life(dt, level);
        self.board_cars();
        // `PCOM_AI_SUICIDE`, "the easiest AI! A random death" (fallen/Source/pcom.cpp): down
        // as if shot with a pistol the first time it runs. The Fallen's bootlegger is made this
        // way on the roof once Harney has spoken, and the first clue waits on his death.
        for p in self.people.iter_mut().filter(|p| p.ai == 15 && !p.player && !p.is_dead() && !p.is_down()) {
            let from = p.fig.pos + Vec3::new(0.0, 0.0, 1.0);
            p.take_hit(from, 10_000, person::ko::KNOCKDOWN, false, false, Weapon::Pistol);
        }
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
            self.people[i].invulnerable = away || self.people[i].protected;
        }
        for i in 0..self.people.len() {
            let c = if Some(i) == player {
                if self.outcome.is_some() || held || self.search.is_some() { Controls::default() } else { input }
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
                self.gunshot(*attacker);
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
                Event::Shot { attacker, victim: None, weapon } if Some(attacker) == self.player && self.aim_creature.is_some() => {
                    let k = self.aim_creature.unwrap();
                    let (from, at) = (self.people[attacker].fig.pos, self.creatures[k].middle());
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    if !level.ground.line_of_sight(from + eye, at) {
                        continue;
                    }
                    let dist = (at - from).length();
                    let chance = (0.95 - dist / (40.0 * 256.0)).clamp(0.4, 0.95);
                    let damage = match weapon {
                        Weapon::Pistol => 70,
                        Weapon::Ak47 => 100,
                        Weapon::Shotgun => (250.0 * (1.0 - dist / (8.0 * 256.0))).max(30.0) as i32,
                        _ => 0,
                    };
                    if (self.rand() % 1000) as f32 / 1000.0 < chance {
                        let mut acts = Vec::new();
                        self.creatures[k].hit(Some((from, Some(attacker))), damage, &mut acts);
                        for a in acts {
                            if let creature::Act::Sound { id, at } = a {
                                self.sounds.push((Sfx::ById { id, replace: false }, at));
                            }
                        }
                    }
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
                Some(way) if self.creature_by_way.contains_key(&way) => {
                    let c = &self.creatures[self.creature_by_way[&way]];
                    (!c.dead).then_some((b.label, c.fig.pos))
                }
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
            // A wheel is modelled about its axle, so it rides on its own radius: the vans and
            // the jeep run on a bigger one than the cars do.
            let wheel_prim = level.item_prims.get(&wheel);
            let radius = wheel_prim.map_or(38.0, |prim| {
                let (lo, hi) = uc_world::level::prim_bounds(prim);
                (hi.y - lo.y) / 2.0
            });
            // The meshes are modelled nose towards -z: turn them round to face the way it drives.
            // The body sits so the wheels are where the original has them in it.
            let lift = radius - vehicle::WHEEL_ABOVE_BODY;
            if let Some(prim) = level.item_prims.get(&body) {
                uc_world::level::prim_batches(prim, |p| v.place(Vec3::new(-p.x, p.y + lift, -p.z)), lighter.as_ref(), &mut out);
            }
            if let Some(prim) = wheel_prim {
                let (ss, cs) = v.wheel_spin.sin_cos();
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
        // Creatures, their fireballs and their lightning.
        for c in self.creatures.iter().filter(|c| c.fig.pos.y > -1.0e4) {
            c.fig.append_batches(&mut out, lighter.as_ref());
        }
        let billboard = |out: &mut BTreeMap<u32, Batch>, page: u32, at: Vec3, r: f32, colour: [u8; 4]| {
            let away = at - eye;
            let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * r;
            let up = right.cross(away).normalize_or(Vec3::Y) * r;
            let corner = |x: f32, y: f32| (at + right * x + up * y) / uc_world::UNIT;
            out.entry(page).or_default().quad(
                [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [colour; 4],
            );
        };
        for f in &self.fireballs {
            billboard(&mut out, uc_world::GLOW, f.pos, 70.0, [255, 200, 110, 255]);
            billboard(&mut out, uc_world::GLOW, f.pos - f.vel * 2.0, 50.0, [255, 120, 40, 160]);
        }
        for s in &self.sparks {
            // A jagged line of bright points, jumping about from frame to frame.
            let steps = ((s.to - s.from).length() / 24.0).clamp(4.0, 64.0) as usize;
            let mut seed = (self.time * 60.0) as u32 ^ 0x9e37;
            for k in 0..=steps {
                let t = k as f32 / steps as f32;
                let jag = |seed: &mut u32| (rand(seed) as f32 / 32768.0 - 1.0) * 14.0 * (1.0 - (2.0 * t - 1.0).powi(2));
                let at = s.from.lerp(s.to, t) + Vec3::new(jag(&mut seed), jag(&mut seed), jag(&mut seed));
                billboard(&mut out, uc_world::GLOW, at, 9.0, [170, 190, 255, 255]);
            }
        }
        // Platforms, wherever they've got to.
        for (p, prim) in self.platforms.iter().zip(&self.plat_prims) {
            if let Some(prim) = prim {
                let (s, c) = p.yaw.sin_cos();
                uc_world::level::prim_batches(prim, |v| p.pos + Vec3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z), lighter.as_ref(), &mut out);
            }
        }
        // Charges sit still where they were put.
        if let Some(prim) = uc_world::level::item_prim(EXPLOSIVES).and_then(|id| level.item_prims.get(&id)) {
            let lift = 4.0 - uc_world::level::prim_bounds(prim).0.y;
            for c in &self.charges {
                uc_world::level::prim_batches(prim, |v| c.pos + Vec3::new(v.x, v.y + lift, v.z), lighter.as_ref(), &mut out);
            }
        }
        // Ground mist: flat sheets blended over whatever is under them.
        let secs = self.time;
        self.mists.quads(secs, |p, uv| {
            out.entry(uc_world::MIST).or_default().quad(p, uv, [[255, 255, 255, 90]; 4]);
        });
        // Drops: small streaks along their way, added like the original's droplet page, in
        // `DIRT_TYPE_URINE`'s colour.
        for d in &self.drops.drops {
            let along = d.vel.normalize_or(Vec3::NEG_Y) * 6.0;
            let across = (d.pos - eye).cross(along).normalize_or(Vec3::X) * 1.5;
            let corner = |a: f32, b: f32| (d.pos + along * a + across * b) / uc_world::UNIT;
            out.entry(uc_world::GLOW).or_default().quad(
                [corner(-1.0, -1.0), corner(-1.0, 1.0), corner(1.0, -1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [[0x77, 0x55, 0x33, 255]; 4],
            );
        }
        // Steam: soft pale puffs blended over the scene, turned to face the camera. The
        // original draws each with a frame of a wispy cloud (`POLY_PAGE_SMOKECLOUD2`), which
        // covers far less than a solid blob does, so they're thinned to match.
        for puff in &self.steam.puffs {
            let [r, g, b, a] = puff.colour();
            let colour = [r, g, b, (a as f32 * 0.35) as u8];
            let away = puff.pos - eye;
            let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * puff.radius();
            let up = right.cross(away).normalize_or(Vec3::Y) * puff.radius();
            let corner = |x: f32, y: f32| (puff.pos + right * x + up * y) / uc_world::UNIT;
            out.entry(uc_world::SHADOW).or_default().quad(
                [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [colour; 4],
            );
        }
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
        // The platforms' props, which the level leaves to the game.
        for &(_, ob) in &level.platforms {
            if let Some(prim) = uc_world::level::load_prim(&level.game, level.map.objects[ob].prim) {
                out.extend(prim.faces.iter().map(|f| f.texture().0));
            }
        }
        // And any creature the mission makes.
        let kinds: BTreeSet<u8> = level.mission.event_points.iter()
            .filter(|e| e.used != 0 && e.kind == uc_formats::ucm::waypoint::CREATE_CREATURE)
            .map(|e| e.data[0] as u8)
            .collect();
        for kind in kinds {
            if let Some(f) = creature_chunk(level, kind).and_then(|c| Figure::anim_prim(c, Vec3::ZERO, 0.0, 1)) {
                out.extend(f.pages());
            }
        }
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

/// A creature's anim prim, `anim%03d.all` (`load_anim_prim_object`), if there is one. The bat
/// has none, being compiled out.
fn creature_chunk(level: &Level, kind: u8) -> Option<std::sync::Arc<uc_formats::all::AnimChunk>> {
    if !matches!(kind, creature::GARGOYLE | creature::BALROG | creature::BANE) {
        return None;
    }
    let path = uc_formats::resolve(&level.game, &format!("data\\anim{kind:03}.all"))?;
    let chunk = uc_formats::all::AnimChunk::parse(&std::fs::read(path).ok()?).ok()?;
    Some(std::sync::Arc::new(chunk))
}

/// The creature with its health showing: the one in the player's sights, or else the
/// nearest still standing within twelve squares. (name, health 0..1)
impl Game {
    pub fn creature_health(&self) -> Option<(&'static str, f32)> {
        let me = self.player()?.fig.pos;
        let c = self.aim_creature.map(|k| &self.creatures[k]).or_else(|| {
            self.creatures.iter()
                .filter(|c| c.targetable() && (c.fig.pos - me).length() < 12.0 * 256.0)
                .min_by(|a, b| (a.fig.pos - me).length().total_cmp(&(b.fig.pos - me).length()))
        })?;
        Some((c.name(), c.health.max(0) as f32 / c.max_health() as f32))
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

/// The script as a platform's route.
struct Route<'a>(&'a Script);

impl plat::Route for Route<'_> {
    fn nearest(&self, at: Vec3, colour: u8, group: u8) -> Option<usize> {
        self.0.nearest_point(at, colour, group)
    }
    fn next(&self, after: usize, colour: u8, group: u8) -> Option<usize> {
        self.0.next_point(after, colour, group)
    }
    fn position(&self, way: usize) -> Option<Vec3> {
        self.0.point_pos(way)
    }
    fn delay(&self, way: usize, default: f32) -> f32 {
        self.0.point_delay(way, default)
    }
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
    fn prim_damaged(&self, pos: Vec3) -> bool {
        self.game.prim_damaged(self.level, pos)
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
        if let Some(&k) = self.creature_by_way.get(&way) {
            let c = &self.creatures[k];
            return Some(script::PersonState { dead: c.dead, down: c.dead, arrested: false, half_dead: c.health * 2 < c.max_health() });
        }
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
        if let Some(&k) = self.creature_by_way.get(&way) {
            let c = &self.creatures[k];
            return Some(script::ThingState { pos: c.fig.pos, angle: c.fig.angle, speed: 0.0 });
        }
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
