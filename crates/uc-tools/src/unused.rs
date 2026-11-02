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
}

/// The spoken lines' recordings a mission plays, as file names in `talk2\`.
fn spoken(name: &str, m: &Mission) -> Vec<String> {
    let texts = m.point_texts();
    let mut out = Vec::new();
    for e in m.event_points.iter().filter(|e| e.used != 0) {
        let Some(text) = texts.get(&e.index) else { continue };
        let n = if e.data[9] > 0 { e.data[9] as usize } else { e.index };
        match e.kind {
            waypoint::MESSAGE => out.push(format!("{name}.ucm{n}.wav")),
            waypoint::CONVERSATION => {
                let lines = text.split('|').filter(|l| !l.trim().is_empty()).count();
                out.extend((0..lines.min(26)).map(|i| format!("{name}.ucm{n}{}.wav", (b'A' + i as u8) as char)));
            }
            _ => {}
        }
    }
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
    let text: String = uc_formats::resolve(game, "data\\urban.sty").and_then(|p| std::fs::read(p).ok())
        .map(|b| b.iter().map(|&c| c as char).collect()).unwrap_or_default();
    text.lines().filter(|l| !l.trim_start().starts_with("//"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split(" : ").map(str::trim).collect();
            f.get(7)?.to_ascii_lowercase().strip_suffix(".ucm").map(str::to_string)
        })
        .collect()
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
    /// as used through its meshes (#468), the prims and the sounds.
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
        assert_eq!(r.unplayed.values().map(Vec::len).sum::<usize>(), 66);
        assert_eq!(r.unplayed["mission2"].len(), 13);
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
