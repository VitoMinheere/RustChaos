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

pub mod bike;
pub mod city;
mod game;
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
pub mod trip;
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
pub fn rand(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(69069).wrapping_add(1);
    *seed >> 16
}

/// A barrel, bin or traffic cone (`BT_*`): knocked about by cars, and blown up by a shot
/// unless it's a cone or a bin.
#[derive(Debug, Clone)]
pub struct Barrel {
    /// `BT_*` (a map's own barrels are 0, `BT_BARREL`).
    pub kind: u8,
    pub prim: u8,
    /// Goes up when shot (`BARREL_shoot`: anything but `BARREL_TYPE_CONE` and `_BIN`).
    pub explodes: bool,
    /// The `create_barrel` point that made it (0 for none).
    pub way: usize,
    /// Blown up (`BARREL_dissapear`): gone from the world, and the point that made it knows.
    pub gone: bool,
    pub pos: Vec3,
    pub vel: Vec2,
    pub yaw: f32,
    /// How far it has toppled over (0 upright .. pi/2 on its side).
    pub tilt: f32,
    /// The barrel it stands on (`bb->on`), if it was put down on top of another.
    pub on: Option<usize>,
    /// Falling speed, once the barrel under it has gone.
    pub vy: f32,
}

impl Barrel {
    /// A `BT_*` from a `create_barrel` point: its prim, and whether it explodes
    /// (`ELEV_load_level`, elev.cpp:1352). Oil drums are their own prim only in Semtex, and
    /// plain barrels everywhere else; LOX drums are always plain barrels.
    pub fn of_kind(kind: u8, semtex: bool) -> (u8, bool) {
        match kind {
            1 => (67, false),
            2 | 4 => (uc_world::level::BARREL_PROP, false),
            5 if semtex => (145, true),
            _ => (141, true),
        }
    }

    /// Its prim (barrels and cones are modelled with z up).
    pub fn prim(&self) -> u8 {
        self.prim
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
/// `MISSION_FLAG_SHOW_CRIMERATE` (fallen/Headers/Mission.h).
const MISSION_FLAG_SHOW_CRIMERATE: i32 = 1 << 1;
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
    Barrel(usize),
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
    /// The city's crime rate, 0..100 (`CRIME_RATE`): the mission's, or 50 if it has none
    /// (elev.cpp:309).
    pub crime_rate: u8,
    /// The mission shows the crime rate on the HUD (`MISSION_FLAG_SHOW_CRIMERATE`).
    pub show_crime_rate: bool,
    /// Deaths and arrests already counted towards the crime rate.
    crimes: BTreeSet<usize>,
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
    /// The mission's motorbikes, each with its figure.
    pub bikes: Vec<(bike::Bike, Option<Figure>)>,
    /// Seconds into the bikes' next turn.
    bike_clock: f32,
    /// People getting into cars: (who, which car).
    entering: Vec<(usize, usize)>,
    creature_by_way: HashMap<usize, usize>,
    pub fireballs: Vec<creature::Fireball>,
    pub sparks: Vec<Spark>,
    /// The creature the player's gun is pointed at, when it isn't a person.
    pub aim_creature: Option<usize>,
    /// The barrel the player's gun is pointed at (`find_target_new` takes barrels too).
    pub aim_barrel: Option<usize>,
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
    /// Leaves, cans and snow lying about round the player.
    pub dirt: dirt::Dirt,
    /// Seconds to the dirt's next turn.
    dirt_clock: f32,
    /// Where each person, then each car, was at the dirt's last turn (NaN for nowhere).
    dirt_marks: Vec<Vec3>,
    /// Steam from the mission's vents.
    pub steam: steam::Steam,
    /// The beams `tripwire` points watch.
    pub trips: trip::Trips,
    /// Buildings whose fences are electrified (`FACET_FLAG_ELECTRIFIED`).
    pub electrified: BTreeSet<u16>,
    /// Seconds to the next sparks off the electrified fences.
    fence_spark_clock: f32,
    /// There's no floor (`GF_NO_FLOOR`, Day Of Reckoning on top of Gunston Tower).
    pub no_floor: bool,
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
            crime_rate: if level.mission.crime_rate == 0 { 50 } else { level.mission.crime_rate.min(100) },
            show_crime_rate: level.mission.flags & MISSION_FLAG_SHOW_CRIMERATE != 0, crimes: BTreeSet::new(),
            searchable: BTreeSet::new(), hidden: Vec::new(), search: None, damaged: BTreeSet::new(), charges: Vec::new(),
            wares: ware::Wares::new(&level.map), indoors: None,
            platforms: level.platforms.iter().map(|&(way, ob)| {
                let o = &level.map.objects[ob];
                let yaw = o.yaw as f32 * std::f32::consts::TAU / 2048.0;
                plat::Platform::new(way, ob, o.prim, Vec3::new(o.x as f32, o.y as f32, o.z as f32), yaw)
            }).collect(),
            plat_prims: level.platforms.iter().map(|&(_, ob)| uc_world::level::load_prim(&level.game, level.map.objects[ob].prim)).collect(),
            plat_clock: 0.0,
            creatures: Vec::new(), bikes: Vec::new(), bike_clock: 0.0, entering: Vec::new(), creature_by_way: HashMap::new(), fireballs: Vec::new(), sparks: Vec::new(),
            aim_creature: None, aim_barrel: None, creature_clock: 0.0, creature_rng: 0xba7,
            pyro: Pyro::default(), mists: Mists::default(), drops: dirt::Drops::default(), dirt: Game::new_dirt(level), dirt_clock: 0.0, dirt_marks: Vec::new(), steam: steam::Steam::default(),
            trips: trip::Trips::new(level, &level.mission),
            electrified: BTreeSet::new(), fence_spark_clock: 0.0, no_floor: false,
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
        // Every map's barrel props become barrels (`OB_convert_dustbins_to_barrels`).
        for &ob in &level.barrel_props {
            let o = &level.map.objects[ob];
            let at = Vec3::new(o.x as f32, o.y as f32, o.z as f32);
            let y = level.ground.surface(&level.map, at.x, at.z, at.y + 32.0);
            let yaw = o.yaw as f32 * std::f32::consts::TAU / 2048.0;
            g.add_barrel(Barrel { kind: 0, prim: uc_world::level::BARREL_PROP, explodes: true, way: 0, gone: false,
                                  pos: Vec3::new(at.x, y, at.z), vel: Vec2::ZERO, yaw, tilt: 0.0, on: None, vy: 0.0 });
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
            p.max_stamina = 128.0 + stats[0] as f32;
            p.stamina = p.max_stamina;
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
        self.trip_wires(dt);
        self.electric_fences(dt, level);
        self.no_floor_falls();
        self.go_indoors(level);
        self.move_platforms(dt, level);
        self.run_creatures(dt, level, self.cutscene.as_ref().is_some_and(|c| c.shot.lock_player));
        self.run_bikes(dt, level);
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
        input.aim = self.aim_creature.map(|k| self.creatures[k].middle())
            .or_else(|| self.aim_barrel.map(|k| self.barrels[k].pos + Vec3::new(0.0, 50.0, 0.0)));
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

        let player = self.player;
        let mut snaps: Vec<_> = self.people.iter().map(|p| p.snap()).collect();
        // Nobody arrests their own bodyguards (`find_arrestee`): those guarding the player,
        // whose `other` point is the player's, or none at all.
        for (s, p) in snaps.iter_mut().zip(&self.people) {
            if p.ai == person::AI_BODYGUARD {
                let client = if p.other == 0 { player } else { self.by_way.get(&p.other).copied() };
                if client.is_some() && client == player {
                    s.arrestable = false;
                }
            }
        }
        let mut events = Vec::new();
        // Who each of the AI fights: an assassin (`PCOM_AI_ASSASIN`, `_SHOOT_DEAD`) goes for
        // the person its `ai_other` point created, until they're dead; everyone else for the
        // player. A fight between other people waits until Darci is near enough to see it
        // (12 squares from the mark), then carries on: the original starts at once, so the
        // crash cops in RTA had shot the drunk long before she got there.
        self.walk_goals();
        self.city_life(dt, level);
        self.board_cars(level);
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
        self.finish_entering();
        self.sync_grapples();
        self.sync_carrying(level);
        self.separate(level);
        self.count_dead_cops();
        self.tally_crime();
        self.blow_dirt(dt, level);

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
            // Really sprinting: out of breath she's only running, whatever is held.
            let sprinting = pl.sprinting;
            self.fight_timer = if fighting { 6.0 } else { (self.fight_timer - dt).max(0.0) };
            self.sprinting = sprinting;
        }
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
