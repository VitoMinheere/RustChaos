//! Urban Chaos in Rust: plays a mission (winit + wgpu).
//!
//! Loads a mission's level from your own install and runs its script: Darci, enemies,
//! messages, navigation beacons, fighting and arrests (see the uc-game crate). With
//! `--explore`, Darci just walks around the level. V switches to a free-flying camera.

mod checklist;
mod menu;
mod progress;
mod sound;

use gilrs::{Axis, Button, Gilrs};
use glam::{vec2, vec3, Vec2, Vec3};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use uc_audio::{Audio, Listener};
use uc_game::{Controls, Game, Mood, Outcome};
use uc_render::{wgpu, Camera, Overlay, Renderer, Settings};
use uc_world::{Batch, Layer, Level, LoadOptions};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

const USAGE: &str = "\
uc-viewer [--game DIR] [--mission NAME] [--explore] [--overrides DIR] [--linear] [--fullbright]
          [--fog START,END | --no-fog] [--no-character] [--cam X,Y,Z,YAW,PITCH]
          [--screenshot FILE [--size WxH] [--sim-walk SECS | --sim-run SECS | --sim SECS]]
          [--dump-pages DIR]

  --game DIR        Urban Chaos install (default: $UC_GAME_DIR, then the GOG location)
  --mission NAME    mission to play, e.g. ftutor1, testdrive1a, police1 (default: the menu)
  --explore         don't run the mission script: just Darci and the level
  --mute            no sound or music
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
  G next weapon. With a gun: punch fires (hold for the AK-47) at the bracketed target,
  T targets the next enemy. Walk over weapons and ammo to pick them up.
  Kick while running: sliding tackle. Punch while walking into someone: grab, then punch
  or kick. Back/side + kick in a fight: back or side kick. Three punches or kicks in time: combo.
Gamepad: left stick move (push fully to run), right stick camera, A/Cross jump,
  X/Square punch, Y/Triangle kick, B/Circle action (hold while running to sprint), LB block,
  RB next weapon, right stick click next target,
  Start restart, Back/Select free camera.
Mouse: M switches between mouse look (default: the cursor is captured) and right-drag look.
Hold Tab for the mission's checklist: objectives, side objectives and badges.
Free camera (V): mouse look, WASD move, Space/E up, C/Q down, Shift fast,
  Ctrl slow.
Both: R restart, 1-6 toggle floor/walls/props/roofs/details/people, H hide help,
  F12 screenshot, Esc quit.";

#[derive(Clone)]
struct Args {
    game: PathBuf,
    mission: String,
    /// `--mission` was given: go straight in (else start at the menu).
    mission_given: bool,
    explore: bool,
    mute: bool,
    weapon: uc_game::Weapon,
    overrides: Option<PathBuf>,
    linear: bool,
    fullbright: bool,
    fog: Option<(f32, f32)>,
    no_character: bool,
    cam: Option<[f32; 5]>,
    screenshot: Option<PathBuf>,
    size: (u32, u32),
    dump_pages: Option<PathBuf>,
    /// Seconds to simulate before a screenshot, and how Darci moves meanwhile (None, walk, run).
    sim: Option<(f32, Option<bool>)>,
}

fn parse_floats(s: &str, n: usize, what: &str) -> Result<Vec<f32>, String> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse::<f32>()).collect::<Result<_, _>>().map_err(|e| format!("{what}: {e}"))?;
    if v.len() != n {
        return Err(format!("{what} needs {n} comma-separated numbers"));
    }
    Ok(v)
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        game: uc_formats::default_install().unwrap_or_default(),
        mission: "ftutor1".into(),
        mission_given: false,
        explore: false,
        mute: false,
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
            "--weapon" => {
                a.weapon = match value()?.as_str() {
                    "knife" => uc_game::Weapon::Knife,
                    "bat" => uc_game::Weapon::Bat,
                    "pistol" => uc_game::Weapon::Pistol,
                    "shotgun" => uc_game::Weapon::Shotgun,
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

/// Free camera in square units; yaw 0 looks along +z.
#[derive(Clone, Copy)]
struct FlyCam {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
}

impl FlyCam {
    fn from(s: [f32; 5]) -> Self {
        FlyCam { pos: vec3(s[0], s[1], s[2]), yaw: s[3].to_radians(), pitch: s[4].to_radians() }
    }
    fn forward(&self) -> Vec3 {
        vec3(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos())
    }
    fn camera(&self) -> Camera {
        Camera { pos: self.pos, target: self.pos + self.forward(), fov_y: 60f32.to_radians(), ortho: None }
    }
}

/// Third-person camera orbiting Darci.
#[derive(Clone, Copy)]
struct Orbit {
    yaw: f32,
    pitch: f32,
    dist: f32,
}

impl Orbit {
    /// Behind a character facing (-sin a, -cos a).
    fn behind(angle: f32) -> Self {
        Orbit { yaw: angle + std::f32::consts::PI, pitch: -0.30, dist: 2.4 }
    }
    fn cam(&self, target: Vec3) -> FlyCam {
        let mut c = FlyCam { pos: Vec3::ZERO, yaw: self.yaw, pitch: self.pitch };
        c.pos = target - c.forward() * self.dist;
        c
    }
}

fn save_png(path: &Path, size: (u32, u32), rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgba).map_err(|e| e.to_string())
}

/// A mission starts: Darci with the badges of missions completed so far, and `--weapon`.
/// Sets a new game up for this run: badge stats from earlier missions, the weapon asked for,
/// and how far the camera sees, beyond which the city's extra people can come and go.
fn arm(game: &mut Game, args: &Args) {
    let weapon = args.weapon;
    if let Some((_, end)) = args.fog {
        game.view_dist = game.view_dist.max(end * 256.0);
    }
    game.set_stats(progress::Progress::load().stats());
    if let Some(p) = game.player {
        game.people[p].give(weapon);
        game.people[p].weapon = weapon;
    }
}

/// Everything about a loaded level that doesn't depend on the window.
struct World {
    args: Args,
    level: Level,
    game: Game,
    fly: FlyCam,
    orbit: Orbit,
    third_person: bool,
    start: [f32; 5],
    layers: Vec<Layer>,
    help: bool,
    /// H pressed: show the help for as long as wanted.
    help_forced: bool,
    /// A win, once recorded: (time, best time before it).
    result: Option<(f32, Option<f32>)>,
    /// The mission's badges and side objectives, and those done in earlier wins.
    goals: uc_game::goals::Goals,
    kept: progress::Done,
    /// The mission's title and objectives (from its briefing), for the checklist.
    title: String,
    objectives: Vec<String>,
    /// Tab held: show the checklist.
    checklist: bool,
}

impl World {
    fn load(args: Args) -> Result<World, String> {
        let opt = LoadOptions { fullbright: args.fullbright, character: !args.no_character, overrides: args.overrides.clone() };
        let level = Level::load(&args.game, &args.mission, &opt)?;
        let mut game = Game::new(&level, !args.explore);
        arm(&mut game, &args);
        if !game.unsupported().is_empty() {
            println!("{}: {} event points aren't supported yet and do nothing: {}", level.name,
                     game.unsupported().len(), game.unsupported().join(", "));
        }
        let (sp, angle) = game.player().map_or(level.spawn, |p| (p.fig.pos, p.fig.angle));
        let start = args.cam.unwrap_or([sp.x / 256.0, sp.y / 256.0 + 0.7, sp.z / 256.0,
                                         (angle + std::f32::consts::PI).to_degrees(), -5.0]);
        let third_person = game.player.is_some() && args.cam.is_none();
        let goals = uc_game::goals::goals(&level.mission);
        let kept = progress::Progress::load().get(&level.name).cloned().unwrap_or_default();
        let (title, briefing) = menu::missions(&args.game)
            .into_iter()
            .find(|m| m.0.eq_ignore_ascii_case(&level.name))
            .map_or((level.name.clone(), String::new()), |m| (m.1, m.2));
        let objectives = checklist::split_briefing(&briefing).1;
        Ok(World {
            fly: FlyCam::from(start),
            orbit: Orbit::behind(angle),
            third_person,
            start,
            layers: Layer::ALL.to_vec(),
            game,
            level,
            args,
            help: true,
            help_forced: false,
            result: None,
            goals,
            kept,
            title,
            objectives,
            checklist: false,
        })
    }

    /// Badges and side objectives done, this run or in earlier wins: (badges, sides).
    fn tally(&self) -> (usize, usize) {
        let badges = self.goals.badges.iter().filter(|b| self.kept.badges.contains(b) || self.game.found.contains(b)).count();
        let done = self.game.sides_done(&self.goals);
        let sides = self.goals.sides.iter().filter(|g| self.kept.sides.contains(&g.id) || done.contains(&g.id)).count();
        (badges, sides)
    }

    fn restart(&mut self) {
        self.result = None;
        self.game = Game::new(&self.level, !self.args.explore);
        arm(&mut self.game, &self.args);
        if let Some(p) = self.game.player() {
            self.orbit = Orbit::behind(p.fig.angle);
        }
    }

    fn settings(&self) -> Settings {
        Settings { sky: self.level.sky, fog: self.args.fog, layers: self.layers.clone() }
    }

    fn eye(&self) -> Option<Vec3> {
        if let Some(v) = self.game.player_vehicle() {
            return Some(v.pos / uc_world::UNIT + vec3(0.0, 0.9, 0.0));
        }
        self.game.player().map(|p| p.fig.pos / uc_world::UNIT + vec3(0.0, 0.62, 0.0))
    }

    fn camera(&self) -> Camera {
        // A mission's fly-by takes over the camera.
        if let Some((pos, target)) = self.game.camera() {
            return Camera { pos: pos / uc_world::UNIT, target: target / uc_world::UNIT, fov_y: 60f32.to_radians(), ortho: None };
        }
        match (self.eye(), self.third_person) {
            (Some(eye), true) => {
                // Further back when driving.
                let mut orbit = Orbit { ..self.orbit };
                if self.game.player_vehicle().is_some() {
                    orbit.dist = orbit.dist.max(4.0);
                }
                orbit.cam(eye).camera()
            }
            _ => self.fly.camera(),
        }
    }

    fn dynamic(&self) -> BTreeMap<u32, Batch> {
        self.game.batches(&self.level, self.camera().pos * uc_world::UNIT)
    }

    /// Builds the level and uploads it to the renderer.
    fn upload(&self, r: &mut Renderer) {
        let t = Instant::now();
        let geo = self.level.build();
        let (textures, overridden) = r.upload_world(&geo, &Game::pages(&self.level), |p| self.level.texture(p));
        // The minimap: the whole city from straight above (8 pixels a square), north up.
        let n = 1024;
        let half = uc_formats::iam::MAP_SIZE as f32 / 2.0;
        let top = Camera { pos: vec3(half, 100.0, half), target: vec3(half, 0.0, half), fov_y: 1.0, ortho: Some(half) };
        let layers: Vec<Layer> = Layer::ALL.iter().copied().filter(|&l| l != Layer::Character).collect();
        let shot = |r: &mut Renderer, cam: &Camera| {
            r.render_to_image((n, n), cam, &Settings { sky: self.level.sky, fog: None, layers: layers.clone() }, &BTreeMap::new(), &Overlay::new())
        };
        let mut rgba = shot(r, &top);
        // Warehouse roofs (and some ceilings) are black from above, as on PC: there, show the
        // ground floor instead, seen from just under a storey and a half up (the camera's
        // near plane cuts off everything higher).
        let low = Camera { pos: vec3(half, 1.6, half), target: vec3(half, 0.0, half), ..top };
        let inside = shot(r, &low);
        let sky = self.level.sky;
        for (px, under) in rgba.chunks_mut(4).zip(inside.chunks(4)) {
            let black = px[0] as u32 + px[1] as u32 + px[2] as u32 <= 40;
            if black && under[..3] != sky {
                px.copy_from_slice(under);
            }
        }
        r.set_minimap(&uc_formats::txc::Texture { width: n, height: n, has_alpha: false, rgba });
        // The two sheets of talking heads, from the level's own clump.
        for (sheet, &page) in uc_game::talk::FACE_PAGES.iter().enumerate() {
            if let Some((t, _)) = self.level.texture(page) {
                r.set_faces(sheet, &t);
            }
        }
        println!(
            "{}: {} floor, {} roof, {} wall, {} prop, {} detail triangles in {} chunks; {} textures ({} from overrides); {} lights; built in {:.2}s",
            self.level.name, geo.count(Layer::Floor), geo.count(Layer::Roofs), geo.count(Layer::Walls),
            geo.count(Layer::Props), geo.count(Layer::Detail), geo.chunks.len(), textures, overridden,
            self.level.lighting.as_ref().map_or(0, |l| l.lights.len()), t.elapsed().as_secs_f32()
        );
    }

    /// The HUD: health, mission messages, beacons, the outcome and help.
    fn hud(&self, size: (u32, u32)) -> Overlay {
        let mut o = Overlay::new();
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).round().max(1.0);
        let white = [235, 235, 225, 255];

        if let Some(p) = self.game.player() {
            // Health bar.
            let frac = (p.health.max(0) as f32 / p.max_health as f32).clamp(0.0, 1.0);
            o.text(12.0 * s, 10.0 * s, s, white, &self.game.player_name().to_uppercase());
            let (bx, by, bw, bh) = (12.0 * s, 20.0 * s, 80.0 * s, 5.0 * s);
            o.rect(bx - s, by - s, bw + 2.0 * s, bh + 2.0 * s, [0, 0, 0, 160]);
            let col = if frac > 0.5 { [90, 200, 90, 255] } else if frac > 0.25 { [230, 190, 60, 255] } else { [220, 60, 50, 255] };
            o.rect(bx, by, bw * frac, bh, col);
            // What's in hand, and the rounds left in a gun.
            let weapon = match p.rounds() {
                Some(n) => format!("{} {n}", p.weapon.name()),
                None => p.weapon.name().to_string(),
            };
            let col = if p.rounds() == Some(0) { [240, 90, 70, 255] } else { white };
            let mut wx = bx + bw + 8.0 * s + o.text(bx + bw + 8.0 * s, by - 2.0 * s, s, col, &weapon);
            // What else is being carried, dimmer: press G for it.
            for (w, rounds) in p.stowed() {
                let text = match rounds {
                    Some(n) => format!("  {} {n}", w.name()),
                    None => format!("  {}", w.name()),
                };
                wx += o.text(wx, by - 2.0 * s, s, [170, 170, 160, 200], &text);
            }
            // Badge stats (gold for those raised in this mission; kept when it's won).
            let mut x = bx;
            for (k, name) in ["STA", "STR", "REF", "CON"].iter().enumerate() {
                let text = format!("{name} {}", self.game.stats[k]);
                let col = if self.game.badges[k] > 0 { [255, 210, 70, 255] } else { [190, 190, 180, 200] };
                x += o.text(x, by + bh + 4.0 * s, s, col, &text) + 8.0 * s;
            }
            // How much of the mission is done: badges and side objectives (with earlier wins).
            let (badges, sides) = self.tally();
            let (nb, ns) = (self.goals.badges.len(), self.goals.sides.len());
            if nb + ns > 0 {
                let text = format!("  BADGES {badges}/{nb}   SIDE {sides}/{ns}");
                let all = badges == nb && sides == ns;
                o.text(x, by + bh + 4.0 * s, s, if all { [255, 210, 70, 255] } else { [190, 190, 180, 200] }, &text);
            }
            // The crime rate, for the missions that show it, pulsing red from 95%
            // (`GF_SHOW_CRIMERATE`, DDEngine/Source/panel.cpp:4789).
            if self.game.show_crime_rate {
                let rate = self.game.crime_rate;
                let col = if rate >= 95 {
                    let g = (128.0 + 127.0 * (self.game.time * 9.0).sin()) as u8;
                    [255, g, g, 255]
                } else {
                    white
                };
                o.text(bx, by + bh + 16.0 * s, s, col, &format!("CRIME RATE {rate}%"));
            }
        }

        // Driving: speed and the car's damage.
        if let Some(v) = self.game.player_vehicle() {
            let mph = (v.speed.abs() / 256.0 * 2.0 * 2.237) as u32;
            let text = format!("{mph} MPH");
            let big = s * 2.0;
            o.text(w - Overlay::text_width(&text, big) - 14.0 * s, h - 30.0 * s, big, white, &text);
            let frac = (v.health.max(0) as f32 / 300.0).clamp(0.0, 1.0);
            let (bx, by, bw, bh) = (w - 94.0 * s, h - 38.0 * s, 80.0 * s, 4.0 * s);
            o.rect(bx - s, by - s, bw + 2.0 * s, bh + 2.0 * s, [0, 0, 0, 160]);
            o.rect(bx, by, bw * frac, bh, if frac > 0.3 { [120, 170, 230, 255] } else { [220, 60, 50, 255] });
        }
        // A road sign the mission flashes.
        if let Some((text, _)) = &self.game.sign {
            let big = s * 3.0;
            let tw = Overlay::text_width(text, big);
            o.rect(w / 2.0 - tw / 2.0 - 10.0 * s, h * 0.2 - 6.0 * s, tw + 20.0 * s, 36.0 * s, [200, 30, 30, 220]);
            o.text(w / 2.0 - tw / 2.0, h * 0.2, big, white, text);
        }

        // A search under way: the panel's box in the middle of the screen, with its bar.
        if let Some(search) = &self.game.search {
            let (bw, bh) = (160.0 * s, 32.0 * s);
            let (bx, by) = (w / 2.0 - bw / 2.0, h * 0.6 - bh / 2.0);
            o.rect(bx, by, bw, bh, [0, 0, 0, 170]);
            let text = "SEARCHING";
            o.text(w / 2.0 - Overlay::text_width(text, s) / 2.0, by + 5.0 * s, s, white, text);
            let (fx, fy, fw, fh) = (bx + 10.0 * s, by + 18.0 * s, bw - 20.0 * s, 6.0 * s);
            o.rect(fx - s, fy - s, fw + 2.0 * s, fh + 2.0 * s, [60, 60, 60, 200]);
            o.rect(fx, fy, fw * search.progress(), fh, [230, 190, 60, 255]);
        }

        // A creature's health, under the clock: the one in the sights, or the nearest.
        if let Some((name, frac)) = self.game.creature_health() {
            let (bw, bh) = (160.0 * s, 5.0 * s);
            let (bx, by) = (w / 2.0 - bw / 2.0, 44.0 * s);
            o.text(w / 2.0 - Overlay::text_width(name, s) / 2.0, by - 11.0 * s, s, white, name);
            o.rect(bx - s, by - s, bw + 2.0 * s, bh + 2.0 * s, [0, 0, 0, 160]);
            o.rect(bx, by, bw * frac, bh, [220, 80, 40, 255]);
        }

        // The mission clock.
        if let Some(t) = self.game.countdown() {
            let secs = t.ceil() as u32;
            let text = format!("{}:{:02}", secs / 60, secs % 60);
            let col = if t < 30.0 { [240, 90, 70, 255] } else { white };
            let big = s * 2.0;
            o.text(w / 2.0 - Overlay::text_width(&text, big) / 2.0, 10.0 * s, big, col, &text);
        }

        // Fly-bys: letterbox and a hint.
        if self.game.cutscene.is_some() {
            o.rect(0.0, 0.0, w, h * 0.1, [0, 0, 0, 255]);
            o.rect(0.0, h * 0.9, w, h * 0.1, [0, 0, 0, 255]);
            let hint = "Space to skip";
            o.text(w - Overlay::text_width(hint, s) - 12.0 * s, h * 0.95 - 4.0 * s, s, [200, 200, 190, 200], hint);
        }

        // The minimap (top right): the city from above around Darci, north up, with the
        // mission's beacons as gold dots (pinned to the edge when further off), enemies in a
        // fight as red dots and Darci as a white one; the beacons are listed underneath.
        if self.game.cutscene.is_none() {
            if let Some(pl) = self.game.player() {
                let driving = self.game.player_vehicle();
                let (me, facing) = match driving {
                    Some(v) => (v.pos, v.forward()),
                    None => (pl.fig.pos, pl.facing_dir()),
                };
                let me = vec2(me.x, me.z) / uc_world::UNIT;
                // Squares from the middle to the edge.
                let reach = if driving.is_some() { 24.0 } else { 12.0 };
                let size_px = 100.0 * s;
                let (mx, my) = (w - size_px - 10.0 * s, 10.0 * s);
                let span = uc_formats::iam::MAP_SIZE as f32;
                // A frame (rectangles are drawn over the map image, so only round its edge).
                let (b, frame) = (2.0 * s, [0, 0, 0, 220]);
                o.rect(mx - b, my - b, size_px + 2.0 * b, b, frame);
                o.rect(mx - b, my + size_px, size_px + 2.0 * b, b, frame);
                o.rect(mx - b, my, b, size_px, frame);
                o.rect(mx + size_px, my, b, size_px, frame);
                o.minimap(mx, my, size_px, size_px,
                          [(me.x - reach) / span, (me.y - reach) / span, (me.x + reach) / span, (me.y + reach) / span],
                          [255, 255, 255, 255]);
                // A point (squares) on the map, pinned inside its edge.
                let at = |p: Vec2| {
                    let d = (p - me) / reach;
                    let k = d.abs().max_element().max(1.0);
                    (vec2(mx, my) + (d / k * 0.94 + Vec2::ONE) * 0.5 * size_px, k > 1.0)
                };
                let dot = |o: &mut Overlay, p: Vec2, r: f32, col: [u8; 4]| {
                    o.rect(p.x - r - s, p.y - r - s, 2.0 * (r + s), 2.0 * (r + s), [0, 0, 0, 220]);
                    o.rect(p.x - r, p.y - r, 2.0 * r, 2.0 * r, col);
                };
                for q in self.game.people.iter().filter(|q| q.aggro() && !q.is_dead()) {
                    let (p, outside) = at(vec2(q.fig.pos.x, q.fig.pos.z) / uc_world::UNIT);
                    if !outside {
                        dot(&mut o, p, 1.5 * s, [230, 60, 50, 255]);
                    }
                }
                let gold = [255, 210, 70, 255];
                let mut y = my + size_px + 6.0 * s;
                for (label, pos) in self.game.beacons() {
                    let (p, _) = at(vec2(pos.x, pos.z) / uc_world::UNIT);
                    dot(&mut o, p, 2.0 * s, gold);
                    let dist = (pl.fig.pos - pos).length() / uc_world::UNIT;
                    let text = format!("{} {dist:.0}", label.trim_end_matches('.'));
                    o.text(w - Overlay::text_width(&text, s) - 10.0 * s, y, s, gold, &text);
                    y += 10.0 * s;
                }
                // Darci, with a tick the way she (or her car) faces.
                let (c, _) = at(me);
                for k in 1..=3 {
                    let t = c + facing * (k as f32 * 1.5 * s);
                    o.rect(t.x - s * 0.75, t.y - s * 0.75, 1.5 * s, 1.5 * s, [255, 255, 255, 255]);
                }
                dot(&mut o, c, 2.0 * s, [255, 255, 255, 255]);
            }
        }

        // Mission messages, newest at the bottom, each with the speaker's face beside it
        // (`PANEL_new_text_draw`). A filmed conversation has the widescreen panel instead.
        let ms = s;
        let max_w = (w * 0.7).min(900.0 * s);
        let mut lines = Vec::new();
        let mut faces = Vec::new();
        if self.game.wide.is_none() {
            let shown: Vec<_> = self.game.messages.iter().filter(|m| !m.filmed).collect();
            for m in &shown[shown.len().saturating_sub(2)..] {
                if let Some(f) = m.face {
                    faces.push((lines.len(), f));
                }
                lines.extend(Overlay::wrap(&m.text, max_w, ms));
                lines.push(String::new());
            }
            lines.pop();
        }
        if !lines.is_empty() {
            let lh = 11.0 * ms;
            let total = lines.len() as f32 * lh;
            let y0 = h - total - 24.0 * s;
            let bx = w / 2.0 - max_w / 2.0 - 10.0 * s;
            o.rect(bx, y0 - 6.0 * s, max_w + 20.0 * s, total + 10.0 * s, [0, 0, 0, 150]);
            for (k, line) in lines.iter().enumerate() {
                o.text(w / 2.0 - Overlay::text_width(line, ms) / 2.0, y0 + k as f32 * lh, ms, white, line);
            }
            // The small faces, 32 pixels on a 480-line screen.
            let fs = 32.0 * h / 480.0;
            for (k, f) in faces {
                let (sheet, uv) = uc_game::talk::face_uv(f, false);
                o.face(sheet, bx - fs - 4.0 * s, y0 + k as f32 * lh - 6.0 * s, fs, fs, uv);
            }
        }

        // A filmed conversation: widescreen bars, one speaker's face top right and the other's
        // bottom left, and the line beside whoever is saying it (`PANEL_new_widescreen`).
        if let Some(wd) = &self.game.wide {
            let k = h / 480.0;
            let bar = 80.0 * k;
            o.rect(0.0, 0.0, w, bar, [0, 0, 0, 255]);
            o.rect(0.0, h - bar, w, bar, [0, 0, 0, 255]);
            let size = 64.0 * k;
            if let Some(f) = wd.top {
                let (sheet, uv) = uc_game::talk::face_uv(f, true);
                o.face(sheet, w - 72.0 * k, 8.0 * k, size, size, uv);
            }
            if let Some(f) = wd.bottom {
                let (sheet, uv) = uc_game::talk::face_uv(f, true);
                o.face(sheet, 8.0 * k, h - 72.0 * k, size, size, uv);
            }
            let room = w - 80.0 * k - 80.0 * k;
            let lines = Overlay::wrap(&wd.text, room, s);
            let lh = 11.0 * s;
            let col = [255, 255, 255, 255];
            if wd.top_talking {
                // Right-aligned against the face, its last line at the foot of the top bar.
                let y0 = bar - 5.0 * k - lines.len() as f32 * lh;
                for (n, line) in lines.iter().enumerate() {
                    o.text(w - 80.0 * k - Overlay::text_width(line, s), y0 + n as f32 * lh, s, col, line);
                }
            } else {
                for (n, line) in lines.iter().enumerate() {
                    o.text(80.0 * k, h - bar + 5.0 * k + n as f32 * lh, s, col, line);
                }
            }
        }

        // Outcome.
        if let Some(out) = &self.game.outcome {
            let (title, sub, col) = match out {
                Outcome::Won => {
                    // The time, and how it compares with the best before.
                    let time = match self.result {
                        Some((t, Some(b))) if t < b => format!("Time {}: a new best (was {})   ", progress::clock(t), progress::clock(b)),
                        Some((t, Some(b))) => format!("Time {} (best {})   ", progress::clock(t), progress::clock(b)),
                        Some((t, None)) => format!("Time {}   ", progress::clock(t)),
                        None => String::new(),
                    };
                    let (badges, sides) = self.tally();
                    let found = if self.goals.badges.is_empty() && self.goals.sides.is_empty() {
                        String::new()
                    } else {
                        format!("Badges {badges}/{}   Side objectives {sides}/{}   ", self.goals.badges.len(), self.goals.sides.len())
                    };
                    ("MISSION COMPLETE".to_string(), format!("{time}{found}Enter: next mission   R: play again   Esc: menu"), [120, 230, 120, 255])
                }
                Outcome::Lost(why) => ("MISSION FAILED".to_string(), format!("{why}. Press R to try again"), [240, 90, 70, 255]),
            };
            let big = s * 3.0;
            o.rect(0.0, h * 0.33, w, 50.0 * s, [0, 0, 0, 160]);
            o.text(w / 2.0 - Overlay::text_width(&title, big) / 2.0, h * 0.33 + 8.0 * s, big, col, &title);
            o.text(w / 2.0 - Overlay::text_width(&sub, s) / 2.0, h * 0.33 + 36.0 * s, s, white, &sub);
        }

        // Tab held: the mission's checklist, ticked as things get done (side objectives and
        // badges from earlier wins count too).
        if self.checklist {
            let won = self.game.outcome == Some(Outcome::Won);
            let (badges, _) = self.tally();
            let done = self.game.sides_done(&self.goals);
            let items = checklist::items(&self.objectives, won, &self.goals, badges,
                                         |id| self.kept.sides.contains(&id) || done.contains(&id));
            let pw = (w * 0.5).max(300.0 * s).min(w - 20.0 * s);
            let bw = pw - 24.0 * s;
            let ph = 52.0 * s + checklist::height(&items, bw, s);
            let (px, py) = (w / 2.0 - pw / 2.0, (h * 0.18).min(h - ph - 10.0 * s));
            o.rect(px, py, pw, ph, [0, 0, 0, 200]);
            let gold = [255, 210, 70, 255];
            o.text(px + 12.0 * s, py + 10.0 * s, s * 2.0, gold, &self.title);
            let time = progress::clock(self.game.time);
            o.text(px + pw - Overlay::text_width(&time, s) - 12.0 * s, py + 14.0 * s, s, white, &time);
            checklist::draw(&mut o, px + 12.0 * s, py + 34.0 * s, bw, s, &items);
        }

        // The controls, for the first 20 seconds (H toggles them).
        if self.help && !self.checklist && self.game.cutscene.is_none() && (self.game.time < 20.0 || self.help_forced) {
            let help = ["WASD move   Shift run   Space jump   E action (hold: sprint, or crouch and crawl)", "J / left mouse punch   K kick   Q block   M mouse look on/off",
                        "G next weapon   T next target   Tab checklist   R restart   V free camera   H hide help"];
            for (k, line) in help.iter().enumerate() {
                o.text(12.0 * s, 44.0 * s + k as f32 * 10.0 * s, s, [200, 200, 190, 200], line);
            }
        }
        o
    }
}

/// Gamepad state (the first connected pad), with deadzones applied.
#[derive(Default)]
struct Pad {
    /// Left stick, y up.
    mv: Vec2,
    /// Right stick, y up.
    look: Vec2,
    held: HashSet<Button>,
    pressed: HashSet<Button>,
}

impl Pad {
    fn held(&self, b: Button) -> bool {
        self.held.contains(&b)
    }
    fn hit(&self, b: Button) -> bool {
        self.pressed.contains(&b)
    }
}

fn deadzone(v: Vec2) -> Vec2 {
    let len = v.length();
    if len < 0.2 { Vec2::ZERO } else { v / len * ((len - 0.2) / 0.8).min(1.0) }
}

#[derive(Default)]
struct Input {
    down: HashSet<KeyCode>,
    pressed: HashSet<KeyCode>,
    rmb: bool,
    /// The mouse turns the camera without a button held (the cursor is captured).
    mouse_look: bool,
    lmb_pressed: bool,
    lmb: bool,
    mouse: Vec2,
    wheel: f32,
    pad: Pad,
}

impl Input {
    fn held(&self, k: KeyCode) -> bool {
        self.down.contains(&k)
    }
    fn hit(&self, k: KeyCode) -> bool {
        self.pressed.contains(&k)
    }
}

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

/// Sound and music state.
struct Sound {
    audio: Audio,
    /// The mission's music folder (1-9).
    music_world: u8,
    /// Fights so far (to alternate the two fight themes).
    fights: u32,
    mood: Mood,
    /// The player is in a warehouse, with this ambience.
    indoors: Option<u8>,
}

impl Sound {
    fn new(args: &Args) -> Option<Sound> {
        if args.mute {
            return None;
        }
        let audio = match Audio::new(&args.game) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("{e}; playing without sound");
                return None;
            }
        };
        Some(Sound { audio, music_world: 1, fights: 0, mood: Mood::Calm, indoors: None })
    }

    /// A mission starts: its music world and the city's ambience.
    fn start(&mut self, world: &World) {
        self.music_world = uc_audio::music_world(&world.args.game, &world.level.name, world.level.mission.music_world);
        self.mood = Mood::Calm;
        self.indoors = None;
        self.audio.hush();
        self.audio.music(None);
        self.audio.ambience(Some("f_CTYAMB.wav"), 0.35);
    }

    /// Back to the menu: quiet.
    fn stop(&mut self) {
        self.audio.hush();
        self.audio.music(None);
        self.audio.ambience(None, 0.0);
        self.audio.engine(None);
    }

    /// Plays this frame's game sounds and keeps the music in step with the action.
    fn update(&mut self, world: &mut World, cam: &Camera, dt: f32) {
        let fwd = (cam.target - cam.pos).normalize_or_zero();
        let listener = Listener { pos: cam.pos * uc_world::UNIT, right: fwd.cross(Vec3::Y).normalize_or_zero() };
        for (sfx, pos) in world.game.take_sounds() {
            sound::play(&mut self.audio, sfx, pos, &listener, &world.level.name);
        }
        let mood = world.game.mood();
        let indoors = world.game.indoors();
        let moved = indoors != self.indoors;
        if moved {
            // Inside a warehouse the city's ambience stops (`process_weather`).
            self.indoors = indoors;
            self.audio.ambience(if indoors.is_none() { Some("f_CTYAMB.wav") } else { None }, 0.35);
        }
        if mood != self.mood || moved {
            if mood == Mood::Fight && mood != self.mood {
                self.fights += 1;
            }
            self.mood = mood;
            if !matches!(mood, Mood::Won | Mood::Lost) {
                // A warehouse with an ambience plays it in place of the music.
                let track = match indoors {
                    Some(a @ 1..=4) => Some(uc_game::ware::AMBIENCE[a as usize - 1].to_string()),
                    _ => sound::music(mood, self.music_world, self.fights),
                };
                self.audio.music(track.as_deref());
            }
        }
        // The engine of the car Darci drives.
        let revs = world.game.player_vehicle().map(|v| (v.speed.abs() / 3000.0).min(1.0));
        self.audio.engine(revs);
        self.audio.update(dt);
    }
}

struct App {
    /// The mission being played (None at the menu).
    world: Option<World>,
    /// The menu, while it's showing.
    menu: Option<menu::Menu>,
    /// Options for loading missions.
    args: Args,
    gfx: Option<Gfx>,
    sound: Option<Sound>,
    pads: Option<Gilrs>,
    input: Input,
    last: Instant,
    title_timer: f32,
    frames: u32,
    shots: u32,
}

impl App {
    fn init_gfx(&mut self, el: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            el.create_window(Window::default_attributes().with_title("Urban Chaos").with_inner_size(LogicalSize::new(1280, 720)))
                .map_err(|e| e.to_string())?,
        );
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("no graphics adapter: {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| format!("no graphics device: {e}"))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface not supported by the adapter")?;
        // Prefer a non-sRGB format so lighting multiplies in gamma space, like the original.
        let caps = surface.get_capabilities(&adapter);
        if let Some(f) = caps.formats.iter().find(|f| !f.is_srgb()) {
            config.format = *f;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);
        let mut renderer = Renderer::new(device, queue, config.format, !self.args.linear);
        if let Some(t) = menu::backdrop(&self.args.game) {
            renderer.set_backdrop(&t);
        }
        if let Some(w) = &self.world {
            w.upload(&mut renderer);
        }
        self.gfx = Some(Gfx { window, surface, config, renderer });
        self.input.mouse_look = true;
        self.capture_cursor(self.menu.is_none());
        Ok(())
    }

    /// Captures (hides and locks) the cursor for mouse look, or gives it back.
    fn capture_cursor(&self, on: bool) {
        let Some(g) = &self.gfx else { return };
        if on {
            // Locked keeps the cursor in place; some platforms only support confining it.
            if g.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                let _ = g.window.set_cursor_grab(CursorGrabMode::Confined);
            }
        } else {
            let _ = g.window.set_cursor_grab(CursorGrabMode::None);
        }
        g.window.set_cursor_visible(!on);
    }

    /// Reads the gamepad: button presses since last frame, sticks and held buttons.
    fn poll_pad(&mut self) {
        let Some(gilrs) = self.pads.as_mut() else { return };
        let pad = &mut self.input.pad;
        pad.pressed.clear();
        while let Some(ev) = gilrs.next_event() {
            if let gilrs::EventType::ButtonPressed(b, _) = ev.event {
                pad.pressed.insert(b);
            }
        }
        pad.held.clear();
        (pad.mv, pad.look) = (Vec2::ZERO, Vec2::ZERO);
        if let Some((_, g)) = gilrs.gamepads().next() {
            pad.mv = deadzone(vec2(g.value(Axis::LeftStickX), g.value(Axis::LeftStickY)));
            pad.look = deadzone(vec2(g.value(Axis::RightStickX), g.value(Axis::RightStickY)));
            for b in [Button::South, Button::East, Button::North, Button::West, Button::RightTrigger, Button::RightTrigger2,
                      Button::LeftTrigger, Button::LeftTrigger2] {
                if g.is_pressed(b) {
                    pad.held.insert(b);
                }
            }
            // The d-pad moves too.
            let d = vec2(g.is_pressed(Button::DPadRight) as i32 as f32 - g.is_pressed(Button::DPadLeft) as i32 as f32,
                         g.is_pressed(Button::DPadUp) as i32 as f32 - g.is_pressed(Button::DPadDown) as i32 as f32);
            if d != Vec2::ZERO {
                pad.mv = d.normalize() * 0.6;
            }
        }
    }

    fn update(&mut self, dt: f32) {
        self.poll_pad();
        if self.input.hit(KeyCode::KeyM) {
            self.input.mouse_look = !self.input.mouse_look;
            self.capture_cursor(self.input.mouse_look);
        }
        let inp = &self.input;
        let pad = &inp.pad;
        let Some(w) = self.world.as_mut() else { return };
        let delta = inp.mouse;
        w.checklist = inp.held(KeyCode::Tab);
        if (inp.hit(KeyCode::KeyV) || pad.hit(Button::Select)) && w.game.player.is_some() {
            w.third_person = !w.third_person;
            if let (false, Some(eye)) = (w.third_person, w.eye()) {
                w.fly = w.orbit.cam(eye);
            }
        }
        if inp.hit(KeyCode::KeyH) {
            let shown = w.help && (w.game.time < 20.0 || w.help_forced);
            w.help = !shown;
            w.help_forced = !shown;
        }
        let keys = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6];
        let layers = [Layer::Floor, Layer::Walls, Layer::Props, Layer::Roofs, Layer::Detail, Layer::Character];
        for (k, l) in keys.iter().zip(layers) {
            if inp.hit(*k) {
                if let Some(i) = w.layers.iter().position(|x| *x == l) {
                    w.layers.remove(i);
                } else {
                    w.layers.push(l);
                }
            }
        }
        if inp.hit(KeyCode::KeyR) || pad.hit(Button::Start) {
            w.restart();
            // Starting over shouldn't be talked over by the run before it.
            if let Some(s) = self.sound.as_mut() {
                s.audio.hush();
            }
            if !w.third_person {
                w.fly = FlyCam::from(w.start);
            }
        }
        let mut controls = Controls::default();
        if w.third_person {
            if inp.rmb || inp.mouse_look {
                w.orbit.yaw -= delta.x * 0.005;
                w.orbit.pitch = (w.orbit.pitch - delta.y * 0.005).clamp(-1.3, 0.4);
            }
            w.orbit.yaw -= pad.look.x * 2.5 * dt;
            w.orbit.pitch = (w.orbit.pitch + pad.look.y * 1.5 * dt).clamp(-1.3, 0.4);
            w.orbit.dist = (w.orbit.dist * (1.0 - inp.wheel * 0.1)).clamp(0.8, 8.0);
            // Movement relative to the camera; screen right = forward x up.
            let fwd = vec2(w.orbit.yaw.sin(), w.orbit.yaw.cos());
            let right = vec2(-fwd.y, fwd.x);
            let mut wish = Vec2::ZERO;
            if inp.held(KeyCode::KeyW) { wish += fwd; }
            if inp.held(KeyCode::KeyS) { wish -= fwd; }
            if inp.held(KeyCode::KeyD) { wish += right; }
            if inp.held(KeyCode::KeyA) { wish -= right; }
            // The stick walks when pushed partway and runs when pushed fully.
            let stick = fwd * pad.mv.y + right * pad.mv.x;
            let action_held = inp.held(KeyCode::KeyE) || inp.held(KeyCode::KeyL) || pad.held(Button::East);
            controls = Controls {
                wish: if wish != Vec2::ZERO { wish.normalize() } else { stick },
                run: inp.held(KeyCode::ShiftLeft) || inp.held(KeyCode::ShiftRight) || pad.mv.length() > 0.85,
                sprint: action_held,
                punch: inp.hit(KeyCode::KeyJ) || inp.lmb_pressed || pad.hit(Button::West),
                kick: inp.hit(KeyCode::KeyK) || pad.hit(Button::North),
                action: inp.hit(KeyCode::KeyE) || inp.hit(KeyCode::KeyL) || pad.hit(Button::East),
                jump: inp.hit(KeyCode::Space) || pad.hit(Button::South),
                block: inp.held(KeyCode::KeyQ) || pad.held(Button::LeftTrigger) || pad.held(Button::LeftTrigger2),
                fire: inp.held(KeyCode::KeyJ) || inp.lmb || pad.held(Button::West),
                switch: inp.hit(KeyCode::KeyG) || pad.hit(Button::RightTrigger),
                cycle_target: inp.hit(KeyCode::KeyT) || pad.hit(Button::RightThumb),
                ..Default::default()
            };
            // Driving: W/S or the triggers (or stick) for speed, A/D or the stick to steer,
            // Space or A for the handbrake. The camera swings round behind the car.
            if let Some(car) = w.game.player_vehicle() {
                let key = |k| if inp.held(k) { 1.0 } else { 0.0 };
                let throttle = key(KeyCode::KeyW) - key(KeyCode::KeyS)
                    + if pad.held(Button::RightTrigger2) { 1.0 } else { 0.0 }
                    - if pad.held(Button::LeftTrigger2) { 1.0 } else { 0.0 } + pad.mv.y;
                let steer = key(KeyCode::KeyD) - key(KeyCode::KeyA) + pad.mv.x;
                let handbrake = inp.held(KeyCode::Space) || pad.held(Button::South);
                controls = Controls {
                    action: controls.action,
                    drive: uc_game::Drive { throttle: throttle.clamp(-1.0, 1.0), steer: steer.clamp(-1.0, 1.0), handbrake },
                    ..Default::default()
                };
                if car.speed.abs() > 100.0 && delta.length_squared() < 1.0 && pad.look.length_squared() < 0.01 {
                    let mut d = (car.yaw - w.orbit.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    if car.speed < 0.0 {
                        d = 0.0;
                    }
                    w.orbit.yaw += d * (2.5 * dt).min(1.0);
                }
            }
        } else {
            if inp.rmb || inp.mouse_look {
                w.fly.yaw -= delta.x * 0.004;
                w.fly.pitch = (w.fly.pitch - delta.y * 0.004).clamp(-1.5, 1.5);
            }
            let mut speed = 6.0;
            if inp.held(KeyCode::ShiftLeft) { speed *= 4.0; }
            if inp.held(KeyCode::ControlLeft) { speed *= 0.25; }
            let fwd = w.fly.forward();
            let right = fwd.cross(Vec3::Y).normalize_or_zero();
            let mut mv = Vec3::ZERO;
            if inp.held(KeyCode::KeyW) { mv += fwd; }
            if inp.held(KeyCode::KeyS) { mv -= fwd; }
            if inp.held(KeyCode::KeyD) { mv += right; }
            if inp.held(KeyCode::KeyA) { mv -= right; }
            if inp.held(KeyCode::Space) || inp.held(KeyCode::KeyE) { mv += Vec3::Y; }
            if inp.held(KeyCode::KeyC) || inp.held(KeyCode::KeyQ) { mv -= Vec3::Y; }
            w.fly.yaw -= pad.look.x * 2.0 * dt;
            w.fly.pitch = (w.fly.pitch + pad.look.y * 1.5 * dt).clamp(-1.5, 1.5);
            let pad_mv = fwd * pad.mv.y + right * pad.mv.x;
            w.fly.pos += (mv.normalize_or_zero() + pad_mv) * speed * dt;
        }
        if w.game.cutscene.is_some() && (inp.hit(KeyCode::Space) || inp.hit(KeyCode::Enter) || pad.hit(Button::South)) {
            w.game.skip_cutscene();
            controls.jump = false;
        }
        w.game.update(dt, &controls, &w.level);
    }

    /// Loads mission `name` and starts playing it.
    fn play(&mut self, name: &str) {
        let mut args = self.args.clone();
        args.mission = name.to_string();
        match World::load(args) {
            Ok(w) => {
                if let Some(g) = self.gfx.as_mut() {
                    g.renderer.clear_textures();
                    w.upload(&mut g.renderer);
                }
                if let Some(s) = self.sound.as_mut() {
                    s.start(&w);
                }
                self.world = Some(w);
                self.menu = None;
                self.capture_cursor(self.input.mouse_look);
            }
            Err(e) => eprintln!("{name}: {e}"),
        }
    }

    /// Back to the mission list.
    fn open_menu(&mut self) {
        let mut m = menu::Menu::new(&self.args.game);
        if let Some(w) = &self.world {
            m.select(&w.level.name);
        }
        self.world = None;
        self.menu = Some(m);
        if let Some(s) = self.sound.as_mut() {
            s.stop();
        }
        self.capture_cursor(false);
    }

    /// The menu: choose with the arrows (or stick/d-pad), Enter (or A) plays.
    fn menu_frame(&mut self, el: &ActiveEventLoop) {
        self.poll_pad();
        let inp = &self.input;
        let Some(m) = self.menu.as_mut() else { return };
        if let Some(i) = m.loading {
            let name = m.entries[i].mission.clone();
            self.play(&name);
            self.input.pressed.clear();
            return;
        }
        if inp.hit(KeyCode::Escape) {
            el.exit();
            return;
        }
        if inp.hit(KeyCode::ArrowUp) || inp.hit(KeyCode::KeyW) || inp.pad.hit(Button::DPadUp) {
            m.up();
        }
        if inp.hit(KeyCode::ArrowDown) || inp.hit(KeyCode::KeyS) || inp.pad.hit(Button::DPadDown) {
            m.down();
        }
        let go = inp.hit(KeyCode::Enter) || inp.hit(KeyCode::NumpadEnter) || inp.hit(KeyCode::Space)
            || inp.pad.hit(Button::South) || inp.pad.hit(Button::Start);
        if go && !m.entries.is_empty() {
            m.loading = Some(m.selected);
        }
        self.input.pressed.clear();
        self.input.lmb_pressed = false;
        self.input.mouse = Vec2::ZERO;
        self.input.wheel = 0.0;
        let Some(g) = self.gfx.as_mut() else { return };
        let size = (g.config.width, g.config.height);
        let overlay = m.overlay(size);
        let frame = match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                g.surface.configure(&g.renderer.device, &g.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        g.renderer.render_2d(&view, size, &overlay);
        g.window.pre_present_notify();
        g.renderer.queue.present(frame);
        g.window.set_title("Urban Chaos");
    }

    fn frame(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        if self.menu.is_some() || self.world.is_none() {
            if self.menu.is_none() {
                self.open_menu();
            }
            self.menu_frame(el);
            return;
        }
        // Esc: back to the menu. After a win, Enter goes on to the next mission.
        if self.input.hit(KeyCode::Escape) {
            self.input.pressed.clear();
            self.open_menu();
            return;
        }
        let won = self.world.as_ref().is_some_and(|w| w.game.outcome == Some(Outcome::Won));
        if won && (self.input.hit(KeyCode::Enter) || self.input.pad.hit(Button::Start)) {
            let current = self.world.as_ref().unwrap().level.name.clone();
            let m = menu::Menu::new(&self.args.game);
            if let Some(next) = m.after(&current) {
                let name = m.entries[next].mission.clone();
                self.input.pressed.clear();
                self.play(&name);
                return;
            }
        }
        self.update(dt);
        let Some(world) = self.world.as_mut() else { return };
        // Remember a win (once) for the menu.
        if world.game.outcome == Some(Outcome::Won) && world.result.is_none() {
            let mut p = progress::Progress::load();
            let before = p.best_time(&world.level.name);
            let sides = world.game.sides_done(&world.goals);
            p.record(&world.level.name, world.game.time, &world.game.found, &sides);
            world.kept = p.get(&world.level.name).cloned().unwrap_or_default();
            world.result = Some((world.game.time, before));
        }
        let screenshot = self.input.hit(KeyCode::F12);
        self.input.pressed.clear();
        self.input.lmb_pressed = false;
        self.input.mouse = Vec2::ZERO;
        self.input.wheel = 0.0;

        let cam = world.camera();
        if let Some(s) = self.sound.as_mut() {
            s.update(world, &cam, dt);
        } else {
            world.game.take_sounds();
        }
        let settings = world.settings();
        let dynamic = world.dynamic();
        let Some(g) = self.gfx.as_mut() else { return };
        let size = (g.config.width, g.config.height);
        let hud = world.hud(size);
        let frame = match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                g.surface.configure(&g.renderer.device, &g.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        g.renderer.render(&view, size, &cam, &settings, &dynamic, &hud);
        g.window.pre_present_notify();
        g.renderer.queue.present(frame);

        if screenshot {
            std::fs::create_dir_all("screenshots").ok();
            self.shots += 1;
            let p = PathBuf::from(format!("screenshots/{}_{:03}.png", world.level.name, self.shots));
            let img = g.renderer.render_to_image(size, &cam, &settings, &dynamic, &hud);
            match save_png(&p, size, &img) {
                Ok(()) => println!("saved {}", p.display()),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
        }

        self.frames += 1;
        self.title_timer += dt;
        if self.title_timer >= 0.5 {
            let fps = self.frames as f32 / self.title_timer;
            let st = g.renderer.stats;
            let mode = match (world.game.player(), world.third_person) {
                (Some(p), true) => format!("Darci at {:.1}, {:.1}", p.fig.pos.x / 256.0, p.fig.pos.z / 256.0),
                _ => format!("free camera at {:.1}, {:.1}, {:.1}", cam.pos.x, cam.pos.y, cam.pos.z),
            };
            g.window.set_title(&format!(
                "Urban Chaos: {}  |  {mode}  |  {fps:.0} fps, {}/{} chunks, {} draws",
                world.level.name, st.chunks_drawn, st.chunks_total, st.draw_calls
            ));
            self.frames = 0;
            self.title_timer = 0.0;
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_none() {
            if let Err(e) = self.init_gfx(el) {
                eprintln!("error: {e}");
                el.exit();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = self.gfx.as_mut() {
                    g.config.width = size.width.max(1);
                    g.config.height = size.height.max(1);
                    g.surface.configure(&g.renderer.device, &g.config);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !event.repeat {
                                self.input.pressed.insert(code);
                            }
                            self.input.down.insert(code);
                        }
                        ElementState::Released => {
                            self.input.down.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Right, .. } => {
                self.input.rmb = state == ElementState::Pressed;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.input.lmb = state == ElementState::Pressed;
                self.input.lmb_pressed |= self.input.lmb;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.wheel += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 50.0,
                };
            }
            WindowEvent::Focused(false) => {
                self.input.down.clear();
                self.input.rmb = false;
                self.capture_cursor(false);
            }
            WindowEvent::Focused(true) => self.capture_cursor(self.input.mouse_look),
            WindowEvent::RedrawRequested => self.frame(el),
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.mouse += vec2(delta.0 as f32, delta.1 as f32);
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(g) = &self.gfx {
            g.window.request_redraw();
        }
    }
}

/// `--screenshot`: render one frame offscreen, without opening a window.
fn headless_screenshot(mut world: World, path: &Path) -> Result<(), String> {
    if let Some((secs, mode)) = world.args.sim {
        let dt = 1.0 / 30.0;
        let ahead = vec2(world.orbit.yaw.sin(), world.orbit.yaw.cos());
        let c = Controls { wish: if mode.is_some() { ahead } else { Vec2::ZERO }, run: mode == Some(true), ..Default::default() };
        for i in 0..(secs / dt) as usize {
            // Standing still, try out punch, punch, kick, kick first (the tutorial's first task).
            let tap = mode.is_none() && i % 30 == 10 && i < 130;
            let c = Controls { punch: tap && i < 70, kick: tap && i >= 70, fire: tap, ..c };
            world.game.skip_cutscene();
            world.game.update(dt, &c, &world.level);
        }
        // Look at the gun's target, or else the nearest enemy standing.
        if let Some(p) = world.game.player() {
            let enemy = world.game.target.or_else(|| {
                world.game.people.iter().enumerate()
                    .filter(|(_, e)| e.hostile() && !e.is_dead() && !e.is_down())
                    .min_by(|a, b| (a.1.fig.pos - p.fig.pos).length().total_cmp(&(b.1.fig.pos - p.fig.pos).length()))
                    .map(|(i, _)| i)
            });
            if let Some(e) = enemy {
                let d = world.game.people[e].fig.pos - p.fig.pos;
                world.orbit.yaw = d.x.atan2(d.z);
            }
        }
    }
    let mut r = Renderer::headless(!world.args.linear)?;
    world.upload(&mut r);
    let size = world.args.size;
    let cam = world.camera();
    let hud = world.hud(size);
    let img = r.render_to_image(size, &cam, &world.settings(), &world.dynamic(), &hud);
    save_png(path, size, &img)?;
    println!("saved {} ({}/{} chunks drawn, {} draw calls, {} triangles)", path.display(),
             r.stats.chunks_drawn, r.stats.chunks_total, r.stats.draw_calls, r.stats.triangles);
    Ok(())
}

/// `--dump-pages`: write every texture the level uses as page_NNNN.png.
fn dump_pages(world: &World, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let geo = world.level.build();
    let mut pages = geo.pages();
    pages.extend(Game::pages(&world.level));
    let mut n = 0;
    for page in pages {
        if let Ok(Some(t)) = world.level.clump.texture(page) {
            save_png(&dir.join(format!("page_{page:04}.png")), (t.width, t.height), &t.rgba)?;
            n += 1;
        }
    }
    println!("wrote {n} textures to {}", dir.display());
    Ok(())
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let screenshot = args.screenshot.clone();
    let dump = args.dump_pages.clone();
    // `--screenshot` without `--mission`: the menu.
    if let (Some(path), false, None) = (&screenshot, args.mission_given, &dump) {
        let mut r = Renderer::headless(true)?;
        if let Some(t) = menu::backdrop(&args.game) {
            r.set_backdrop(&t);
        }
        let img = r.render_2d_to_image(args.size, &menu::Menu::new(&args.game).overlay(args.size));
        save_png(path, args.size, &img)?;
        println!("saved {}", path.display());
        return Ok(());
    }
    if dump.is_some() || screenshot.is_some() {
        let world = World::load(args)?;
        if let Some(dir) = dump {
            return dump_pages(&world, &dir);
        }
        return headless_screenshot(world, &screenshot.unwrap());
    }
    let el = EventLoop::new().map_err(|e| e.to_string())?;
    let mut sound = Sound::new(&args);
    let pads = Gilrs::new().map_err(|e| eprintln!("gamepads unavailable: {e}")).ok();
    // Straight into a mission with --mission, else the menu.
    let (world, menu) = if args.mission_given {
        let w = World::load(args.clone())?;
        if let Some(s) = sound.as_mut() {
            s.start(&w);
        }
        (Some(w), None)
    } else {
        (None, Some(menu::Menu::new(&args.game)))
    };
    let mut app = App {
        world, menu, args, gfx: None, sound, pads, input: Input::default(), last: Instant::now(), title_timer: 0.0,
        frames: 0, shots: 0,
    };
    el.run_app(&mut app).map_err(|e| e.to_string())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
