//! Loading a mission's level from an install and turning it into geometry.

use uc_formats::units::radians;
use crate::crinkle::ReliefQuad;
use crate::geometry::{Builder, Layer, WorldGeometry, UNIT, UNTEXTURED};
use glam::{vec3, Vec3};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use crate::character::{ep_angle, People};
use crate::ground::Ground;
use std::sync::Arc;
use uc_formats::all::AnimChunk;
use uc_formats::facet;
use uc_formats::iam::{facet_type, flags, Map, BUILDING_TYPE_WAREHOUSE, KERB_HEIGHT, MAP_SIZE};
use uc_formats::lgt::{Lighter, Lighting};
use uc_formats::prm::Prim;
use uc_formats::tma::StyleTable;
use uc_formats::txc::{entry_page, Clump, Texture};
use uc_formats::ucm::{waypoint, Mission};
use uc_formats::resolve;

/// `ROOF_SHIFT` (Editor/Headers/Prim.h): RoofFace4.DY is in units of 8.
const ROOF_SHIFT: i32 = 3;

/// Floor quad corner k sits at (x + (k & 1), z + (k >> 1)); rotation picks the UV set
/// (`TEXTURE_get_minitexturebits_uvs`, DDEngine/Source/texture.cpp:2292).
pub const FLOOR_UVS: [[[f32; 2]; 4]; 4] = [
    [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
    [[1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 1.0]],
    [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]],
];

pub struct LoadOptions {
    /// Ignore the level's baked lighting.
    pub fullbright: bool,
    /// Load people's models and animations.
    pub character: bool,
    /// Folder of `page_NNNN.png` files that replace clump textures (see [`override_path`]).
    pub overrides: Option<PathBuf>,
}

pub struct Level {
    pub game: PathBuf,
    pub name: String,
    pub mission: Mission,
    pub map: Map,
    pub clump: Clump,
    pub styles: Option<StyleTable>,
    pub lighting: Option<Lighting>,
    /// Surfaces, walls and ladders people move on.
    pub ground: Ground,
    /// Darci's start: feet position in world units and angle in radians (facing (-sin, -cos)).
    pub spawn: (Vec3, f32),
    pub sky: [u8; 3],
    pub people: Option<People>,
    /// Prims for items and barrels that missions place (`ITEM_PRIMS`).
    pub item_prims: HashMap<u8, Prim>,
    /// Weapons as held (256 + `PersonID >> 5` - 1: pistol, knife, shotgun, bat, AK-47) and
    /// muzzle flashes (261 pistol, 262 shotgun, 263 AK-47), modelled in the hand's space.
    pub weapon_prims: HashMap<u16, Prim>,
    /// The mission's moving platforms: (the `link_platform` point, the prop it moves, as an
    /// index into the map's objects). The game draws those props, not the level.
    pub platforms: Vec<(usize, usize)>,
    /// Props that are barrels, which the game turns into barrels it moves and blows up
    /// (`OB_convert_dustbins_to_barrels`, fallen/Source/ob.cpp:1627): as indices into the
    /// map's objects. They're left out of the scenery and of what people walk into.
    pub barrel_props: Vec<usize>,
    /// The world pages whose pictures aren't this world's own: a mission with no clump
    /// borrows them from another world by number ([`borrowed_clump`]). Empty when the
    /// mission has a clump of its own.
    pub stand_in_pages: BTreeSet<u32>,
    overrides: Option<PathBuf>,
    /// The prim texture pages that see through (`textype.txt`'s T, M, A or D, or the page
    /// after an I), counted from [`PRIM_PAGES`].
    see_through_prims: BTreeSet<u32>,
}

/// Where the prims' texture pages start (`POLY_load_texture_flags(..., 11 * 64)`,
/// texture.cpp:712).
pub const PRIM_PAGES: u32 = 11 * 64;
/// Where the prims' pages end and the extra pages begin (the sky, the moon, the leaves,
/// rubbish and snowflakes from `extras`, `TEXTURE_NUM_STANDARD`), which follow no
/// `textype.txt` of the prims'.
pub const EXTRA_PAGES: u32 = 22 * 64;

/// Whether `page` is one of the prims' pages, which follow `shared/prims/textype.txt`.
pub fn is_prim_page(page: u32) -> bool {
    (PRIM_PAGES..EXTRA_PAGES).contains(&page)
}

/// The pages `textype.txt` makes see-through (`POLY_load_texture_flags`,
/// polyrenderstate.cpp:82): 'T' black is transparent, 'M' the alpha channel, 'A' additive,
/// 'D' alpha masked; and 'I' makes the next page a transparent self-illumination map. The
/// rest are drawn solid, whatever their alpha channel holds.
pub fn see_through_pages(text: &str) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("Page ") else { continue };
        let Some((n, flags)) = rest.split_once(':') else { continue };
        let Ok(n) = n.trim().parse::<u32>() else { continue };
        let flags = flags.to_ascii_uppercase();
        if flags.contains(['T', 'M', 'A', 'D']) {
            out.insert(n);
        }
        if flags.contains('I') {
            out.insert(n + 1);
        }
    }
    out
}

fn read(game: &Path, rel: &str) -> Result<Vec<u8>, String> {
    let p = resolve(game, rel).ok_or(format!("{rel} not found in {}", game.display()))?;
    std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))
}

/// Whether texture page `p` belongs to the map's world (`TEXTURE_load_needed`,
/// DDEngine/Source/texture.cpp:770): pages 0-255 come from `world<n>\`, 512-575 from
/// `world<n>\insides\`. The rest are shared by every world.
pub fn world_page(p: u32) -> bool {
    p < 64 * 4 || (64 * 8..64 * 9).contains(&p)
}

/// Maps whose climbable fences come out as the wrong picture, and the page they're drawn
/// with instead. `multi1.iam` (Capflag1, the network game's map) has no clump of its own,
/// and its fences' styles point at world 3's window and door pages: they take the chain
/// link the downtown missions' fences use (RTA's 284, a shared page every world has).
const REFENCED: [(&str, u32); 1] = [("data\\multi1.iam", 284)];

/// The page the fences of `map_file` are drawn with instead of their own, if any.
fn fence_page(map_file: &str) -> Option<u32> {
    REFENCED.iter().find(|(m, _)| m.eq_ignore_ascii_case(map_file)).map(|&(_, p)| p)
}

/// The world whose textures stand in for a world no shipped mission uses (`freeoncd1.iam`
/// and `training1.iam` are world 2): the city of RTA and the other downtown missions. It
/// also stands in for world 6 (`subdock1.iam`'s), which the install has nothing of at all,
/// not even the style table its walls are drawn from.
const STAND_IN_WORLD: i32 = 3;

/// Maps whose water is a page of their own world rather than [`WATER_PAGE`], and that page.
/// `subdock1.iam`'s harbour is world 6's page 242, which no install has: it becomes the sea
/// of the island maps (`jung3`, `skymap30`), moving with the waves.
const SEAS: [(&str, u16); 1] = [("data\\subdock1.iam", 242)];

/// Gives `map_file`'s water, if it's listed in [`SEAS`], the sea's page.
fn to_sea(map_file: &str, map: &mut Map) {
    let Some(&(_, page)) = SEAS.iter().find(|(m, _)| m.eq_ignore_ascii_case(map_file)) else { return };
    for sq in map.squares.iter_mut().filter(|sq| sq.page() == page) {
        sq.texture = (sq.texture & !0x3FF) | WATER_PAGE;
    }
}

/// The missions that send Mako into the store on Gangland's map (`gang1.iam`): the man with
/// the drop-off's files and the shoppers are inside it, on its floor 256 up.
const STORE_MISSIONS: [&str; 2] = ["gang order3", "gang3"];

/// Gangland's store, finished so it can be walked into. The map has it built as a warehouse
/// is, but never made one: its shell (building 40) is a plain house, and its insides (43,
/// `BUILDING_TYPE_CRATE_IN`, the counters, shelves and back rooms) are the sort the original
/// only draws from inside a warehouse (aeng.cpp:12399). The front wall at the back of its
/// porch is split for a doorway one square wide, (110,66) to (110,65), with a lintel over it,
/// just as bball2's club's door is, but that square was left a wall (`STOREY_TYPE_NORMAL`)
/// rather than a doorway (`STOREY_TYPE_DOOR`). The store has no other way in, and nothing
/// in either mission opens one, so in the original nobody gets in: Gang order3 and gang3
/// can't be finished there. Here the shell is a warehouse (`BUILDING_TYPE_WAREHOUSE`) and that
/// square of wall its doorway, which `slide_along` lets people through (collide.cpp:4488).
fn open_the_store(mission_name: &str, map_file: &str, map: &mut Map) {
    if !map_file.eq_ignore_ascii_case(r"data\gang1.iam") || !STORE_MISSIONS.contains(&mission_name.to_ascii_lowercase().as_str()) {
        return;
    }
    const SHELL: usize = 40;
    let Some(b) = map.buildings.get(SHELL) else { return };
    let mut walls = b.start_facet as usize..b.end_facet as usize;
    let doorway = walls.find(|&i| {
        let f = &map.facets[i];
        f.kind == facet_type::NORMAL && (f.x, f.z, f.y[0]) == ([110, 110], [66, 65], 256)
    });
    // Only the map as shipped: an edited one may have it some other way.
    let Some(doorway) = doorway else { return };
    map.buildings[SHELL].kind = BUILDING_TYPE_WAREHOUSE;
    map.facets[doorway].kind = facet_type::DOOR;
}

/// Textures for a mission the install has no clump for: the shipped missions only have
/// clumps of their own, packed from the loose textures (`server\textures\`) that the
/// install no longer has. The pages come from the clumps of other missions: first those on
/// the same map, then those in the same world; the shared pages from any. A world no
/// clump covers borrows `STAND_IN_WORLD`'s pages, which fit its style table only roughly.
/// Also returns the world pages it had to take from another world (`Level::stand_in_pages`).
fn borrowed_clump(game: &Path, map_file: &str, texture_set: Option<i32>) -> (Clump, BTreeSet<u32>) {
    // (rank, mission): 0 same map, 1 same world, 2 the stand-in world, 3 any other.
    let mut donors: Vec<(u8, String)> = Vec::new();
    let mut sets: HashMap<String, Option<i32>> = HashMap::new();
    let entries = resolve(game, "levels").and_then(|d| std::fs::read_dir(d).ok());
    for e in entries.into_iter().flatten().flatten() {
        let file = e.file_name().to_string_lossy().into_owned();
        let Some(name) = file.strip_suffix(".ucm").or_else(|| file.strip_suffix(".UCM")) else { continue };
        if resolve(game, &format!("clumps\\{name}.txc")).is_none() {
            continue;
        }
        let Some(m) = std::fs::read(e.path()).ok().and_then(|b| Mission::parse(&b).ok()) else { continue };
        let rank = if m.map.eq_ignore_ascii_case(map_file) {
            0
        } else {
            let set = sets.entry(m.map.to_ascii_lowercase()).or_insert_with(|| {
                read(game, &m.map).ok().and_then(|b| Map::parse(&b).ok()).and_then(|map| map.texture_set)
            });
            if *set == texture_set { 1 } else if *set == Some(STAND_IN_WORLD) { 2 } else { 3 }
        };
        donors.push((rank, name.to_string()));
    }
    donors.sort();
    let own_world = donors.iter().any(|&(rank, _)| rank < 2);
    let mut clump = Clump::empty();
    let mut others = Vec::new();
    let mut own = BTreeSet::new();
    for (rank, name) in donors {
        if let Some(c) = read(game, &format!("clumps\\{name}.txc")).ok().and_then(|b| Clump::parse(b).ok()) {
            if rank < 2 {
                own.extend(c.ids().map(entry_page));
            }
            // A page's crinkle is taken along with the page.
            clump.fill_from(&c, |id| rank < 2 || (rank == 2 && !own_world) || !world_page(entry_page(id)));
            if rank >= 2 {
                others.push(c);
            }
        }
    }
    // The world pages none of the world's own missions use aren't anywhere in the install
    // (Officer Training's world 2 lacks nine of them): rather than leave holes they take the
    // stand-in world's page of the same number, then any other's. Not the right picture, but
    // something of the right sort in the right place.
    for c in &others {
        clump.fill_from(c, |_| true);
    }
    let stand_ins = clump.ids().map(entry_page).filter(|&p| world_page(p) && !own.contains(&p)).collect();
    (clump, stand_ins)
}

/// The override for `page`, if there is one: the most particular of
/// `<dir>/<mission>/page_NNNN.png`, `<dir>/world<N>/page_NNNN.png` (the map's own texture
/// set, so a world that borrows another's pictures can be given its own) and
/// `<dir>/page_NNNN.png`.
pub fn override_path(dir: &Path, mission: &str, world: Option<i32>, page: u32) -> Option<PathBuf> {
    let file = format!("page_{page:04}.png");
    let world = world.map(|w| dir.join(format!("world{w}")).join(&file));
    [Some(dir.join(mission).join(&file)), world, Some(dir.join(&file))].into_iter().flatten().find(|p| p.is_file())
}

/// What the unfinished missions never got (not in the original): an ending, and beacons for
/// the objectives, which none has a single one of.
///
/// Takeover1 sends Darci to bring two hostages out to the extraction point, and says "Worker
/// 1 rescued." and "Worker 2 rescued." as each gets there (points 65 and 64), but nothing
/// ends it: here an `end_game_win` point goes off once both have. Each hostage (points 62
/// and 63) has a beacon until they're rescued, and the extraction point one throughout.
///
/// subdock1 has a beacon on the dock operator (point 37), and one on the rendezvous (point
/// 42) once the chopper (41) comes.
///
/// gang3, an early Gang order3, sends Mako to shake the man in the store (point 134) down
/// for the files from a drop-off and says "Good work!" (point 138) once he's back at the
/// den with them, but nothing ends it: here an `end_game_win` point goes off with it. The
/// man has a beacon until the files are in hand (point 139), the den one from then on.
///
/// Crimescene follows the trail of a shooting to Harley Blvd, where the brutes who knew of
/// it were to meet "some geek" (point 76), and stops there, with a workman (point 79) and two
/// Wildcats minding him. Here the workman is the geek: arresting him once the trail has led
/// there wins, and killing him, the only lead, loses. He has a beacon from then on.
///
/// police5 has someone bomb Darci's station, and five Wildcats out beating people up in the
/// West District, each with a point that goes off when he's arrested (146 to 150), but
/// nothing ends it: here arresting all five wins, and killing any of them loses. Each has a
/// beacon until he's brought in.
///
/// Getaway1 has a beacon on the police contact (points 162 and 180), whose talk (163) sends
/// Darci after Mr X (131) as he walks from the TV station to his van (164) and drives off.
/// Taking him down wins (150) and his van getting away loses (165 and 178), but nothing shows
/// where he is: only a message (154) on getting near York park, which he passes on the way.
/// Here the contact's beacon goes once the talk's over (179), and Mr X has one from then on.
///
/// Transmit1's `prim_damaged` point (120), which wins it once the transmitter on the TV
/// building is blown up, is just over the edge of the map's block of props from the
/// transmitter's: `EWAY_create_cond` looks for the nearest prop in the point's own block,
/// so in the original it watches a prop that can't be damaged, and the mission can't be
/// won. Here the point is on the transmitter.
///
/// `mission_name` is the mission's file name: gang3's own name is "Gang order3", whose
/// mission it was made from.
pub fn finish_unfinished(mission_name: &str, mission: &mut Mission) {
    use uc_formats::ucm::{on_trigger, trigger, waypoint, EventPoint};
    let name = mission_name.to_ascii_lowercase();
    if name == "transmit1" {
        if let Some(e) = mission.event_points.iter_mut().find(|e| e.index == 120 && e.used != 0 && e.trigger == trigger::PRIM_DAMAGED) {
            (e.x, e.z) = (20228, 18180);
        }
        return;
    }
    // Not twice: an edited mission keeps what it was given.
    let has = |kind: u8| mission.event_points.iter().any(|e| e.used != 0 && e.kind == kind);
    let beacons = mission.event_points.iter().filter(|e| e.used != 0 && e.kind == waypoint::NAV_BEACON).count();
    let done = match name.as_str() {
        "crimescene" | "police5" => has(waypoint::END_GAME_WIN),
        // It has the contact's of its own.
        "getaway1" => beacons > 1,
        _ => beacons > 0,
    };
    if !matches!(name.as_str(), "takeover1" | "subdock1" | "gang3" | "crimescene" | "police5" | "getaway1") || done {
        return;
    }
    let point = |m: &Mission, i: usize| m.event_points.iter().find(|e| e.index == i && e.used != 0).cloned();
    // A point with nothing set but where it is. Not its colour and group either: an active
    // point of a patrol's colour and group is a stop on its route (`EWAY_find_waypoint`).
    let at = |e: &EventPoint| EventPoint {
        kind: 0, trigger: trigger::NONE, on_trigger: on_trigger::ACTIVE, ep_ref: 0, ep_ref_bool: 0, after_timer: 0,
        data: [0; 10], radius: 0, next: 0, prev: 0, colour: 0, group: 0, ..e.clone()
    };
    let beacon = |e: &EventPoint, track: usize| EventPoint { kind: waypoint::NAV_BEACON, data: [0, track as i32, 0, 0, 0, 0, 0, 0, 0, 0], ..at(e) };
    if name == "takeover1" {
        let (Some(two), Some(one)) = (point(mission, 64), point(mission, 65)) else { return };
        if !mission.event_points.iter().any(|e| e.used != 0 && e.kind == waypoint::END_GAME_WIN) {
            add_point(mission, EventPoint { kind: waypoint::END_GAME_WIN, trigger: trigger::BOOLEAN_AND, ep_ref: 64, ep_ref_bool: 65, ..at(&two) }, None);
        }
        for (worker, rescued, label) in [(62, &one, "Worker 1"), (63, &two, "Worker 2")] {
            let Some(w) = point(mission, worker) else { continue };
            let b = add_point(mission, beacon(&w, worker), Some(label));
            let kill = EventPoint { kind: waypoint::KILL_WAYPOINT, trigger: trigger::DEPENDENCY, ep_ref: rescued.index as u16, ..at(rescued) };
            add_point(mission, EventPoint { data: [b as i32, 0, 0, 0, 0, 0, 0, 0, 0, 0], ..kill }, None);
        }
        add_point(mission, beacon(&one, 0), Some("Extraction point"));
    } else if name == "crimescene" || name == "police5" {
        // Who's to be brought in, with the point that goes off when they're arrested, what
        // they're called, what's said once it's done and what if one's killed, and the point
        // they're wanted from (0 for the start).
        type Wanted = Vec<(usize, Option<usize>)>;
        let (wanted, label, done_text, lost_text, from): (Wanted, &str, &str, &str, usize) = if name == "crimescene" {
            (vec![(79, None)], "The geek",
             "You're the geek them brutes were meetin', huh? You an' me are gonna have a little talk about 24th Street...",
             "D'arci, you just killed our only lead on that shooting!", 75)
        } else {
            (vec![(93, Some(148)), (94, Some(146)), (137, Some(147)), (138, Some(149)), (139, Some(150))], "Wildcat",
             "That's the last of 'em, Stern. Now maybe we'll find out who bombed the station.",
             "We needed those Wildcats alive, Stern! Now we'll never find out who bombed the station.", 0)
        };
        let Some(start) = point(mission, wanted[0].0) else { return };
        if name == "crimescene" {
            // Nothing points Darci at the workman (58) who leads her to the brutes: once the
            // one on the roof (44) has talked (36) or is dead, a beacon on him, until he's
            // brought her to the alley (64).
            let (Some(talk), Some(workman), Some(alley)) = (point(mission, 36), point(mission, 58), point(mission, 64)) else { return };
            let talked = add_point(mission, EventPoint { kind: waypoint::SIMPLE, trigger: trigger::CONVERSATION_OVER, ep_ref: talk.index as u16, ..at(&talk) }, None);
            let dead = add_point(mission, EventPoint { kind: waypoint::SIMPLE, trigger: trigger::KILLED, ep_ref: 44, ..at(&talk) }, None);
            let either = add_point(mission, EventPoint { kind: waypoint::SIMPLE, trigger: trigger::BOOLEAN_OR, ep_ref: talked as u16, ep_ref_bool: dead as u16, ..at(&talk) }, None);
            let b = add_point(mission, EventPoint { trigger: trigger::DEPENDENCY, ep_ref: either as u16, ..beacon(&workman, 58) }, Some("The workman"));
            let kill = EventPoint { kind: waypoint::KILL_WAYPOINT, trigger: trigger::DEPENDENCY, ep_ref: alley.index as u16, ..at(&alley) };
            add_point(mission, EventPoint { data: [b as i32, 0, 0, 0, 0, 0, 0, 0, 0, 0], ..kill }, None);
        }
        let say = |text_from: &EventPoint, secs: i32| EventPoint { kind: waypoint::MESSAGE, data: [0, secs, 0, 0, 0, 0, 0, 0, 0, 0], ..at(text_from) };
        let mut got = Vec::new();
        for &(who, arrested) in &wanted {
            let Some(w) = point(mission, who) else { return };
            let arrested = arrested.unwrap_or_else(|| add_point(mission, EventPoint { kind: waypoint::SIMPLE, trigger: trigger::PERSON_ARRESTED, ep_ref: who as u16, ..at(&w) }, None));
            let b = add_point(mission, EventPoint { trigger: if from == 0 { trigger::NONE } else { trigger::DEPENDENCY }, ep_ref: from as u16, ..beacon(&w, who) }, Some(label));
            let kill = EventPoint { kind: waypoint::KILL_WAYPOINT, trigger: trigger::DEPENDENCY, ep_ref: arrested as u16, ..at(&w) };
            add_point(mission, EventPoint { data: [b as i32, 0, 0, 0, 0, 0, 0, 0, 0, 0], ..kill }, None);
            let killed = EventPoint { trigger: trigger::KILLED_NOT_ARRESTED, ep_ref: who as u16, ..say(&w, 6) };
            let killed = add_point(mission, killed, Some(lost_text));
            add_point(mission, EventPoint { kind: waypoint::END_GAME_LOSE, trigger: trigger::DEPENDENCY, ep_ref: killed as u16, ..at(&w) }, None);
            got.push(arrested);
        }
        // All of them in: an `and` of each with the ones before.
        let mut all = got[0];
        for &g in &got[1..] {
            all = add_point(mission, EventPoint { kind: waypoint::SIMPLE, trigger: trigger::BOOLEAN_AND, ep_ref: all as u16, ep_ref_bool: g as u16, ..at(&start) }, None);
        }
        if from != 0 {
            all = add_point(mission, EventPoint { kind: waypoint::SIMPLE, trigger: trigger::BOOLEAN_AND, ep_ref: from as u16, ep_ref_bool: all as u16, ..at(&start) }, None);
        }
        let said = add_point(mission, EventPoint { trigger: trigger::DEPENDENCY, ep_ref: all as u16, ..say(&start, 6) }, Some(done_text));
        add_point(mission, EventPoint { kind: waypoint::END_GAME_WIN, trigger: trigger::DEPENDENCY, ep_ref: said as u16, ..at(&start) }, None);
    } else if name == "gang3" {
        let (Some(man), Some(files), Some(den), Some(good)) = (point(mission, 134), point(mission, 139), point(mission, 137), point(mission, 138)) else { return };
        if !mission.event_points.iter().any(|e| e.used != 0 && e.kind == waypoint::END_GAME_WIN) {
            add_point(mission, EventPoint { kind: waypoint::END_GAME_WIN, trigger: trigger::DEPENDENCY, ep_ref: good.index as u16, ..at(&good) }, None);
        }
        let b = add_point(mission, beacon(&man, 134), Some("The store"));
        let kill = EventPoint { kind: waypoint::KILL_WAYPOINT, trigger: trigger::DEPENDENCY, ep_ref: files.index as u16, ..at(&files) };
        add_point(mission, EventPoint { data: [b as i32, 0, 0, 0, 0, 0, 0, 0, 0, 0], ..kill }, None);
        add_point(mission, EventPoint { trigger: trigger::DEPENDENCY, ep_ref: files.index as u16, ..beacon(&den, 0) }, Some("The den"));
    } else if name == "getaway1" {
        let (Some(contact), Some(talked), Some(mr_x)) = (point(mission, 180), point(mission, 179), point(mission, 131)) else { return };
        let kill = EventPoint { kind: waypoint::KILL_WAYPOINT, trigger: trigger::DEPENDENCY, ep_ref: talked.index as u16, ..at(&contact) };
        add_point(mission, EventPoint { data: [contact.index as i32, 0, 0, 0, 0, 0, 0, 0, 0, 0], ..kill }, None);
        add_point(mission, EventPoint { trigger: trigger::DEPENDENCY, ep_ref: talked.index as u16, ..beacon(&mr_x, mr_x.index) }, Some("Mr X"));
    } else {
        let (Some(op), Some(chopper), Some(rendezvous)) = (point(mission, 37), point(mission, 41), point(mission, 42)) else { return };
        add_point(mission, beacon(&op, 37), Some("Dock operator"));
        add_point(mission, EventPoint { trigger: trigger::DEPENDENCY, ep_ref: chopper.index as u16, ..beacon(&rendezvous, 0) }, Some("Chopper"));
    }
}

/// Adds `ep` to the mission as its last point, with its message if it has one, and returns
/// its index. Messages go to the points that have them in index order (`Mission::point_texts`).
fn add_point(mission: &mut Mission, ep: uc_formats::ucm::EventPoint, text: Option<&str>) -> usize {
    debug_assert_eq!(uc_formats::ucm::has_text(ep.kind), text.is_some());
    let index = mission.event_points.iter().map(|e| e.index).max().unwrap_or(0) + 1;
    mission.event_points.push(uc_formats::ucm::EventPoint { index, ..ep });
    if let Some(text) = text {
        mission.texts.push(uc_formats::ucm::TextEntry::Message { text: text.to_string(), tail: Vec::new() });
    }
    index
}

/// The map a mission is played on when the install hasn't got its own: that of the mission
/// it became. Finale1b's `Rooftest.iam` is missing, and Day Of Reckoning has its
/// `Rooftest2.iam`.
pub fn stand_in_map(mission_name: &str) -> Option<&'static str> {
    const STAND_INS: [(&str, &str); 1] = [("Finale1b", r"Data\Rooftest2.iam")];
    STAND_INS.iter().find(|(m, _)| m.eq_ignore_ascii_case(mission_name)).map(|&(_, map)| map)
}

impl Level {
    pub fn load(game: &Path, mission_name: &str, opt: &LoadOptions) -> Result<Level, String> {
        let mut mission = Mission::parse(&read(game, &format!("levels\\{mission_name}.ucm"))?)
            .map_err(|e| format!("{mission_name}.ucm: {e}"))?;
        finish_unfinished(mission_name, &mut mission);
        if read(game, &mission.map).is_err() {
            if let Some(map) = stand_in_map(mission_name) {
                mission.map = map.to_string();
            }
        }
        Level::load_with_mission(game, mission_name, mission, opt)
    }

    /// [`Level::load`] with `mission` in place of the install's `<mission_name>.ucm`: an
    /// edited one (#185). Its map and lighting are still read from the install, and its
    /// textures still looked up by `mission_name`. An edited mission keeps what
    /// [`finish_unfinished`] added when it was loaded, or doesn't if the editor took it out.
    pub fn load_with_mission(game: &Path, mission_name: &str, mission: Mission, opt: &LoadOptions) -> Result<Level, String> {
        let mut map = Map::parse(&read(game, &mission.map)?).map_err(|e| format!("{}: {e}", mission.map))?;
        to_sea(&mission.map, &mut map);
        open_the_store(mission_name, &mission.map, &mut map);
        // The mission's no-go squares: nobody walks into them, routes go round them, and
        // someone walking up to one stops short (elev.cpp:1813, after `MAV_precalculate`
        // and `WAND_init`, so they're turned off on top of whatever those found).
        for (k, &z) in mission.zones.iter().enumerate().take(MAP_SIZE * MAP_SIZE) {
            if z & uc_formats::ucm::zone_flags::NO_GO != 0 {
                map.squares[k].flags |= flags::NOGO;
            }
        }
        // A world the install has nothing of borrows the stand-in's style table and pages.
        let style_table = |w: i32| read(game, &format!("server\\textures\\world{w}\\style.tma")).ok();
        let world = map.texture_set.map(|w| if style_table(w).is_some() { w } else { STAND_IN_WORLD });
        // Textures are looked up by mission name, not by map (TEXTURE_initialise_clumping).
        let (clump, mut stand_in_pages) = match read(game, &format!("clumps\\{mission_name}.txc")) {
            Ok(b) => (Clump::parse(b).map_err(|e| e.to_string())?, BTreeSet::new()),
            Err(_) => borrowed_clump(game, &mission.map, world),
        };
        // Borrowing the stand-in's world, none of its world pages are its own.
        if world != map.texture_set {
            stand_in_pages = clump.ids().map(entry_page).filter(|&p| world_page(p)).collect();
        }
        let styles = world.and_then(style_table).and_then(|b| StyleTable::parse(&b).ok());
        let lighting = if opt.fullbright { None } else { read(game, &mission.lighting).ok().and_then(|b| Lighting::parse(&b).ok()) };
        let sky = lighting.as_ref().map_or([20, 22, 30], |l| l.sky);

        // The mission's player spawn.
        let platforms = platform_props(&map, &mission);
        let barrel_props: Vec<usize> = map.objects.iter().enumerate().filter(|(_, o)| o.prim == BARREL_PROP).map(|(i, _)| i).collect();
        let moving = platforms.iter().map(|&(_, ob)| ob).chain(barrel_props.iter().copied()).collect();
        let mut ground = Ground::new(&map, game, &moving);
        if mission_name.eq_ignore_ascii_case("Capflag1") {
            // The network game's map was never tuned for play: under the fence rules each base
            // is fenced in, every way out unclimbable or angle-topped, so there every fence can
            // be climbed over.
            ground.fence_climb.fill(crate::ground::FenceClimb { climbable: true, over: true });
        }
        let ep = mission.event_points.iter().find(|e| e.kind == waypoint::CREATE_PLAYER);
        let (sx, sy, sz) = ep.map_or((64.0 * UNIT, 0.0, 64.0 * UNIT), |e| (e.x as f32, e.y as f32, e.z as f32));
        let angle = ep.map_or(0.0, |e| ep_angle(e.direction));
        let spawn = (vec3(sx, ground.surface(&map, sx, sz, sy), sz), angle);
        let people = if opt.character {
            let chunk = |name: &str| -> Result<Arc<AnimChunk>, String> {
                let c = AnimChunk::parse(&read(game, &format!("data\\{name}"))?).map_err(|e| format!("{name}: {e}"))?;
                Ok(Arc::new(c))
            };
            // In the combat tutorial the gang meshes are swapped for trainers (setup_people_anims,
            // fallen/Source/Anim.cpp).
            let trainer = if mission_name.eq_ignore_ascii_case("ftutor1") { chunk("trainer.all").ok() } else { None };
            // Roper has a body of his own (`game_chunk[ANIM_TYPE_ROPER]`, Anim.cpp); without it
            // he falls back to a thug's.
            let roper = chunk("roper.all").ok();
            // Everyone animates from Darci's set, with Roper's carrying (and being carried)
            // filled in from his file into the slots from `ANIM_END` up (Anim.cpp:1398).
            let mut darci = AnimChunk::parse(&read(game, "data\\darci1.all")?).map_err(|e| format!("darci1.all: {e}"))?;
            if let Some(r) = &roper {
                for &(from, to) in &crate::character::CARRY_ANIMS {
                    darci.graft(r, from, to);
                }
            }
            let thug = chunk("rthug.all")?;
            // And the civilians' and Roper's others (Anim.cpp:1373).
            let mut files: HashMap<&str, Option<Arc<AnimChunk>>> = HashMap::new();
            files.insert("rthug.all", Some(thug.clone()));
            for &(file, from, to) in &crate::character::EXTRA_ANIMS {
                if let Some(f) = files.entry(file).or_insert_with(|| chunk(file).ok()) {
                    darci.graft(f, from, to);
                }
            }
            Some(People { darci: Arc::new(darci), thug, trainer, roper })
        } else {
            None
        };
        Ok(Level {
            game: game.to_path_buf(), name: mission_name.to_string(), mission, map, clump, styles, lighting,
            ground, spawn, sky, people, overrides: opt.overrides.clone(), platforms, barrel_props, stand_in_pages,
            see_through_prims: resolve(game, "server\\textures\\shared\\prims\\textype.txt")
                .and_then(|p| std::fs::read_to_string(p).ok())
                .map_or_else(Default::default, |t| see_through_pages(&t)),
            item_prims: ITEM_PRIMS.iter().map(|&(_, id)| id).chain(EXTRA_PRIMS)
                .filter_map(|id| Some((id, load_prim(game, id)?))).collect(),
            weapon_prims: (256..=263).filter_map(|id| Some((id, load_prim(game, id)?))).collect(),
        })
    }

    pub fn lighter(&self) -> Option<Lighter<'_>> {
        self.lighting.as_ref().map(|l| Lighter::new(l, &self.map.objects))
    }

    /// The texture for a page: an override PNG if present ([`override_path`]), else the
    /// clump's.
    pub fn texture(&self, page: u32) -> Option<(Texture, bool)> {
        if let Some(p) = self.overrides.as_deref().and_then(|d| override_path(d, &self.name, self.map.texture_set, page)) {
            if let Some(t) = load_png(&p) {
                return Some((t, true));
            }
        }
        let mut t = self.clump.texture(page).ok().flatten()?;
        // A prim page `textype.txt` doesn't make see-through is drawn solid, as the original
        // only alpha-tests pages it flags (polyrenderstate.cpp:1525). The mine's (#85) has an
        // alpha channel that's mostly clear, which would otherwise hide most of it.
        if is_prim_page(page) && t.has_alpha && !self.see_through_prims.contains(&(page - PRIM_PAGES)) {
            t.has_alpha = false;
            for px in t.rgba.chunks_mut(4) {
                px[3] = 255;
            }
        }
        Some((t, false))
    }

    /// The floor squares on the sea, which [`Level::build`] leaves out for the game to draw
    /// with the waves, lit as the rest of the floor is.
    pub fn sea(&self) -> Vec<SeaSquare> {
        let lighter = self.lighter();
        let sh = Shader { lighter: lighter.as_ref() };
        let map = &self.map;
        let mut out = Vec::new();
        for x in 0..MAP_SIZE - 1 {
            for z in 0..MAP_SIZE - 1 {
                if !is_sea_square(map, x, z) {
                    continue;
                }
                let sq = map.square(x, z);
                let corners = [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)];
                let height = corners.map(|(cx, cz)| map.corner_alt(cx, cz) as f32);
                let p = [0, 1, 2, 3].map(|i| vec3(corners[i].0 as f32, height[i] / UNIT, corners[i].1 as f32));
                let mut colour = sh.quad(&p);
                if sq.has(flags::WATER) {
                    colour = colour.map(|v| [(v[0] as f32 * 0.6) as u8, (v[1] as f32 * 0.7) as u8, v[2], 255]);
                }
                out.push(SeaSquare {
                    page: sq.page() as u32,
                    uv: FLOOR_UVS[sq.rotation() as usize],
                    corners,
                    height,
                    wave: corners.map(|(cx, cz)| sea_point(map, cx, cz)),
                    colour,
                });
            }
        }
        out
    }

    /// Builds all static geometry, lit with the level's baked lighting.
    pub fn build(&self) -> WorldGeometry {
        let lighter = self.lighter();
        let sh = Shader { lighter: lighter.as_ref() };
        let ids: std::collections::HashSet<u32> = self.clump.ids().collect();
        let mut b = Builder::default();
        build_floor(&mut b, &self.map, &sh, |p| !ids.contains(&p));
        build_roof_faces(&mut b, &self.map, &sh, |p| ids.contains(&p));
        build_walkables(&mut b, &self.map, &sh);
        // Each page's crinkle, as an index into `crinkles`.
        let mut crinkles = Vec::new();
        let mut crinkle_of: HashMap<u32, Option<u32>> = HashMap::new();
        if let Some(styles) = &self.styles {
            let refence = fence_page(&self.mission.map);
            for i in 1..self.map.facets.len() {
                let kind = self.map.facets[i].kind;
                // A doorway is a hole in the wall: the original draws a recess round it and
                // never the facet itself (`AENG_draw_box_around_recessed_door`, aeng.cpp:9981).
                if kind == facet_type::DOOR {
                    continue;
                }
                let fence = matches!(kind, uc_formats::iam::facet_type::FENCE | uc_formats::iam::facet_type::FENCE_FLAT);
                for q in facet::wall_quads(&self.map, styles, i) {
                    let p = q.corners.map(|c| vec3(c[0], c[1], c[2]) / UNIT);
                    let page = refence.filter(|_| fence).unwrap_or(q.page);
                    let crinkle = *crinkle_of.entry(page).or_insert_with(|| {
                        let c = self.clump.crinkle(page).ok().flatten()?;
                        crinkles.push(c);
                        Some(crinkles.len() as u32 - 1)
                    });
                    match crinkle {
                        Some(crinkle) => b.crinkled(ReliefQuad { corners: p, uv: q.uv, colour: sh.quad(&p), page, crinkle }),
                        None => b.batch(p[0], Layer::Walls, page).quad(p, q.uv, sh.quad(&p)),
                    }
                }
            }
        }
        let moving: std::collections::BTreeSet<usize> = self.platforms.iter().map(|&(_, ob)| ob).chain(self.barrel_props.iter().copied()).collect();
        build_props(&mut b, &self.map, &self.game, &sh, &moving);
        build_details(&mut b, &self.map, &sh);
        WorldGeometry { crinkles, ..b.finish() }
    }
}

fn load_png(path: &Path) -> Option<Texture> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px.chunks(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Texture { width: info.width, height: info.height, has_alpha: rgba.chunks(4).any(|c| c[3] < 255), rgba })
}

/// Vertex lighting: the level's baked lights if available, else full bright.
pub struct Shader<'a> {
    pub lighter: Option<&'a Lighter<'a>>,
}

impl Shader<'_> {
    /// `p` in square units, `n` any length.
    pub fn at(&self, p: Vec3, n: Vec3) -> [u8; 4] {
        match self.lighter {
            Some(l) => {
                let [r, g, b] = l.light((p * UNIT).to_array(), n.to_array());
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]
            }
            None => [255; 4],
        }
    }
    pub fn quad(&self, p: &[Vec3; 4]) -> [[u8; 4]; 4] {
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        p.map(|v| self.at(v, n))
    }
}

/// The floor texture that's water (`MAV_SPARE_FLAG_WATER`, set in `MAV_precalculate` where a
/// square's page is 454, fallen/Source/mav.cpp:1430). The other two pages it checks for,
/// 99456 and 99457, can't fit in a square's ten bits.
pub const WATER_PAGE: u16 = 454;

/// A floor square with a corner on the water: drawn by the game a frame at a time, its water
/// corners rising and falling with the sea (`AENG_lower` for a `MAV_SPARE_FLAG_WATER` point,
/// aeng.cpp:7628).
#[derive(Debug, Clone)]
pub struct SeaSquare {
    pub page: u32,
    pub uv: [[f32; 2]; 4],
    /// The corners, in map squares, and their height (world units) before the waves.
    pub corners: [(usize, usize); 4],
    pub height: [f32; 4],
    /// Which corners are on the water and move.
    pub wave: [bool; 4],
    pub colour: [[u8; 4]; 4],
}

/// The point (x, z) moves with the sea: its square is water.
fn sea_point(map: &Map, x: usize, z: usize) -> bool {
    x < MAP_SIZE && z < MAP_SIZE && map.square(x, z).page() == WATER_PAGE
}

/// The squares [`build_floor`] leaves to the game because a corner is on the sea: ground
/// squares (not rooftops, not road) with a water point at any corner.
fn is_sea_square(map: &Map, x: usize, z: usize) -> bool {
    let sq = map.square(x, z);
    !sq.has(flags::ROOF_EXISTS) && !sq.has(flags::SINK_SQUARE)
        && [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)].iter().any(|&(cx, cz)| sea_point(map, cx, cz))
}

fn scale(c: [[u8; 4]; 4], k: f32) -> [[u8; 4]; 4] {
    c.map(|v| [(v[0] as f32 * k) as u8, (v[1] as f32 * k) as u8, (v[2] as f32 * k) as u8, v[3]])
}

fn build_floor(b: &mut Builder, map: &Map, sh: &Shader, skip_unseen: impl Fn(u32) -> bool) {
    let upper = |x: usize, z: usize| map.corner_alt(x, z) as f32;
    let lower = |x: usize, z: usize| {
        let sink = map.square(x.min(MAP_SIZE - 1), z.min(MAP_SIZE - 1)).has(flags::SINK_POINT);
        upper(x, z) - if sink { KERB_HEIGHT as f32 } else { 0.0 }
    };
    for x in 0..MAP_SIZE - 1 {
        for z in 0..MAP_SIZE - 1 {
            let sq = map.square(x, z);
            let page = sq.page() as u32;
            let uv = FLOOR_UVS[sq.rotation() as usize];
            let corners = [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)];

            // Rooftop: AENG_draw_city's "fake roof", flat at PAP_Hi.Height << 6.
            if sq.has(flags::ROOF_EXISTS) {
                let y = (sq.height as f32 * 64.0) / UNIT;
                let p = corners.map(|(cx, cz)| vec3(cx as f32, y, cz as f32));
                b.batch(p[0], Layer::Roofs, page).quad(p, uv, p.map(|v| sh.at(v, Vec3::Y)));
                continue;
            }
            // Ground under closed buildings is never visible and has no texture in the clump.
            if sq.has(flags::HIDDEN) && skip_unseen(page) {
                continue;
            }
            // The sea moves: the game draws it ([`Level::sea`]).
            if is_sea_square(map, x, z) {
                continue;
            }
            let sunk = sq.has(flags::SINK_SQUARE);
            let h = |cx: usize, cz: usize| if sunk { lower(cx, cz) } else { upper(cx, cz) };
            let p = corners.map(|(cx, cz)| vec3(cx as f32, h(cx, cz) / UNIT, cz as f32));
            let mut c = sh.quad(&p);
            if sq.has(flags::WATER) {
                c = c.map(|v| [(v[0] as f32 * 0.6) as u8, (v[1] as f32 * 0.7) as u8, v[2], 255]);
            }
            b.batch(p[0], Layer::Floor, page).quad(p, uv, c);

            // Kerb: vertical strip where a sunk square meets a raised neighbour.
            if sunk {
                let neighbours = [((x + 1, z), (x + 1, z), (x + 1, z + 1)),
                                  ((x.wrapping_sub(1), z), (x, z), (x, z + 1)),
                                  ((x, z + 1), (x, z + 1), (x + 1, z + 1)),
                                  ((x, z.wrapping_sub(1)), (x, z), (x + 1, z))];
                for ((nx, nz), a, bb) in neighbours {
                    if nx >= MAP_SIZE || nz >= MAP_SIZE || map.square(nx, nz).has(flags::SINK_SQUARE) {
                        continue;
                    }
                    let (ua, ub, la, lb) = (upper(a.0, a.1), upper(bb.0, bb.1), lower(a.0, a.1), lower(bb.0, bb.1));
                    if ua - la < 1.0 && ub - lb < 1.0 {
                        continue;
                    }
                    let q = [vec3(a.0 as f32, ua / UNIT, a.1 as f32), vec3(bb.0 as f32, ub / UNIT, bb.1 as f32),
                             vec3(a.0 as f32, la / UNIT, a.1 as f32), vec3(bb.0 as f32, lb / UNIT, bb.1 as f32)];
                    let kerb = map.square(nx, nz).page() as u32;
                    b.batch(q[0], Layer::Floor, kerb).quad(q, [[0.0, 0.0], [1.0, 0.0], [0.0, 0.125], [1.0, 0.125]],
                                                          scale(sh.quad(&q), 0.8));
                }
            }
        }
    }
}

/// Walkable roof faces (`FACET_draw_walkable`, facet.cpp:4116), each textured with the floor
/// square under it (`ph->Texture`, facet.cpp:4357). A warehouse's are drawn dark instead:
/// the PC engine draws them black from below, and from above with textures from the
/// editor's `.map` files (`WARE_roof_tex`, ware.cpp:242), which the game doesn't ship. So
/// is a face whose square is hidden and has no page in the clump.
fn build_roof_faces(b: &mut Builder, map: &Map, sh: &Shader, has_page: impl Fn(u32) -> bool) {
    for rf in map.roof_faces.iter().skip(1) {
        let (x, z) = ((rf.rx & 127) as usize, (rf.rz & 127) as usize);
        let sq = map.square(x, z);
        if sq.has(flags::ROOF_EXISTS) {
            continue;
        }
        let y0 = rf.y as i32;
        let ys = [y0, y0 + ((rf.dy[0] as i32) << ROOF_SHIFT), y0 + ((rf.dy[2] as i32) << ROOF_SHIFT),
                  y0 + ((rf.dy[1] as i32) << ROOF_SHIFT)];
        let (xf, zf) = (x as f32, z as f32);
        let p = [vec3(xf, ys[0] as f32 / UNIT, zf), vec3(xf + 1.0, ys[1] as f32 / UNIT, zf),
                 vec3(xf, ys[2] as f32 / UNIT, zf + 1.0), vec3(xf + 1.0, ys[3] as f32 / UNIT, zf + 1.0)];
        let warehouse = map.buildings.get(rf.building as usize).is_some_and(|b| b.kind == BUILDING_TYPE_WAREHOUSE);
        if warehouse || (sq.has(flags::HIDDEN) && !has_page(sq.page() as u32)) {
            b.batch(p[0], Layer::Roofs, UNTEXTURED).quad(p, [[0.0; 2]; 4], [[10, 10, 12, 255]; 4]);
        } else {
            b.batch(p[0], Layer::Roofs, sq.page() as u32).quad(p, FLOOR_UVS[sq.rotation() as usize], sh.quad(&p));
        }
    }
}

/// An older map's roofs and walkable tops (`uc_formats::iam::WalkablesLayout::Old`): prim
/// faces in world units, textured as a prim's are.
fn build_walkables(b: &mut Builder, map: &Map, sh: &Shader) {
    let Some(w) = &map.walkables else { return };
    for f in &w.faces {
        let (page, uv) = f.texture();
        let pts: Vec<Vec3> = f.points.iter().map(|&i| w.points[i as usize]).map(|v| vec3(v[0] as f32, v[1] as f32, v[2] as f32) / UNIT).collect();
        let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
        let col: Vec<[u8; 4]> = pts.iter().map(|&p| sh.at(p, n)).collect();
        let batch = b.batch(pts[0], Layer::Roofs, page);
        if pts.len() == 4 {
            batch.quad([pts[0], pts[1], pts[2], pts[3]], [uv[0], uv[1], uv[2], uv[3]], [col[0], col[1], col[2], col[3]]);
        } else {
            batch.tri([pts[0], pts[1], pts[2]], [uv[0], uv[1], uv[2]], [col[0], col[1], col[2]]);
        }
    }
}

/// Item types (`IT_*`, fallen/Headers/Mission.h) and their prims (`PRIM_OBJ_ITEM_*`,
/// Editor/Headers/Prim.h): the weapons and health, then the things a mission has Darci fetch
/// -- a key, a keycard, files, a floppy, a crowbar, a gas mask, a wrench, a video, gloves and
/// weedkiller. The Fallen's five clues are a keycard, three files and a video.
/// Badges (`create_treasure`) aren't `IT_*` items; they're kinds 100-103 here: stamina,
/// strength, reflexes and constitution, one of whose four models the original picks at random.
pub const ITEM_PRIMS: [(u8, u8); 27] = [
    (1, 36), (2, 143), (3, 142), (4, 127), (5, 166), (6, 120), (8, 205), (9, 141), (10, 255), (11, 253),
    (12, 254), (13, 144), (14, 232), (15, 233), (16, 234), (17, 235), (18, 236), (19, 237), (20, 238),
    (21, 239), (22, 168), (23, 167),
    (100, 81), (101, 94), (102, 71), (103, 39),
    // The sniper rifle, lying about as the AK-47 does (and drawn longer).
    (200, 120),
];

/// Other prims missions place: traffic cone, barrels, vehicle bodies and wheels, and the
/// helicopter's body and rotors (74, 75).
/// The prop every map's barrels are (prim 33, `PRIM_OBJ_BIN`, which ob.cpp calls "the
/// barrel prim").
pub const BARREL_PROP: u8 = 33;

pub const EXTRA_PRIMS: [u8; 19] = [33, 67, 145, 188, 88, 87, 108, 220, 155, 150, 159, 90, 91, 131, 27, 74, 75, CAN_PRIM, 86];

/// `TEXTURE_page_leaf`, `_rubbish` and `_snowflake` (`TEXTURE_NUM_STANDARD` + 35, 64 and
/// 74): `leaf.tga`, `rubbish.tga` and `snowflake.tga`, for the dirt.
pub const LEAF_PAGE: u32 = 22 * 64 + 35;
pub const RUBBISH_PAGE: u32 = 22 * 64 + 64;
pub const SNOWFLAKE_PAGE: u32 = 22 * 64 + 74;

/// `PRIM_OBJ_CAN`: the cans lying in the street.
pub const CAN_PRIM: u8 = 76;

/// The prim for item type `kind`.
pub fn item_prim(kind: u8) -> Option<u8> {
    ITEM_PRIMS.iter().find(|&&(k, _)| k == kind).map(|&(_, p)| p)
}

/// A prim's bounding box (world units): (lowest, highest corner).
pub fn prim_bounds(prim: &Prim) -> (Vec3, Vec3) {
    prim.points.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
        let p = vec3(p[0] as f32, p[1] as f32, p[2] as f32);
        (lo.min(p), hi.max(p))
    })
}

/// Adds a prim's triangles to `out` (square units), each point placed by `place` (prim space
/// to world units), lit by `lighter`.
pub fn prim_batches(prim: &Prim, place: impl Fn(Vec3) -> Vec3, lighter: Option<&Lighter>,
                    out: &mut std::collections::BTreeMap<u32, crate::geometry::Batch>) {
    let sh = Shader { lighter };
    for f in &prim.faces {
        let (page, uv) = f.texture();
        let pts: Vec<Vec3> = f.points.iter()
            .map(|&i| { let p = prim.points[i as usize]; place(vec3(p[0] as f32, p[1] as f32, p[2] as f32)) / UNIT })
            .collect();
        let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
        let col: Vec<[u8; 4]> = pts.iter().map(|&p| sh.at(p, n)).collect();
        let batch = out.entry(page).or_default();
        if pts.len() == 4 {
            batch.quad([pts[0], pts[1], pts[2], pts[3]], [uv[0], uv[1], uv[2], uv[3]], [col[0], col[1], col[2], col[3]]);
        } else {
            batch.tri([pts[0], pts[1], pts[2]], [uv[0], uv[1], uv[2]], [col[0], col[1], col[2]]);
        }
    }
}

/// Loads prop prim `id` (`nprimNNN.prm`, else the older `primNNN.prm`).
pub fn load_prim(game: &Path, id: impl Into<u16>) -> Option<Prim> {
    let id = id.into();
    let path = resolve(game, &format!("server\\prims\\nprim{id:03}.prm"))
        .or_else(|| resolve(game, &format!("server\\prims\\prim{id:03}.prm")))?;
    Prim::parse(&std::fs::read(path).ok()?).ok()
}

/// The props a mission's `link_platform` points take (`PLAT_create`): for each, the one
/// nearest it in the map cells within three squares, counting height at a quarter. Each is
/// taken off the map as it's taken (`OB_remove`), so no two points share one.
fn platform_props(map: &Map, mission: &Mission) -> Vec<(usize, usize)> {
    let mut taken = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for e in mission.event_points.iter().filter(|e| e.used != 0 && e.kind == waypoint::LINK_PLATFORM) {
        let cell = |v: i32| (v.clamp(0, 0x7fff)) >> 10;
        let (x1, z1, x2, z2) = (cell(e.x - 0x300), cell(e.z - 0x300), cell(e.x + 0x300), cell(e.z + 0x300));
        let best = map.objects.iter().enumerate()
            .filter(|(i, o)| !taken.contains(i) && o.prim != 0)
            .filter(|(_, o)| (x1..=x2).contains(&(o.x >> 10)) && (z1..=z2).contains(&(o.z >> 10)))
            .min_by_key(|(_, o)| (o.x - e.x).abs() + (o.z - e.z).abs() + ((o.y - e.y).abs() >> 2))
            .map(|(i, _)| i);
        if let Some(ob) = best {
            taken.insert(ob);
            out.push((e.index, ob));
        }
    }
    out
}

fn build_props(b: &mut Builder, map: &Map, game: &Path, sh: &Shader, moving: &std::collections::BTreeSet<usize>) {
    let mut cache: HashMap<u8, Option<Prim>> = HashMap::new();
    for ob in map.objects.iter().enumerate().filter(|(i, _)| !moving.contains(i)).map(|(_, o)| o) {
        let Some(prim) = cache.entry(ob.prim).or_insert_with(|| load_prim(game, ob.prim)) else { continue };
        let a = radians(ob.yaw as f32);
        let (s, c) = a.sin_cos();
        let origin = vec3(ob.x as f32, ob.y as f32, ob.z as f32);
        // MESH_draw_poly: world = transpose(MATRIX_calc(yaw)) * local.
        let place = |p: [i16; 3]| {
            let (x, y, z) = (p[0] as f32, p[1] as f32, p[2] as f32);
            (origin + vec3(c * x + s * z, y, -s * x + c * z)) / UNIT
        };
        for f in &prim.faces {
            let (page, uv) = f.texture();
            let pts: Vec<Vec3> = f.points.iter().map(|&i| place(prim.points[i as usize])).collect();
            let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
            let col: Vec<[u8; 4]> = pts.iter().map(|&p| sh.at(p, n)).collect();
            let batch = b.batch(origin / UNIT, Layer::Props, page);
            if pts.len() == 4 {
                batch.quad([pts[0], pts[1], pts[2], pts[3]], [uv[0], uv[1], uv[2], uv[3]], [col[0], col[1], col[2], col[3]]);
            } else {
                batch.tri([pts[0], pts[1], pts[2]], [uv[0], uv[1], uv[2]], [col[0], col[1], col[2]]);
            }
        }
    }
}

/// A box between two points (square units) with the given half-width, as four side quads.
fn beam(b: &mut Builder, from: Vec3, to: Vec3, half: f32, color: [u8; 4]) {
    let dir = (to - from).normalize_or_zero();
    let side = if dir.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = dir.cross(side).normalize_or_zero() * half;
    let v = dir.cross(u).normalize_or_zero() * half;
    let corners = [u + v, u - v, -u - v, -u + v];
    let batch = b.batch(from, Layer::Detail, UNTEXTURED);
    for i in 0..4 {
        let (o1, o2) = (corners[i], corners[(i + 1) % 4]);
        let shade = [0.75, 1.0, 0.6, 0.85][i];
        let c = [(color[0] as f32 * shade) as u8, (color[1] as f32 * shade) as u8, (color[2] as f32 * shade) as u8, 255];
        batch.quad([to + o1, to + o2, from + o1, from + o2], [[0.0; 2]; 4], [c; 4]);
    }
}

/// Cables (`cable_draw`, facet.cpp:950) and ladders (`DRAW_ladder`, facet.cpp:5201).
fn build_details(b: &mut Builder, map: &Map, sh: &Shader) {
    for f in map.facets.iter().skip(1) {
        let (x1, z1, x2, z2) = (f.x[0] as f32 * 256.0, f.z[0] as f32 * 256.0, f.x[1] as f32 * 256.0, f.z[1] as f32 * 256.0);
        match f.kind {
            facet_type::CABLE => {
                let pts: Vec<Vec3> = crate::ground::cable_points(f).into_iter().map(|p| p / UNIT).collect();
                let count = pts.len() - 1;
                let col = sh.at(pts[count / 2], Vec3::Y);
                let col = [col[0] / 4, col[1] / 4, col[2] / 4, 255];
                for w in pts.windows(2) {
                    beam(b, w[0], w[1], 2.0 / UNIT, col);
                }
            }
            facet_type::LADDER => {
                // The ladder fills the middle third of the facet, set out from the wall by 1/8 of its length.
                let (dx, dz) = (x2 - x1, z2 - z1);
                let (ax, az) = (x1 + dx / 3.0 + dz / 8.0, z1 + dz / 3.0 - dx / 8.0);
                let (bx, bz) = (x2 - dx / 3.0 + dz / 8.0, z2 - dz / 3.0 - dx / 8.0);
                let y0 = f.y[0] as f32;
                let top = y0 + f.height as f32 * 64.0;
                let col = sh.at(vec3(ax, y0, az) / UNIT, Vec3::Y);
                let col = [(col[0] as f32 * 0.55) as u8, (col[1] as f32 * 0.55) as u8, (col[2] as f32 * 0.6) as u8, 255];
                for (x, z) in [(ax, az), (bx, bz)] {
                    beam(b, vec3(x, y0, z) / UNIT, vec3(x, top, z) / UNIT, 3.0 / UNIT, col);
                }
                for k in 1..=f.height as i32 {
                    let y = y0 + k as f32 * 64.0 - 8.0;
                    beam(b, vec3(ax, y, az) / UNIT, vec3(bx, y, bz) / UNIT, 2.0 / UNIT, col);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod see_through_tests {
    use super::*;

    #[test]
    fn textype_flags_pick_the_see_through_pages() {
        let t = "# comment\nPage 000: T\nPage 022: M\nPage 025: MS\nPage 030: W\nPage 040: I\nPage 050: S\n";
        let s = see_through_pages(t);
        assert_eq!(s.into_iter().collect::<Vec<_>>(), vec![0, 22, 25, 41]);
    }

    /// The leaves, the sky and the other extras aren't prim pages: their see-through edges
    /// stay (#135).
    #[test]
    fn the_extras_are_not_prim_pages() {
        assert!(is_prim_page(788));
        for page in [LEAF_PAGE, RUBBISH_PAGE, SNOWFLAKE_PAGE, crate::SKY_PAGE, crate::MOON_PAGE] {
            assert!(!is_prim_page(page), "page {page}");
        }
    }
}

#[cfg(test)]
mod override_tests {
    use super::*;

    /// The mission's folder wins over the world's, and the world's over the shared one.
    #[test]
    fn the_most_particular_override_wins() {
        let dir = std::env::temp_dir().join(format!("uc-overrides-{}", std::process::id()));
        for sub in ["", "world7", "botanic1"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        for (sub, page) in [("", 124), ("world7", 124), ("world7", 125), ("botanic1", 125), ("", 126)] {
            std::fs::write(dir.join(sub).join(format!("page_{page:04}.png")), b"").unwrap();
        }
        let at = |m: &str, w, page| override_path(&dir, m, w, page).map(|p| p.strip_prefix(&dir).unwrap().to_path_buf());
        assert_eq!(at("backalley1", Some(7), 124), Some(Path::new("world7").join("page_0124.png")));
        assert_eq!(at("botanic1", Some(7), 125), Some(Path::new("botanic1").join("page_0125.png")));
        assert_eq!(at("police1", Some(3), 124), Some(PathBuf::from("page_0124.png")));
        assert_eq!(at("police1", None, 126), Some(PathBuf::from("page_0126.png")));
        assert_eq!(at("police1", Some(3), 127), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
