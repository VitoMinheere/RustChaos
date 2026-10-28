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
pub mod chopper;
pub mod barrel;
pub mod canid;
pub mod city;
pub mod cloth;
pub mod game;
pub use game::{Aiming, Show, Sight, Snipe};
pub mod creature;
pub mod dike;
pub mod dirt;
pub mod goals;
pub mod lights;
pub mod gunfx;
pub mod hm;
pub mod mist;
pub mod person;
pub mod plat;
pub mod puddle;
pub mod smap;
pub mod steps;
pub mod pow;
pub mod pyro;
pub mod ribbon;
pub mod road;
pub mod script;
pub mod sm;
pub mod steam;
pub mod switch;
pub mod talk;
pub mod tracks;
pub mod trip;
pub mod vehicle;
pub mod wand;
pub mod ware;

use uc_formats::units::radians;
use glam::{vec2, Vec2, Vec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uc_world::character::person as ptype;
use uc_world::{Batch, Figure, Level};

pub use mist::Mists;
pub use person::{Controls, Person, Sfx, State, Weapon};
pub use pyro::Pyro;
pub use script::Shot;
pub use vehicle::{Drive, Vehicle};
pub use game::mines::Mine;
pub use game::grenades::Grenade;

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
    /// Its base.
    pub pos: Vec3,
    pub yaw: f32,
    /// How far it has toppled over (0 upright, pi/2 on its side, pi upside down).
    pub tilt: f32,
    /// The barrel it stands on (`bb->on`), if it was put down on top of another.
    pub on: Option<usize>,
    /// While it's moving, the two spheres it is (`BARREL_Sphere`, [`barrel`]).
    pub spheres: Option<[barrel::Sphere; 2]>,
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
/// `SUB_STATE_SEARCH_PRIM`), or through someone's pockets (`SUB_STATE_SEARCH_CORPSE`).
#[derive(Debug, Clone, Copy)]
pub struct Search {
    pub what: Searched,
    /// Seconds spent at it so far.
    pub time: f32,
}

/// What's being searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Searched {
    /// A prop, as an index into the map's objects.
    Prop(usize),
    /// Someone dead, out cold or arrested, as an index into the people.
    Body(usize),
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
/// How far (squares) a bodyguard looks out for whoever is going for the person they guard,
/// and goes for something to arm themselves with. The original's sphere is `0x800`, eight;
/// half again keeps them in it as a fight moves about.
pub(crate) const GUARDS: f32 = 12.0;

/// Someone at the wheel who isn't the player (`pcom_move_state` and its arguments).
#[derive(Debug, Clone, Copy, Default)]
struct Driving {
    /// The road being driven down, from node and to node (`PCOM_MOVE_STATE_DRIVE_DOWN`).
    road: Option<(usize, usize)>,
    /// The road picked for after this one, once the junction is near (`InsideRoom`).
    next: usize,
    /// Seconds left of backing off something the car ran into (`PCOM_MOVE_SUBSTATE_3PTURN`).
    reversing: f32,
    /// Beat Cop's drunk driver (#130): weaving across the road, and stopping for nobody.
    drunk: bool,
    /// Beat Cop's getaway driver: twice the limit, and stopping for nobody.
    reckless: bool,
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

/// What the music should be doing (`MUSIC_MODE_*`, fallen/Headers/music.h; chosen by
/// `context_music`, fallen/Source/Controls.cpp:1372).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Calm,
    /// At the wheel (`MUSIC_MODE_DRIVING`).
    Drive,
    /// Crawling (`MUSIC_MODE_CRAWLING`).
    Crawl,
    Fight,
    Sprint,
    /// A mission with a theme of its own for all of it (`MUSIC_bodge_code`,
    /// `FRONTEND_diddle_music`): the combat, assault and driving training and Day Of
    /// Reckoning.
    Theme(Theme),
    /// A visible countdown under 30 seconds (`MUSIC_MODE_TIMER`): a tune played once.
    Timer,
    Won,
    Lost,
    /// Lost with the player still standing (`MUSIC_MODE_CHAOS`, Game.cpp:2377).
    Chaos,
}

/// The missions that have a tune of their own throughout (`MUSIC_bodge_code`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    CombatTraining,
    AssaultTraining,
    DrivingTraining,
    Reckoning,
}

impl Theme {
    /// `FRONTEND_diddle_music` (fallen/Source/frontend.cpp:7375): by the mission's file name.
    pub fn of(mission: &str) -> Option<Theme> {
        let m = mission.to_ascii_lowercase();
        Some(match () {
            _ if m.starts_with("fight") || m.starts_with("ftutor") => Theme::CombatTraining,
            _ if m.starts_with("assault") => Theme::AssaultTraining,
            _ if m.starts_with("testdrive") => Theme::DrivingTraining,
            _ if m.starts_with("finale1") => Theme::Reckoning,
            _ => return None,
        })
    }
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

/// The sniper rifle as an item: not one of the original's `IT_*`, so a number none of them
/// use.
pub const SNIPER: u8 = 200;

/// A sniper rifle shot (`SNIPE_shoot`: `HIT_TYPE_GUN_SHOT_M` for 260).
pub const SNIPER_DAMAGE: i32 = 260;

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
    Mine(usize),
    Vehicle(usize),
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

/// A briefing still to be read out ([`Game::brief`]).
#[derive(Debug, Clone, Default)]
struct Briefing {
    story: Option<String>,
    objectives: Option<String>,
    /// The script has said something of its own already.
    heard: bool,
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

/// The game's random number generators (each `rand`'s state), kept apart so that, as in the
/// original, one system's dice don't change another's.
#[derive(Debug, Clone)]
struct Rngs {
    /// The mission's own: spawns, the AI's choices.
    main: u32,
    /// The route search's (its bias against odd moves).
    mav: u32,
    barrel: u32,
    creature: u32,
    /// The dogs' (#110).
    dog: u32,
    /// The city's extra people's.
    city: u32,
}

impl Default for Rngs {
    fn default() -> Rngs {
        Rngs { main: 1, mav: 0x3a7, barrel: 0xba44, creature: 0xba7, dog: 0xd06, city: 0xc17 }
    }
}

/// Seconds into the next turn of each system that runs at a fixed rate, whatever the frame
/// rate.
#[derive(Debug, Clone, Default)]
struct Clocks {
    plat: f32,
    bike: f32,
    chopper: f32,
    barrel: f32,
    creature: f32,
    dirt: f32,
    balloon: f32,
    /// To the next sparks off the electrified fences.
    fence_spark: f32,
}

/// How hard the game is (#131). Normal is the original's own tuning, untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }

    /// Added to an enemy's skill as they're made.
    pub(crate) fn skill_change(self) -> i32 {
        self.skill()
    }

    /// Added to an enemy's skill (0 to 15).
    fn skill(self) -> i32 {
        match self {
            Difficulty::Easy => -3,
            Difficulty::Normal => 0,
            Difficulty::Hard => 3,
        }
    }

    /// Times the wait between an enemy's shots.
    fn shot_wait(self) -> f32 {
        match self {
            Difficulty::Easy => 1.5,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 0.75,
        }
    }

    /// The damage the player takes.
    fn hurt(self, damage: i32) -> i32 {
        match self {
            Difficulty::Easy => damage * 3 / 5,
            Difficulty::Normal => damage,
            Difficulty::Hard => damage * 7 / 5,
        }
    }

    /// Times how fast the player's aim settles.
    pub(crate) fn steadying(self) -> f32 {
        match self {
            Difficulty::Easy => 1.5,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 0.7,
        }
    }

    /// Times how fast a search goes.
    pub(crate) fn searching(self) -> f32 {
        match self {
            Difficulty::Easy => 1.5,
            _ => 1.0,
        }
    }

    /// Added to the chance (percent) of something to find on an endless run's thugs.
    pub(crate) fn finds(self) -> i32 {
        match self {
            Difficulty::Easy => 10,
            Difficulty::Normal => 0,
            Difficulty::Hard => -10,
        }
    }
}

/// One game turn, in seconds: the game always advances by whole turns (#128). Thirty a
/// second, the rate everything here was tuned and tested at; the systems the original runs
/// at its own twenty a second still do, on their own clocks.
pub const TURN: f32 = 1.0 / 30.0;

/// The prims that are a switch to press (`PRIM_OBJ_SWITCH_OFF`) and a valve to turn
/// (`PRIM_OBJ_VALVE`).
const PRIM_SWITCH_OFF: u8 = 216;
const PRIM_VALVE: u8 = 196;

/// Where everything that moves is, and which way it faces: kept from the start of each turn
/// to draw between turns.
#[derive(Debug, Clone, Default)]
struct Before {
    people: Vec<(Vec3, f32)>,
    vehicles: Vec<(Vec3, f32)>,
    creatures: Vec<(Vec3, f32)>,
    dogs: Vec<(Vec3, f32)>,
    bikes: Vec<Option<(Vec3, f32)>>,
}

impl Before {
    fn of(g: &Game) -> Before {
        Before {
            people: g.people.iter().map(|p| (p.fig.pos, p.fig.angle)).collect(),
            vehicles: g.vehicles.iter().map(|v| (v.pos, v.yaw)).collect(),
            creatures: g.creatures.iter().map(|c| (c.fig.pos, c.fig.angle)).collect(),
            dogs: g.dogs.iter().map(|d| (d.fig.pos, d.fig.angle)).collect(),
            bikes: g.bikes.iter().map(|(_, f)| f.as_ref().map(|f| (f.pos, f.angle))).collect(),
        }
    }
}

pub struct Game {
    pub people: Vec<Person>,
    pub player: Option<usize>,
    by_way: HashMap<usize, usize>,
    script: Option<Script>,
    pub messages: Vec<Message>,
    pub outcome: Option<Outcome>,
    pub time: f32,
    /// The game's random number generators.
    rngs: Rngs,
    /// How far into its next turn each system that runs at a fixed rate is.
    clocks: Clocks,
    /// Real time not yet played: less than a [`TURN`].
    turn_clock: f32,
    /// The turns played.
    turns: u64,
    /// This run as a ghost, while it's kept (#186).
    recording: Option<game::ghost::Track>,
    /// The checkpoints passed: (event point, the turn after which it had gone off).
    passed: Vec<(usize, u64)>,
    /// Every turn played, while they're kept for a replay (#184).
    keeping: Option<game::replay::Keeping>,
    /// Presses waiting for the next turn.
    pending: Controls,
    /// Where everything was at the start of the last turn.
    before: Before,
    /// Where everything really is, while it's drawn between turns.
    drawing: Option<Before>,
    /// An endless run's waves (#129), when this is one.
    endless: Option<game::endless::Endless>,
    /// A Beat Cop shift (#130), when this is one.
    beat: Option<game::beat::Beat>,
    /// How hard it is ([`Game::set_difficulty`]).
    difficulty: Difficulty,
    /// Capture the flag against a bot (#133), when this is a game of it.
    ctf: Option<game::ctf::Ctf>,
    /// The console's switches (#147).
    pub cheats: game::cheats::Cheats,
    /// What the player's done this mission (#132).
    pub record: game::stats::Stats,
    stats_track: game::stats::Track,
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
    /// What's still to be said of the mission's briefing ([`Game::brief`]).
    briefing: Briefing,
    /// The players who pressed action this turn (one in single player).
    used: Vec<usize>,
    /// Who the player's gun is aimed at.
    pub target: Option<usize>,
    /// What else the player's gun is pointed at, and how steady it is.
    pub aiming: Aiming,
    /// Darci's stats: stamina, strength, reflexes, constitution (badges from earlier
    /// missions, see `set_stats`, plus `badges`).
    pub stats: [u8; 4],
    /// Badges picked up in this mission.
    pub badges: [u8; 4],
    /// The points of the badges picked up in this mission.
    pub found: BTreeSet<usize>,
    /// What the script and the crime rate have counted already.
    tally: game::Tally,
    /// The city's crime rate, 0..100 (`CRIME_RATE`): the mission's, or 50 if it has none
    /// (elev.cpp:309).
    pub crime_rate: u8,
    /// The player's red marks: innocents they've killed (`Genus.Player->RedMarks`). More than
    /// one at the end of a mission as Darci, and the chief has words (Game.cpp:2783).
    pub red_marks: u8,
    /// The mission shows the crime rate on the HUD (`MISSION_FLAG_SHOW_CRIMERATE`).
    pub show_crime_rate: bool,
    /// Props damaged (`OB_FLAG_DAMAGED`), as indices into the map's objects.
    pub damaged: BTreeSet<usize>,
    /// Hydrants knocked over and gushing: where, which way the jet goes (radians, as a
    /// facing), and the seconds left of it.
    pub hydrants: Vec<(Vec3, f32, f32)>,
    /// Villains seen at it this turn, for a cop to come after (`arrest_me`, at most
    /// `MAX_ARREST_ME`).
    arrest_me: Vec<usize>,
    /// How long nothing has happened (`timer_bored`, Person.cpp:3762): seconds since the
    /// player last fought, shot at someone or watched a scene.
    pub bored: f32,
    /// Grenades thrown and not yet gone off.
    pub grenades: Vec<Grenade>,
    /// The player's grenade, primed and still in hand: seconds to going off.
    pub primed: Option<f32>,
    /// Darci's grappling hook, while it's out of her hand (#109).
    pub hook: Option<game::hook::Hook>,
    /// The balloons, held and let go (`BALLOON_balloon`).
    pub balloons: Vec<game::balloons::Balloon>,
    /// Explosives set down and waiting to go off.
    pub charges: Vec<Charge>,
    /// The mission's moving platforms, made or still waiting to be.
    pub platforms: Vec<plat::Platform>,
    /// Their prims, loaded once.
    plat_prims: Vec<Option<uc_formats::prm::Prim>>,
    /// The map's warehouses, with the ambience the mission has given each.
    pub wares: ware::Wares,
    /// The warehouse the player is in, and the ambience it had as they came in.
    indoors: Option<(usize, u8)>,
    /// The mission's creatures (`CLASS_BAT`).
    pub creatures: Vec<creature::Creature>,
    /// Dogs (#110), an unreleased extra: none unless asked for.
    pub dogs: Vec<canid::Canid>,
    /// Their model, once one is made.
    dog_chunk: Option<std::sync::Arc<uc_formats::all::AnimChunk>>,
    /// Looking through the sniper rifle's scope (`SNIPE_on`).
    snipe: Option<game::Snipe>,
    /// The mission's motorbikes, each with its figure.
    pub bikes: Vec<(bike::Bike, Option<Figure>)>,
    /// The navigation map and everyone's last route.
    routes: game::Routes,
    /// The mission's helicopters.
    pub choppers: Vec<chopper::Chopper>,
    /// Where everyone was at the barrels' last turn, to knock barrels with as they walk.
    barrel_marks: Vec<Vec3>,
    /// People getting into cars: (who, which car).
    entering: Vec<(usize, usize)>,
    creature_by_way: HashMap<usize, usize>,
    pub fireballs: Vec<creature::Fireball>,
    pub sparks: Vec<Spark>,
    /// Mines laid by the mission (`IT_MINE`).
    pub mines: Vec<Mine>,
    /// Props that can be searched (`OB_FLAG_SEARCHABLE`), as indices into the map's objects.
    pub searchable: BTreeSet<usize>,
    /// Items stashed in a prop, waiting to be found: (the prop, the item).
    pub hidden: Vec<(usize, Item)>,
    /// The player's search, while one is going on.
    pub search: Option<Search>,
    /// Explosions and fires burning.
    pub pyro: Pyro,
    /// The PlayStation's explosions (`POW_*`), which only the console sets off.
    pub pow: pow::Pow,
    /// The original's unused physics (#111), made from the console: sphere-matter, the dike
    /// (with what its rider presses, `dike::ACCEL` and so on, and time not yet run) and
    /// cloth.
    pub sm: sm::SphereMatter,
    pub dike: Option<dike::Dike>,
    pub dike_control: u8,
    dike_clock: f32,
    pub cloths: cloth::Cloths,
    /// Hypermatter, each object's prim (to draw it through its lattice), the floor under
    /// where the last was dropped, and time not yet run.
    pub hm: hm::Hypermatter,
    hm_prims: HashMap<usize, uc_formats::prm::Prim>,
    hm_floor: f32,
    hm_clock: f32,
    /// Sheets of ground mist the mission has laid down.
    pub mists: Mists,
    /// Drops in the air.
    pub drops: dirt::Drops,
    /// Sparks, casings and blood from gunfire and blows.
    pub gunfx: gunfx::GunFx,
    /// The puddles a rainy level has, and the drips in them.
    pub puddles: puddle::Puddles,
    /// What footsteps sound like on each floor texture.
    steps: steps::Steps,
    /// It's night (`NIGHT_FLAG_DAYTIME` clear): the cars have their lights on.
    pub night: bool,
    /// Who was in a fight last turn, for the gangs to see who's just gone into one.
    fighting: Vec<bool>,
    /// Where on the level's benches, sofas and armchairs someone can sit: (where, facing).
    seats: Vec<(Vec3, f32)>,
    /// The player's sit on a bench, for the health it gives ([`Game::sit_on_benches`]).
    bench: game::Bench,
    /// Noises made this turn for the AI to hear: (what, where, who made it).
    pub(crate) noises: Vec<(person::Noise, Vec3, Option<usize>)>,
    /// The floor squares on the sea, drawn with the waves.
    sea: Vec<uc_world::level::SeaSquare>,
    /// Leaves, cans and snow lying about round the player.
    pub dirt: dirt::Dirt,
    /// Which extras are drawn (the Options screen's).
    pub show: Show,
    /// Where each person, then each car, was at the dirt's last turn (NaN for nowhere).
    dirt_marks: Vec<Vec3>,
    /// Footprints and tyre tracks on the ground.
    pub tracks: tracks::Tracks,
    /// Each car's wheels where they were at its last skid mark (`oldX`, `oldZ`).
    skid_marks: Vec<Option<[Vec2; 4]>>,
    /// Strips trailing behind things: the bikes' exhaust.
    pub ribbons: ribbon::Ribbons,
    /// Steam from the mission's vents.
    pub steam: steam::Steam,
    /// The beams `tripwire` points watch.
    pub trips: trip::Trips,
    pub switches: switch::Switches,
    /// Buildings whose fences are electrified (`FACET_FLAG_ELECTRIFIED`).
    pub electrified: BTreeSet<u16>,
    /// There's no floor (`GF_NO_FLOOR`, Day Of Reckoning on top of Gunston Tower).
    pub no_floor: bool,
    /// Seconds since the end of the world began (`PYRO_GAMEOVER`), if it has.
    pub end_of_world: Option<f32>,
    /// The mission's own tune, played throughout ([`Theme::of`]).
    theme: Option<Theme>,
    /// How hard the camera is shaking from a blast nearby, 0 to 255 (`fc->shake`): it's
    /// thrown about by up to half that in world units each way.
    pub shake: f32,
    shake_turns: f32,
    /// Estate of Emergency, where the tramp's face is the boss's (`estate`).
    estate: bool,
    /// Grim Gardens, where jumping onto a platform puts you on it (`playing_level`).
    botanicc: bool,
    /// The roads, who's driving them, the pavements and what the people on them say.
    streets: game::Streets,
    /// How far the player can see (world units): the city's extra people further away than
    /// this are out of sight, and may be moved. The original's draw distance by default; a
    /// renderer that sees further should say so.
    pub view_dist: f32,
    /// The player had a gun out last frame.
    gun_drawn: bool,
    /// What to call the player, for the HUD.
    player_name: &'static str,
    /// Co-op (#182): the seats after the first, and the seat whose player-only state is in
    /// the fields above ([`Game::focus`]).
    partners: Vec<game::coop::Partner>,
    focused: usize,
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
            people: Vec::new(),
            player: None,
            by_way: HashMap::new(),
            script,
            messages: Vec::new(),
            briefing: Briefing::default(),
            outcome: None,
            time: 0.0,
            rngs: Rngs::default(),
            clocks: Clocks::default(),
            turn_clock: 0.0,
            turns: 0,
            recording: None,
            passed: Vec::new(),
            keeping: None,
            pending: Controls::default(),
            before: Before::default(),
            drawing: None,
            endless: None,
            beat: None,
            difficulty: Difficulty::Normal,
            ctf: None,
            cheats: Default::default(),
            record: Default::default(),
            stats_track: Default::default(),
            sounds: Vec::new(),
            fight_timer: 0.0,
            sprinting: false,
            items: Vec::new(),
            vehicles: Vec::new(),
            veh_by_way: HashMap::new(),
            barrels: Vec::new(),
            sign: None,
            cutscene: None,
            convo: None,
            wide: None,
            used: Vec::new(),
            target: None,
            aiming: Aiming::default(),
            stats: [0; 4],
            badges: [0; 4],
            found: BTreeSet::new(),
            tally: game::Tally::default(),
            crime_rate: if level.mission.crime_rate == 0 { 50 } else { level.mission.crime_rate.min(100) },
            red_marks: 0,
            show_crime_rate: level.mission.flags & MISSION_FLAG_SHOW_CRIMERATE != 0,
            damaged: BTreeSet::new(),
            hydrants: Vec::new(),
            arrest_me: Vec::new(),
            bored: 0.0,
            grenades: Vec::new(),
            primed: None,
            hook: None,
            balloons: Vec::new(),
            charges: Vec::new(),
            platforms: level.platforms.iter().map(|&(way, ob)| {
                let o = &level.map.objects[ob];
                let yaw = radians(o.yaw as f32);
                plat::Platform::new(way, ob, o.prim, Vec3::new(o.x as f32, o.y as f32, o.z as f32), yaw)
            }).collect(),
            plat_prims: level.platforms.iter().map(|&(_, ob)| uc_world::level::load_prim(&level.game, level.map.objects[ob].prim)).collect(),
            wares: ware::Wares::new(&level.map),
            indoors: None,
            creatures: Vec::new(),
            dogs: Vec::new(),
            dog_chunk: None,
            snipe: None,
            bikes: Vec::new(),
            routes: game::Routes::new(level),
            choppers: Vec::new(),
            barrel_marks: Vec::new(),
            entering: Vec::new(),
            creature_by_way: HashMap::new(),
            fireballs: Vec::new(),
            sparks: Vec::new(),
            mines: Vec::new(),
            searchable: BTreeSet::new(),
            hidden: Vec::new(),
            search: None,
            pyro: Pyro::default(),
            pow: pow::Pow::default(),
            sm: sm::SphereMatter::default(),
            dike: None,
            dike_control: 0,
            dike_clock: 0.0,
            cloths: cloth::Cloths::default(),
            hm: hm::Hypermatter::default(),
            hm_prims: HashMap::new(),
            hm_floor: 0.0,
            hm_clock: 0.0,
            mists: Mists::default(),
            drops: dirt::Drops::default(),
            gunfx: gunfx::GunFx::default(),
            puddles: puddle::Puddles::default(),
            steps: steps::Steps::load(&level.game, level.map.texture_set),
            night: Self::raining(level),
            fighting: Vec::new(),
            seats: game::seats(level),
            bench: game::Bench::default(),
            noises: Vec::new(),
            sea: level.sea(),
            dirt: Game::new_dirt(level),
            show: Default::default(),
            dirt_marks: Vec::new(),
            tracks: tracks::Tracks::default(),
            skid_marks: Vec::new(),
            ribbons: ribbon::Ribbons::default(),
            steam: steam::Steam::default(),
            trips: trip::Trips::new(level, &level.mission),
            switches: switch::Switches::new(level),
            electrified: BTreeSet::new(),
            no_floor: false,
            end_of_world: None,
            theme: Theme::of(&level.name),
            shake: 0.0,
            shake_turns: 0.0,
            estate: level.mission.mission_name.eq_ignore_ascii_case("estate2"),
            botanicc: level.name.eq_ignore_ascii_case("botanicc"),
            streets: game::Streets::new(level),
            view_dist: city::DRAW_DIST,
            gun_drawn: false,
            player_name: "",
            partners: Vec::new(),
            focused: 0,
        };
        if !scripted {
            g.spawn_player(level, level.spawn.0, level.spawn.1, 0, ptype::DARCI, 0);
        }
        // Puddles round the city when it's raining (elev.cpp:2598).
        if Self::raining(level) {
            g.puddles = puddle::Puddles::precalculate(&level.map, |x, z| g.routes.mav.height(x, z) as i32);
        }
        g.sync_wmove(level);
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
            let yaw = radians(o.yaw as f32);
            g.add_barrel(Barrel { kind: 0, prim: uc_world::level::BARREL_PROP, explodes: true, way: 0, gone: false,
                                  pos: Vec3::new(at.x, y, at.z), yaw, tilt: 0.0, on: None, spheres: None });
        }
        g
    }

    /// Event points the script can't run yet.
    pub fn unsupported(&self) -> &[String] {
        self.script.as_ref().map_or(&[], |s| &s.unsupported)
    }

    /// Has the radio read out a briefing at the start, once any opening fly-by is over and
    /// the script's own opening messages are done: the `story` (left out if the script has
    /// already said something of its own), then the `objectives`. For missions the game never
    /// finished, whose scripts say little of what there is to do.
    pub fn brief(&mut self, story: Option<String>, objectives: String) {
        self.briefing = Briefing { story, objectives: Some(objectives), heard: false };
    }

    /// The next line of the briefing, when nothing else is being said.
    fn say_briefing(&mut self) {
        if self.briefing.objectives.is_none() {
            return;
        }
        self.briefing.heard |= self.messages.iter().any(|m| !m.filmed);
        if self.cutscene.is_some() || self.wide.is_some() || self.in_conversation() || !self.messages.is_empty() || self.time < 3.0 {
            return;
        }
        let b = &mut self.briefing;
        let story = b.story.take().filter(|_| !b.heard);
        let Some(text) = story.or_else(|| b.objectives.take()) else { return };
        // Long enough to read.
        let left = (text.len() as f32 / 15.0).clamp(5.0, 16.0);
        self.messages.push(Message { text, left, face: Some(talk::face::RADIO), filmed: false });
    }

    /// A hash of the state that matters to how the game plays on: the time, the outcome, the
    /// dice, and where everyone and everything is and what they're doing. Two runs given the
    /// same inputs must agree on it every frame (what lockstep multiplayer and replays need,
    /// #114).
    pub fn state_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        // SipHash with fixed keys: the same on every machine and every run.
        let mut h = std::hash::DefaultHasher::new();
        let v = |h: &mut std::hash::DefaultHasher, v: Vec3| [v.x, v.y, v.z].map(f32::to_bits).hash(h);
        self.time.to_bits().hash(&mut h);
        format!("{:?}", self.outcome).hash(&mut h);
        let r = &self.rngs;
        [r.main, r.mav, r.barrel, r.creature, r.dog, r.city].hash(&mut h);
        for p in &self.people {
            p.removed.hash(&mut h);
            v(&mut h, p.fig.pos);
            p.fig.angle.to_bits().hash(&mut h);
            p.fig.anim().hash(&mut h);
            p.health.hash(&mut h);
            format!("{:?}", p.state).hash(&mut h);
        }
        for c in &self.vehicles {
            v(&mut h, c.pos);
            [c.yaw, c.speed].map(f32::to_bits).hash(&mut h);
        }
        for b in &self.barrels {
            v(&mut h, b.pos);
            b.gone.hash(&mut h);
        }
        for d in &self.dogs {
            v(&mut h, d.fig.pos);
            format!("{:?}", d.state).hash(&mut h);
        }
        for m in &self.messages {
            m.text.hash(&mut h);
        }
        h.finish()
    }

    fn rand(&mut self) -> u32 {
        rand(&mut self.rngs.main)
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

    /// Advances the game by `dt` seconds of real time with the player's `input`: as many
    /// whole turns of [`TURN`] as are due, so how it plays out doesn't depend on the frame
    /// rate. What's held (moving, running, firing) is as `input` has it now; what's pressed
    /// (punch, jump, action...) is kept until a turn takes it, so a tap between turns isn't
    /// lost, and only the first turn of a frame sees it.
    pub fn update(&mut self, dt: f32, input: &Controls, level: &Level) {
        let p = &mut self.pending;
        p.punch |= input.punch;
        p.kick |= input.kick;
        p.action |= input.action;
        p.jump |= input.jump;
        p.switch |= input.switch;
        p.cycle_target |= input.cycle_target;
        p.scope |= input.scope;
        p.taunt |= input.taunt;
        self.turn_clock += dt;
        while self.turn_clock >= TURN {
            self.turn_clock -= TURN;
            let p = std::mem::take(&mut self.pending);
            let c = Controls {
                punch: p.punch, kick: p.kick, action: p.action, jump: p.jump, switch: p.switch,
                cycle_target: p.cycle_target, scope: p.scope, taunt: p.taunt, ..*input
            };
            let mut inputs = vec![Controls::default(); self.seats()];
            inputs[0] = c;
            self.step(&inputs, level);
        }
    }

    /// One turn, with one `Controls` per seat (seat 0 first; missing seats stand still). What
    /// a co-op session plays: every machine steps the same turns with the same inputs. The
    /// seat in focus stays in focus.
    pub fn step(&mut self, inputs: &[Controls], level: &Level) {
        let focused = self.focused;
        self.focus(0);
        self.before = Before::of(self);
        self.turn(inputs, level);
        self.turns += 1;
        self.keep_ghost();
        self.keep_turn(inputs);
        self.focus(focused);
    }

    /// Where between turns to draw, 0 to 1, for a caller stepping turns itself
    /// ([`Game::step`]) rather than through [`Game::update`].
    pub fn set_between(&mut self, t: f32) {
        self.turn_clock = t.clamp(0.0, 1.0) * TURN;
    }

    /// How hard the game is.
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    /// Sets how hard the game is, for everyone made so far and everyone made from now on.
    pub fn set_difficulty(&mut self, d: Difficulty) {
        let change = d.skill() - self.difficulty.skill();
        self.difficulty = d;
        for i in 0..self.people.len() {
            self.tune(i, change);
        }
    }

    /// Person `i`'s skill moved by `change`, and how long they wait between shots, for the
    /// difficulty.
    pub(crate) fn tune(&mut self, i: usize, change: i32) {
        let d = self.difficulty;
        let p = &mut self.people[i];
        if p.player {
            return;
        }
        if change != 0 {
            p.skill = (p.skill as i32 + change).clamp(0, 15) as u8;
        }
        p.shot_wait = d.shot_wait();
    }

    /// `damage` to person `i`, as the difficulty has it if they're the player.
    pub(crate) fn hurt(&self, i: usize, damage: i32) -> i32 {
        if self.people[i].player && self.cheats.god {
            return 0;
        }
        if self.people[i].player { self.difficulty.hurt(damage) } else { damage }
    }

    /// The turns played so far.
    pub fn turns(&self) -> u64 {
        self.turns
    }

    /// How far into the next turn real time is, 0 to 1: where to draw between the last two.
    pub fn between(&self) -> f32 {
        (self.turn_clock / TURN).clamp(0.0, 1.0)
    }

    /// Puts everyone and everything that moves where they are [`Game::between`] the last two
    /// turns, for drawing; [`Game::end_draw`] puts them back. Anything that jumped further
    /// than two squares in a turn (a teleport, a respawn) is drawn where it is now.
    pub fn begin_draw(&mut self) {
        if self.drawing.is_some() {
            return;
        }
        let now = Before::of(self);
        let t = self.between();
        let mix = |a: (Vec3, f32), b: (Vec3, f32)| {
            if (b.0 - a.0).length() > 512.0 {
                return b;
            }
            let turn = (b.1 - a.1 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            (a.0.lerp(b.0, t), a.1 + turn * t)
        };
        let b = &self.before;
        for (i, p) in self.people.iter_mut().enumerate() {
            if let (Some(&was), Some(&is)) = (b.people.get(i), now.people.get(i)) {
                (p.fig.pos, p.fig.angle) = mix(was, is);
            }
        }
        for (i, v) in self.vehicles.iter_mut().enumerate() {
            if let (Some(&was), Some(&is)) = (b.vehicles.get(i), now.vehicles.get(i)) {
                (v.pos, v.yaw) = mix(was, is);
            }
        }
        for (i, c) in self.creatures.iter_mut().enumerate() {
            if let (Some(&was), Some(&is)) = (b.creatures.get(i), now.creatures.get(i)) {
                (c.fig.pos, c.fig.angle) = mix(was, is);
            }
        }
        for (i, d) in self.dogs.iter_mut().enumerate() {
            if let (Some(&was), Some(&is)) = (b.dogs.get(i), now.dogs.get(i)) {
                (d.fig.pos, d.fig.angle) = mix(was, is);
            }
        }
        for (i, (_, f)) in self.bikes.iter_mut().enumerate() {
            if let (Some(f), Some(Some(was)), Some(Some(is))) = (f.as_mut(), b.bikes.get(i), now.bikes.get(i)) {
                (f.pos, f.angle) = mix(*was, *is);
            }
        }
        self.drawing = Some(now);
    }

    /// Back to where everything really is, after [`Game::begin_draw`].
    pub fn end_draw(&mut self) {
        let Some(now) = self.drawing.take() else { return };
        for (p, &(pos, angle)) in self.people.iter_mut().zip(&now.people) {
            (p.fig.pos, p.fig.angle) = (pos, angle);
        }
        for (v, &(pos, yaw)) in self.vehicles.iter_mut().zip(&now.vehicles) {
            (v.pos, v.yaw) = (pos, yaw);
        }
        for (c, &(pos, angle)) in self.creatures.iter_mut().zip(&now.creatures) {
            (c.fig.pos, c.fig.angle) = (pos, angle);
        }
        for (d, &(pos, angle)) in self.dogs.iter_mut().zip(&now.dogs) {
            (d.fig.pos, d.fig.angle) = (pos, angle);
        }
        for ((_, f), was) in self.bikes.iter_mut().zip(&now.bikes) {
            if let (Some(f), Some((pos, angle))) = (f.as_mut(), was) {
                (f.pos, f.angle) = (*pos, *angle);
            }
        }
    }

    /// One turn of the game with each seat's input, seat 0 in focus. What's only the
    /// player's (aiming, using things, grenades, the hook, searching) runs once for each seat,
    /// with that seat in focus.
    fn turn(&mut self, inputs: &[Controls], level: &Level) {
        let dt = TURN;
        let seats = self.seats();
        let seat_input = |s: usize| inputs.get(s).copied().unwrap_or_default();
        self.time += dt;
        for m in &mut self.messages {
            m.left -= dt;
        }
        self.messages.retain(|m| m.left > 0.0);
        self.say_briefing();
        self.used = (0..seats).filter(|&s| seat_input(s).action).filter_map(|s| self.seat_person(s)).collect();
        for &i in &self.used.clone() {
            self.switches.flip_near(self.people[i].fig.pos);
            // A switch or a valve on the map (`OB_find_type`, `FIND_OB_SWITCH_OR_VALVE`,
            // within 0x180): she presses or turns it.
            let at = self.people[i].fig.pos;
            let near = level.map.objects.iter()
                .filter(|o| matches!(o.prim, PRIM_SWITCH_OFF | PRIM_VALVE))
                .map(|o| (o, Vec3::new(o.x as f32, o.y as f32, o.z as f32)))
                .filter(|(_, p)| (*p - at).length() < 0x180 as f32)
                .min_by(|a, b| (a.1 - at).length().total_cmp(&(b.1 - at).length()));
            if let Some((o, p)) = near {
                let (yaw, switch) = (o.yaw, o.prim == PRIM_SWITCH_OFF);
                self.people[i].work_switch(p, yaw, switch);
            }
        }
        for s in 0..seats {
            self.focus(s);
            let c = seat_input(s);
            if self.outcome.is_none() {
                self.update_target(c.cycle_target, level);
                self.aim_at_people();
            }
        }
        self.focus(0);
        if self.outcome.is_none() {
            self.run_script(dt, level);
        }
        self.run_endless(dt, level);
        self.run_beat(dt, level);
        self.run_cheats();
        self.run_stats(level);
        let pressed = (0..seats).map(seat_input).any(|c| c.punch || c.kick || c.jump || c.action);
        self.update_scenes(dt, level, pressed);
        self.pyro.update(dt);
        self.pow.update(dt);
        self.run_physics(dt, level);
        if self.outcome.is_none() {
            self.charges(dt, level);
        }
        self.vent_steam(dt, level);
        self.trip_wires(dt);
        self.wreck_cars();
        if self.outcome.is_none() {
            self.run_mines(dt, level);
            for s in 0..seats {
                self.focus(s);
                self.run_primed(dt, level);
            }
            self.focus(0);
            self.run_grenades(dt, level);
            for s in 0..seats {
                self.focus(s);
                self.run_hook(dt, level);
            }
            self.focus(0);
        }
        self.electric_fences(dt, level);
        self.no_floor_falls();
        self.end_the_world(dt);
        self.settle_shake(dt);
        self.go_indoors(level);
        self.move_platforms(dt, level);
        // Lightning dies away (the Men in Black's and the fences' too, where there are no
        // creatures).
        for s in &mut self.sparks {
            s.left -= dt;
        }
        self.sparks.retain(|s| s.left > 0.0);
        self.run_creatures(dt, level, self.cutscene.as_ref().is_some_and(|c| c.shot.lock_player));
        self.run_bikes(dt, level);
        self.fly_choppers(dt, level);
        self.roll_barrels(dt, level);
        // From just under the pelvis, for as long as they're on their feet (the original asks
        // only that they're peeing, but its pelvis is never anywhere sensible once they're down).
        let streams: Vec<(Vec3, Vec2)> = self.people.iter()
            .filter(|p| p.peeing && !p.removed && !p.is_down() && !p.is_dead())
            .map(|p| (p.fig.hips() - Vec3::new(0.0, 16.0, 0.0), p.facing_dir()))
            .collect();
        self.drops.update(dt, &streams, |at| level.ground.surface(&level.map, at.x, at.z, at.y + 8.0));
        self.gunfx.update(dt, |at| level.ground.surface(&level.map, at.x, at.z, at.y + 8.0));
        let near = self.player().map(|p| p.fig.pos);
        self.puddles.update(dt, Self::raining(level) && self.indoors.is_none(), near);
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
        // Each seat in turn, its player in focus: (who, their controls, searching or not).
        let mut playing: Vec<(usize, Controls, bool)> = Vec::with_capacity(seats);
        for s in 0..seats {
            self.focus(s);
            // Getting in and out of cars and talking to people take the action key first.
            let mut input = seat_input(s);
            // Through the scope she stands still, and the keys are the scope's.
            if self.scope(dt, &input) {
                input = Controls::default();
            }
            input.target = self.target;
            self.steady_aim(dt);
            input.aim = self.aiming.creature.map(|k| self.creatures[k].middle())
                .or_else(|| self.aiming.barrel.map(|k| self.barrels[k].pos + Vec3::new(0.0, 50.0, 0.0)))
                .or_else(|| self.aiming.mine.map(|k| self.mines[k].pos))
                .or_else(|| self.aiming.vehicle.map(|k| self.vehicles[k].pos + Vec3::new(0.0, 64.0, 0.0)));
            if self.outcome.is_none() && !held && input.action && self.use_something(level) {
                input.action = false;
            }
            // Attack with a grenade in hand primes it, then throws it (`player_activate_in_hand`).
            if let (true, false, true, Some(pi)) = (self.outcome.is_none(), held, input.punch, self.player) {
                let p = &self.people[pi];
                if p.weapon == Weapon::Grenade && p.grenades > 0 {
                    self.use_grenade(pi);
                    input.punch = false;
                } else if p.weapon == Weapon::Hook && p.hook {
                    self.use_hook(pi);
                    input.punch = false;
                }
            }
            // Fly-bys that hold the player still hold the car too (it rolls to a stop).
            if held || self.outcome.is_some() {
                input.drive = Drive::default();
            }
            // Action held: the viewer passes it on as `sprint`.
            self.searching(dt, input.sprint, level);
            if let Some(pi) = self.player {
                playing.push((pi, input, self.search.is_some()));
            }
        }
        let drives: Vec<(usize, Drive)> = playing.iter().map(|&(pi, c, _)| (pi, c.drive)).collect();
        self.drive(dt, &drives, level);
        self.ride(&drives, level);

        let player = self.player;
        let mut snaps: Vec<_> = self.people.iter().map(|p| p.snap()).collect();
        for s in &mut snaps {
            s.zone = level.mission.zones_at(s.pos.x, s.pos.z);
        }
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
        self.follow_on_see(level);
        self.walk_goals();
        self.run_ctf(dt, level);
        self.arm_bodyguards(level);
        self.find_ways(level);
        self.city_life(dt, level);
        self.board_cars(level);
        self.hitch_rides(level);
        // `PCOM_AI_SUICIDE`, "the easiest AI! A random death" (fallen/Source/pcom.cpp): down
        // as if shot with a pistol the first time it runs. The Fallen's bootlegger is made this
        // way on the roof once Harney has spoken, and the first clue waits on his death.
        for p in self.people.iter_mut().filter(|p| p.ai == 15 && !p.player && !p.is_dead() && !p.is_down()) {
            let from = p.fig.pos + Vec3::new(0.0, 0.0, 1.0);
            p.take_hit(from, 10_000, person::ko::KNOCKDOWN, false, false, Weapon::Pistol);
        }
        let darci = player.map(|p| self.people[p].fig.pos);
        // "Assasins change once they've killed their targets" (pcom.cpp:11078): to a gang
        // member wandering about.
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if matches!(p.ai, 3 | 21) && p.other != 0 && self.by_way.get(&p.other).is_some_and(|&f| self.people[f].is_dead()) {
                let p = &mut self.people[i];
                (p.ai, p.mv) = (person::AI_GANG, person::MOVE_WANDER);
                p.calm_down();
            }
        }
        self.bullies(&snaps, dt, level);
        for i in 0..self.people.len() {
            let p = &self.people[i];
            let foe = match (p.ai, p.other) {
                // Capture the flag: each seat's player fights the other's.
                _ if self.ctf_foe(i).is_some() => self.ctf_foe(i),
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
                    // Hit by anyone but their client, they fight back (`PCOM_attack_happened`,
                    // pcom.cpp:13732: "goto fight"), the player too.
                    let hit_back = p.hurt_by
                        .filter(|&j| j != i && Some(j) != client && j < self.people.len())
                        .filter(|&j| {
                            let o = &self.people[j];
                            !o.removed && !o.is_dead() && (o.fig.pos - p.fig.pos).length() < GUARDS * 256.0
                        });
                    hit_back.or_else(|| self.people
                        .iter()
                        .enumerate()
                        .filter(|(j, o)| {
                            *j != i && o.hostile() && o.aggro() && !o.is_dead()
                                && (o.foe == client || o.foe == Some(i))
                                && (o.fig.pos - p.fig.pos).length() < GUARDS * 256.0
                                && level.ground.line_of_sight(p.fig.pos + eye, o.fig.pos + eye)
                        })
                        .min_by(|a, b| {
                            let d = |o: &Person| (o.fig.pos - p.fig.pos).length();
                            d(a.1).total_cmp(&d(b.1))
                        })
                        .map(|(j, _)| j))
                }
                // Someone out to get a particular person (a bully's pick, a gang mate's fight).
                _ if p.victim.is_some() => p.victim,
                // A bully with nobody picked on has nobody to fight.
                (person::AI_BULLY, _) => None,
                // `PCOM_AI_KILL_COLOUR` (pcom.cpp:11607): whoever is nearest of the colour of
                // the person its `ai_other` point made, alive, not one of the city's extras,
                // and in its zone if it has one; no matter how far, seen or not.
                (person::AI_KILL_COLOUR, w) if w != 0 => {
                    let colour = self.by_way.get(&w).map(|&e| self.people[e].colour);
                    colour.and_then(|c| {
                        self.people.iter().enumerate()
                            .filter(|&(j, o)| {
                                j != i && o.colour == c && !o.removed && !o.is_dead() && !o.fake
                                    && (p.zone == 0 || level.mission.zones_at(o.fig.pos.x, o.fig.pos.z) & p.zone != 0)
                            })
                            .min_by(|a, b| (a.1.fig.pos - p.fig.pos).length().total_cmp(&(b.1.fig.pos - p.fig.pos).length()))
                            .map(|(j, _)| j)
                    })
                }
                _ => self.nearest_seat(p.fig.pos),
            };
            // Twenty blocks from the player, a bodyguard is out of harm's way.
            let away = p.ai == person::AI_BODYGUARD
                && darci.is_some_and(|d| (d - p.fig.pos).length() > 20.0 * 256.0);
            self.people[i].foe = foe;
            // God mode (#147): nothing lands on the player.
            self.people[i].invulnerable = away || self.people[i].protected || (self.cheats.god && Some(i) == self.player);
        }
        for i in 0..self.people.len() {
            let seat = playing.iter().find(|&&(pi, _, _)| pi == i).filter(|_| self.people[i].player);
            let c = if let Some(&(_, input, searching)) = seat {
                if self.outcome.is_some() || held || searching { Controls::default() } else { input }
            } else {
                let mut rng = self.rngs.main;
                let mut rand = || { rng = rng.wrapping_mul(69069).wrapping_add(1); rng >> 16 };
                let c = if frozen || self.cheats.freeze { Controls::default() } else { self.people[i].think(i, &snaps, &level.ground, dt, &mut rand) };
                self.rngs.main = rng;
                c
            };
            self.people[i].step(i, &c, &snaps, &level.ground, &level.map, dt, &mut events);
        }
        self.raise_alerts();
        self.hear_noises(level);
        self.sit_down();
        self.strike_poses();
        self.join_dance(dt, &playing);
        self.apply(events, level);
        self.back_up_gangs();
        for s in 0..seats {
            self.focus(s);
            self.pick_up();
        }
        self.focus(0);
        self.sit_on_benches(&playing, dt);
        self.partners_back_up(dt, level);
        for p in &mut self.people {
            p.advance(dt, &level.ground);
        }
        self.finish_entering();
        self.sync_grapples();
        self.sync_carrying(level);
        self.separate(level);
        // `SlideOdd`: one more turn sliding along something odd, or back to none if they
        // moved freely (collide.cpp:6659).
        for p in &mut self.people {
            match p.slid_odd.take() {
                Some(true) => p.slide_odd = p.slide_odd.saturating_add(1),
                Some(false) => p.slide_odd = 0,
                None => {}
            }
        }
        self.count_dead_cops();
        self.tally_crime();
        self.men_in_black(dt, level);
        self.blow_dirt(dt, level);
        self.lay_prints(level);
        self.step_sounds(level);
        self.burn(level);
        self.gush();
        self.do_arrests(level);
        self.breathe_out(level);
        self.bleed(level);
        self.splatter(level);
        self.run_balloons(dt, level);

        // Capture the flag: knocked out, she comes back at home ([`Game::run_ctf`]).
        if let (Some(p), None, None) = (self.player, &self.outcome, &self.ctf) {
            if self.people[p].knocked_out {
                let who = self.player_name();
                self.finish(Outcome::Lost(format!("{who} was knocked out")));
            }
        }
        // Each seat's music follows their own player; the boredom clock follows seat 0's.
        for s in (0..seats).rev() {
            self.focus(s);
            let Some(pl) = self.player() else { continue };
            let pos = pl.fig.pos;
            let fighting = matches!(pl.state, State::Attack { .. } | State::Grapple { .. } | State::Hurt)
                || self.people.iter().any(|p| p.aggro() && (p.fig.pos - pos).length() < 6.0 * 256.0);
            // Really sprinting: out of breath she's only running, whatever is held.
            let sprinting = pl.sprinting;
            self.fight_timer = if fighting { 6.0 } else { (self.fight_timer - dt).max(0.0) };
            self.sprinting = sprinting;
            if s == 0 {
                // A fight, a fly-by or a conversation resets it; otherwise it counts up.
                if fighting || self.cutscene.is_some() || self.in_conversation() {
                    self.bored = 0.0;
                } else {
                    self.bored += dt;
                }
            }
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
