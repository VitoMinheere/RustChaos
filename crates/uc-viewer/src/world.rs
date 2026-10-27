//! A level being played or looked at: loading it, its camera and what it uploads to the renderer.

use crate::*;
use uc_net::replay::{Mode, Recorder, Replay, Start};

/// How a new game for `args` starts: the mission, or an endless run on its map (with
/// `--seed`'s seed, or one from the clock); with the badge stats from earlier missions, the
/// difficulty, the weapon asked for, how far the camera sees (beyond which the city's extra
/// people can come and go), and any dogs asked for.
fn start(args: &Args) -> Start {
    let mode = if args.endless || args.beat {
        let seed = args.seed.unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32)
        });
        println!("endless: {} with seed {seed}", args.mission);
        let (centre, radius) = args.arena;
        let centre = centre.map(|(x, z)| [x * 256.0, z * 256.0]);
        if args.beat { Mode::Beat { seed, centre, shift: args.shift } } else { Mode::Endless { seed, centre, radius } }
    } else if args.explore {
        Mode::Explore
    } else {
        Mode::Mission
    };
    let p = progress::Progress::load();
    Start {
        weapon: args.weapon,
        dogs: args.dogs as u32,
        view_dist: args.fog.map(|(_, end)| end * 256.0),
        ..Start::new(&args.mission, mode, p.difficulty(), p.stats())
    }
}

/// A new game for `args`, and how it started.
fn new_game(level: &Level, args: &Args) -> (Game, Start) {
    let start = start(args);
    let game = start.game(level);
    if args.dogs > 0 {
        println!("dogs: {} of {} let loose", game.dogs.len(), args.dogs);
    }
    (game, start)
}

/// Everything about a loaded level that doesn't depend on the window.
pub(crate) struct World {
    pub(crate) args: Args,
    pub(crate) level: Level,
    pub(crate) game: Game,
    pub(crate) fly: FlyCam,
    pub(crate) orbit: Orbit,
    /// Looking round (F held, #203): the view from close behind her head, while it lasts.
    pub(crate) look: Option<Orbit>,
    pub(crate) third_person: bool,
    pub(crate) start: [f32; 5],
    pub(crate) layers: Vec<Layer>,
    pub(crate) help: bool,
    /// H pressed: show the help for as long as wanted.
    pub(crate) help_forced: bool,
    /// The controls the help shows, as they're bound (#202).
    pub(crate) help_lines: Vec<String>,
    /// A win, once recorded: (time, best time before it).
    pub(crate) result: Option<(f32, Option<f32>)>,
    /// The mission's badges and side objectives, and those done in earlier wins.
    pub(crate) goals: uc_game::goals::Goals,
    pub(crate) kept: progress::Done,
    /// The mission's title and objectives (from its briefing), for the checklist.
    pub(crate) title: String,
    pub(crate) objectives: Vec<String>,
    /// Tab held: show the checklist.
    pub(crate) checklist: bool,
    /// The chief's word at the end of a mission with too many civilians dead.
    pub(crate) civ_warning: Option<String>,
    /// The in-game menu, while it's up.
    pub(crate) pause: Option<Pause>,
    /// The developer console, and the free camera's teleport.
    pub(crate) console: Console,
    /// An endless run that's over and saved: whether it beat the best, and the best before.
    pub(crate) endless_recorded: Option<(bool, Option<(i64, u32)>)>,
    /// The progress as it stood, for the achievements (#132).
    pub(crate) known: progress::Progress,
    /// This game's stats are in the saved totals.
    pub(crate) stats_saved: bool,
    /// Achievements just earned, on screen for the seconds left.
    pub(crate) popups: Vec<(String, f32)>,
    /// Seconds to the next look for achievements earned.
    ach_clock: f32,
    /// The walls with a crinkle, from the last upload.
    pub(crate) relief: Relief,
    /// Crinkles are on (`--no-crinkles` starts with them off; the console's `crinkles`
    /// switches them, as in the original, Controls.cpp:450).
    pub(crate) crinkles: bool,
    /// Co-op (#182): this player's seat. The game is focused on it.
    pub(crate) seat: Option<usize>,
    /// The race against the best run's ghost, on the missions that have one (#186).
    pub(crate) race: Option<ghosts::Race>,
    /// The ghost is shown (B switches it, and it's remembered).
    pub(crate) ghost_on: bool,
    /// This run, kept for a replay (#184) until it's saved, or until the console changes it.
    pub(crate) recorder: Option<Recorder>,
    /// Where this run's replay was saved, and whether it's been kept for good.
    pub(crate) replay_saved: Option<(PathBuf, bool)>,
    /// A replay being watched, not a game being played.
    pub(crate) replay: Option<replays::Viewing>,
    /// The mission editor (#185), while the mission's being edited: the game is stopped.
    pub(crate) editor: Option<editor::Editor>,
    /// An edited mission being played: nothing it does goes into the progress or a replay.
    pub(crate) edited: bool,
}

/// The ghost race for a new game of `level`, if it's one of the missions raced (and played
/// as the mission, not explored or as an endless run).
fn race(game: &mut Game, level: &Level, args: &Args) -> Option<ghosts::Race> {
    (ghosts::raced(&level.name) && !args.explore && !args.endless && !args.beat).then(|| ghosts::Race::start(game, &level.name))
}

impl World {
    pub(crate) fn load(args: Args) -> Result<World, String> {
        World::load_with(args, None, None, None, new_game)
    }

    /// Mission `args.mission` in the editor (#185): `editor` if one's being returned to, else
    /// its saved edit, or the shipped mission. The level's loaded with the edit (when it
    /// makes a mission), but the game isn't run.
    pub(crate) fn load_edit(args: Args, editor: Option<editor::Editor>) -> Result<World, String> {
        let mut ed = match editor {
            Some(e) => Some(e),
            None => editor::saved(&args.mission).map(|d| editor::Editor::with(&args.mission, d)),
        };
        let mission = ed.as_ref().and_then(|e| e.mission().ok());
        let mut w = World::load_with(Args { cam: None, ..args }, None, None, mission, new_game)?;
        let ed = ed.take().unwrap_or_else(|| editor::Editor::new(&w.args.mission, &w.level.mission));
        w.editor = Some(ed);
        w.third_person = false;
        w.recorder = None;
        w.race = None;
        w.help = false;
        Ok(w)
    }

    /// `mission`, an edit of `args.mission`, played.
    pub(crate) fn load_edited(args: Args, mission: uc_formats::ucm::Mission) -> Result<World, String> {
        let args = Args { explore: false, endless: false, beat: false, cam: None, ..args };
        let mut w = World::load_with(args, None, None, Some(mission), new_game)?;
        w.edited = true;
        w.recorder = None;
        w.race = None;
        Ok(w)
    }

    /// Whether this game goes into the progress (best times, stats, achievements): not a
    /// replay, and not an edited mission.
    pub(crate) fn counts(&self) -> bool {
        self.replay.is_none() && !self.edited && self.editor.is_none()
    }

    /// `replay`, to watch.
    pub(crate) fn load_replay(args: Args, replay: Replay) -> Result<World, String> {
        let start = replay.start.clone();
        let args = Args {
            mission: start.mission.clone(), explore: start.mode == Mode::Explore, endless: matches!(start.mode, Mode::Endless { .. }),
            beat: matches!(start.mode, Mode::Beat { .. }), dogs: start.dogs as usize, weapon: start.weapon, cam: None,
            no_character: false, ..args
        };
        World::load_with(args, None, Some(replay), None, |level, _| (start.game(level), start.clone()))
    }

    /// A co-op game's mission, as `setup` has it, played from `seat`. Nothing here may
    /// change the game but `setup`: every player's machine has to start the same one.
    pub(crate) fn load_coop(args: Args, setup: &uc_net::Setup, seat: usize) -> Result<World, String> {
        let args = Args { mission: setup.mission.clone(), explore: false, endless: false, beat: false, dogs: 0, weapon: uc_game::Weapon::None, cam: None, ..args };
        World::load_with(args, Some(seat), None, None, |level, _| {
            let mut game = setup.game(level);
            game.focus(seat);
            (game, uc_net::replay::Start::coop(setup))
        })
    }

    /// Loads the level for `args` (with `mission` in place of the install's, if given) and
    /// starts the game `make` makes on it.
    fn load_with(args: Args, seat: Option<usize>, replay: Option<Replay>, mission: Option<uc_formats::ucm::Mission>,
                 make: impl FnOnce(&Level, &Args) -> (Game, Start)) -> Result<World, String> {
        let opt = LoadOptions { fullbright: args.fullbright, character: !args.no_character, overrides: args.overrides.clone() };
        let level = match mission {
            Some(m) => Level::load_with_mission(&args.game, &args.mission, m, &opt)?,
            None => Level::load(&args.game, &args.mission, &opt)?,
        };
        let (mut game, how) = make(&level, &args);
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
        let (story, objectives) = checklist::split_briefing(&briefing);
        // The missions the game never released have little said at the start: the radio
        // reads out their briefing (just the objectives where the script opens by itself).
        if menu::unreleased(&level.name) && !objectives.is_empty() {
            game.brief((!story.is_empty()).then_some(story), format!("Objectives: {}", objectives.join(" ")));
        }
        let crinkles = args.crinkles;
        // A co-op run isn't a solo one to race or keep as the best.
        let race = if seat.is_none() && replay.is_none() { race(&mut game, &level, &args) } else { None };
        // Kept from turn 0 (not without the people: that's for looking at the level).
        let recorder = (replay.is_none() && !args.no_character).then(|| Recorder::new(&mut game, how));
        let replay = replay.map(replays::Viewing::new);
        Ok(World {
            fly: FlyCam::from(start),
            orbit: Orbit::behind(angle),
            look: None,
            third_person,
            start,
            layers: Layer::ALL.to_vec(),
            game,
            level,
            args,
            help: true,
            help_forced: false,
            help_lines: Vec::new(),
            result: None,
            goals,
            kept,
            title,
            objectives,
            checklist: false,
            pause: None,
            civ_warning: None,
            console: Console::default(),
            endless_recorded: None,
            known: progress::Progress::load(),
            stats_saved: false,
            popups: Vec::new(),
            ach_clock: 1.0,
            relief: Relief::default(),
            crinkles,
            seat,
            race,
            ghost_on: progress::Progress::load().ghosts(),
            recorder,
            replay_saved: None,
            replay,
            editor: None,
            edited: false,
        })
    }

    /// Badges and side objectives done, this run or in earlier wins: (badges, sides).
    pub(crate) fn tally(&self) -> (usize, usize) {
        let badges = self.goals.badges.iter().filter(|b| self.kept.badges.contains(b) || self.game.found.contains(b)).count();
        let done = self.game.sides_done(&self.goals);
        let sides = self.goals.sides.iter().filter(|g| self.kept.sides.contains(&g.id) || done.contains(&g.id)).count();
        (badges, sides)
    }

    /// Adds this game's stats to the saved totals, once: when it's over, or left.
    pub(crate) fn save_stats(&mut self) {
        if self.stats_saved || !self.counts() {
            return;
        }
        self.stats_saved = true;
        progress::Progress::load().add_stats(&self.game.record);
        self.known = progress::Progress::load();
    }

    /// Once a second: achievements earned (by the totals with this game's added), popped up
    /// and saved.
    pub(crate) fn check_achievements(&mut self, dt: f32) {
        for p in &mut self.popups {
            p.1 -= dt;
        }
        self.popups.retain(|p| p.1 > 0.0);
        self.ach_clock -= dt;
        if self.ach_clock > 0.0 {
            return;
        }
        self.ach_clock = 1.0;
        if !self.counts() {
            return;
        }
        let totals = if self.stats_saved { self.known.totals.clone() } else { self.known.totals.plus(&self.game.record) };
        for a in achievements::newly(&self.known, &totals) {
            self.known.achieved.insert(a.id.to_string());
            progress::Progress::load().achieve(a.id);
            self.popups.push((format!("ACHIEVEMENT: {}  ({})", a.name.to_uppercase(), a.text), 6.0));
        }
    }

    /// Starts over (a replay: from the beginning).
    pub(crate) fn restart(&mut self) {
        if let Some(v) = self.replay.as_mut() {
            self.game = v.playback.replay.start.game(&self.level);
            v.playback.rewind();
        } else {
            self.restart_game();
        }
        if let Some(p) = self.game.player() {
            self.orbit = Orbit::behind(p.fig.angle);
        }
    }

    fn restart_game(&mut self) {
        self.save_replay();
        self.save_stats();
        self.stats_saved = false;
        self.result = None;
        self.civ_warning = None;
        let start;
        (self.game, start) = new_game(&self.level, &self.args);
        let counts = self.counts();
        self.recorder = (counts && !self.args.no_character).then(|| Recorder::new(&mut self.game, start));
        self.replay_saved = None;
        self.race = if counts { race(&mut self.game, &self.level, &self.args) } else { None };
        self.endless_recorded = None;
    }

    pub(crate) fn settings(&self) -> Settings {
        // No floor (`GF_NO_FLOOR`): the street under Gunston Tower isn't drawn
        // (`AENG_draw`, aeng.cpp:9287); the roofs still are.
        let layers: Vec<Layer> = self.layers.iter().copied().filter(|&l| !(self.game.no_floor && l == Layer::Floor)).collect();
        // The raised walls come with the dynamic batches, which only the character layer draws.
        let crinkles = self.crinkles && layers.contains(&Layer::Character);
        // The game's dynamic lights, in square units.
        let lights = self.game.lights().into_iter()
            .map(|l| uc_render::Light { pos: l.pos / uc_world::UNIT, radius: l.radius / uc_world::UNIT, rgb: l.rgb })
            .collect();
        let shadows = self.game.shadow_maps();
        Settings { sky: self.level.sky, fog: self.args.fog, layers, crinkles, lights, shadows }
    }

    pub(crate) fn eye(&self) -> Option<Vec3> {
        if let Some(v) = self.game.player_vehicle() {
            return Some(v.pos / uc_world::UNIT + vec3(0.0, 0.9, 0.0));
        }
        // Against a wall looking round its end, the camera leads round the corner.
        self.game.player().map(|p| {
            let peek = p.peek().unwrap_or_default();
            (p.fig.pos + vec3(peek.x, 0.0, peek.y)) / uc_world::UNIT + vec3(0.0, 0.62, 0.0)
        })
    }

    pub(crate) fn camera(&self) -> Camera {
        // The editor's is always the free camera.
        if self.editor.is_some() {
            return self.fly.camera();
        }
        // A mission's fly-by takes over the camera.
        if let Some((pos, target)) = self.game.camera() {
            return Camera { pos: pos / uc_world::UNIT, target: target / uc_world::UNIT, fov_y: 60f32.to_radians(), ortho: None };
        }
        // Through the scope: from her head, zoomed in as far as the lens has come.
        if let Some(s) = self.game.sniping() {
            let (dir, zoom) = s.view();
            let eye = s.eye / uc_world::UNIT;
            let fov_y = 2.0 * ((30f32.to_radians()).tan() / zoom).atan();
            return Camera { pos: eye, target: eye + dir, fov_y, ortho: None };
        }
        // Looking round: from close behind her head.
        if let (Some(look), Some(p), true) = (self.look, self.game.player(), self.third_person) {
            let head = p.fig.pos / uc_world::UNIT + vec3(0.0, cam::LOOK_ABOVE, 0.0);
            let mut cam = look.cam(head);
            cam.pos = self.clear_of_buildings(head, cam.pos);
            return cam.camera();
        }
        match (self.eye(), self.third_person) {
            (Some(eye), true) => {
                // Further back when driving, and closer in with a gun out (200 of 256,
                // `FC_alter_for_pos`, fc.cpp:65).
                let mut orbit = Orbit { ..self.orbit };
                if self.game.player_vehicle().is_some() {
                    orbit.dist = orbit.dist.max(4.0);
                } else if self.game.player().is_some_and(|p| p.gun_out()) {
                    orbit.dist *= 200.0 / 256.0;
                }
                let mut cam = orbit.cam(eye);
                cam.pos = self.clear_of_buildings(eye, cam.pos) + self.shaken();
                // With no floor the camera never goes below 0x280 (fc.cpp:1650).
                if self.game.no_floor {
                    cam.pos.y = cam.pos.y.max(0x280 as f32 / uc_world::UNIT);
                }
                cam.camera()
            }
            _ => self.fly.camera(),
        }
    }

    /// Where a blast nearby throws the camera this frame (`fc->shake`, fc.cpp:1913): up to
    /// half the shake either way on each axis, in world units, at random. Only where it is
    /// moves, not what it looks at, so the view jolts round.
    fn shaken(&self) -> Vec3 {
        let s = self.game.shake;
        if s < 1.0 {
            return Vec3::ZERO;
        }
        let seed = (self.game.time * 1000.0) as u32;
        let r = |k: u32| {
            let mut x = seed.wrapping_mul(0x9e37_79b1) ^ k.wrapping_mul(0x85eb_ca6b);
            x ^= x >> 15;
            x = x.wrapping_mul(0x2c1b_3c6d);
            x ^= x >> 12;
            (x % s as u32) as f32 - s / 2.0
        };
        vec3(r(1), r(2), r(3)) / 2.0 / uc_world::UNIT
    }

    /// Brings the follow camera in front of a building that stands between it and `eye`, so
    /// she stays in view: the line from her to where the camera wants to be is tested a step at
    /// a time against the city's heights (`MAV_height_los_slow`, mav.cpp:2470), and the camera
    /// goes where the line first runs into something, as `FC_force_camera_behind` does when it
    /// finds no clear place (fc.cpp:819). Under a roof (in a warehouse) it's the other way
    /// round: the camera stays under it. The original first tries swinging the camera round a
    /// little; here the player steers the orbit, so it only comes in.
    fn clear_of_buildings(&self, eye: Vec3, want: Vec3) -> Vec3 {
        let (map, ground, unit) = (&self.level.map, &self.level.ground, uc_world::UNIT);
        let under = |p: Vec3| ground.map_height(map, p.x * unit, p.z * unit) > p.y * unit;
        let indoors = under(eye);
        let (d, len) = (want - eye, (want - eye).length());
        if len < 1e-3 {
            return want;
        }
        // A sample every eighth of a square, and the camera stops a quarter of a square short.
        let n = (len * 8.0).ceil() as usize;
        match (1..=n).map(|i| i as f32 / n as f32).find(|&t| under(eye + d * t) != indoors) {
            Some(t) => eye + d * ((t * len - 0.25).max(0.1) / len),
            None => want,
        }
    }

    pub(crate) fn dynamic(&self) -> BTreeMap<u32, Batch> {
        let cam = self.camera();
        let mut out = self.game.batches(&self.level, cam.pos * uc_world::UNIT);
        if self.args.rain {
            self.game.rain(&self.level, cam.pos * uc_world::UNIT, cam.target - cam.pos, &mut out);
        }
        if self.crinkles && self.layers.contains(&Layer::Walls) {
            self.relief.draw(cam.pos, &mut out);
        }
        if let (true, Some(best)) = (self.ghost_on, self.race.as_ref().and_then(|r| r.best.as_ref())) {
            self.game.ghost_batches(best, &self.level, &mut out);
        }
        out
    }

    /// Builds the level and uploads it to the renderer.
    pub(crate) fn upload(&mut self, r: &mut Renderer) {
        let t = Instant::now();
        let geo = self.level.build();
        self.relief = geo.relief();
        let (textures, overridden) = r.upload_world(&geo, &Game::pages(&self.level), |p| self.level.texture(p));
        // The minimap: the whole city from straight above (8 pixels a square), north up.
        let n = 1024;
        let half = uc_formats::iam::MAP_SIZE as f32 / 2.0;
        let top = Camera { pos: vec3(half, 100.0, half), target: vec3(half, 0.0, half), fov_y: 1.0, ortho: Some(half) };
        // Of what the game draws, only the sea's floor (the dynamic batches come with the
        // character layer).
        let layers: Vec<Layer> = Layer::ALL.to_vec();
        let mut sea = BTreeMap::new();
        self.game.draw_sea(&mut sea);
        let shot = |r: &mut Renderer, cam: &Camera| {
            r.render_to_image((n, n), cam, &Settings { sky: self.level.sky, fog: None, layers: layers.clone(), crinkles: false, lights: Vec::new(), shadows: Vec::new() }, &sea, &Overlay::new())
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
            "{}: {} floor, {} roof, {} wall ({} with crinkles, {} kinds), {} prop, {} detail triangles in {} chunks; {} textures ({} from overrides); {} lights; built in {:.2}s",
            self.level.name, geo.count(Layer::Floor), geo.count(Layer::Roofs), geo.count(Layer::Walls),
            geo.crinkled_count(), geo.crinkles.len(),
            geo.count(Layer::Props), geo.count(Layer::Detail), geo.chunks.len(), textures, overridden,
            self.level.lighting.as_ref().map_or(0, |l| l.lights.len()), t.elapsed().as_secs_f32()
        );
    }

}
