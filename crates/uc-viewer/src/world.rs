//! A level being played or looked at: loading it, its camera and what it uploads to the renderer.

use crate::*;

/// A mission starts: Darci with the badges of missions completed so far, and `--weapon`.
/// Sets a new game up for this run: badge stats from earlier missions, the weapon asked for,
/// how far the camera sees, beyond which the city's extra people can come and go, and any
/// dogs asked for.
pub(crate) fn arm(game: &mut Game, level: &Level, args: &Args) {
    let weapon = args.weapon;
    if let Some((_, end)) = args.fog {
        game.view_dist = game.view_dist.max(end * 256.0);
    }
    let progress = progress::Progress::load();
    game.set_stats(progress.stats());
    game.set_difficulty(progress.difficulty());
    if let Some(p) = game.player {
        game.people[p].give(weapon);
        game.people[p].weapon = weapon;
    }
    if args.dogs > 0 {
        let n = game.let_dogs_out(level, args.dogs);
        println!("dogs: {n} of {} let loose", args.dogs);
    }
}

/// The game for `args`: the mission, or an endless run on its map (with `--seed`'s seed, or
/// one from the clock).
fn new_game(level: &Level, args: &Args) -> Game {
    if !args.endless && !args.beat {
        return Game::new(level, !args.explore);
    }
    let seed = args.seed.unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32)
    });
    println!("endless: {} with seed {seed}", level.name);
    let (centre, radius) = args.arena;
    let centre = centre.map(|(x, z)| glam::vec2(x * 256.0, z * 256.0));
    if args.beat {
        return Game::new_beat(level, seed, centre, args.shift);
    }
    Game::new_endless(level, seed, centre, radius)
}

/// Everything about a loaded level that doesn't depend on the window.
pub(crate) struct World {
    pub(crate) args: Args,
    pub(crate) level: Level,
    pub(crate) game: Game,
    pub(crate) fly: FlyCam,
    pub(crate) orbit: Orbit,
    pub(crate) third_person: bool,
    pub(crate) start: [f32; 5],
    pub(crate) layers: Vec<Layer>,
    pub(crate) help: bool,
    /// H pressed: show the help for as long as wanted.
    pub(crate) help_forced: bool,
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
}

impl World {
    pub(crate) fn load(args: Args) -> Result<World, String> {
        let opt = LoadOptions { fullbright: args.fullbright, character: !args.no_character, overrides: args.overrides.clone() };
        let level = Level::load(&args.game, &args.mission, &opt)?;
        let mut game = new_game(&level, &args);
        arm(&mut game, &level, &args);
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
            pause: None,
            civ_warning: None,
            console: Console::default(),
            endless_recorded: None,
            known: progress::Progress::load(),
            stats_saved: false,
            popups: Vec::new(),
            ach_clock: 1.0,
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
        if self.stats_saved {
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
        let totals = if self.stats_saved { self.known.totals.clone() } else { self.known.totals.plus(&self.game.record) };
        for a in achievements::newly(&self.known, &totals) {
            self.known.achieved.insert(a.id.to_string());
            progress::Progress::load().achieve(a.id);
            self.popups.push((format!("ACHIEVEMENT: {}  ({})", a.name.to_uppercase(), a.text), 6.0));
        }
    }

    pub(crate) fn restart(&mut self) {
        self.save_stats();
        self.stats_saved = false;
        self.result = None;
        self.civ_warning = None;
        self.game = new_game(&self.level, &self.args);
        arm(&mut self.game, &self.level, &self.args);
        self.endless_recorded = None;
        if let Some(p) = self.game.player() {
            self.orbit = Orbit::behind(p.fig.angle);
        }
    }

    pub(crate) fn settings(&self) -> Settings {
        // No floor (`GF_NO_FLOOR`): the street under Gunston Tower isn't drawn
        // (`AENG_draw`, aeng.cpp:9287); the roofs still are.
        let layers = self.layers.iter().copied().filter(|&l| !(self.game.no_floor && l == Layer::Floor)).collect();
        Settings { sky: self.level.sky, fog: self.args.fog, layers }
    }

    pub(crate) fn eye(&self) -> Option<Vec3> {
        if let Some(v) = self.game.player_vehicle() {
            return Some(v.pos / uc_world::UNIT + vec3(0.0, 0.9, 0.0));
        }
        self.game.player().map(|p| p.fig.pos / uc_world::UNIT + vec3(0.0, 0.62, 0.0))
    }

    pub(crate) fn camera(&self) -> Camera {
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
        match (self.eye(), self.third_person) {
            (Some(eye), true) => {
                // Further back when driving.
                let mut orbit = Orbit { ..self.orbit };
                if self.game.player_vehicle().is_some() {
                    orbit.dist = orbit.dist.max(4.0);
                }
                let mut cam = orbit.cam(eye);
                cam.pos = self.clear_of_buildings(eye, cam.pos);
                // With no floor the camera never goes below 0x280 (fc.cpp:1650).
                if self.game.no_floor {
                    cam.pos.y = cam.pos.y.max(0x280 as f32 / uc_world::UNIT);
                }
                cam.camera()
            }
            _ => self.fly.camera(),
        }
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
        self.game.rain(&self.level, cam.pos * uc_world::UNIT, cam.target - cam.pos, &mut out);
        out
    }

    /// Builds the level and uploads it to the renderer.
    pub(crate) fn upload(&self, r: &mut Renderer) {
        let t = Instant::now();
        let geo = self.level.build();
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
            r.render_to_image((n, n), cam, &Settings { sky: self.level.sky, fog: None, layers: layers.clone() }, &sea, &Overlay::new())
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

}
