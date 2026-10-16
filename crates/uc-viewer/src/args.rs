//! The command line.

use crate::*;

pub(crate) const USAGE: &str = "\
uc-viewer [--game DIR] [--mission NAME] [--windowed] [--explore] [--overrides DIR] [--linear] [--fullbright]
          [--fog START,END | --no-fog] [--no-character] [--cam X,Y,Z,YAW,PITCH]
          [--screenshot FILE [--size WxH] [--sim-walk SECS | --sim-run SECS | --sim SECS]]
          [--dump-pages DIR]

  --game DIR        Urban Chaos install (default: $UC_GAME_DIR, then the GOG location)
  --mission NAME    mission to play, e.g. ftutor1, testdrive1a, police1 (default: the menu)
  --explore         don't run the mission script: just Darci and the level
  --mute            no sound or music
  --windowed        in a window (default: full screen; F11 or Alt+Enter switches)
  --weapon W        start holding a weapon (knife, bat, pistol, shotgun, ak)
  --overrides DIR   folder of page_NNNN.png files that replace clump textures
  --dump-pages DIR  write every texture this level uses as DIR/page_NNNN.png and exit
  --linear          smooth texture magnification (default: crisp nearest-neighbour)
  --fullbright      ignore the level's baked lighting
  --fog START,END   distance fog in map squares (default 20,40); --no-fog disables it
  --no-character    no people; start in the free camera
  --cam ...         start in the free camera: position in map squares, yaw/pitch in degrees
  --screenshot FILE render one frame offscreen to FILE (PNG) and exit, without a window
  --size WxH        screenshot size (default 1280x720)
  --sim SECS        before the screenshot, run the game for SECS seconds (skipping fly-bys),
                    punching twice and kicking twice first (testing)
  --sim-walk SECS   the same, walking Darci forward; --sim-run SECS running

Darci: WASD move (relative to the camera), Shift run, J or left mouse punch, K kick, Q block,
  E action (arrest, climb; hold while running to sprint), mouse turns the camera, wheel zoom.
  C or Left Ctrl crouch (hold; move to crawl). Punch someone with their back to her, close by,
  to knock them out cold from behind (crouching or crawling up is quietest).
  G next weapon. With a gun: punch fires (hold for the AK-47) at the bracketed target,
  T targets the next enemy. Walk over weapons and ammo to pick them up. With the sniper rifle,
  Z looks through its scope: WASD turn it, punch fires, Z again to put it down.
  Kick while running: sliding tackle. Punch while walking into someone: grab, then punch
  or kick. Back/side + kick in a fight: back or side kick. Three punches or kicks in time: combo.
Gamepad: left stick move (push fully to run), right stick camera, A/Cross jump,
  X/Square punch, Y/Triangle kick, B/Circle action (hold while running to sprint), LB block,
  RB next weapon, right stick click next target, D-pad down crouch,
  Start pause, Back/Select free camera.
Mouse: M switches between mouse look (default: the cursor is captured) and right-drag look.
Hold Tab for the mission's checklist: objectives, side objectives and badges.
Free camera (V): mouse look, WASD move, Space/E up, C/Q down, Shift fast,
  Ctrl slow.
Both: R restart, 1-6 toggle floor/walls/props/roofs/details/people, H hide help,
  F12 screenshot, Esc pause (resume, restart or abandon).";

#[derive(Clone)]
pub(crate) struct Args {
    pub(crate) game: PathBuf,
    pub(crate) mission: String,
    /// `--mission` was given: go straight in (else start at the menu).
    pub(crate) mission_given: bool,
    pub(crate) explore: bool,
    pub(crate) mute: bool,
    /// In a window rather than full screen.
    pub(crate) windowed: bool,
    pub(crate) weapon: uc_game::Weapon,
    pub(crate) overrides: Option<PathBuf>,
    pub(crate) linear: bool,
    pub(crate) fullbright: bool,
    pub(crate) fog: Option<(f32, f32)>,
    pub(crate) no_character: bool,
    pub(crate) cam: Option<[f32; 5]>,
    pub(crate) screenshot: Option<PathBuf>,
    pub(crate) size: (u32, u32),
    pub(crate) dump_pages: Option<PathBuf>,
    /// Seconds to simulate before a screenshot, and how Darci moves meanwhile (None, walk, run).
    pub(crate) sim: Option<(f32, Option<bool>)>,
}

pub(crate) fn parse_floats(s: &str, n: usize, what: &str) -> Result<Vec<f32>, String> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse::<f32>()).collect::<Result<_, _>>().map_err(|e| format!("{what}: {e}"))?;
    if v.len() != n {
        return Err(format!("{what} needs {n} comma-separated numbers"));
    }
    Ok(v)
}

pub(crate) fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        game: uc_formats::default_install().unwrap_or_default(),
        mission: "ftutor1".into(),
        mission_given: false,
        explore: false,
        mute: false,
        windowed: false,
        weapon: uc_game::Weapon::None,
        overrides: None,
        linear: false,
        fullbright: false,
        fog: Some((20.0, 40.0)),
        no_character: false,
        cam: None,
        screenshot: None,
        size: (1280, 720),
        dump_pages: None,
        sim: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--game" => a.game = PathBuf::from(value()?),
            "--mission" => {
                a.mission = value()?.trim_end_matches(".ucm").to_string();
                a.mission_given = true;
            }
            "--explore" => a.explore = true,
            "--mute" => a.mute = true,
            "--windowed" => a.windowed = true,
            "--weapon" => {
                a.weapon = match value()?.as_str() {
                    "knife" => uc_game::Weapon::Knife,
                    "bat" => uc_game::Weapon::Bat,
                    "pistol" => uc_game::Weapon::Pistol,
                    "shotgun" => uc_game::Weapon::Shotgun,
                    "sniper" => uc_game::Weapon::Sniper,
                    "ak" | "ak47" => uc_game::Weapon::Ak47,
                    w => return Err(format!("--weapon: unknown weapon {w} (knife, bat, pistol, shotgun, ak)")),
                }
            }
            "--overrides" => a.overrides = Some(PathBuf::from(value()?)),
            "--linear" => a.linear = true,
            "--fullbright" => a.fullbright = true,
            "--no-fog" => a.fog = None,
            "--fog" => {
                let v = parse_floats(&value()?, 2, "--fog")?;
                a.fog = Some((v[0], v[1]));
            }
            "--no-character" => a.no_character = true,
            "--screenshot" => a.screenshot = Some(PathBuf::from(value()?)),
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size needs WxH")?;
                a.size = (w.parse().map_err(|_| "--size: bad width")?, h.parse().map_err(|_| "--size: bad height")?);
            }
            "--dump-pages" => a.dump_pages = Some(PathBuf::from(value()?)),
            "--sim" | "--sim-walk" | "--sim-run" => {
                let secs = value()?.parse::<f32>().map_err(|e| format!("{arg}: {e}"))?;
                a.sim = Some((secs, match arg.as_str() { "--sim-walk" => Some(false), "--sim-run" => Some(true), _ => None }));
            }
            "--cam" => {
                let v = parse_floats(&value()?, 5, "--cam")?;
                a.cam = Some([v[0], v[1], v[2], v[3], v[4]]);
            }
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    if !a.game.exists() {
        return Err(format!("Urban Chaos install not found; pass --game DIR or set UC_GAME_DIR\n\n{USAGE}"));
    }
    Ok(a)
}
