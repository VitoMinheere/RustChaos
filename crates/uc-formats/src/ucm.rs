//! `.ucm` missions: `ELEV_load_level` (fallen/Source/elev.cpp:193). See docs/MISSIONS.md.
//!
//! The model is lossless: besides what the game reads, it keeps the bytes the game ignores
//! (text after string terminators, leftover data in unused event-point slots, cutscene
//! padding), so `Mission::write` reproduces a parsed file byte for byte. Those extras are
//! optional: a mission built from scratch can leave them empty.

use crate::reader::{count, latin1, Reader, Result, Writer};

pub const MAX_EVENT_POINTS: usize = 512;
pub const NAME_SIZE: usize = 260;
pub const ZONE_SIZE: usize = 128 * 128;
pub const SKILL_LEVELS: usize = 254;

/// `WPT_*`: what an event point does (fallen/Headers/Mission.h; names in [`names::WAYPOINT`]).
pub mod waypoint {
    pub const SIMPLE: u8 = 1;
    pub const CREATE_PLAYER: u8 = 2;
    pub const CREATE_ENEMIES: u8 = 3;
    pub const CREATE_VEHICLE: u8 = 4;
    pub const CREATE_ITEM: u8 = 5;
    pub const CREATE_CREATURE: u8 = 6;
    pub const CREATE_TREASURE: u8 = 31;
    pub const CREATE_CAMERA: u8 = 7;
    pub const CREATE_TARGET: u8 = 8;
    pub const CAMERA_WAYPOINT: u8 = 10;
    pub const MESSAGE: u8 = 12;
    pub const VISUAL_EFFECT: u8 = 14;
    pub const ACTIVATE_PRIM: u8 = 20;
    pub const CREATE_TRAP: u8 = 21;
    pub const CREATE_MIST: u8 = 47;
    pub const SOUND_EFFECT: u8 = 13;
    pub const CUT_SCENE: u8 = 15;
    pub const END_GAME_LOSE: u8 = 18;
    pub const ADJUST_ENEMY: u8 = 22;
    pub const LINK_PLATFORM: u8 = 23;
    pub const END_GAME_WIN: u8 = 26;
    pub const NAV_BEACON: u8 = 27;
    pub const SPOT_EFFECT: u8 = 28;
    pub const CREATE_BARREL: u8 = 29;
    pub const KILL_WAYPOINT: u8 = 30;
    pub const BONUS_POINTS: u8 = 32;
    pub const GROUP_LIFE: u8 = 33;
    pub const GROUP_DEATH: u8 = 34;
    pub const CONVERSATION: u8 = 35;
    pub const TRANSFER_PLAYER: u8 = 40;
    pub const INCREMENT: u8 = 37;
    pub const RESET_COUNTER: u8 = 46;
    pub const GROUP_RESET: u8 = 44;
    pub const EXTEND: u8 = 50;
    pub const MOVE_THING: u8 = 51;
    pub const MAKE_PERSON_PEE: u8 = 52;
    pub const STALL_CAR: u8 = 49;
    pub const MAKE_SEARCHABLE: u8 = 42;
    pub const LOCK_VEHICLE: u8 = 43;
    pub const SHAKE_CAMERA: u8 = 57;
    pub const CONE_PENALTIES: u8 = 53;
    pub const SIGN: u8 = 54;
    pub const WAREFX: u8 = 55;
}

/// `TT_*`: what triggers an event point (names in [`names::TRIGGER`]).
pub mod trigger {
    pub const NONE: u8 = 0;
    pub const DEPENDENCY: u8 = 1;
    pub const RADIUS: u8 = 2;
    pub const TRIPWIRE: u8 = 4;
    pub const TIMER: u8 = 11;
    pub const BOOLEAN_AND: u8 = 13;
    pub const BOOLEAN_OR: u8 = 14;
    pub const KILLED: u8 = 17;
    pub const COUNTDOWN: u8 = 19;
    pub const VISIBLE_COUNTDOWN: u8 = 21;
    pub const CUBOID: u8 = 22;
    pub const ENEMY_RADIUS: u8 = 20;
    pub const HALF_DEAD: u8 = 23;
    pub const GROUP_DEAD: u8 = 24;
    pub const PERSON_SEEN: u8 = 25;
    pub const PERSON_USED: u8 = 26;
    pub const PLAYER_USES_RADIUS: u8 = 27;
    pub const PERSON_ARRESTED: u8 = 29;
    pub const CONVERSATION_OVER: u8 = 30;
    pub const COUNTER: u8 = 31;
    pub const KILLED_NOT_ARRESTED: u8 = 32;
    pub const PERSON_IN_VEHICLE: u8 = 36;
    pub const ITEM_HELD: u8 = 15;
    pub const SPECIFIC_ITEM_HELD: u8 = 39;
    pub const PLAYER_FIRES_GUN: u8 = 41;
    pub const PRIM_DAMAGED: u8 = 28;
    pub const THING_RADIUS_DIR: u8 = 37;
    pub const RANDOM: u8 = 40;
    pub const MOVE_RADIUS_DIR: u8 = 44;
    pub const DARCI_GRABBED: u8 = 42;
    pub const PUNCHED_AND_KICKED: u8 = 43;
}

/// `OT_*`: what a triggered event point does afterwards (names in [`names::ON_TRIGGER`]).
pub mod on_trigger {
    pub const NONE: u8 = 0;
    pub const ACTIVE: u8 = 1;
    pub const ACTIVE_WHILE: u8 = 2;
    pub const ACTIVE_TIME: u8 = 3;
    pub const ACTIVE_DIE: u8 = 4;
}

/// Event point types that carry one message string each (counted as `mess_count` by the loader).
pub fn has_text(kind: u8) -> bool {
    matches!(kind, waypoint::MESSAGE | waypoint::NAV_BEACON | waypoint::BONUS_POINTS | waypoint::CONVERSATION)
}

/// One used `EventPoint` (74 bytes on disk).
#[derive(Debug, Clone, PartialEq)]
pub struct EventPoint {
    pub index: usize,
    pub colour: u8,
    pub group: u8,
    pub kind: u8,
    /// Normally 1; any nonzero value marks the slot as used.
    pub used: u8,
    pub trigger: u8,
    pub on_trigger: u8,
    pub direction: u8,
    pub flags: u8,
    pub ep_ref: u16,
    pub ep_ref_bool: u16,
    pub after_timer: u16,
    pub data: [i32; 10],
    pub radius: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub next: u16,
    pub prev: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CutscenePacket {
    pub kind: u8,
    pub flags: u8,
    pub index: u16,
    pub start: u16,
    pub length: u16,
    pub pos: [i32; 3],
    pub angle: u16,
    pub pitch: u16,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CutsceneChannel {
    pub kind: u8,
    pub flags: u8,
    /// Padding bytes, kept for byte-exact round trips.
    pub pad: [u8; 2],
    pub index: u16,
    /// The 1999 editor's in-memory pointer, meaningless on disk; kept for round trips.
    pub pointer: u32,
    pub packets: Vec<CutscenePacket>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cutscene {
    pub version: u8,
    pub channels: Vec<CutsceneChannel>,
}

/// One entry of the text section, in file order.
#[derive(Debug, Clone, PartialEq)]
pub enum TextEntry {
    /// `tail` holds bytes after the terminator for old fixed-size (260-byte) messages.
    Message { text: String, tail: Vec<u8> },
    Cutscene(Cutscene),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mission {
    pub version: i32,
    pub flags: i32,
    pub brief_name: String,
    pub lighting: String,
    /// e.g. `Data\disturb1.iam`, relative to the install directory.
    pub map: String,
    pub mission_name: String,
    pub citsez: String,
    /// Bytes after the terminator of each of the five name fields (usually empty).
    pub name_tails: [Vec<u8>; 5],
    pub map_index: u16,
    pub used_points: u16,
    pub free_points: u16,
    pub crime_rate: u8,
    pub fake_civs: u8,
    pub event_points: Vec<EventPoint>,
    /// Unused slots that still hold data (editor leftovers the game ignores), by slot index.
    pub unused_slots: Vec<(usize, [u8; 74])>,
    /// Per-AI-type skill levels (version > 5); ignored by the game.
    pub skill_levels: Vec<u8>,
    pub boredom_rate: u8,
    pub fake_cars: u8,
    pub music_world: u8,
    /// Messages and cutscenes, in file order.
    pub texts: Vec<TextEntry>,
    /// `ZF_*` flags, x-major 128x128 (version >= 2).
    pub zones: Vec<u8>,
}

impl Mission {
    /// World position of the first `create_player` event point.
    pub fn player_start(&self) -> Option<[i32; 3]> {
        self.event_points
            .iter()
            .find(|e| e.kind == waypoint::CREATE_PLAYER)
            .map(|e| [e.x, e.y, e.z])
    }

    pub fn messages(&self) -> impl Iterator<Item = &str> {
        self.texts.iter().filter_map(|t| match t {
            TextEntry::Message { text, .. } => Some(text.as_str()),
            _ => None,
        })
    }

    /// Message text by event-point index: the n-th message belongs to the n-th text-carrying
    /// point in index order (the loader's `mess_count`).
    pub fn point_texts(&self) -> std::collections::BTreeMap<usize, String> {
        let mut points: Vec<usize> = self.event_points.iter().filter(|e| has_text(e.kind)).map(|e| e.index).collect();
        points.sort_unstable();
        points.into_iter().zip(self.messages()).map(|(i, t)| (i, t.to_string())).collect()
    }

    pub fn cutscenes(&self) -> impl Iterator<Item = &Cutscene> {
        self.texts.iter().filter_map(|t| match t {
            TextEntry::Cutscene(c) => Some(c),
            _ => None,
        })
    }

    pub fn parse(data: &[u8]) -> Result<Mission> {
        let mut r = Reader::new(data);
        let version = r.i32()?;
        let flags = r.i32()?;
        let mut names: [String; 5] = Default::default();
        let mut name_tails: [Vec<u8>; 5] = Default::default();
        for i in 0..5 {
            let (text, tail) = split_cstr(r.take(NAME_SIZE)?);
            names[i] = text;
            name_tails[i] = tail;
        }
        let [brief_name, lighting, map, mission_name, citsez] = names;
        let (map_index, used_points, free_points) = (r.u16()?, r.u16()?, r.u16()?);
        let crime_rate = r.u8()?;
        let fake_civs = r.u8()?;

        let mut event_points = Vec::new();
        let mut unused_slots = Vec::new();
        let (mut n_msg, mut n_cut) = (0usize, 0usize);
        for index in 0..MAX_EVENT_POINTS {
            let raw = r.take(74)?;
            let mut rr = Reader::new(raw);
            let h = rr.take(8)?;
            let (colour, group, kind, used, trigger, on_trigger, direction, ep_flags) =
                (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
            let (ep_ref, ep_ref_bool, after_timer) = (rr.u16()?, rr.u16()?, rr.u16()?);
            let mut d = [0i32; 10];
            for v in d.iter_mut() {
                *v = rr.i32()?;
            }
            let (radius, x, y, z) = (rr.i32()?, rr.i32()?, rr.i32()?, rr.i32()?);
            let (next, prev) = (rr.u16()?, rr.u16()?);
            if used == 0 {
                if raw.iter().any(|&b| b != 0) {
                    let mut a = [0u8; 74];
                    a.copy_from_slice(raw);
                    unused_slots.push((index, a));
                }
                continue;
            }
            n_msg += has_text(kind) as usize;
            n_cut += (kind == waypoint::CUT_SCENE) as usize;
            event_points.push(EventPoint {
                index, colour, group, kind, used, trigger, on_trigger, direction, flags: ep_flags,
                ep_ref, ep_ref_bool, after_timer, data: d, radius, x, y, z, next, prev,
            });
        }
        let skill_levels = if version > 5 { r.take(SKILL_LEVELS)?.to_vec() } else { Vec::new() };
        let boredom_rate = r.u8()?;
        let (mut fake_cars, mut music_world) = (0, 0);
        if version > 8 {
            fake_cars = r.u8()?;
            music_world = r.u8()?;
        }
        let mut texts = Vec::new();
        if version < 8 {
            for _ in 0..n_msg {
                if version > 4 {
                    let v = r.i32()?;
                    let n = count(&mut r, v)?;
                    texts.push(TextEntry::Message { text: latin1(r.take(n)?), tail: Vec::new() });
                } else {
                    let (text, tail) = split_cstr(r.take(NAME_SIZE)?);
                    texts.push(TextEntry::Message { text, tail });
                }
            }
        } else {
            for _ in 0..n_msg + n_cut {
                match r.u8()? {
                    1 => {
                        let v = r.i32()?;
                        let n = count(&mut r, v)?;
                        texts.push(TextEntry::Message { text: latin1(r.take(n)?), tail: Vec::new() });
                    }
                    2 => texts.push(TextEntry::Cutscene(read_cutscene(&mut r)?)),
                    k => return r.error(format!("unknown text entry kind {k}")),
                }
            }
        }
        let zones = if version >= 2 { r.take(ZONE_SIZE)?.to_vec() } else { Vec::new() };
        r.expect_end()?;
        Ok(Mission {
            version, flags, brief_name, lighting, map, mission_name, citsez, name_tails,
            map_index, used_points, free_points, crime_rate, fake_civs, event_points, unused_slots,
            skill_levels, boredom_rate, fake_cars, music_world, texts, zones,
        })
    }

    /// Checks the invariants the game's loader relies on.
    pub fn validate(&self) -> std::result::Result<(), String> {
        let mut seen = [false; MAX_EVENT_POINTS];
        for e in &self.event_points {
            if e.index >= MAX_EVENT_POINTS {
                return Err(format!("event point index {} is out of range (max {})", e.index, MAX_EVENT_POINTS - 1));
            }
            if std::mem::replace(&mut seen[e.index], true) {
                return Err(format!("event point index {} is used twice", e.index));
            }
            if e.used == 0 {
                return Err(format!("event point {} has used = 0", e.index));
            }
        }
        let n_msg = self.event_points.iter().filter(|e| has_text(e.kind)).count();
        let n_cut = self.event_points.iter().filter(|e| e.kind == waypoint::CUT_SCENE).count();
        let (m, c) = (self.messages().count(), self.cutscenes().count());
        if m != n_msg || c != n_cut {
            return Err(format!(
                "{n_msg} text event points and {n_cut} cut-scene points need {n_msg} messages and {n_cut} cutscenes, found {m} and {c}"
            ));
        }
        if self.version < 8 && c > 0 {
            return Err("cutscenes need version >= 8".into());
        }
        if self.version > 5 && self.skill_levels.len() != SKILL_LEVELS {
            return Err(format!("version {} needs {SKILL_LEVELS} skill levels", self.version));
        }
        if self.version >= 2 && self.zones.len() != ZONE_SIZE {
            return Err(format!("version {} needs a {ZONE_SIZE}-byte zone map", self.version));
        }
        Ok(())
    }

    /// Serialises the mission in the game's format (the inverse of `parse`).
    pub fn write(&self) -> std::result::Result<Vec<u8>, String> {
        self.validate()?;
        let mut w = Writer::default();
        w.i32(self.version);
        w.i32(self.flags);
        let names = [&self.brief_name, &self.lighting, &self.map, &self.mission_name, &self.citsez];
        for (name, tail) in names.iter().zip(&self.name_tails) {
            w.cstr(name, tail, NAME_SIZE)?;
        }
        w.u16(self.map_index);
        w.u16(self.used_points);
        w.u16(self.free_points);
        w.u8(self.crime_rate);
        w.u8(self.fake_civs);

        let mut slots: Vec<Option<[u8; 74]>> = vec![None; MAX_EVENT_POINTS];
        for (i, raw) in &self.unused_slots {
            if *i < MAX_EVENT_POINTS {
                slots[*i] = Some(*raw);
            }
        }
        for e in &self.event_points {
            let mut ew = Writer::default();
            ew.raw(&[e.colour, e.group, e.kind, e.used, e.trigger, e.on_trigger, e.direction, e.flags]);
            ew.u16(e.ep_ref);
            ew.u16(e.ep_ref_bool);
            ew.u16(e.after_timer);
            for v in e.data {
                ew.i32(v);
            }
            for v in [e.radius, e.x, e.y, e.z] {
                ew.i32(v);
            }
            ew.u16(e.next);
            ew.u16(e.prev);
            let mut a = [0u8; 74];
            a.copy_from_slice(&ew.bytes);
            slots[e.index] = Some(a);
        }
        for s in slots {
            w.raw(&s.unwrap_or([0u8; 74]));
        }
        if self.version > 5 {
            w.raw(&self.skill_levels);
        }
        w.u8(self.boredom_rate);
        if self.version > 8 {
            w.u8(self.fake_cars);
            w.u8(self.music_world);
        }
        for t in &self.texts {
            match t {
                TextEntry::Message { text, tail } => {
                    if self.version >= 8 {
                        w.u8(1);
                    }
                    if self.version > 4 {
                        let b = crate::reader::to_latin1(text)?;
                        w.i32(b.len() as i32);
                        w.raw(&b);
                    } else {
                        w.cstr(text, tail, NAME_SIZE)?;
                    }
                }
                TextEntry::Cutscene(c) => {
                    w.u8(2);
                    write_cutscene(&mut w, c)?;
                }
            }
        }
        if self.version >= 2 {
            w.raw(&self.zones);
        }
        Ok(w.bytes)
    }
}

/// Splits a fixed-size string field into its text and whatever follows the terminator
/// (empty if that is all zeros).
fn split_cstr(b: &[u8]) -> (String, Vec<u8>) {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let tail = if end < b.len() { &b[end + 1..] } else { &[][..] };
    let tail = if tail.iter().any(|&c| c != 0) { tail.to_vec() } else { Vec::new() };
    (latin1(&b[..end]), tail)
}

/// `PLAYCUTS_Read` (fallen/Source/playcuts.cpp).
fn read_cutscene(r: &mut Reader) -> Result<Cutscene> {
    let version = r.u8()?;
    let nch = r.u8()?;
    let mut channels = Vec::new();
    for _ in 0..nch {
        let (kind, flags) = (r.u8()?, r.u8()?);
        let pad = [r.u8()?, r.u8()?];
        let (index, npackets) = (r.u16()?, r.u16()?);
        let pointer = r.u32()?;
        let mut packets = Vec::new();
        for _ in 0..npackets {
            let (pkind, pflags) = (r.u8()?, r.u8()?);
            let (pindex, start, length) = (r.u16()?, r.u16()?, r.u16()?);
            let pos = [r.i32()?, r.i32()?, r.i32()?];
            let (angle, pitch) = (r.u16()?, r.u16()?);
            let text = if pkind == 5 && pos[0] != 0 {
                let v = r.i32()?;
                let n = count(r, v)?;
                Some(latin1(r.take(n)?))
            } else {
                None
            };
            packets.push(CutscenePacket { kind: pkind, flags: pflags, index: pindex, start, length, pos, angle, pitch, text });
        }
        channels.push(CutsceneChannel { kind, flags, pad, index, pointer, packets });
    }
    Ok(Cutscene { version, channels })
}

fn write_cutscene(w: &mut Writer, c: &Cutscene) -> std::result::Result<(), String> {
    w.u8(c.version);
    w.u8(u8::try_from(c.channels.len()).map_err(|_| "a cutscene has more than 255 channels")?);
    for ch in &c.channels {
        w.u8(ch.kind);
        w.u8(ch.flags);
        w.raw(&ch.pad);
        w.u16(ch.index);
        w.u16(u16::try_from(ch.packets.len()).map_err(|_| "a cutscene channel has too many packets")?);
        w.u32(ch.pointer);
        for p in &ch.packets {
            w.u8(p.kind);
            w.u8(p.flags);
            w.u16(p.index);
            w.u16(p.start);
            w.u16(p.length);
            // A text packet's pos.x is nonzero exactly when text follows.
            let has_text = p.kind == 5 && p.text.is_some();
            let x = if p.kind == 5 { if has_text && p.pos[0] == 0 { 1 } else if !has_text { 0 } else { p.pos[0] } } else { p.pos[0] };
            w.i32(x);
            w.i32(p.pos[1]);
            w.i32(p.pos[2]);
            w.u16(p.angle);
            w.u16(p.pitch);
            if let (5, Some(t)) = (p.kind, &p.text) {
                let b = crate::reader::to_latin1(t)?;
                w.i32(b.len() as i32);
                w.raw(&b);
            }
        }
    }
    Ok(())
}

/// Names for mission enums, from fallen/Headers/Mission.h (lower-case, without prefixes).
pub mod names {
    /// `WPT_*`: what an event point does.
    pub const WAYPOINT: [&str; 58] = [
        "none", "simple", "create_player", "create_enemies", "create_vehicle", "create_item",
        "create_creature", "create_camera", "create_target", "create_map_exit", "camera_waypoint",
        "target_waypoint", "message", "sound_effect", "visual_effect", "cut_scene", "teleport",
        "teleport_target", "end_game_lose", "shout", "activate_prim", "create_trap", "adjust_enemy",
        "link_platform", "create_bomb", "burn_prim", "end_game_win", "nav_beacon", "spot_effect",
        "create_barrel", "kill_waypoint", "create_treasure", "bonus_points", "group_life",
        "group_death", "conversation", "interesting", "increment", "dynamic_light", "gothere_dothis",
        "transfer_player", "autosave", "make_searchable", "lock_vehicle", "group_reset",
        "count_up_timer", "reset_counter", "create_mist", "enemy_flags", "stall_car", "extend",
        "move_thing", "make_person_pee", "cone_penalties", "sign", "warefx", "no_floor", "shake_camera",
    ];
    /// `TT_*`: what triggers an event point.
    pub const TRIGGER: [&str; 45] = [
        "none", "dependency", "radius", "door", "tripwire", "pressure_pad", "electric_fence",
        "water_level", "security_camera", "switch", "anim_prim", "timer", "shout_all", "boolean_and",
        "boolean_or", "item_held", "item_seen", "killed", "shout_any", "countdown", "enemy_radius",
        "visible_countdown", "cuboid", "half_dead", "group_dead", "person_seen", "person_used",
        "player_uses_radius", "prim_damaged", "person_arrested", "conversation_over", "counter",
        "killed_not_arrested", "crime_rate_above", "crime_rate_below", "person_is_murderer",
        "person_in_vehicle", "thing_radius_dir", "player_carry_person", "specific_item_held",
        "random", "player_fires_gun", "darci_grabbed", "punched_and_kicked", "move_radius_dir",
    ];
    /// `OT_*`: how a triggered event point behaves afterwards.
    pub const ON_TRIGGER: [&str; 5] = ["none", "active", "active_while", "active_time", "active_die"];
    /// `PT_*`: player types (create_player data[0]).
    pub const PLAYER: [&str; 5] = ["none", "darci", "roper", "cop", "gang"];
    /// `ET_*`: enemy types (create_enemies data[0] low 16 bits).
    pub const ENEMY: [&str; 24] = [
        "none", "civ", "civ_balloon", "slag", "ugly_fat_slag", "workman", "gang_rasta",
        "gang_red", "gang_grey", "gang_rasta_pistol", "gang_red_shotgun", "gang_grey_ak47", "cop",
        "cop_pistol", "cop_shotgun", "cop_ak47", "hostage", "workman_grenade", "tramp", "mib1",
        "mib2", "mib3", "darci", "roper",
    ];

    pub fn lookup(table: &[&str], name: &str) -> Option<u8> {
        table.iter().position(|n| n.eq_ignore_ascii_case(name)).map(|i| i as u8)
    }
}
