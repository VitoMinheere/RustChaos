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
//!   a file, `Font3D::Font3D`), where `data\` is only the start of the names it builds. A `.sex` is the text version of the `.imp` beside it, and is counted with it.

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
    Ok(r)
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

    /// Against the install: what #467 and #470 found by hand, and the outro's textures
    /// counted as used through its meshes (#468).
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
        for used in ["Textures\\photo1.TGA", "Textures\\shot9.TGA", "Meshes\\heart.SEX", "data\\font3d\\All\\0.SEX",
                     "server\\textures\\extras\\olyfont2.tga", "data\\e3load.tga"] {
            assert!(!r.unreferenced.iter().any(|p| p == used), "{used} counted as unused");
        }
    }
}
