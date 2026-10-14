//! Mission scripts: runs a mission's event points the way `EWAY_process` does
//! (fallen/Source/eway.cpp), with the editor-to-runtime conversion of `ELEV_load_level`
//! (fallen/Source/elev.cpp) folded in.
//!
//! Every tick, each event point that isn't dead is checked in index order:
//! - inactive points whose trigger holds become active and do their action;
//! - active points apply their "on trigger" rule: `active` points stay active for good,
//!   `active_while` points drop back when the trigger stops holding, `active_time` points
//!   drop back after a delay (and can then trigger again, which is how hints repeat until a
//!   `kill_waypoint` stops them).
//!
//! Points belong to a colour and group. `group_death` points switch every other point of
//! theirs off (dead) and `group_life` back on; missions use this to hold back waves of
//! enemies until the previous wave is down (`group_dead` triggers).
//!
//! The script never touches people directly: it reads their state through [`Query`] and
//! asks the game for changes with [`Command`]s. The game reports back when a camera reaches a
//! waypoint ([`Script::camera_reached`]) and when a conversation ends
//! ([`Script::conversation_over`]).
//!
//! Units: countdown triggers are in 1/100 s, `active_time` delays in 1/10 s, message
//! durations in seconds (0 = 8 s), time extensions in seconds, as the original says.

use glam::{vec3, Vec3};
use uc_formats::ucm::{on_trigger, trigger, waypoint, EventPoint, Mission};

/// What the script needs to know about the game.
pub trait Query {
    fn player_pos(&self) -> Option<Vec3>;
    /// The state of the person created by event point `way`, if it exists.
    fn person(&self, way: usize) -> Option<PersonState>;
    /// Where the person or vehicle created by event point `way` is.
    fn thing(&self, way: usize) -> Option<ThingState>;
    /// The person created by `person` is in the vehicle created by `vehicle` (0 = any vehicle).
    fn in_vehicle(&self, person: usize, vehicle: usize) -> bool;
    /// The player has someone in a grapple hold.
    fn player_grabbing(&self) -> bool;
    /// The player pressed "action" this frame (for switches: `player_uses_radius`).
    fn player_used(&self) -> bool;
    /// The player has fired a gun at some point (`GF_PLAYER_FIRED_GUN`, which the original
    /// sets once and never clears).
    fn player_fired(&self) -> bool {
        false
    }
    /// The player has picked up the item created by event point `way`
    /// (`EWAY_FLAG_GOTITEM`, which `specific_item_held` watches: it stays true afterwards,
    /// even for something used up on the spot like a health pack).
    fn got_item(&self, _way: usize) -> bool {
        false
    }
    /// The prop a `prim_damaged` point at `pos` watches has been damaged
    /// (`EWAY_COND_PRIM_DAMAGED`).
    fn prim_damaged(&self, _pos: Vec3) -> bool {
        false
    }
    /// The city's crime rate, 0..100 (`CRIME_RATE`).
    fn crime_rate(&self) -> u8 {
        50
    }
    /// The beam a `tripwire` point `way` watches is broken (`TRIP_activated`).
    fn tripped(&self, _way: usize) -> bool {
        false
    }
    /// The person created by `a` can see the one created by `b` (`can_a_see_b`).
    fn sees(&self, _a: usize, _b: usize) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PersonState {
    /// Lying on the ground (maybe only for a moment).
    pub down: bool,
    /// Out of the fight for good: knocked out, arrested or gone.
    pub dead: bool,
    pub arrested: bool,
    pub half_dead: bool,
}

/// A person's or vehicle's whereabouts.
#[derive(Debug, Clone, Copy)]
pub struct ThingState {
    pub pos: Vec3,
    /// Facing as a person's engine angle (facing (-sin, -cos)).
    pub angle: f32,
    /// World units per second.
    pub speed: f32,
}

/// An enemy as an event point describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemySpec {
    /// `PERSON_*` type.
    pub person: u8,
    /// `PCOM_AI_*`.
    pub ai: u8,
    /// `PCOM_BENT_*` flags.
    pub bent: u8,
    /// For fight-test dummies, the `PCOM_COMBAT_*` moves that knock them out.
    pub ko_moves: u8,
    /// 0..15: how well they fight.
    pub skill: u8,
    /// `HAS_*` weapons (fallen/Headers/Mission.h): 1 pistol, 0x20 knife, 0x40 bat.
    pub has: u8,
    /// `PCOM_MOVE_*`.
    pub mv: u8,
    /// `follow` (`Data[1]`): for `PCOM_MOVE_FOLLOW`, the event point that creates the person
    /// to keep up with (0 = the player).
    pub follow: usize,
    /// The point's colour (`pcom_colour`): its low two bits pick the clothes.
    pub colour: u8,
    /// `ai_other`: for assassins, the event point that creates the person to kill.
    pub other: usize,
    /// The point's group (`pcom_group`).
    pub group: u8,
    /// Nothing hurts them (`FLAG2_PERSON_INVULNERABLE`): bit 4 of the zone byte.
    pub invulnerable: bool,
    /// Guilty of something (`FLAG2_PERSON_GUILTY`): bit 5 of the zone byte. A cop can only be
    /// arrested if so.
    pub guilty: bool,
}

/// A camera fly-by (`create_camera` with its `camera_waypoint`s and `create_target`).
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    pub start: Vec3,
    /// (event point, position, speed in world units per second, seconds to wait there).
    pub waypoints: Vec<(usize, Vec3, f32, f32)>,
    pub target: Look,
    /// The player can't move during the shot.
    pub lock_player: bool,
}

/// What a fly-by looks at: its `create_target`, by `Data[1]` (`CT_*`, fallen/Source/elev.cpp).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Look {
    /// The target point itself (`CT_NORMAL`).
    Place(Vec3),
    /// Whatever event point `way` made, once it has, and the target point `at` until then
    /// (`CT_ATTACHED`).
    Thing { way: usize, at: Vec3 },
    /// The nearest living person within 0x500 of `at` (`CT_NEAREST_LIVING`).
    Near(Vec3),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `person`: the `PERSON_*` type the player takes on (Darci in most missions);
    /// `has`: the `HAS_*` weapons they start carrying.
    SpawnPlayer { way: usize, pos: Vec3, angle: f32, person: u8, has: u8 },
    SpawnEnemy { way: usize, pos: Vec3, angle: f32, spec: EnemySpec },
    /// Change the AI of the person created by `way`.
    /// `other`: an assassin's mark (event point); `mv`: the new `PCOM_MOVE_*`; `colour` and
    /// `group`: the adjusting point's (`PCOM_change_person_attributes`).
    Adjust { way: usize, ai: u8, bent: u8, ko_moves: u8, other: usize, mv: u8, follow: usize, colour: u8, group: u8 },
    /// Remove the person (or item) created by `way` from the game.
    Remove { way: usize },
    /// Move the person or item created by `way`.
    Move { way: usize, pos: Vec3, angle: f32 },
    /// Show `text` for `secs`; `way` is the event point (its voice line, if recorded).
    /// `speech`: the recording's number (`talk2\<mission>.ucm<speech>.wav`); `speaker`: who
    /// says it.
    Message { way: usize, speech: usize, text: String, secs: f32, speaker: Speaker },
    /// Put item `kind` (`IT_*`) at `pos`; `stashed` hides it in the nearest prop instead,
    /// to be found by searching it (`EWAY_ARG_ITEM_STASHED_IN_PRIM`).
    Item { way: usize, kind: u8, pos: Vec3, stashed: bool },
    /// A creature of `kind` (`EWAY_SUBTYPE_ANIMAL_*`: 2 gargoyle, 3 Balrog, 4 Bane).
    Creature { way: usize, kind: u8, pos: Vec3, angle: f32 },
    /// Give the warehouse round `pos` ambience `ambience` (`EWAY_DO_WAREFX`: 0 none, 1-4).
    WareFx { pos: Vec3, ambience: u8 },
    /// Start or stop the person created by `way` peeing (`FLAG_PERSON_PEEING`).
    Pee { way: usize, on: bool },
    /// Switch off the wire `tripwire` point `way` watches (`TRIP_deactivate`).
    Untrip { way: usize },
    /// Let the prop nearest `pos` be searched (`WPT_MAKE_SEARCHABLE`).
    Searchable { pos: Vec3 },
    Camera { way: usize, shot: Shot },
    /// Lines said in turn by the people created by the given event points (0 = the player);
    /// report back with [`Script::conversation_over`]. `a` and `b` are the two points
    /// (`Data[1]`, `Data[2]`); an `ambient` one (`Data[3]`) is overheard rather than filmed.
    Conversation { way: usize, speech: usize, lines: Vec<(usize, String)>, a: usize, b: usize, ambient: bool },
    /// Platform point `way` sets its prop moving (`EWAY_DO_CREATE_PLATFORM`): `speed` and
    /// `flags` are its `Data[0]` and `Data[1]`.
    Platform { way: usize, colour: u8, group: u8, speed: i32, flags: i32 },
    /// The player becomes the person point `way` made (`EWAY_DO_TRANSFER_PLAYER`).
    Transfer { way: usize },
    /// A vehicle of `VEH_TYPE_*` `kind`, and what it takes to get into it.
    Vehicle { way: usize, kind: u8, pos: Vec3, angle: f32, key: crate::vehicle::Key },
    /// Stop the vehicle created by `way` for good.
    Stall { way: usize },
    /// Lock or unlock the vehicle created by `way` (`EWAY_DO_LOCK_VEHICLE`).
    Lock { way: usize, locked: bool },
    /// A bang at `pos`: `kinds` is `PYRO_construct`'s bitfield (1 twanger, 2 explosion,
    /// 4 dust wave, 8 streamers, 16 fire, 32 dome, 64/128 firebomb) and `scale` its size.
    Explode { way: usize, pos: Vec3, kinds: u8, scale: f32 },
    /// Barrel (`BT_*`: 0 barrel, 1 traffic cone, 2 bin...) at `pos`.
    Barrel { way: usize, kind: u8, pos: Vec3 },
    /// Flash a road sign (`PANEL_SIGN_WHICH_*`), mirrored if `flip`.
    Sign { which: u8, flip: bool },
    /// Open the door nearest `pos` (`EWAY_DO_CONTROL_DOOR`).
    OpenDoor { pos: Vec3 },
    /// Lay a sheet of ground mist centred on `pos` (`EWAY_DO_CREATE_MIST`).
    Mist { pos: Vec3 },
    /// Play the game's sound `id` at `pos`. `replace` is `play_glue_wave`'s type 1, which cuts
    /// off whatever it last started rather than letting them overlap.
    Sound { id: usize, pos: Vec3, replace: bool },
    Win,
    Lose,
}

/// Who says a message (`EWAY_MESSAGE_WHO_*` in its `Data[2]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    /// Despatch, over the radio.
    Radio,
    /// The person the event point made.
    Person(usize),
    /// A street name (`EWAY_MESSAGE_WHO_STREETNAME`): a sign, not a voice.
    Street,
    /// Tutorial text (`EWAY_MESSAGE_WHO_TUTORIAL`).
    Tutorial,
}

/// A navigation beacon to show.
#[derive(Debug, Clone)]
pub struct Beacon {
    pub label: String,
    pub pos: Vec3,
    /// Follow the person created by this event point instead.
    pub track: Option<usize>,
}

pub const PCOM_AI_FIGHT_TEST: u8 = 12;

/// `AP_*` (fallen/Headers/Mission.h): what an `activate_prim` point works on. The loader
/// switches on it rather than ever making an `EWAY_DO_ACTIVATE_PRIM`, which is compiled out
/// (`#ifdef ANIM_PRIM`): a door becomes `EWAY_DO_CONTROL_DOOR`, a fence
/// `EWAY_DO_ELECTRIFY_FENCE`, and a security camera nothing at all (`ELEV_load_level`).
mod activate {
    pub const DOOR: i32 = 0;
    pub const ELECTRIC_FENCE: i32 = 1;
}

/// Whether [`Script`] knows what to do with a `TT_*` trigger. The ones it doesn't are reported
/// alongside the unsupported point types: a point whose trigger never fires is as dead as one
/// whose action does nothing.
fn trigger_supported(t: u8) -> bool {
    use trigger::*;
    matches!(t, NONE | DEPENDENCY | RADIUS | TIMER | BOOLEAN_AND | BOOLEAN_OR | KILLED | COUNTDOWN
        | ENEMY_RADIUS | VISIBLE_COUNTDOWN | CUBOID | HALF_DEAD | GROUP_DEAD | PERSON_SEEN
        | PERSON_USED | PLAYER_USES_RADIUS | PERSON_ARRESTED | CONVERSATION_OVER | COUNTER
        | KILLED_NOT_ARRESTED | PERSON_IN_VEHICLE | SPECIFIC_ITEM_HELD | THING_RADIUS_DIR | RANDOM
        | DARCI_GRABBED | PUNCHED_AND_KICKED | MOVE_RADIUS_DIR | ITEM_HELD | PLAYER_FIRES_GUN | PRIM_DAMAGED
        | CRIME_RATE_ABOVE | CRIME_RATE_BELOW | TRIPWIRE)
}

/// `EWAY_ARG_ITEM_STASHED_IN_PRIM` (fallen/Headers/eway.h): a `create_item`'s item is hidden
/// inside the nearest prop and only turns up when someone searches it.
const STASHED_IN_PRIM: i32 = 2;

/// `IT_BARREL`: a `create_item` that puts down barrels.
const IT_BARREL: i32 = 9;

/// `EWAY_counter[7]`: how many cops have been killed.
const COPS_KILLED: usize = 7;

/// `PT_*` (an event point's player type) to `PERSON_*`, through `PLAYER_*`
/// (`ELEV_load_level` in elev.cpp and `PLAYER_create` in Player.cpp): Nitro Car's Mako is a
/// rasta thug.
fn player_person(pt: i32) -> u8 {
    use uc_world::character::person::*;
    match pt {
        2 => ROPER,
        3 => COP,
        4 => THUG_RASTA,
        _ => DARCI,
    }
}

/// What the player starts carrying. `Data[1]` is a "no gun" flag: the loader folds it into
/// the player's subtype (`ELEV_load_level`) and `EWAY_DO_CREATE_PLAYER` reads it back out as
/// `has = (subtype & 8) ? 0 : PCOM_HAS_GUN`, so a point that leaves it clear hands over a
/// loaded pistol. Every training and street mission sets it and starts Darci empty-handed;
/// Nitro Car doesn't, so Mako is carrying one from the off.
fn player_has(d1: i32) -> u8 {
    if d1 != 0 { 0 } else { 1 }
}

/// `ET_*` (enemy type) to `PERSON_*` (convert_ET_to_PT in elev.cpp).
fn enemy_person(et: i32) -> u8 {
    use uc_world::character::person::*;
    match et {
        3 => SLAG_TART,
        4 => SLAG_FATUGLY,
        5 | 17 => MECHANIC,
        6 | 9 => THUG_RASTA,
        7 | 10 => THUG_RED,
        8 | 11 => THUG_GREY,
        12..=15 => COP,
        16 => HOSTAGE,
        18 => TRAMP,
        19 => MIB1,
        20 => MIB2,
        21 => MIB3,
        22 => DARCI,
        23 => ROPER,
        _ => CIV,
    }
}

/// Weapons that come with an enemy type (`ET_GANG_RASTA_PISTOL` and so on).
fn enemy_has(et: i32) -> u8 {
    match et {
        9 | 13 => 1,
        10 | 14 => 2,
        11 | 15 => 4,
        _ => 0,
    }
}

/// `ai_other` (`Data[7]`): the event point whose person an assassin kills or a bodyguard
/// guards. (Fight-test dummies keep their knockout moves there instead.)
fn ai_other(ai: u8, other: i32) -> usize {
    if ai == PCOM_AI_FIGHT_TEST { 0 } else { (other & 0xFFFF) as usize }
}

fn ko_moves(ai: u8, other: i32) -> u8 {
    if ai == PCOM_AI_FIGHT_TEST { (other >> 16) as u8 } else { 0 }
}

struct Way {
    ep: EventPoint,
    text: Option<String>,
    active: bool,
    /// No longer processed (but stays active if it was).
    dead: bool,
    /// Stopped by a kill_waypoint: beacons disappear.
    killed: bool,
    /// A conversation has been said.
    finished: bool,
    /// Seconds until an `active_time` point drops back.
    timer: Option<f32>,
    /// Countdown trigger: seconds left.
    countdown: f32,
    /// punched_and_kicked: the player's move counts when it started watching.
    baseline: Option<(u32, u32)>,
}

impl Way {
    fn pos(&self) -> Vec3 {
        vec3(self.ep.x as f32, self.ep.y as f32, self.ep.z as f32)
    }
}

pub struct Script {
    ways: Vec<Option<Way>>,
    counters: [u32; 16],
    punches: u32,
    kicks: u32,
    time: f32,
    rng: u32,
    /// Event points this script can't run, by type name (reported once).
    pub unsupported: Vec<String>,
    /// The person the player is talking to, while their points go off (`EWAY_used_thing`):
    /// what they set off is theirs to say.
    used: Option<usize>,
    commands: Vec<Command>,
    finished: bool,
}

/// The number of a message's or conversation's recording: `Data[9]`, which the original
/// keeps as the speech file's number (`EWAY_create`'s `magic_index`, fallen/Source/eway.cpp);
/// the point's own number when that's unset.
fn speech(ep: &EventPoint) -> usize {
    if ep.data[9] > 0 { ep.data[9] as usize } else { ep.index }
}

/// Seconds for a countdown radius (1/100 s).
fn hundredths(r: i32) -> f32 {
    r.max(0) as f32 / 100.0
}

impl Script {
    pub fn new(m: &Mission) -> Script {
        let texts = m.point_texts();
        let size = m.event_points.iter().map(|e| e.index + 1).max().unwrap_or(1);
        let mut ways: Vec<Option<Way>> = (0..size).map(|_| None).collect();
        let mut unsupported = Vec::new();
        let mut commands = Vec::new();
        for ep in &m.event_points {
            if ep.used == 0 {
                continue;
            }
            use waypoint::*;
            let supported = matches!(ep.kind, SIMPLE | CREATE_PLAYER | CREATE_ENEMIES | CREATE_ITEM | CREATE_CAMERA
                | CREATE_TARGET | CAMERA_WAYPOINT | MESSAGE | NAV_BEACON | KILL_WAYPOINT | INCREMENT | ADJUST_ENEMY
                | END_GAME_WIN | END_GAME_LOSE | BONUS_POINTS | GROUP_LIFE | GROUP_DEATH | GROUP_RESET | EXTEND
                | MOVE_THING | CONVERSATION | CREATE_BARREL | CREATE_VEHICLE | STALL_CAR | SIGN | CONE_PENALTIES
                | SPOT_EFFECT | CREATE_TREASURE | LOCK_VEHICLE | VISUAL_EFFECT | RESET_COUNTER | ACTIVATE_PRIM | CREATE_MIST | SOUND_EFFECT | SHAKE_CAMERA
                | MAKE_SEARCHABLE | MAKE_PERSON_PEE | CREATE_TRAP | CREATE_CREATURE | WAREFX | TRANSFER_PLAYER | LINK_PLATFORM);
            // An `activate_prim` only counts as supported when it opens a door: the fences
            // and the cameras are still to do.
            let supported = supported && (ep.kind != ACTIVATE_PRIM || ep.data[0] == activate::DOOR);
            if !supported {
                let name = uc_formats::ucm::names::WAYPOINT.get(ep.kind as usize).copied().unwrap_or("?");
                unsupported.push(format!("{} {}", ep.index, name));
            }
            // What sets a point off matters as much as what it does: a trigger this doesn't
            // know simply never fires, and the point sits there for the whole mission.
            if !trigger_supported(ep.trigger) {
                let name = uc_formats::ucm::names::TRIGGER.get(ep.trigger as usize).copied().unwrap_or("?");
                unsupported.push(format!("{} trigger {}", ep.index, name));
            }
            // `ELEV_load_level` flags the prop as the level loads and never makes a waypoint
            // of it (`goto dont_create_a_waypoint`): its trigger is never even looked at.
            if ep.kind == MAKE_SEARCHABLE {
                commands.push(Command::Searchable { pos: vec3(ep.x as f32, ep.y as f32, ep.z as f32) });
                continue;
            }
            // Barrels as an item aren't a waypoint either: `Data[1]` plain barrels put down as
            // the level loads, a few units either side of the point (elev.cpp:1078).
            if ep.kind == CREATE_ITEM && ep.data[0] == IT_BARREL {
                for k in 0..ep.data[1].max(0) {
                    let off = (k % 3 - 1) as f32 * 7.0;
                    commands.push(Command::Barrel { way: 0, kind: 0, pos: vec3(ep.x as f32 + off, ep.y as f32, ep.z as f32 - off) });
                }
                continue;
            }
            ways[ep.index] = Some(Way {
                ep: ep.clone(), text: texts.get(&ep.index).cloned(), active: false, dead: false, killed: false,
                finished: false, timer: None, countdown: hundredths(ep.radius), baseline: None,
            });
        }
        Script {
            ways, counters: [0; 16], punches: 0, kicks: 0, time: 0.0, rng: 0x5eed, unsupported,
            commands, finished: false, used: None,
        }
    }

    fn rand(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(69069).wrapping_add(1);
        self.rng >> 16
    }

    /// A cop has been killed. The original keeps the tally in counter 7, bumped wherever
    /// someone dies (`PERSON_COP` in Person.cpp), and missions read it: Rat Catch's Boss wants
    /// five of them as a punishment.
    pub fn note_cop_killed(&mut self) {
        self.counters[COPS_KILLED] = self.counters[COPS_KILLED].saturating_add(1);
    }

    /// A Balrog's quarry has died: counter 8 keeps the tally (`EWAY_counter[8]`, kept to 255).
    pub fn note_balrog_kill(&mut self) {
        self.counters[8] = (self.counters[8] + 1).min(255);
    }

    /// The player started a punch / kick (for punched_and_kicked triggers).
    pub fn note_punch(&mut self) {
        self.punches += 1;
    }
    pub fn note_kick(&mut self) {
        self.kicks += 1;
    }

    fn way(&self, i: usize) -> Option<&Way> {
        self.ways.get(i).and_then(|w| w.as_ref())
    }

    /// Whether event point `i` is active (for tests and debugging).
    pub fn point_active(&self, i: usize) -> bool {
        self.is_active(i)
    }

    fn is_active(&self, i: usize) -> bool {
        self.way(i).is_some_and(|w| w.active)
    }

    /// The camera reached `camera_waypoint` point `i` (these only trigger this way).
    pub fn camera_reached(&mut self, i: usize, q: &dyn Query) {
        if self.way(i).is_some_and(|w| !w.active && !w.dead) {
            self.activate(i, q);
        }
    }

    /// The player "used" (talked to) the person created by `way`: points waiting for that
    /// go active (EWAY_COND_PERSON_USED is set from outside, like this).
    pub fn person_used(&mut self, way: usize, q: &dyn Query) {
        let waiting: Vec<usize> = self.ways.iter().flatten()
            .filter(|w| w.ep.trigger == trigger::PERSON_USED && w.ep.ep_ref as usize == way && !w.active && !w.dead)
            .map(|w| w.ep.index).collect();
        self.used = Some(way);
        for i in waiting {
            self.activate(i, q);
        }
        self.used = None;
    }

    /// Conversation point `i` has been said.
    pub fn conversation_over(&mut self, i: usize) {
        if let Some(Some(w)) = self.ways.get_mut(i) {
            w.finished = true;
        }
    }

    /// The visible countdown on screen, in seconds, if one is running.
    pub fn countdown(&self) -> Option<f32> {
        self.ways
            .iter()
            .flatten()
            .find(|w| {
                w.ep.trigger == trigger::VISIBLE_COUNTDOWN && !w.active && !w.dead
                    && (w.ep.ep_ref == 0 || self.is_active(w.ep.ep_ref as usize))
            })
            .map(|w| w.countdown)
    }

    /// The people of colour `colour` the script has created: (any alive, any dead).
    fn colour_status(&self, colour: u8, q: &dyn Query) -> (bool, bool) {
        let (mut alive, mut dead) = (false, false);
        for w in self.ways.iter().flatten().filter(|w| w.ep.colour == colour && w.ep.kind == waypoint::CREATE_ENEMIES) {
            if let Some(p) = q.person(w.ep.index) {
                if p.dead { dead = true } else { alive = true }
            }
        }
        (alive, dead)
    }

    fn condition(&mut self, i: usize, dt: f32, q: &dyn Query) -> bool {
        let w = self.ways[i].as_ref().unwrap();
        // Camera waypoints only trigger when the camera gets there (EWAY_COND_CAMERA_AT).
        if w.ep.kind == waypoint::CAMERA_WAYPOINT {
            return false;
        }
        let (r1, r2, radius, pos) = (w.ep.ep_ref as usize, w.ep.ep_ref_bool as usize, w.ep.radius, w.pos());
        let person = |way: usize| q.person(way);
        match w.ep.trigger {
            trigger::NONE => true,
            trigger::DEPENDENCY => self.is_active(r1),
            trigger::BOOLEAN_AND => self.is_active(r1) && self.is_active(r2),
            trigger::BOOLEAN_OR => self.is_active(r1) || self.is_active(r2),
            trigger::RADIUS => q.player_pos().is_some_and(|p| p.distance(pos) < radius as f32),
            trigger::CUBOID => q.player_pos().is_some_and(|p| {
                (p.x - pos.x).abs() <= (radius & 0xFFFF) as f32 && (p.z - pos.z).abs() <= (radius >> 16) as f32
            }),
            trigger::PLAYER_USES_RADIUS => q.player_used() && q.player_pos().is_some_and(|p| p.distance(pos) < radius as f32),
            trigger::TIMER => self.time >= hundredths(radius),
            trigger::COUNTDOWN | trigger::VISIBLE_COUNTDOWN => {
                if r1 != 0 && !self.is_active(r1) {
                    return false;
                }
                let w = self.ways[i].as_mut().unwrap();
                w.countdown = (w.countdown - dt).max(0.0);
                w.countdown <= 0.0
            }
            trigger::KILLED => person(r1).is_some_and(|p| p.dead),
            trigger::KILLED_NOT_ARRESTED => person(r1).is_some_and(|p| p.dead && !p.arrested),
            trigger::PERSON_ARRESTED => person(r1).is_some_and(|p| p.arrested),
            trigger::HALF_DEAD => person(r1).is_some_and(|p| p.half_dead),
            trigger::GROUP_DEAD => {
                let (alive, dead) = self.colour_status(w.ep.colour, q);
                dead && !alive
            }
            trigger::CONVERSATION_OVER => self.way(r1).is_some_and(|c| c.finished),
            trigger::COUNTER => self.counters.get(r1).is_some_and(|&c| c >= radius.max(0) as u32),
            trigger::DARCI_GRABBED => q.player_grabbing(),
            trigger::PUNCHED_AND_KICKED => {
                if !self.is_active(r1) {
                    return false;
                }
                let (p, k) = (self.punches, self.kicks);
                let w = self.ways[i].as_mut().unwrap();
                let (bp, bk) = *w.baseline.get_or_insert((p, k));
                p >= bp + 2 && k >= bk + 2
            }
            // Radii in quarter squares, as the loader rounds them (Radius / 64).
            trigger::ENEMY_RADIUS => q.thing(r1).is_some_and(|t| t.pos.distance(pos) < (radius / 64 * 64) as f32),
            trigger::PERSON_IN_VEHICLE => q.in_vehicle(r1, r2),
            trigger::THING_RADIUS_DIR | trigger::MOVE_RADIUS_DIR => q.thing(r1).is_some_and(|t| {
                // Within the radius, facing the point's direction (within 1/16 of a turn);
                // thing_radius_dir also wants it stopped (parking).
                let mut da = (t.angle - uc_world::character::ep_angle(w.ep.direction)).rem_euclid(std::f32::consts::TAU);
                if da > std::f32::consts::PI {
                    da -= std::f32::consts::TAU;
                }
                let stopped = w.ep.trigger == trigger::MOVE_RADIUS_DIR || t.speed.abs() < 80.0;
                t.pos.distance(pos) < (radius / 64 * 64) as f32 && da.abs() <= std::f32::consts::TAU / 16.0 && stopped
            }),
            trigger::PERSON_USED => false,
            // Both of these read as "the player has the thing that point made". The original
            // tells them apart by what it looks the item up with, but `ELEV_load_level` puts an
            // event point in either one's `arg1`, and in the missions that use them they mean
            // the same thing.
            trigger::SPECIFIC_ITEM_HELD | trigger::ITEM_HELD => q.got_item(r1),
            trigger::PLAYER_FIRES_GUN => q.player_fired(),
            trigger::PRIM_DAMAGED => q.prim_damaged(pos),
            // The threshold is a percentage times 100 (`Radius / 100`, elev.cpp:634).
            trigger::CRIME_RATE_ABOVE => q.crime_rate() as i32 >= radius / 100,
            trigger::CRIME_RATE_BELOW => q.crime_rate() as i32 <= radius / 100,
            trigger::TRIPWIRE => q.tripped(w.ep.index),
            trigger::PERSON_SEEN => q.sees(r1, r2),
            trigger::RANDOM => {
                // Once the dependency is active, one of the points sharing it (or none) goes
                // active and all of them die (EWAY_COND_RANDOM).
                if self.is_active(r1) {
                    let same: Vec<usize> = self.ways.iter().flatten()
                        .filter(|o| o.ep.trigger == trigger::RANDOM && o.ep.ep_ref as usize == r1 && !o.dead)
                        .map(|o| o.ep.index).collect();
                    let which = self.rand() as usize % (same.len() + 1);
                    if let Some(&chosen) = same.get(which) {
                        self.activate(chosen, q);
                    }
                    for s in same {
                        self.ways[s].as_mut().unwrap().dead = true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// The point an attached camera target at `at` follows: the nearest point that makes
    /// something, within 0x300 (`EWAY_fix_do`, fallen/Source/eway.cpp). The Fallen's first
    /// fly-by is attached this way to the player's start, so it watches Darci.
    fn attached(&self, at: Vec3) -> Option<usize> {
        use waypoint::*;
        self.ways.iter().flatten()
            .filter(|w| matches!(w.ep.kind, CREATE_PLAYER | CREATE_CREATURE | CREATE_ENEMIES | CREATE_ITEM | CREATE_VEHICLE | LINK_PLATFORM))
            .map(|w| (w.ep.index, crate::creature::qdist3(w.pos() - at)))
            .filter(|&(_, d)| d < 0x300 as f32)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// The fly-by that point `i` (a `create_camera`) starts.
    fn shot(&self, i: usize) -> Option<Shot> {
        let cam = self.way(i)?;
        let same = |w: &&Way| w.ep.colour == cam.ep.colour && w.ep.group == cam.ep.group;
        let t = self.ways.iter().flatten().filter(same).find(|w| w.ep.kind == waypoint::CREATE_TARGET)?;
        let at = t.pos();
        let target = match t.ep.data[1] {
            2 => self.attached(at).map_or(Look::Place(at), |way| Look::Thing { way, at }),
            3 => Look::Near(at),
            _ => Look::Place(at),
        };
        let speed = |d: &[i32; 10]| ((d[2] >> 2).max(4) as f32) * 40.0;
        let waypoints = self.ways.iter().flatten().filter(same).filter(|w| w.ep.kind == waypoint::CAMERA_WAYPOINT)
            .map(|w| (w.ep.index, w.pos(), speed(&w.ep.data), w.ep.data[3].max(0) as f32 / 10.0))
            .collect();
        Some(Shot { start: cam.pos(), waypoints, target, lock_player: cam.ep.data[4] != 0 })
    }

    fn activate(&mut self, i: usize, q: &dyn Query) {
        let w = self.ways[i].as_mut().unwrap();
        w.active = true;
        let ep = w.ep.clone();
        let pos = w.pos();
        let angle = uc_world::character::ep_angle(ep.direction);
        let d = ep.data;
        match ep.kind {
            waypoint::CREATE_PLAYER => {
                let spawn = Command::SpawnPlayer { way: i, pos, angle, person: player_person(d[0]), has: player_has(d[1]) };
                self.commands.push(spawn);
            }
            waypoint::CREATE_ENEMIES => {
                let ai = (d[5] & 0xFFFF) as u8;
                let et = d[0] & 0xFFFF;
                // The zone byte, `Data[4] >> 8`, carries flags above its four zone bits
                // (`ELEV_load_level`, and `EWAY_create_enemy` in fallen/Source/eway.cpp).
                // Fight-test dummies are always made invulnerable too ("ugh!"), but they have
                // their own rules here: only their lesson move puts them down.
                let zone = d[4] >> 8;
                let spec = EnemySpec {
                    person: enemy_person(et), ai, bent: d[4] as u8, ko_moves: ko_moves(ai, d[7]),
                    skill: ((d[5] >> 16) as u8).min(15), has: ((d[2] >> 16) as u8) | enemy_has(et), mv: (d[3] + 1) as u8,
                    follow: (d[1] & 0xFFFF) as usize, colour: w.ep.colour, other: ai_other(ai, d[7]), group: w.ep.group,
                    invulnerable: zone & (1 << 4) != 0 && ai != PCOM_AI_FIGHT_TEST, guilty: zone & (1 << 5) != 0,
                };
                self.commands.push(Command::SpawnEnemy { way: i, pos, angle, spec });
            }
            waypoint::ADJUST_ENEMY => {
                let ai = d[5] as u8;
                self.commands.push(Command::Adjust {
                    way: d[6] as usize, ai, bent: d[4] as u8, ko_moves: ko_moves(ai, d[7]), other: ai_other(ai, d[7]),
                    mv: (d[3] + 1) as u8, follow: (d[1] & 0xFFFF) as usize, colour: w.ep.colour, group: w.ep.group,
                });
            }
            waypoint::MESSAGE | waypoint::BONUS_POINTS => {
                if let Some(text) = w.text.clone() {
                    let secs = if ep.kind == waypoint::MESSAGE && d[1] > 0 { d[1] as f32 } else { 8.0 };
                    // Set off by talking to someone, it's theirs to say; otherwise `Data[2]`.
                    let speaker = match (self.used, d[2]) {
                        (Some(p), _) => Speaker::Person(p),
                        _ if ep.kind == waypoint::BONUS_POINTS => Speaker::Radio,
                        (None, 0) => Speaker::Radio,
                        (None, 0xffff) => Speaker::Street,
                        (None, 0xfffe) => Speaker::Tutorial,
                        (None, p) => Speaker::Person(p.max(0) as usize),
                    };
                    self.commands.push(Command::Message { way: i, speech: speech(&w.ep), text: text.trim().to_string(), secs, speaker });
                }
            }
            waypoint::CONVERSATION => {
                let text = w.text.clone().unwrap_or_default();
                let lines = text.split('|').map(str::trim).filter(|l| !l.is_empty()).enumerate()
                    .map(|(n, l)| (if n % 2 == 0 { d[1] } else { d[2] }.max(0) as usize, l.to_string())).collect();
                let (a, b) = (d[1].max(0) as usize, d[2].max(0) as usize);
                self.commands.push(Command::Conversation { way: i, speech: speech(&w.ep), lines, a, b, ambient: d[3] != 0 });
            }
            waypoint::NAV_BEACON => {
                // A beacon tracking a person who doesn't exist yet waits for them.
                if d[1] != 0 && q.person(d[1] as usize).is_none() {
                    w.active = false;
                    return;
                }
            }
            // `Data[2]` is the item's flags (`ELEV_load_level`).
            // A bat makes nothing: `BAT_create` for them is compiled out of the shipped game.
            waypoint::CREATE_CREATURE if d[0] as u8 == crate::creature::BAT => {}
            waypoint::CREATE_CREATURE => {
                self.commands.push(Command::Creature { way: i, kind: d[0] as u8, pos, angle })
            }
            waypoint::CREATE_ITEM => {
                self.commands.push(Command::Item { way: i, kind: d[0] as u8, pos, stashed: d[2] & STASHED_IN_PRIM != 0 })
            }
            // A badge. The original picks which of the four at random (`alloc_special`);
            // here each point keeps its own, so badges can be tracked from run to run.
            waypoint::CREATE_TREASURE => {
                let kind = crate::BADGE + crate::goals::badge_stat(i) as u8;
                self.commands.push(Command::Item { way: i, kind, pos, stashed: false });
            }
            waypoint::CREATE_VEHICLE => {
                let kind = crate::vehicle::from_vt(d[0]);
                let key = crate::vehicle::Key::from_vk(d[3]);
                self.commands.push(Command::Vehicle { way: i, kind, pos, angle, key });
            }
            waypoint::STALL_CAR => self.commands.push(Command::Stall { way: d[0] as usize }),
            // `Data[1]` set locks, clear unlocks (`ELEV_load_level`).
            waypoint::LOCK_VEHICLE => {
                self.commands.push(Command::Lock { way: d[0] as usize, locked: d[1] != 0 })
            }
            waypoint::VISUAL_EFFECT => {
                self.commands.push(Command::Explode { way: i, pos, kinds: d[0] as u8, scale: d[1].max(0) as f32 })
            }
            waypoint::CREATE_BARREL => self.commands.push(Command::Barrel { way: i, kind: d[0] as u8, pos }),
            waypoint::SIGN => self.commands.push(Command::Sign { which: d[0] as u8, flip: d[1] != 0 }),
            waypoint::WAREFX => self.commands.push(Command::WareFx { pos, ambience: d[0] as u8 }),
            waypoint::LINK_PLATFORM => {
                self.commands.push(Command::Platform { way: i, colour: ep.colour, group: ep.group, speed: d[0], flags: d[1] })
            }
            waypoint::TRANSFER_PLAYER => self.commands.push(Command::Transfer { way: d[0].max(0) as usize }),
            // Only the door kind does anything here; an electric fence or a security camera
            // is left for later (and the camera does nothing in the original either).
            waypoint::CREATE_MIST => self.commands.push(Command::Mist { pos }),
            // `EWAY_DO_SHAKE_CAMERA` is an empty case in the shipped game (eway.cpp:6289), so
            // this does nothing on purpose rather than inventing a shake it never had. Where
            // the original really does shake the camera is `create_shockwave`, which the car
            // bombs already go through.
            waypoint::SHAKE_CAMERA => {}
            // `Data[0]` picks `play_glue_wave`'s type and `Data[1]` is the sound's id.
            waypoint::SOUND_EFFECT => {
                self.commands.push(Command::Sound { id: d[1].max(0) as usize, pos, replace: d[0] == 1 })
            }
            waypoint::ACTIVATE_PRIM if d[0] == activate::DOOR => {
                self.commands.push(Command::OpenDoor { pos })
            }
            waypoint::ACTIVATE_PRIM => {
                let _ = activate::ELECTRIC_FENCE;
            }
            waypoint::CREATE_CAMERA => {
                if let Some(shot) = self.shot(i) {
                    self.commands.push(Command::Camera { way: i, shot });
                }
            }
            waypoint::MOVE_THING => self.commands.push(Command::Move { way: d[0] as usize, pos, angle }),
            // `Data[0]` is the point that made the person. Someone not made yet is left alone,
            // and the point doesn't wait for them: RTA's and Arms Breaker's come before their
            // person's point, so in the original nobody is ever seen at it.
            waypoint::MAKE_PERSON_PEE => self.commands.push(Command::Pee { way: d[0] as usize, on: true }),
            waypoint::EXTEND => {
                if let Some(Some(t)) = self.ways.get_mut(d[0] as usize) {
                    t.countdown += d[1].max(0) as f32;
                }
            }
            waypoint::GROUP_LIFE | waypoint::GROUP_DEATH | waypoint::GROUP_RESET => {
                for o in self.ways.iter_mut().flatten() {
                    if o.ep.colour == ep.colour && o.ep.group == ep.group
                        && !matches!(o.ep.kind, waypoint::GROUP_LIFE | waypoint::GROUP_DEATH | waypoint::GROUP_RESET)
                    {
                        o.dead = ep.kind == waypoint::GROUP_DEATH;
                        if ep.kind == waypoint::GROUP_RESET {
                            o.active = false;
                        }
                    }
                }
            }
            waypoint::KILL_WAYPOINT => {
                let target = d[0] as usize;
                if let Some(Some(t)) = self.ways.get_mut(target) {
                    t.dead = true;
                    t.killed = true;
                    // Killing a pee point is what stops the person peeing.
                    if t.ep.kind == waypoint::MAKE_PERSON_PEE {
                        self.commands.push(Command::Pee { way: t.ep.data[0] as usize, on: false });
                    }
                    // And killing a tripwire point switches its wire off (eway.cpp:5669).
                    if t.ep.trigger == trigger::TRIPWIRE {
                        self.commands.push(Command::Untrip { way: target });
                    }
                    // A car goes up where it stands; a person is carried off the map. Either
                    // way the point forgets it ever made them, so triggers watching for their
                    // death stop firing (`EWAY_DO_KILL_WAYPOINT`).
                    if matches!(t.ep.kind, waypoint::CREATE_ENEMIES | waypoint::CREATE_ITEM | waypoint::CREATE_VEHICLE | waypoint::CREATE_CREATURE) {
                        self.commands.push(Command::Remove { way: target });
                    }
                }
            }
            waypoint::INCREMENT => {
                if let Some(c) = self.counters.get_mut(d[1] as usize) {
                    *c += 1;
                }
            }
            // `Data[0]` names the counter, or clears the lot when it's zero.
            waypoint::RESET_COUNTER => match self.counters.get_mut(d[0] as usize) {
                Some(c) if d[0] != 0 => *c = 0,
                _ => self.counters = [0; 16],
            },
            waypoint::END_GAME_WIN | waypoint::END_GAME_LOSE if !self.finished => {
                self.finished = true;
                self.commands.push(if ep.kind == waypoint::END_GAME_WIN { Command::Win } else { Command::Lose });
            }
            _ => {}
        }
        let w = self.ways[i].as_mut().unwrap();
        match ep.on_trigger {
            on_trigger::ACTIVE_TIME => w.timer = Some(ep.after_timer as f32 / 10.0),
            on_trigger::ACTIVE_WHILE => {}
            on_trigger::ACTIVE_DIE => {
                w.dead = true;
                w.active = false;
            }
            _ => w.dead = true,
        }
    }

    /// Runs one tick and returns what the game should do.
    pub fn update(&mut self, dt: f32, q: &dyn Query) -> Vec<Command> {
        self.time += dt;
        for i in 0..self.ways.len() {
            let Some(w) = &self.ways[i] else { continue };
            if w.dead {
                continue;
            }
            if w.active {
                let w = self.ways[i].as_mut().unwrap();
                if let Some(t) = &mut w.timer {
                    *t -= dt;
                    if *t <= 0.0 {
                        w.timer = None;
                        w.active = false;
                    }
                } else if w.ep.on_trigger == on_trigger::ACTIVE_WHILE && !self.condition(i, dt, q) {
                    self.ways[i].as_mut().unwrap().active = false;
                }
            } else if self.condition(i, dt, q) {
                self.activate(i, q);
            }
        }
        std::mem::take(&mut self.commands)
    }

    /// The active points of `colour` and `group`, in order, with their positions: a
    /// patrol's route (`EWAY_find_waypoint`, `EWAY_find_nearest_waypoint`).
    pub fn patrol_points(&self, colour: u8, group: u8) -> Vec<(usize, Vec3)> {
        self.ways
            .iter()
            .flatten()
            .filter(|w| w.active && w.ep.colour == colour && w.ep.group == group)
            .map(|w| (w.ep.index, w.pos()))
            .collect()
    }

    /// Seconds since the mission started (`EWAY_time`, which is in hundredths).
    pub fn time(&self) -> f32 {
        self.time
    }

    /// The steam vents blowing: `create_trap` points that are active and not dead, which is
    /// what `EWAY_process` asks before it calls `EWAY_process_emit_steam`. A trap that stays
    /// active for good is marked dead the moment it goes active (`EWAY_STAY_ALWAYS`), so it
    /// never blows at all; the ones that do are `active_while` and `active_time` points.
    pub fn vents(&self) -> Vec<crate::steam::Vent> {
        self.ways
            .iter()
            .flatten()
            .filter(|w| w.ep.kind == waypoint::CREATE_TRAP && w.active && !w.dead)
            .map(|w| {
                let d = &w.ep.data;
                crate::steam::Vent {
                    pos: w.pos(),
                    angle: uc_world::character::ep_angle(w.ep.direction),
                    dir: d[4] as u8,
                    routine: d[3] as u32,
                    speed: (d[1] & 0x3f) as u32,
                    steps: (d[2] & 0x0f) as u32,
                    range: (d[5] & 0x3f) as u32,
                }
            })
            .collect()
    }

    /// The active point of `colour` and `group` nearest `at` (`EWAY_find_nearest_waypoint`).
    pub fn nearest_point(&self, at: Vec3, colour: u8, group: u8) -> Option<usize> {
        let q = |d: Vec3| {
            let d = d.abs();
            d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0
        };
        self.ways.iter().flatten()
            .filter(|w| w.active && w.ep.colour == colour && w.ep.group == group)
            .min_by(|a, b| q(a.pos() - at).total_cmp(&q(b.pos() - at)))
            .map(|w| w.ep.index)
    }

    /// The next active point of `colour` and `group` after `after`, by number, coming round
    /// again (`EWAY_find_waypoint`).
    pub fn next_point(&self, after: usize, colour: u8, group: u8) -> Option<usize> {
        let n = self.ways.len();
        (1..=n).map(|k| (after + k) % n).find(|&i| {
            self.way(i).is_some_and(|w| w.active && w.ep.colour == colour && w.ep.group == group)
        })
    }

    pub fn point_pos(&self, i: usize) -> Option<Vec3> {
        self.way(i).map(|w| w.pos())
    }

    /// How long a platform waits at point `i`, in milliseconds (`EWAY_get_delay`): a simple
    /// point's `Data[0]` in tenths of a second, `default` at anything else.
    pub fn point_delay(&self, i: usize, default: f32) -> f32 {
        match self.way(i) {
            Some(w) if w.ep.kind == waypoint::SIMPLE => w.ep.data[0] as f32 * 100.0,
            _ => default,
        }
    }

    /// Beacons that are currently shown.
    pub fn beacons(&self) -> Vec<Beacon> {
        self.ways
            .iter()
            .flatten()
            .filter(|w| w.ep.kind == waypoint::NAV_BEACON && w.active && !w.killed)
            .map(|w| Beacon {
                label: w.text.clone().unwrap_or_default(),
                pos: w.pos(),
                track: (w.ep.data[1] != 0).then_some(w.ep.data[1] as usize),
            })
            .collect()
    }
}
