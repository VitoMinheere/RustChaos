//! Missions as JSON (feature `json`): a readable, editable form of `.ucm` files.
//!
//! - Enum fields (`type`, `trigger`, `on_trigger`) are written as names such as
//!   `"create_player"` and accept either a name or a number when read back.
//! - Message text and cutscenes sit on the event point that uses them, instead of in a
//!   separate list. Their order in the `.ucm` is rebuilt from the event-point order.
//! - **Clean** export (default) keeps only what the game reads. Leftover bytes (unused
//!   event-point slots, text after string terminators, editor pointers) are dropped, so a
//!   re-imported mission behaves identically but isn't byte-identical.
//! - **Lossless** export also keeps those bytes in a `raw` section; importing it reproduces
//!   the original file exactly.

use crate::ucm::{has_text, names, waypoint, Cutscene, CutsceneChannel, CutscenePacket, EventPoint, Mission, TextEntry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT: &str = "urban-chaos-mission";
pub const FORMAT_VERSION: u32 = 1;

/// An enum value: written as its name when known, read from a name or a number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Named {
    Name(String),
    Number(u8),
}

impl Named {
    fn from(v: u8, table: &[&str]) -> Named {
        table.get(v as usize).map_or(Named::Number(v), |n| Named::Name(n.to_string()))
    }
    fn resolve(&self, table: &[&str], what: &str) -> Result<u8, String> {
        match self {
            Named::Number(n) => Ok(*n),
            Named::Name(s) => names::lookup(table, s).ok_or_else(|| format!("unknown {what} {s:?}")),
        }
    }
}

fn is_zero_u32(v: &u32) -> bool { *v == 0 }
fn is_zero_pad(v: &[u8; 2]) -> bool { *v == [0, 0] }
fn is_one(v: &u8) -> bool { *v == 1 }
fn one() -> u8 { 1 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Files {
    #[serde(default)]
    pub brief: String,
    pub lighting: String,
    pub map: String,
    #[serde(default)]
    pub mission_name: String,
    #[serde(default)]
    pub citsez: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPointJson {
    pub index: usize,
    #[serde(rename = "type")]
    pub kind: Named,
    pub trigger: Named,
    pub on_trigger: Named,
    /// Human-readable summary of `data` for common types (ignored on import).
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "_summary")]
    pub summary: Option<String>,
    pub colour: u8,
    pub group: u8,
    pub direction: u8,
    pub flags: u8,
    pub ep_ref: u16,
    pub ep_ref_bool: u16,
    pub after_timer: u16,
    pub data: [i32; 10],
    pub radius: i32,
    pub pos: [i32; 3],
    pub next: u16,
    pub prev: u16,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub used: u8,
    /// For message, nav_beacon, bonus_points and conversation points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// For cut_scene points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutscene: Option<CutsceneJson>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutsceneJson {
    pub version: u8,
    pub channels: Vec<ChannelJson>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelJson {
    /// 0 unused, 1 character, 2 camera, 3 spot sound, 4 visual effect (fallen/Headers/playcuts.h).
    pub kind: u8,
    pub flags: u8,
    pub index: u16,
    pub packets: Vec<PacketJson>,
    #[serde(default, skip_serializing_if = "is_zero_pad")]
    pub pad: [u8; 2],
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub pointer: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketJson {
    /// 1 animation, 2 action, 3 sound, 4 camera record, 5 text.
    pub kind: u8,
    pub flags: u8,
    pub index: u16,
    pub start: u16,
    pub length: u16,
    pub pos: [i32; 3],
    pub angle: u16,
    pub pitch: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Bytes the game ignores, present only in lossless exports. Byte strings are hex.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawJson {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name_tails: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub unused_slots: BTreeMap<usize, String>,
    /// Event-point index of each text entry in file order, when it differs from index order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_order: Vec<usize>,
    /// Bytes after the terminator of old fixed-size messages, by event-point index.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub message_tails: BTreeMap<usize, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionJson {
    pub format: String,
    pub format_version: u32,
    /// `.ucm` file version (10 for every shipped mission).
    pub version: i32,
    /// `MISSION_FLAG_*`: 1 used, 2 show crime rate, 4 cars with road prims.
    pub flags: i32,
    pub files: Files,
    pub map_index: u16,
    pub used_points: u16,
    pub free_points: u16,
    pub crime_rate: u8,
    pub fake_civs: u8,
    pub boredom_rate: u8,
    #[serde(default)]
    pub fake_cars: u8,
    #[serde(default)]
    pub music_world: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_levels: Vec<u8>,
    pub event_points: Vec<EventPointJson>,
    /// 128 rows (one per map x) of 128 two-digit hex `ZF_*` flags (one per z).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawJson>,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("odd-length hex string ({} chars)", s.len()));
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("bad hex: {e}"))).collect()
}

fn summary(e: &EventPoint) -> Option<String> {
    let d = e.data;
    let name = |t: &[&str], v: i32| t.get(v as usize).map_or(v.to_string(), |s| s.to_string());
    match e.kind {
        waypoint::CREATE_PLAYER => Some(format!("player {}", name(&names::PLAYER, d[0]))),
        waypoint::CREATE_ENEMIES => Some(format!(
            "{} x{}, ai {}, skill {}, move {}",
            name(&names::ENEMY, d[0] & 0xFFFF), (d[0] >> 16).max(1), d[5] & 0xFFFF, d[5] >> 16, d[3] + 1
        )),
        _ => None,
    }
}

/// Converts a mission to its JSON form. `lossless` keeps the bytes the game ignores.
pub fn to_json(m: &Mission, lossless: bool) -> MissionJson {
    // Pair text entries with their event points: the n-th message belongs to the n-th
    // text-carrying point in index order, likewise for cutscenes (as the loader counts them).
    let mut points: Vec<&EventPoint> = m.event_points.iter().collect();
    points.sort_by_key(|e| e.index);
    let text_points: Vec<usize> = points.iter().filter(|e| has_text(e.kind)).map(|e| e.index).collect();
    let cut_points: Vec<usize> = points.iter().filter(|e| e.kind == waypoint::CUT_SCENE).map(|e| e.index).collect();
    let (mut texts, mut cuts, mut tails) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    let (mut mi, mut ci) = (0, 0);
    let mut file_order = Vec::new();
    for t in &m.texts {
        match t {
            TextEntry::Message { text, tail } => {
                let idx = text_points[mi];
                mi += 1;
                texts.insert(idx, text.clone());
                if !tail.is_empty() {
                    tails.insert(idx, hex(tail));
                }
                file_order.push(idx);
            }
            TextEntry::Cutscene(c) => {
                let idx = cut_points[ci];
                ci += 1;
                cuts.insert(idx, c.clone());
                file_order.push(idx);
            }
        }
    }
    let mut index_order: Vec<usize> = file_order.clone();
    index_order.sort_unstable();

    let event_points = points
        .iter()
        .map(|e| {
            let mut data = e.data;
            if !lossless && has_text(e.kind) && data[0] != 0 {
                data[0] = 1; // an editor pointer on disk; the game only needs "nonzero"
            }
            EventPointJson {
                index: e.index,
                kind: Named::from(e.kind, &names::WAYPOINT),
                trigger: Named::from(e.trigger, &names::TRIGGER),
                on_trigger: Named::from(e.on_trigger, &names::ON_TRIGGER),
                summary: summary(e),
                colour: e.colour,
                group: e.group,
                direction: e.direction,
                flags: e.flags,
                ep_ref: e.ep_ref,
                ep_ref_bool: e.ep_ref_bool,
                after_timer: e.after_timer,
                data,
                radius: e.radius,
                pos: [e.x, e.y, e.z],
                next: e.next,
                prev: e.prev,
                used: if lossless { e.used } else { 1 },
                text: texts.get(&e.index).cloned(),
                cutscene: cuts.get(&e.index).map(|c| cutscene_json(c, lossless)),
            }
        })
        .collect();

    let raw = lossless.then(|| RawJson {
        name_tails: if m.name_tails.iter().any(|t| !t.is_empty()) { m.name_tails.iter().map(|t| hex(t)).collect() } else { Vec::new() },
        unused_slots: m.unused_slots.iter().map(|(i, b)| (*i, hex(b))).collect(),
        text_order: if file_order != index_order { file_order } else { Vec::new() },
        message_tails: tails,
    });

    MissionJson {
        format: FORMAT.into(),
        format_version: FORMAT_VERSION,
        version: m.version,
        flags: m.flags,
        files: Files {
            brief: m.brief_name.clone(),
            lighting: m.lighting.clone(),
            map: m.map.clone(),
            mission_name: m.mission_name.clone(),
            citsez: m.citsez.clone(),
        },
        map_index: m.map_index,
        used_points: m.used_points,
        free_points: m.free_points,
        crime_rate: m.crime_rate,
        fake_civs: m.fake_civs,
        boredom_rate: m.boredom_rate,
        fake_cars: m.fake_cars,
        music_world: m.music_world,
        skill_levels: m.skill_levels.clone(),
        event_points,
        zones: m.zones.chunks(128).map(hex).collect(),
        raw: raw.filter(|r| *r != RawJson::default()),
    }
}

fn cutscene_json(c: &Cutscene, lossless: bool) -> CutsceneJson {
    CutsceneJson {
        version: c.version,
        channels: c
            .channels
            .iter()
            .map(|ch| ChannelJson {
                kind: ch.kind,
                flags: ch.flags,
                index: ch.index,
                pad: if lossless { ch.pad } else { [0, 0] },
                pointer: if lossless { ch.pointer } else { 0 },
                packets: ch
                    .packets
                    .iter()
                    .map(|p| PacketJson {
                        kind: p.kind, flags: p.flags, index: p.index, start: p.start, length: p.length,
                        pos: p.pos, angle: p.angle, pitch: p.pitch, text: p.text.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Converts JSON back into a mission (ready for `Mission::write`).
pub fn from_json(j: &MissionJson) -> Result<Mission, String> {
    if j.format != FORMAT {
        return Err(format!("not a mission file (format {:?}, expected {FORMAT:?})", j.format));
    }
    if j.format_version > FORMAT_VERSION {
        return Err(format!("format_version {} is newer than this tool ({FORMAT_VERSION})", j.format_version));
    }
    let raw = j.raw.clone().unwrap_or_default();
    let mut event_points = Vec::with_capacity(j.event_points.len());
    let mut texts: BTreeMap<usize, TextEntry> = BTreeMap::new();
    for e in &j.event_points {
        let kind = e.kind.resolve(&names::WAYPOINT, "event point type")?;
        let at = |msg: String| format!("event point {}: {msg}", e.index);
        if has_text(kind) {
            let tail = match raw.message_tails.get(&e.index) { Some(h) => unhex(h).map_err(at)?, None => Vec::new() };
            texts.insert(e.index, TextEntry::Message { text: e.text.clone().unwrap_or_default(), tail });
        } else if e.text.is_some() {
            return Err(at("only message, nav_beacon, bonus_points and conversation points can have text".into()));
        }
        if kind == waypoint::CUT_SCENE {
            let c = e.cutscene.as_ref().ok_or_else(|| at("cut_scene point without a cutscene".into()))?;
            texts.insert(e.index, TextEntry::Cutscene(cutscene_from(c)));
        } else if e.cutscene.is_some() {
            return Err(at("only cut_scene points can have a cutscene".into()));
        }
        event_points.push(EventPoint {
            index: e.index,
            colour: e.colour,
            group: e.group,
            kind,
            used: e.used,
            trigger: e.trigger.resolve(&names::TRIGGER, "trigger").map_err(at)?,
            on_trigger: e.on_trigger.resolve(&names::ON_TRIGGER, "on_trigger").map_err(at)?,
            direction: e.direction,
            flags: e.flags,
            ep_ref: e.ep_ref,
            ep_ref_bool: e.ep_ref_bool,
            after_timer: e.after_timer,
            data: e.data,
            radius: e.radius,
            x: e.pos[0],
            y: e.pos[1],
            z: e.pos[2],
            next: e.next,
            prev: e.prev,
        });
    }
    let order: Vec<usize> = if raw.text_order.is_empty() { texts.keys().copied().collect() } else { raw.text_order.clone() };
    let mut ordered = Vec::with_capacity(texts.len());
    for idx in order {
        ordered.push(texts.remove(&idx).ok_or_else(|| format!("raw.text_order refers to event point {idx}, which has no text"))?);
    }
    if !texts.is_empty() {
        return Err("raw.text_order is missing some text entries".into());
    }

    let mut name_tails: [Vec<u8>; 5] = Default::default();
    for (i, t) in raw.name_tails.iter().enumerate().take(5) {
        name_tails[i] = unhex(t)?;
    }
    let mut unused_slots = Vec::new();
    for (i, h) in &raw.unused_slots {
        let b = unhex(h)?;
        let a: [u8; 74] = b.try_into().map_err(|_| format!("raw.unused_slots[{i}] is not 74 bytes"))?;
        unused_slots.push((*i, a));
    }
    let mut zones = Vec::with_capacity(128 * 128);
    for (row, h) in j.zones.iter().enumerate() {
        let b = unhex(h).map_err(|e| format!("zones row {row}: {e}"))?;
        if b.len() != 128 {
            return Err(format!("zones row {row} has {} cells, expected 128", b.len()));
        }
        zones.extend(b);
    }
    if !j.zones.is_empty() && j.zones.len() != 128 {
        return Err(format!("zones has {} rows, expected 128", j.zones.len()));
    }
    let m = Mission {
        version: j.version,
        flags: j.flags,
        brief_name: j.files.brief.clone(),
        lighting: j.files.lighting.clone(),
        map: j.files.map.clone(),
        mission_name: j.files.mission_name.clone(),
        citsez: j.files.citsez.clone(),
        name_tails,
        map_index: j.map_index,
        used_points: j.used_points,
        free_points: j.free_points,
        crime_rate: j.crime_rate,
        fake_civs: j.fake_civs,
        event_points,
        unused_slots,
        skill_levels: j.skill_levels.clone(),
        boredom_rate: j.boredom_rate,
        fake_cars: j.fake_cars,
        music_world: j.music_world,
        texts: ordered,
        zones,
    };
    m.validate()?;
    Ok(m)
}

fn cutscene_from(c: &CutsceneJson) -> Cutscene {
    Cutscene {
        version: c.version,
        channels: c
            .channels
            .iter()
            .map(|ch| CutsceneChannel {
                kind: ch.kind,
                flags: ch.flags,
                pad: ch.pad,
                index: ch.index,
                pointer: ch.pointer,
                packets: ch
                    .packets
                    .iter()
                    .map(|p| CutscenePacket {
                        kind: p.kind, flags: p.flags, index: p.index, start: p.start, length: p.length,
                        pos: p.pos, angle: p.angle, pitch: p.pitch, text: p.text.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}
