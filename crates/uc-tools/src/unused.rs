//! `uc unused`: what's in the install that nothing uses (#471). So far:
//!
//! - **Missions:** the `.ucm` files in `levels\` that `data\urban.sty` doesn't list, with
//!   their file version and whether they parse (#470).
//! - **Voice lines:** the recordings in `talk2\` of a mission that no message or conversation
//!   in it plays (#467). A message plays `<mission>.ucm<n>.wav` and a conversation's lines
//!   `<mission>.ucm<n>A.wav`, `B`... where `n` is the point's `Data[9]`, or its own number when
//!   that's unset (`EWAY_create`'s `magic_index`, fallen/Source/eway.cpp).
//! - **Loose textures and meshes:** the `.tga`, `.imp` and `.sex` files outside the clumps
//!   that nothing refers to. What refers to them is `fallen.exe` itself, read for the file
//!   names in it (whole, or as `sprintf` patterns such as `tex%03d.tga`), and the meshes it
//!   loads, which name their own textures. A folder it names counts for the files directly in
//!   it only if it names none of them one by one: `data\font3d\all\` is read whole (a letter
//!   a file, `Font3D::Font3D`), where `data\` is only the start of the names it builds. A
//!   `.sex` is the text version of the `.imp` beside it, and is counted with it.
//! - **Prims:** the files in `server\prims\` the released game never draws. It draws the
//!   objects on the map and the ones the code loads (`OB_load_needed_prims`, ob.cpp), and
//!   `load_prim_object` (io.cpp) only takes numbers 1 to 265, `nprimNNN.prm` before the old
//!   `primNNN.prm`, and skips 238. A map is a released one if a mission `urban.sty` lists or
//!   `fallen.exe` names is on it.
//! - **Sounds:** the `.wav` files in `data\sfx\` and `talk2\misc\` that the sound table in
//!   `fallen.exe` (`sound_list`, sound_id.cpp) doesn't name. Every sound the code or a mission
//!   plays is one of its ids. A name with a `-` in it is read from `talk2\misc\`, so the
//!   copies of the taunts in `data\sfx\1622\darci\` and the like aren't played, and a music
//!   track from the folder of a released mission's music world ([`sound_paths`]). Which ids
//!   nothing plays isn't checked: the code picks most of them from ranges (`S_WIND_START +
//!   n`), and the table in the exe isn't the one in the source.
//! - **Text:** the files in `text\` that neither `fallen.exe`, `config.ini` (the language)
//!   nor a released mission (its chatter file, `citsez`) names, and the ones named that
//!   aren't there. Which of `lang_english.txt`'s lines are shown isn't checked: the release's
//!   front end, which shows most of them, isn't in the source.
//! - **Animations:** the `.all` files the PC game never loads (`setup_people_anims`,
//!   Anim.cpp:926, names them; `anim%03d.all` is an anim prim a released map places or a
//!   released mission's creature), and the animations in `darci1.all`, the people's, that
//!   neither the code ([`PLAYED_ANIMS`]) nor a released mission's cutscene plays. The other
//!   files number their animations their own way (`NROPER_*`, `CIV_*`), and aren't checked
//!   one by one.
//! - **Mission messages never shown:** messages no point owns, and those of points waiting
//!   for one that isn't in the mission ([`dead_points`]), in every mission that parses.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use uc_formats::ucm::{waypoint, Mission};

/// A mission file in `levels\`.
pub struct MissionFile {
    pub name: String,
    /// The file's version (its first four bytes): every released mission is 10.
    pub version: i32,
    pub size: usize,
    /// Why it doesn't parse, if it doesn't.
    pub error: Option<String>,
    /// In `urban.sty`'s list.
    pub listed: bool,
    /// Named in `fallen.exe` (the missions it treats specially).
    pub in_exe: bool,
}

#[derive(Default)]
pub struct Report {
    pub missions: Vec<MissionFile>,
    /// Recordings in `talk2\` of a mission that parses.
    pub recordings: usize,
    /// Of those, the ones nothing plays, by mission.
    pub unplayed: BTreeMap<String, Vec<String>>,
    /// Named as a mission's (`<name>.ucm<n>.wav`) but there's no such mission.
    pub strays: Vec<String>,
    /// A mission's, but with more than one letter after the number (`Semtex.ucm106EB.wav`),
    /// which no line's name has: nothing plays them.
    pub odd: Vec<String>,
    /// Recordings of the missions that don't parse, which can't be checked.
    pub unchecked: BTreeMap<String, usize>,
    /// Loose textures and meshes: how many, and the ones nothing refers to (paths relative to
    /// the install).
    pub loose: usize,
    pub unreferenced: Vec<String>,
    /// Prims: how many files in `server\prims\`, the ones the released game never draws, and
    /// the numbers something places that have no file.
    pub prims: usize,
    pub unused_prims: Vec<PrimFile>,
    pub missing_prims: Vec<(u16, String)>,
    /// Sounds: how many in `data\sfx\` and `talk2\misc\`, the ones nothing plays, and the
    /// ones the sound table names that aren't there (paths relative to the install; a music
    /// track as `musicNN\`, when no world has it).
    pub sounds: usize,
    pub unused_sounds: Vec<String>,
    pub missing_sounds: Vec<String>,
    /// The music worlds the released missions play in.
    pub music_worlds: BTreeSet<u32>,
    /// Text files in `text\`: how many, the ones nothing names (with the unreleased
    /// missions that name them as their chatter), and the ones named that aren't there.
    pub texts: usize,
    pub unused_texts: Vec<(String, Vec<String>)>,
    pub missing_texts: Vec<(String, Vec<String>)>,
    /// Mission messages that are never shown: the mission, whether it's released, the point
    /// (none for a message no point owns) and the text. Unreleased missions too, as the port
    /// plays them.
    pub unshown: Vec<(String, bool, Option<usize>, String)>,
    /// Animation files (`.all`): how many, the ones the PC game never loads (with why), and
    /// the ones it names that aren't there.
    pub anim_files: usize,
    pub unused_anim_files: Vec<(String, String)>,
    pub missing_anim_files: Vec<String>,
    /// `darci1.all`, the people's animations: how many it has, and the ones nothing plays,
    /// with their `ANIM_*` names where they have one.
    pub people_anims: usize,
    pub unplayed_anims: Vec<(usize, &'static str)>,
}

/// A prim file in `server\prims\` the released game never draws.
pub struct PrimFile {
    pub file: String,
    pub id: u16,
    /// The name it was saved under in the editor.
    pub name: String,
    pub why: PrimWhy,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrimWhy {
    /// A `primNNN.prm` with an `nprimNNN.prm` of the same number, which is loaded instead.
    Shadowed,
    /// A number `load_prim_object` doesn't load: 0, or above 265.
    OutOfRange,
    /// A name it never builds (`nprim088bak.prm`).
    OddName,
    /// 238 (the gloves): `load_prim_object` returns before loading it.
    Skipped,
    /// Only on the maps of missions `urban.sty` doesn't list, nor `fallen.exe` names (those
    /// missions, by map).
    Unreleased(Vec<String>),
    /// Only on maps no mission uses.
    UnusedMap(Vec<String>),
    /// On no map, and the code doesn't load it.
    Nowhere,
}

pub fn run(game: &Path) -> Result<(), String> {
    let r = report(game)?;
    print(&r);
    Ok(())
}

pub fn report(game: &Path) -> Result<Report, String> {
    let exe = uc_formats::resolve(game, "fallen.exe").and_then(|p| std::fs::read(p).ok())
        .ok_or("fallen.exe not found in the install")?;
    let exe_strings = strings(&exe);
    let mut r = Report::default();

    // Missions.
    let listed = listed_missions(game);
    let levels = uc_formats::resolve(game, "levels").ok_or("no levels folder")?;
    let mut parsed: Vec<(String, Mission)> = Vec::new();
    for path in files(&levels).into_iter().filter(|p| ext(p) == "ucm") {
        let name = stem(&path);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let version = bytes.get(..4).map_or(0, |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let lower = name.to_ascii_lowercase();
        let in_exe = exe_strings.iter().any(|s| {
            let s = s.to_ascii_lowercase();
            s.trim_start_matches('\\') == format!("{lower}.ucm")
        });
        let error = match Mission::parse(&bytes) {
            Ok(m) => {
                parsed.push((name.clone(), m));
                None
            }
            Err(e) => Some(e.to_string()),
        };
        r.missions.push(MissionFile { listed: listed.contains(&lower), name, version, size: bytes.len(), error, in_exe });
    }
    r.missions.sort_by_key(|m| m.name.to_ascii_lowercase());

    // Voice lines.
    let mut played: BTreeSet<String> = BTreeSet::new();
    for (name, m) in &parsed {
        played.extend(spoken(name, m).into_iter().map(|f| f.to_ascii_lowercase()));
    }
    let parsed_names: BTreeMap<String, &str> = parsed.iter().map(|(n, _)| (n.to_ascii_lowercase(), n.as_str())).collect();
    let broken: BTreeSet<String> = r.missions.iter().filter(|m| m.error.is_some()).map(|m| m.name.to_ascii_lowercase()).collect();
    let talk2 = uc_formats::resolve(game, "talk2").ok_or("no talk2 folder")?;
    let mut recordings: Vec<String> = files(&talk2).iter().filter(|p| ext(p) == "wav").map(|p| file_name(p)).collect();
    recordings.sort_by_key(|f| f.to_ascii_lowercase());
    for f in recordings {
        // Only a mission's lines: the shared shouts (`GRED-ALERT`...) and briefings aren't.
        let Some((mission, letters)) = mission_of(&f) else { continue };
        if letters > 1 && (parsed_names.contains_key(&mission) || broken.contains(&mission)) {
            r.odd.push(f);
        } else if let Some(&m) = parsed_names.get(&mission) {
            r.recordings += 1;
            if !played.contains(&f.to_ascii_lowercase()) {
                r.unplayed.entry(m.to_string()).or_default().push(f);
            }
        } else if broken.contains(&mission) {
            *r.unchecked.entry(mission).or_default() += 1;
        } else {
            r.strays.push(f);
        }
    }

    // In the order they're numbered: `46` before `101`.
    for files in r.unplayed.values_mut() {
        files.sort_by_key(|f| {
            let rest = f.to_ascii_lowercase().rsplit_once(".ucm").map_or(String::new(), |(_, r)| r.to_string());
            let n: u32 = rest.chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0);
            (n, rest)
        });
    }

    // Loose textures and meshes.
    let refs = Refs::new(&exe_strings);
    let loose: Vec<String> = walk(game).into_iter()
        .filter(|p| matches!(ext(p).as_str(), "tga" | "imp" | "sex"))
        .filter_map(|p| Some(p.strip_prefix(game).ok()?.to_string_lossy().replace('/', "\\")))
        .collect();
    // The meshes the game loads, and the textures they name.
    let meshes: Vec<&String> = loose.iter().filter(|p| ext(Path::new(p)) == "imp" && refs.names(p)).collect();
    let mut named: BTreeSet<String> = BTreeSet::new();
    for m in &meshes {
        let bytes = std::fs::read(game.join(m.replace('\\', "/"))).unwrap_or_default();
        named.extend(strings(&bytes).iter().flat_map(|s| s.split_whitespace()).filter(|w| w.to_ascii_lowercase().ends_with(".tga"))
            .map(|w| base(w).to_ascii_lowercase()));
    }
    let twin = |p: &str| -> bool {
        let lower = p.to_ascii_lowercase();
        lower.strip_suffix(".sex").is_some_and(|s| meshes.iter().any(|m| m.to_ascii_lowercase() == format!("{s}.imp")))
    };
    r.loose = loose.len();
    r.unreferenced = loose.iter()
        .filter(|p| !refs.names(p) && !refs.in_folder(p, &loose) && !named.contains(&base(p).to_ascii_lowercase()) && !twin(p))
        .cloned().collect();
    r.unreferenced.sort_by_key(|p| p.to_ascii_lowercase());

    let released: BTreeSet<String> = r.missions.iter().filter(|m| m.listed || m.in_exe).map(|m| m.name.to_ascii_lowercase()).collect();
    prims(game, &parsed, &released, &mut r);
    sounds(game, &exe_strings, &parsed, &released, &mut r);
    texts(game, &exe_strings, &parsed, &released, &mut r);
    anims(game, &exe_strings, &parsed, &released, &mut r);
    Ok(r)
}

/// The prims the code loads whatever the map (`OB_load_needed_prims`, fallen/Source/ob.cpp),
/// besides the map's own objects.
const CODE_PRIMS: &[u16] = &[
    // `load_general_prims`: the stat ups, the chopper, the cars and vans, a can, a barrel, a
    // cone, 145, the mine and the thermodroid, and the weapons in hand with their flashes. The
    // hook, the balloon and the bike (`#ifdef BIKE`, not defined) aren't loaded.
    71, 94, 81, 39, 74, 75, 87, 88, 220, 108, 155, 150, 159, 90, 91, 131, 27, 76, 141, 67, 145, 188, 201,
    256, 257, 258, 259, 260, 261, 262, 263,
    // The specials (`SPECIAL_info`, Special.cpp), then the key, 117 and the bin.
    36, 143, 142, 127, 166, 167, 168, 120, 205, 255, 253, 254, 144, 232, 233, 234, 237, 238, 239, 84, 236,
    117, 33,
];
/// `PRIM_OBJ_SWITCH_OFF` and `_ON`: a map with the one also loads the other.
const SWITCH: (u16, u16) = (216, 157);

/// `released`: the missions `urban.sty` lists or `fallen.exe` names, lowercased.
fn prims(game: &Path, parsed: &[(String, Mission)], released: &BTreeSet<String>, r: &mut Report) {
    let Some(dir) = uc_formats::resolve(game, "server\\prims") else { return };
    // The maps, by file name, and the missions on each, released or not.
    let mut on_map: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for (name, m) in parsed {
        let entry = on_map.entry(base(&m.map).to_ascii_lowercase()).or_default();
        if released.contains(&name.to_ascii_lowercase()) { &mut entry.0 } else { &mut entry.1 }.push(name.clone());
    }
    // Every placed prim: on which maps.
    let mut placed: BTreeMap<u16, BTreeSet<String>> = BTreeMap::new();
    for path in walk(game).into_iter().filter(|p| ext(p) == "iam") {
        let Ok(map) = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| uc_formats::iam::Map::parse(&b).map_err(|e| e.to_string())) else { continue };
        let file = file_name(&path).to_ascii_lowercase();
        for o in map.objects.iter().filter(|o| o.prim != 0) {
            placed.entry(o.prim as u16).or_default().insert(file.clone());
            if o.prim as u16 == SWITCH.0 {
                placed.entry(SWITCH.1).or_default().insert(file.clone());
            }
        }
    }
    let on_released = |m: &String| on_map.get(m).is_some_and(|(l, _)| !l.is_empty());
    let unreleased = |m: &String| on_map.get(m).is_some_and(|(l, u)| l.is_empty() && !u.is_empty());

    let name_of = |path: &Path| std::fs::read(path).ok().and_then(|b| uc_formats::prm::Prim::parse(&b).ok())
        .map_or(String::new(), |p| p.name.split(|c: char| c.is_control()).next().unwrap_or("").trim().to_string());
    let all: Vec<PathBuf> = files(&dir).into_iter().filter(|p| ext(p) == "prm").collect();
    r.prims = all.len();
    // `nprimNNN` or `primNNN`: the number, and whether it's the new layout.
    let number = |p: &Path| -> Option<(u16, bool)> {
        let s = stem(p).to_ascii_lowercase();
        let (new, digits) = match s.strip_prefix("nprim") { Some(d) => (true, d), None => (false, s.strip_prefix("prim")?) };
        (digits.len() == 3).then_some(())?;
        Some((digits.parse().ok()?, new))
    };
    let mut files: Vec<(u16, bool, PathBuf)> = Vec::new();
    for p in all {
        match number(&p) {
            Some((id, new)) => files.push((id, new, p)),
            None => r.unused_prims.push(PrimFile { file: file_name(&p), id: 0, name: name_of(&p), why: PrimWhy::OddName }),
        }
    }
    files.sort_by_key(|(id, new, _)| (*id, !*new));
    let has_new: BTreeSet<u16> = files.iter().filter(|f| f.1).map(|f| f.0).collect();
    for (id, new, path) in &files {
        let why = if !new && has_new.contains(id) {
            PrimWhy::Shadowed
        } else if *id == 0 || *id > 265 {
            PrimWhy::OutOfRange
        } else if *id == 238 {
            PrimWhy::Skipped
        } else if CODE_PRIMS.contains(id) {
            continue;
        } else {
            let maps = placed.get(id).cloned().unwrap_or_default();
            if maps.iter().any(on_released) {
                continue;
            }
            let un: Vec<String> = maps.iter().filter(|m| unreleased(m))
                .flat_map(|m| on_map[m].1.iter().map(move |n| format!("{n} ({m})"))).collect();
            if !un.is_empty() {
                PrimWhy::Unreleased(un)
            } else if !maps.is_empty() {
                PrimWhy::UnusedMap(maps.into_iter().collect())
            } else {
                PrimWhy::Nowhere
            }
        };
        r.unused_prims.push(PrimFile { file: file_name(path), id: *id, name: name_of(path), why });
    }
    // Placed on a released map, or loaded by the code, but there's no file.
    let have: BTreeSet<u16> = files.iter().map(|f| f.0).collect();
    for id in CODE_PRIMS.iter().copied().filter(|id| !have.contains(id) && *id != 238) {
        r.missing_prims.push((id, "loaded by the code".into()));
    }
    for (id, maps) in &placed {
        let rel: Vec<&String> = maps.iter().filter(|m| on_released(m)).collect();
        if !have.contains(id) && !CODE_PRIMS.contains(id) && !rel.is_empty() {
            r.missing_prims.push((*id, format!("on {}", rel.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))));
        }
    }
    r.missing_prims.sort();
    r.unused_prims.sort_by_key(|p| (p.id, p.file.to_ascii_lowercase()));
}

/// The sound a name in the sound table plays, relative to the install (`GetFullName`,
/// DDLibrary/Source/MFX_Miles.cpp:442): a name with a `-` in it (the taunts and grunts) is
/// read from `talk2\misc\`, and a `musicNN\` name from the folder of the mission's music
/// world, in the place of `NN`. The others are in `data\sfx\1622\`. Lowercased.
fn sound_paths(name: &str, worlds: &BTreeSet<u32>) -> Vec<String> {
    let name = name.to_ascii_lowercase().replace('/', "\\");
    if name.contains('-') {
        vec![format!("talk2\\misc\\{}", base(&name))]
    } else if let Some(rest) = name.strip_prefix("music").filter(|r| r.len() > 2 && r.as_bytes()[..2].iter().all(u8::is_ascii_digit)) {
        worlds.iter().map(|w| format!("data\\sfx\\1622\\music{w:02}{}", &rest[2..])).collect()
    } else {
        vec![format!("data\\sfx\\1622\\{name}")]
    }
}

/// `released`: the missions `urban.sty` lists or `fallen.exe` names, lowercased.
fn sounds(game: &Path, exe_strings: &[String], parsed: &[(String, Mission)], released: &BTreeSet<String>, r: &mut Report) {
    // The music world: the mission's, unless `levels\mworlds.txt` has one for it (elev.cpp:1843);
    // none is world 1 (MFX_Miles.cpp:455).
    let mworlds: BTreeMap<String, u32> = uc_formats::resolve(game, "levels\\mworlds.txt").and_then(|p| std::fs::read(p).ok())
        .map(|b| b.iter().map(|&c| c as char).collect::<String>()).unwrap_or_default()
        .lines().filter_map(|l| {
            let (name, w) = l.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), w.trim().parse().ok()?))
        }).collect();
    r.music_worlds = parsed.iter().filter(|(n, _)| released.contains(&n.to_ascii_lowercase()))
        .map(|(n, m)| mworlds.get(&n.to_ascii_lowercase()).copied().unwrap_or(m.music_world as u32).max(1))
        .collect();
    // The sound table (`sound_list`, sound_id.cpp), as the exe has it: its `.wav` names, less
    // the mission recordings in `talk2\`, which are the voice lines'.
    let table: Vec<String> = exe_strings.iter()
        .map(|s| s.trim().trim_start_matches("*DL*").trim().to_string())
        .filter(|s| s.to_ascii_lowercase().ends_with(".wav") && !s.contains('%') && !s.to_ascii_lowercase().starts_with("talk2\\"))
        .collect();
    let mut played: BTreeSet<String> = BTreeSet::new();
    let exists = |p: &str| uc_formats::resolve(game, p).is_some();
    for name in &table {
        let paths = sound_paths(name, &r.music_worlds);
        // A music track counts as there if any world has it.
        if !paths.iter().any(|p| exists(p)) {
            // With no worlds, a music track is in no folder.
            let music = sound_paths(name, &BTreeSet::new()).is_empty();
            r.missing_sounds.push(if music { format!("data\\sfx\\1622\\musicNN{}", &name[7..]) } else { paths[0].clone() });
        }
        played.extend(paths);
    }
    r.missing_sounds.sort();
    r.missing_sounds.dedup();
    let mut on_disk: Vec<String> = ["data\\sfx", "talk2\\misc"].iter()
        .filter_map(|d| uc_formats::resolve(game, d))
        .flat_map(|d| walk(&d))
        .filter(|p| ext(p) == "wav")
        .filter_map(|p| Some(p.strip_prefix(game).ok()?.to_string_lossy().replace('/', "\\")))
        .collect();
    on_disk.sort_by_key(|p| p.to_ascii_lowercase());
    r.sounds = on_disk.len();
    r.unused_sounds = on_disk.into_iter().filter(|p| !played.contains(&p.to_ascii_lowercase())).collect();
}

/// `released`: the missions `urban.sty` lists or `fallen.exe` names, lowercased.
fn texts(game: &Path, exe_strings: &[String], parsed: &[(String, Mission)], released: &BTreeSet<String>, r: &mut Report) {
    // What names a text file: the exe, `config.ini`'s language, and each mission's chatter
    // file (`EWAY_load_fake_wander_text`, its `citsez` field).
    let config: String = std::fs::read(game.join("config.ini")).map(|b| b.iter().map(|&c| c as char).collect()).unwrap_or_default();
    let mut named: BTreeMap<String, (bool, Vec<String>)> = BTreeMap::new();
    let key = |s: &str| s.trim().to_ascii_lowercase().replace('/', "\\");
    for s in exe_strings.iter().map(String::as_str).chain(config.lines().filter_map(|l| l.split_once('=').map(|(_, v)| v))) {
        let k = key(s);
        if k.starts_with("text\\") && k.ends_with(".txt") {
            named.entry(k).or_default().0 = true;
        }
    }
    for (name, m) in parsed {
        let k = key(&m.citsez);
        if k.starts_with("text\\") && k.ends_with(".txt") {
            let e = named.entry(k).or_default();
            if released.contains(&name.to_ascii_lowercase()) {
                e.0 = true;
            } else {
                e.1.push(name.clone());
            }
        }
    }
    let Some(dir) = uc_formats::resolve(game, "text") else { return };
    let mut on_disk: Vec<String> = walk(&dir).into_iter().filter(|p| ext(p) == "txt")
        .filter_map(|p| Some(p.strip_prefix(game).ok()?.to_string_lossy().replace('/', "\\")))
        .collect();
    on_disk.sort_by_key(|p| p.to_ascii_lowercase());
    r.texts = on_disk.len();
    for p in &on_disk {
        // `config.ini` chooses the language: any `lang_*.txt` it could name counts.
        let k = key(p);
        let language = k.strip_prefix("text\\lang_").is_some_and(|l| !l.contains('\\'));
        match named.get(&k) {
            Some((true, _)) => {}
            _ if language => {}
            Some((false, by)) => r.unused_texts.push((p.clone(), by.clone())),
            None => r.unused_texts.push((p.clone(), Vec::new())),
        }
    }
    let have: BTreeSet<String> = on_disk.iter().map(|p| key(p)).collect();
    r.missing_texts = named.into_iter().filter(|(k, (used, _))| *used && !have.contains(k))
        .map(|(k, _)| {
            let by: Vec<String> = parsed.iter().filter(|(n, m)| released.contains(&n.to_ascii_lowercase()) && key(&m.citsez) == k).map(|(n, _)| n.clone()).collect();
            (k, by)
        })
        .collect();

    // The messages that are never shown.
    for (name, m) in parsed {
        let rel = released.contains(&name.to_ascii_lowercase());
        let owners = m.event_points.iter().filter(|e| uc_formats::ucm::has_text(e.kind)).count();
        for t in m.messages().skip(owners) {
            r.unshown.push((name.clone(), rel, None, t.to_string()));
        }
        let dead = dead_points(m);
        for (i, t) in m.point_texts() {
            if dead.contains(&i) {
                r.unshown.push((name.clone(), rel, Some(i), t));
            }
        }
    }
}

/// The points of `m` that can never go off because what they wait for isn't in the mission:
/// a `dependency` (or a `countdown`'s or `random`'s) on a point that isn't there, a
/// `boolean_and` with either point missing, a `boolean_or` with both, or any of those on a
/// point that can never go off itself. Not when the condition is inverted (`INVERSE`),
/// when a missing point makes it always true. A point is "there" if it's used.
fn dead_points(m: &Mission) -> BTreeSet<usize> {
    use uc_formats::ucm::{point_flags::INVERSE, trigger};
    let there: BTreeSet<usize> = m.event_points.iter().map(|e| e.index).collect();
    let mut dead: BTreeSet<usize> = BTreeSet::new();
    loop {
        let ok = |i: u16| there.contains(&(i as usize)) && !dead.contains(&(i as usize));
        let more: Vec<usize> = m.event_points.iter()
            .filter(|e| !dead.contains(&e.index) && e.flags & INVERSE == 0)
            .filter(|e| match e.trigger {
                trigger::DEPENDENCY | trigger::RANDOM => !ok(e.ep_ref),
                // A countdown with no point to wait for starts at once (`r1 != 0`).
                trigger::COUNTDOWN | trigger::VISIBLE_COUNTDOWN => e.ep_ref != 0 && !ok(e.ep_ref),
                trigger::BOOLEAN_AND => !ok(e.ep_ref) || !ok(e.ep_ref_bool),
                trigger::BOOLEAN_OR => !ok(e.ep_ref) && !ok(e.ep_ref_bool),
                _ => false,
            })
            .map(|e| e.index).collect();
        if more.is_empty() {
            return dead;
        }
        dead.extend(more);
    }
}

/// The people's animations (`darci1.all`, `ANIM_*` in fallen/Headers/animate.h) the PC game
/// code starts: every id it names in fallen/Source and fallen/DDEngine, comments left out,
/// less the four it names only in code that never runs (`HIT_WALL` under `if(0)`,
/// `STOP_RUN` in the `else` of an `if(1)`, interfac.cpp:2483, `NECK_SNAP` and `WARM_HANDS`
/// never started; #342). No animation is started by a bare number.
const PLAYED_ANIMS: &[usize] = &[
    1, 2, 3, 7, 8, 13, 14, 15, 16, 19, 20, 25, 33, 34, 37, 38, 39, 40, 41, 42,
    43, 45, 53, 54, 57, 58, 59, 60, 62, 64, 65, 68, 77, 78, 81, 86, 88, 89, 91, 92,
    96, 104, 105, 109, 110, 117, 121, 122, 124, 128, 129, 130, 131, 134, 137, 138, 140, 141, 143, 144,
    146, 147, 148, 150, 151, 154, 155, 156, 157, 158, 159, 161, 162, 164, 165, 166, 167, 168, 169, 170,
    171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 184, 188, 189, 193, 194, 198, 199, 200, 201,
    202, 206, 208, 209, 210, 212, 213, 216, 217, 218, 219, 222, 223, 225, 226, 227, 228, 229, 230, 231,
    232, 233, 234, 235, 236, 238, 240, 245, 246, 247, 248, 250, 252, 254, 258, 259, 260, 261, 262, 263,
    264, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284,
    285, 286, 287, 288, 290, 292, 293, 294, 295, 296, 297, 298, 304, 305, 306, 307, 309, 310, 311, 312,
    313, 314, 315, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 332, 333, 334, 335, 336, 337, 338,
    339, 341, 342, 343, 344, 345, 346, 347, 348, 349, 350, 351, 353, 355, 358, 360, 368, 370, 371, 375,
    376, 377, 378, 380, 382, 383, 384, 386, 387, 388, 389, 390, 391, 392, 394, 395, 396, 397, 398, 399,
    400,
];

/// The `ANIM_*` names of the ids [`PLAYED_ANIMS`] leaves out (animate.h).
const ANIM_NAMES: &[(usize, &str)] = &[
    (10, "DRAW_GUN"),
    (12, "JUMP_UP_GRAB"),
    (18, "SIDLE"),
    (22, "HEAD_HIT_BIG"),
    (24, "GUT_HIT_BIG"),
    (26, "HEAD_HIT_SMALL"),
    (27, "GUT_HIT_SMALL"),
    (28, "HEAD_HIT_BEHIND"),
    (32, "JUMP2"),
    (36, "PULL_UP"),
    (46, "RUN_JUMP_RIGHT"),
    (47, "FALL_LONG"),
    (69, "BSTEP"),
    (80, "FATAL_FALL"),
    (82, "SHOTGUN_IDLE_OLD"),
    (83, "SHOTGUN_SHOOT"),
    (85, "SHOTGUN_WALK"),
    (87, "LOB2"),
    (90, "EXIT_CAR"),
    (93, "FLIP_RIGHT_CONT"),
    (94, "FLIP_LEFT_CONT"),
    (99, "HOSTAGE_STAND"),
    (100, "HOSTAGE_SIT"),
    (101, "HOSTAGE_DESPAIR"),
    (102, "HOSTAGE_READY"),
    (103, "HOSTAGE_RELEASE"),
    (112, "BLOCK_LOW"),
    (118, "RUN_JUMP_MLEFT"),
    (132, "TURN90_LEFT"),
    (133, "TURN90_RIGHT"),
    (135, "LEVER"),
    (139, "BATMAN"),
    (142, "GET_DOWN"),
    (149, "PUNCH_COMBO3B"),
    (163, "PUNCH_BEHIND"),
    (185, "PUNCH_COMBO2b"),
    (186, "PUNCH_COMBO3b"),
    (187, "KICK_COMBO3b"),
    (211, "SHOTGUN_WHIP3"),
    (221, "FIGHT_SHUFFLE"),
    (237, "BIKE_RIDE"),
    (242, "LEG_SWEEP_LOW"),
    (253, "AK_AIM"),
    (255, "SNIPE_AIM"),
    (256, "SNIPE_FIRE"),
    (257, "SNIPE_NORM"),
    (308, "SHOT_DEAD_UZI"),
    (316, "HIT_WALL"),
    (317, "DIVE_LEFT"),
    (318, "DIVE_RIGHT"),
    (340, "STOP_RUN"),
    (352, "PRESS_WALL_FREEZE_R_PISTOL"),
    (354, "PRESS_WALL_FREEZE_R_AK"),
    (356, "PRESS_WALL_FREEZE_R"),
    (361, "BIKE_FALL_OVER_BARS"),
    (379, "FIGHT_SHUFFLE_BAT"),
    (381, "LAND_VERT_AK"),
];

/// The files `fallen.exe` names that only the PlayStation build loads (`save_psx`,
/// `setup_people_anims`, fallen/Source/Anim.cpp:937).
const PSX_ANIMS: &[&str] = &["psxroper.all", "roper_up.all"];

/// `MAP_THING_TYPE_ANIM_PRIM` (Editor/Headers/Thing.h): a map thing that's an animated prim,
/// `anim<index>.all`.
const MAP_ANIM_PRIM: u16 = 8;

/// `released`: the missions `urban.sty` lists or `fallen.exe` names, lowercased.
fn anims(game: &Path, exe_strings: &[String], parsed: &[(String, Mission)], released: &BTreeSet<String>, r: &mut Report) {
    let Some(data) = uc_formats::resolve(game, "data") else { return };
    let mut on_disk: Vec<PathBuf> = files(&data).into_iter().filter(|p| ext(p) == "all").collect();
    on_disk.sort_by_key(|p| file_name(p).to_ascii_lowercase());
    r.anim_files = on_disk.len();
    let named: BTreeSet<String> = exe_strings.iter().map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| s.ends_with(".all") && !s.contains('%')).collect();

    // The anim prims a released mission loads (`load_needed_anim_prims`,
    // `load_level_anim_prims`, io.cpp:525): the ones on its map, and its creatures', but not
    // the bat's, which is never made (`BAT_create` is compiled out).
    let rel: Vec<&Mission> = parsed.iter().filter(|(n, _)| released.contains(&n.to_ascii_lowercase())).map(|(_, m)| m).collect();
    let maps: BTreeSet<String> = rel.iter().map(|m| base(&m.map).to_ascii_lowercase()).collect();
    let mut prims: BTreeSet<u16> = BTreeSet::new();
    // Every map's, for saying where the others are.
    let mut placed: BTreeMap<u16, BTreeSet<String>> = BTreeMap::new();
    for path in walk(game).into_iter().filter(|p| ext(p) == "iam") {
        let file = file_name(&path).to_ascii_lowercase();
        if let Ok(map) = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| uc_formats::iam::Map::parse(&b).map_err(|e| e.to_string())) {
            for t in map.things.iter().filter(|t| t.kind == MAP_ANIM_PRIM) {
                placed.entry(t.index_other).or_default().insert(file.clone());
                if maps.contains(&file) {
                    prims.insert(t.index_other);
                }
            }
        }
    }
    for m in &rel {
        prims.extend(m.event_points.iter().filter(|e| e.kind == waypoint::CREATE_CREATURE && (2..=4).contains(&e.data[0])).map(|e| e.data[0] as u16));
    }
    for p in &on_disk {
        let lower = file_name(p).to_ascii_lowercase();
        let why = if let Some(n) = lower.strip_prefix("anim").and_then(|s| s.strip_suffix(".all")).filter(|n| n.len() == 3 && n.bytes().all(|c| c.is_ascii_digit())) {
            let n: u16 = n.parse().unwrap_or(0);
            (!prims.contains(&n)).then(|| match placed.get(&n) {
                Some(on) => format!("an anim prim only on maps no released mission uses ({})", on.iter().cloned().collect::<Vec<_>>().join(", ")),
                None => "an anim prim on no map, and no released mission's creature".to_string(),
            })
        } else if PSX_ANIMS.contains(&lower.as_str()) {
            Some("the PlayStation's".to_string())
        } else {
            (!named.contains(&lower) && !named.contains(&format!("data\\{lower}"))).then(|| "not named".to_string())
        };
        if let Some(why) = why {
            r.unused_anim_files.push((file_name(p), why));
        }
    }
    let have: BTreeSet<String> = on_disk.iter().map(|p| file_name(p).to_ascii_lowercase()).collect();
    r.missing_anim_files = named.iter().map(|n| base(n).to_string()).filter(|n| !have.contains(n)).collect();

    // The people's animations nothing plays: not by the code, nor by a released mission's
    // cutscene (an animation packet's index, less 1024 for Roper's, `PT_ANIM`, cutscene.cpp:1390).
    let Some(chunk) = uc_formats::resolve(game, "data\\darci1.all").and_then(|p| std::fs::read(p).ok())
        .and_then(|b| uc_formats::all::AnimChunk::parse(&b).ok()) else { return };
    let mut played: BTreeSet<usize> = PLAYED_ANIMS.iter().copied().collect();
    for m in &rel {
        for c in m.cutscenes() {
            played.extend(c.channels.iter().flat_map(|ch| &ch.packets).filter(|p| p.kind == 1 && p.index < 1024).map(|p| p.index as usize));
        }
    }
    let present: Vec<usize> = chunk.anims.iter().enumerate().filter(|(_, a)| a.is_some()).map(|(i, _)| i).collect();
    r.people_anims = present.len();
    r.unplayed_anims = present.into_iter().filter(|i| !played.contains(i))
        .map(|i| (i, ANIM_NAMES.iter().find(|(n, _)| *n == i).map_or("", |(_, s)| s)))
        .collect();
}

fn print(r: &Report) {
    let unlisted: Vec<&MissionFile> = r.missions.iter().filter(|m| !m.listed).collect();
    println!("Missions: {} files in levels\\, {} of them not in urban.sty's list:", r.missions.len(), unlisted.len());
    for m in unlisted {
        let exe = if m.in_exe { ", named in fallen.exe" } else { "" };
        match &m.error {
            None => println!("  {:<16} version {:>2}, {:>6} bytes, parses{exe}", m.name, m.version, m.size),
            Some(e) => println!("  {:<16} version {:>2}, {:>6} bytes, doesn't parse: {e}{exe}", m.name, m.version, m.size),
        }
    }
    let n: usize = r.unplayed.values().map(Vec::len).sum();
    println!();
    println!("Voice lines: {} recordings in talk2\\ belong to a mission that parses; {n} of them nothing plays:", r.recordings);
    let mut by: Vec<(&String, &Vec<String>)> = r.unplayed.iter().collect();
    by.sort_by_key(|(m, f)| (std::cmp::Reverse(f.len()), m.to_ascii_lowercase()));
    for (m, f) in by {
        println!("  {m} ({}): {}", f.len(), f.join(", "));
    }
    if !r.odd.is_empty() {
        println!("  with more than one letter after the number, which no line's name has: {}", r.odd.join(", "));
    }
    if !r.strays.is_empty() {
        println!("  named as a mission's, but no such mission: {}", r.strays.join(", "));
    }
    if !r.unchecked.is_empty() {
        let list: Vec<String> = r.unchecked.iter().map(|(m, n)| format!("{m} ({n})")).collect();
        println!("  not checked, their missions don't parse: {}", list.join(", "));
    }
    println!();
    println!("Loose textures and meshes: {} files, {} that neither fallen.exe nor a mesh it loads refers to:", r.loose, r.unreferenced.len());
    for p in &r.unreferenced {
        println!("  {p}");
    }
    println!();
    println!("Prims: {} files in server\\prims\\, {} the released game never draws:", r.prims, r.unused_prims.len());
    let shadowed: Vec<String> = r.unused_prims.iter().filter(|p| p.why == PrimWhy::Shadowed).map(|p| p.id.to_string()).collect();
    if !shadowed.is_empty() {
        println!("  old primNNN.prm files with an nprimNNN.prm of the same number, which is loaded instead ({}): {}",
            shadowed.len(), shadowed.join(" "));
    }
    type Pick = fn(&PrimWhy) -> bool;
    let groups: [(&str, Pick); 6] = [
        ("a name load_prim_object never builds", |w| *w == PrimWhy::OddName),
        ("a number load_prim_object doesn't load (0, or above 265)", |w| *w == PrimWhy::OutOfRange),
        ("skipped by load_prim_object", |w| *w == PrimWhy::Skipped),
        ("only on the maps of missions neither urban.sty nor fallen.exe names", |w| matches!(w, PrimWhy::Unreleased(_))),
        ("only on maps no mission uses", |w| matches!(w, PrimWhy::UnusedMap(_))),
        ("on no map, and the code doesn't load it", |w| *w == PrimWhy::Nowhere),
    ];
    for (title, pick) in groups {
        let list: Vec<&PrimFile> = r.unused_prims.iter().filter(|p| pick(&p.why)).collect();
        if list.is_empty() {
            continue;
        }
        println!("  {title} ({}):", list.len());
        for p in list {
            let at = match &p.why {
                PrimWhy::Unreleased(m) | PrimWhy::UnusedMap(m) => format!(": {}", m.join(", ")),
                _ => String::new(),
            };
            println!("    {:<14} {:<24}{at}", p.file, p.name);
        }
    }
    if !r.missing_prims.is_empty() {
        let list: Vec<String> = r.missing_prims.iter().map(|(id, w)| format!("{id} ({w})")).collect();
        println!("  placed or loaded, but no file: {}", list.join("; "));
    }
    println!();
    let worlds: Vec<String> = r.music_worlds.iter().map(u32::to_string).collect();
    println!("Sounds: {} in data\\sfx\\ and talk2\\misc\\, {} that the sound table in fallen.exe doesn't name (the released missions' music worlds are {}):",
        r.sounds, r.unused_sounds.len(), worlds.join(", "));
    let mut by: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for p in &r.unused_sounds {
        let (dir, file) = p.rsplit_once('\\').unwrap_or(("", p));
        by.entry(dir.to_string()).or_default().push(file);
    }
    for (dir, files) in by {
        println!("  {dir}\\ ({}): {}", files.len(), files.join(", "));
    }
    if !r.missing_sounds.is_empty() {
        println!("  named in the table, but not there ({}): {}", r.missing_sounds.len(), r.missing_sounds.join(", "));
    }
    println!();
    println!("Text: {} files in text\\, {} that neither fallen.exe, config.ini nor a released mission names:", r.texts, r.unused_texts.len());
    // A folder of nothing but unused files on one line (`text\backup\`).
    let parent = |p: &str| p.rsplit_once('\\').map_or(String::new(), |(d, _)| d.to_string());
    let whole: BTreeSet<String> = r.unused_texts.iter().map(|(p, _)| parent(p)).filter(|d| d != "text").collect();
    for d in &whole {
        let files: Vec<&str> = r.unused_texts.iter().filter(|(p, _)| parent(p) == *d).map(|(p, _)| base(p)).collect();
        println!("  {d}\\ ({}): {}", files.len(), files.join(", "));
    }
    for (p, by) in r.unused_texts.iter().filter(|(p, _)| !whole.contains(&parent(p))) {
        let by = if by.is_empty() { String::new() } else { format!(": the chatter of {}", by.join(", ")) };
        println!("  {p}{by}");
    }
    if !r.missing_texts.is_empty() {
        let list: Vec<String> = r.missing_texts.iter()
            .map(|(p, by)| if by.is_empty() { p.clone() } else { format!("{p} ({})", by.join(", ")) }).collect();
        println!("  named, but not there: {}", list.join(", "));
    }
    println!("  (which of lang_english.txt's lines are shown isn't checked: the release's front end isn't in the source)");
    println!();
    println!();
    println!("Animation files: {} .all files in data\\, {} the PC game never loads:", r.anim_files, r.unused_anim_files.len());
    let mut by: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (f, why) in &r.unused_anim_files {
        by.entry(why.as_str()).or_default().push(f.as_str());
    }
    for (why, files) in by {
        println!("  {why} ({}): {}", files.len(), files.join(", "));
    }
    if !r.missing_anim_files.is_empty() {
        println!("  named, but not there: {}", r.missing_anim_files.join(", "));
    }
    println!();
    println!("People's animations: {} in darci1.all, {} that neither the code nor a released mission's cutscene plays:", r.people_anims, r.unplayed_anims.len());
    let list: Vec<String> = r.unplayed_anims.iter().map(|(i, n)| if n.is_empty() { i.to_string() } else { format!("{i} {n}") }).collect();
    println!("  {}", list.join(", "));
    println!("  (the other files' own animations aren't checked one by one: they're numbered their own way, NROPER_*, CIV_*)");
    println!();
    println!("Mission messages never shown ({}): no point owns them, or their point waits for one that isn't in the mission:", r.unshown.len());
    for (m, rel, point, t) in &r.unshown {
        let at = point.map_or("no point".to_string(), |p| format!("point {p}"));
        let rel = if *rel { "" } else { ", unreleased" };
        println!("  {m} ({at}{rel}): {t}");
    }
}

/// The spoken lines' recordings a mission plays, as file names in `talk2\`.
fn spoken(name: &str, m: &Mission) -> Vec<String> {
    let texts = m.point_texts();
    let mut out = Vec::new();
    for e in m.event_points.iter().filter(|e| e.used != 0) {
        let Some(text) = texts.get(&e.index) else { continue };
        let n = if e.data[9] > 0 { e.data[9] as usize } else { e.index };
        match e.kind {
            // A bonus points point says its line as a message does (`Command::Message`).
            waypoint::MESSAGE | waypoint::BONUS_POINTS => out.push(format!("{name}.ucm{n}.wav")),
            waypoint::CONVERSATION => {
                let lines = text.split('|').filter(|l| !l.trim().is_empty()).count();
                out.extend((0..lines.min(26)).map(|i| format!("{name}.ucm{n}{}.wav", (b'A' + i as u8) as char)));
            }
            _ => {}
        }
    }
    // A line with no recording of its own says the one it was recorded as under its old
    // number ([`uc_formats::speech::RENUMBERED`]).
    let under: Vec<String> = out.iter()
        .filter_map(|f| uc_formats::speech::RENUMBERED.iter().find(|(line, _)| line.eq_ignore_ascii_case(f)).map(|(_, r)| r.to_string()))
        .collect();
    out.extend(under);
    out
}

/// The mission a recording is named for (`<mission>.ucm<n>[letter].wav`), lowercased, and
/// how many letters follow the number.
fn mission_of(file: &str) -> Option<(String, usize)> {
    let lower = file.to_ascii_lowercase();
    let (mission, rest) = lower.rsplit_once(".ucm")?;
    let rest = rest.strip_suffix(".wav")?;
    let digits = rest.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
        .then(|| (mission.to_string(), rest.len() - digits.len()))
}

/// The mission files `urban.sty` lists (its lines' eighth field), lowercased without `.ucm`.
fn listed_missions(game: &Path) -> BTreeSet<String> {
    let text = uc_formats::sty::read(game);
    uc_formats::sty::lines(&text).filter_map(|l| Some(l.mission()?.to_ascii_lowercase())).collect()
}

/// What `fallen.exe` names: files, by name or `sprintf` pattern, and folders it reads whole.
struct Refs {
    /// File names (no folder), lowercased; with `%` patterns.
    names: Vec<String>,
    /// Folders, lowercased, with `\` and a trailing `\`.
    folders: Vec<String>,
}

impl Refs {
    fn new(strings: &[String]) -> Refs {
        let (mut names, mut folders) = (Vec::new(), Vec::new());
        for s in strings {
            let s = s.trim().to_ascii_lowercase().replace('/', "\\");
            if s.ends_with('\\') && !s.contains('%') && s.contains('\\') {
                folders.push(s);
            } else if [".tga", ".imp", ".sex"].iter().any(|e| s.ends_with(e)) {
                names.push(base(&s).to_string());
            }
        }
        Refs { names, folders }
    }

    /// Whether `path` (relative to the install) is named, by name or pattern.
    fn names(&self, path: &str) -> bool {
        let lower = path.to_ascii_lowercase();
        self.names.iter().any(|p| matches(p, base(&lower)))
    }

    /// Whether `path` is in a folder that's named and read whole: none of `all` (every file,
    /// relative to the install) directly in it is named itself.
    fn in_folder(&self, path: &str, all: &[String]) -> bool {
        let lower = path.to_ascii_lowercase();
        let parent = |p: &str| p.rsplit_once('\\').map_or(String::new(), |(d, _)| format!("{d}\\"));
        let dir = parent(&lower);
        self.folders.contains(&dir)
            && !all.iter().any(|p| parent(&p.to_ascii_lowercase()) == dir && self.names(p))
    }
}

/// Whether `name` fits the `sprintf` pattern `pat`: `%s` is anything, `%d` one or more
/// digits, `%03d` (or `%3d`) that many, `%c` one character.
fn matches(pat: &str, name: &str) -> bool {
    let (p, n) = (pat.as_bytes(), name.as_bytes());
    if p.is_empty() {
        return n.is_empty();
    }
    if p[0] != b'%' {
        return !n.is_empty() && n[0] == p[0] && matches(&pat[1..], &name[1..]);
    }
    // `%`, a width (`03`, `3`, or none), then the conversion.
    let digits = p[1..].iter().take_while(|c| c.is_ascii_digit()).count();
    let width: usize = pat[1..1 + digits].parse().unwrap_or(0);
    let Some(&conv) = p.get(1 + digits) else { return false };
    let rest = &pat[2 + digits..];
    match Some(conv) {
        Some(b's') => (0..=n.len()).any(|i| matches(rest, &name[i..])),
        Some(b'c') => !n.is_empty() && matches(rest, &name[1..]),
        Some(b'd') => {
            let digits = n.iter().take_while(|c| c.is_ascii_digit()).count();
            if width > 0 {
                digits >= width && matches(rest, &name[width..])
            } else {
                (1..=digits).any(|i| matches(rest, &name[i..]))
            }
        }
        _ => false,
    }
}

/// The runs of four or more printable characters in `bytes`, as `strings` finds them.
fn strings(bytes: &[u8]) -> Vec<String> {
    bytes.split(|&b| !(0x20..0x7f).contains(&b))
        .filter(|s| s.len() >= 4)
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

fn base(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

fn ext(p: &Path) -> String {
    p.extension().map_or(String::new(), |e| e.to_string_lossy().to_ascii_lowercase())
}

fn stem(p: &Path) -> String {
    p.file_stem().map_or(String::new(), |s| s.to_string_lossy().into_owned())
}

fn file_name(p: &Path) -> String {
    p.file_name().map_or(String::new(), |s| s.to_string_lossy().into_owned())
}

fn files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect()).unwrap_or_default()
}

/// Every file under `dir`.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).collect::<Vec<_>>()).unwrap_or_default() {
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprintf_patterns_match_the_names_they_make() {
        assert!(matches("tex%03d.tga", "tex007.tga"));
        assert!(!matches("tex%03d.tga", "tex7.tga"));
        assert!(matches("%ssky.tga", "sky.tga"));
        assert!(matches("%stex%03dhi.tga", "world3tex012hi.tga"));
        assert!(matches("%s.ucm%d%c.wav", "police1.ucm12b.wav"));
        assert!(matches("shot%d.tga", "shot15.tga"));
        assert!(!matches("shot.tga", "shot1.tga"));
        assert!(matches("font.tga", "font.tga"));
    }

    #[test]
    fn a_recording_is_named_for_its_mission() {
        assert_eq!(mission_of("Semtex.ucm71A.wav"), Some(("semtex".into(), 1)));
        assert_eq!(mission_of("Semtex.ucm106EB.wav"), Some(("semtex".into(), 2)));
        assert_eq!(mission_of("Copy of Semtex.ucm106.wav"), Some(("copy of semtex".into(), 0)));
        assert_eq!(mission_of("police1.ucm190.wav"), Some(("police1".into(), 0)));
        assert_eq!(mission_of("GRED-ALERT.wav"), None);
        assert_eq!(mission_of("roperm24.wav"), None);
    }

    #[test]
    fn folders_and_names_from_the_exe() {
        let refs = Refs::new(&["data\\font3d\\all\\".into(), "data\\".into(), "Meshes\\heart.imp".into(),
                               "e3load.tga".into(), "%ssex%03dhi.sex".into()]);
        let all: Vec<String> = ["data\\font3d\\All\\0.SEX", "data\\font3d\\Chaos\\0.SEX", "data\\e3load.tga", "data\\e3miss.tga"]
            .map(String::from).to_vec();
        assert!(refs.in_folder(&all[0], &all));
        assert!(!refs.in_folder(&all[1], &all));
        // `data\` is a folder it builds names in: its files count only when named.
        assert!(refs.names(&all[2]));
        assert!(!refs.names(&all[3]) && !refs.in_folder(&all[3], &all));
        assert!(refs.names("Meshes\\heart.imp"));
        assert!(!refs.names("Meshes\\heart.SEX"));
    }

    /// Against the install: what #467 and #470 found by hand, the outro's textures counted
    /// as used through its meshes (#468), the prims, the sounds, the text and the animations.
    #[test]
    fn the_install_matches_what_was_found_by_hand() {
        let Some(game) = uc_formats::default_install() else { return };
        let r = report(&game).unwrap();
        // The editor's old file versions (5 and before): none listed. Whether they parse is
        // #470's to say; every later one does.
        let old: Vec<&MissionFile> = r.missions.iter().filter(|m| m.version <= 5).collect();
        assert_eq!(old.len(), 16);
        assert!(old.iter().all(|m| !m.listed));
        assert!(r.missions.iter().filter(|m| m.version > 5).all(|m| m.error.is_none()));
        assert!(r.missions.iter().any(|m| m.name == "army1" && m.version == 5));
        // Of #467's 66, 21 are bonus points' lines and 5 are said by lines renumbered since
        // (`uc_formats::speech::RENUMBERED`): docs/scripts/unplayed-recordings.md has the rest.
        assert_eq!(r.unplayed.values().map(Vec::len).sum::<usize>(), 40);
        assert_eq!(r.unplayed["mission2"].len(), 5);
        assert_eq!(r.strays, ["Copy of Semtex.ucm106.wav"]);
        assert_eq!(r.odd.len(), 7);
        // The front end's pictures no code that shipped loads (Attract.cpp's are commented
        // out), and the two 3D fonts besides `all`.
        for unused in ["data\\gamelogo.tga", "data\\e3title.tga", "data\\font3d\\Chaos\\A.SEX"] {
            assert!(r.unreferenced.iter().any(|p| p == unused), "{unused} counted as used");
        }
        assert_eq!(r.prims, 433);
        assert!(r.missing_prims.is_empty(), "{:?}", r.missing_prims);
        let why = |f: &str| r.unused_prims.iter().find(|p| p.file == f).map(|p| &p.why);
        assert_eq!(r.unused_prims.iter().filter(|p| p.why == PrimWhy::Shadowed).count(), 168);
        assert_eq!(why("nprim088bak.prm"), Some(&PrimWhy::OddName));
        assert_eq!(why("nprim238.prm"), Some(&PrimWhy::Skipped));
        // The chopper's searchlight (#466), and the bike's frame (`#ifdef BIKE` is off).
        assert_eq!(why("prim151.prm"), Some(&PrimWhy::Nowhere));
        assert_eq!(why("prim140.prm"), Some(&PrimWhy::Nowhere));
        assert!(matches!(why("nprim243.prm"), Some(PrimWhy::Unreleased(m)) if m.iter().any(|m| m.starts_with("botanic1"))));
        assert_eq!(r.sounds, 1077);
        // The taunts are read from talk2\misc\, not their folders in data\sfx\1622\; the
        // barks nothing names; a footstep's copy at the top.
        for unused in ["data\\sfx\\1622\\darci\\DARCI-TAUNT01.wav", "talk2\\misc\\COP-ALERT01.wav", "data\\sfx\\1622\\SoftStep1.wav"] {
            assert!(r.unused_sounds.iter().any(|p| p == unused), "{unused} counted as played");
        }
        for played in ["talk2\\misc\\DARCI-TAUNT01.wav", "talk2\\misc\\GRED-PAIN01.wav", "data\\sfx\\1622\\Music04\\darci\\Fight1.wav",
                       "data\\sfx\\1622\\Footsteps\\SoftStep1.wav"] {
            assert!(!r.unused_sounds.iter().any(|p| p == played), "{played} counted as unused");
        }
        assert_eq!(r.unused_sounds.iter().filter(|p| p.starts_with("talk2\\misc\\")).count(), 337);
        assert!(r.missing_sounds.iter().any(|p| p == "data\\sfx\\1622\\musicNN\\Sneak.wav"));
        assert_eq!(r.texts, 56);
        assert!(r.unused_texts.iter().any(|(p, by)| p == "text\\silly.txt" && by.is_empty()));
        assert!(r.unused_texts.iter().any(|(p, _)| p == "text\\backup\\police11.txt"));
        // Chatter a released mission names, and the language.
        for used in ["text\\police11.txt", "text\\citsez.txt", "text\\annoyed.txt", "text\\lang_french.txt"] {
            assert!(!r.unused_texts.iter().any(|(p, _)| p == used), "{used} counted as unused");
        }
        // Factory1 names a newer chatter file than the one there.
        assert!(r.missing_texts.iter().any(|(p, by)| p == "text\\factory13.txt" && by == &["factory1"]));
        // Carbomb2's debrief waits on points 187 and 188, which aren't in it.
        let dead: Vec<Option<usize>> = r.unshown.iter().filter(|u| u.0 == "carbomb2").map(|u| u.2).collect();
        assert_eq!(dead, [Some(172), Some(182)]);
        assert!(r.unshown.iter().all(|u| !u.1), "a released mission's message is never shown: {:?}", r.unshown);
        assert_eq!(r.anim_files, 60);
        let why = |f: &str| r.unused_anim_files.iter().find(|(n, _)| n == f).map(|(_, w)| w.as_str());
        assert_eq!(why("psxroper.all"), Some("the PlayStation's"));
        assert_eq!(why("DOG2.all"), Some("not named"));
        assert!(why("anim010.all").is_some_and(|w| w.contains("botanic.iam")));
        // The people's, Roper's, the civilians', and the creatures'.
        for used in ["darci1.all", "roper.all", "rthug.all", "newciv.all", "anim002.all", "anim003.all", "anim004.all"] {
            assert_eq!(why(used), None, "{used} counted as unused");
        }
        assert_eq!(r.missing_anim_files, ["roper_up.all"]);
        assert_eq!(r.people_anims, 274);
        let unplayed: Vec<usize> = r.unplayed_anims.iter().map(|(i, _)| *i).collect();
        // Dead code (#342), the hostage's, and the dives.
        for id in [316, 340, 99, 100, 317, 318] {
            assert!(unplayed.contains(&id), "{id} counted as played");
        }
        // In the file and played: the run, getting on a ladder, drawing a pistol.
        let chunk = uc_formats::all::AnimChunk::parse(&std::fs::read(uc_formats::resolve(&game, "data\\darci1.all").unwrap()).unwrap()).unwrap();
        for id in [198, 13, 296] {
            assert!(chunk.anims[id].is_some(), "{id} isn't in darci1.all");
            assert!(!unplayed.contains(&id), "{id} counted as unplayed");
        }
        // A map object, a car body and a special.
        for used in ["nprim141.prm", "nprim108.prm", "nprim232.prm"] {
            assert_eq!(why(used), None, "{used} counted as unused");
        }
        for used in ["Textures\\photo1.TGA", "Textures\\shot9.TGA", "Meshes\\heart.SEX", "data\\font3d\\All\\0.SEX",
                     "server\\textures\\extras\\olyfont2.tga", "data\\e3load.tga"] {
            assert!(!r.unreferenced.iter().any(|p| p == used), "{used} counted as unused");
        }
    }
}
