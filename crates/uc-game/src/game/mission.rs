//! The mission script: running it, carrying out its commands, and answering its questions.

use crate::*;

/// What the script and the crime rate have counted already, so nothing counts twice.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tally {
    /// The `create_item` points whose item the player has picked up (`EWAY_FLAG_GOTITEM`,
    /// which the `specific_item_held` trigger watches).
    pub(crate) taken: BTreeSet<usize>,
    /// Cops already counted towards the script's tally of them.
    pub(crate) dead_cops: BTreeSet<usize>,
    /// Deaths and arrests already counted towards the crime rate.
    pub(crate) crimes: BTreeSet<usize>,
    /// The player has fired a gun. `GF_PLAYER_FIRED_GUN` is set once and never cleared, so a
    /// mission that cares only ever asks whether it has happened at all.
    pub(crate) player_fired: bool,
    /// Cars already blown up, so each goes up once.
    pub(crate) blown: BTreeSet<usize>,
}

/// Officer Training's (`train11`) pistol at the shooting range: point 29.
const TRAINING_PISTOL: usize = 29;

impl Game {
    /// Where person `i` is: their car's position while driving.
    pub(crate) fn thing_pos(&self, i: usize) -> Vec3 {
        match self.people[i].state {
            State::InCar { vehicle } => self.vehicles[vehicle].pos,
            _ => self.people[i].fig.pos,
        }
    }

    pub(crate) fn run_script(&mut self, dt: f32, level: &Level) {
        let Some(mut script) = self.script.take() else { return };
        let commands = script.update(dt, &Scene { game: self, level });
        self.script = Some(script);
        for c in commands {
            match c {
                Command::SpawnPlayer { way, pos, angle, person, has } => {
                    self.spawn_player(level, pos, angle, way, person, has);
                    if let Some(p) = self.player {
                        self.by_way.insert(way, p);
                    }
                }
                Command::SpawnEnemy { way, pos, angle, spec } => {
                    if let Some(i) = self.spawn(level, spec.person, false, way, pos, angle) {
                        let p = &mut self.people[i];
                        (p.ai, p.bent, p.ko_moves) = (spec.ai, spec.bent, spec.ko_moves);
                        (p.skill, p.mv, p.weapon) = (spec.skill, spec.mv, Weapon::from_has(spec.has));
                        p.follow = spec.follow;
                        p.drop = game::special_item(spec.drop);
                        p.fig.clothes = spec.colour & 3;
                        p.fig.jacket = spec.skill / 4;
                        p.other = spec.other;
                        (p.colour, p.group) = (spec.colour, spec.group);
                        (p.protected, p.guilty) = (spec.invulnerable, spec.guilty);
                        p.give(p.weapon);
                    }
                }
                Command::Adjust { way, ai, bent, ko_moves, other, mv, follow, colour, group } => {
                    if let Some(&i) = self.by_way.get(&way) {
                        let p = &mut self.people[i];
                        (p.ai, p.bent, p.ko_moves, p.other, p.mv, p.follow) = (ai, bent, ko_moves, other, mv, follow);
                        (p.colour, p.group) = (colour, group);
                    }
                }
                Command::Remove { way } => {
                    if let Some(&i) = self.by_way.get(&way) {
                        self.people[i].removed = true;
                    }
                    if let Some(k) = self.creature_by_way.remove(&way) {
                        self.creatures[k].state = creature::State::Dead;
                        self.creatures[k].fig.pos.y -= 1.0e5;
                    }
                    self.items.retain(|it| it.way != way);
                    self.hidden.retain(|(_, it)| it.way != way);
                    // A car a kill_waypoint takes away blows up where it stands: the original
                    // wrecks it (1050 damage) and sets off a shockwave that catches the player
                    // too, then forgets it was ever created, so a "killed" trigger watching
                    // the car won't fire afterwards (`EWAY_DO_KILL_WAYPOINT`, eway.cpp).
                    if let Some(v) = self.veh_by_way.remove(&way) {
                        let at = self.vehicles[v].pos;
                        self.vehicles[v].health = 0;
                        self.tally.blown.insert(v);
                        if let Some(d) = self.vehicles[v].driver.take() {
                            self.people[d].state = State::Move;
                            self.people[d].fig.pos = at;
                        }
                        self.pyro.burst(at + Vec3::new(0.0, 48.0, 0.0), pyro::CAR_BOMB, 400.0);
                        self.sounds.push((Sfx::Explosion, at));
                        self.shockwave(level, at, 768.0, 350, Blame::Nobody);
                    }
                }
                Command::Move { way, pos, angle } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    if let Some(&i) = self.by_way.get(&way) {
                        let p = &mut self.people[i];
                        if !p.is_dead() {
                            p.fig.pos = Vec3::new(pos.x, y, pos.z);
                            p.fig.angle = angle;
                        }
                    }
                    // Only people, vehicles and animals: `EWAY_get_person` gives nothing for a
                    // `create_item` point (eway.cpp:6196), so the `move_thing` points several
                    // missions aim at items (Gun Hunt's four for its stashed gun) do nothing.
                }
                Command::Item { way, kind, pos, .. } if kind == game::mines::IT_MINE => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    self.mines.push(Mine { way, pos: Vec3::new(pos.x, y, pos.z), fuse: None, gone: false });
                }
                Command::Item { way, kind, pos, stashed } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    let item = Item { way, kind, pos: Vec3::new(pos.x, y, pos.z) };
                    // A stashed item goes into the nearest prop within eight squares, which
                    // becomes searchable; with none that near it is left lying in the open
                    // (`EWAY_create_item`).
                    match find_ob(level, pos, 2048.0, None).filter(|_| stashed) {
                        Some(ob) => {
                            self.searchable.insert(ob);
                            self.hidden.push((ob, item));
                        }
                        None => self.items.push(item),
                    }
                    // Not in the original: Officer Training's shooting range has the sniper
                    // rifle lying beside its pistol, to try the scope out on the targets.
                    if way == TRAINING_PISTOL && level.mission.mission_name.eq_ignore_ascii_case("train11") {
                        let at = Vec3::new(pos.x + 96.0, 0.0, pos.z);
                        let y = level.ground.surface(&level.map, at.x, at.z, pos.y + 32.0);
                        self.items.push(Item { way: 0, kind: crate::SNIPER, pos: Vec3::new(at.x, y, at.z) });
                    }
                }
                Command::Bike { way, pos, angle } => self.add_bike(level, way, pos, angle),
                Command::Creature { way, kind, pos, angle } => {
                    let Some(chunk) = creature_chunk(level, kind) else {
                        eprintln!("mission: no anim prim for creature {kind}");
                        continue;
                    };
                    if let Some(fig) = Figure::anim_prim(chunk, pos, angle, 1) {
                        let floor = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 128.0);
                        self.creatures.push(creature::Creature::new(way, kind, fig, pos, angle, floor));
                        self.creature_by_way.insert(way, self.creatures.len() - 1);
                    }
                }
                Command::WareFx { pos, ambience } => {
                    match self.wares.which_contains(&level.map, (pos.x as i32) >> 8, (pos.z as i32) >> 8) {
                        Some(w) => self.wares.wares[w].ambience = ambience,
                        // The original says so too, in its debug output.
                        None => eprintln!("mission: warehouse ambience point at {:.0},{:.0} is not inside a warehouse", pos.x, pos.z),
                    }
                }
                Command::Untrip { way } => self.trips.deactivate(way),
                Command::Pee { way, on } => {
                    if let Some(&i) = self.by_way.get(&way) {
                        self.people[i].peeing = on;
                    }
                }
                Command::Searchable { pos } => {
                    if let Some(ob) = find_ob(level, pos, 256.0, None) {
                        self.searchable.insert(ob);
                    }
                }
                Command::Camera { way, shot } => {
                    let _ = way;
                    self.cutscene = Some(Cutscene { pos: shot.start, shot, leg: 0, wait: 0.0 });
                }
                Command::Conversation { way, speech, lines, a, b, ambient } => {
                    // A conversation still going is cut short but counts as said, as in the
                    // original (`EWAY_finish_conversation`): what waits on it still happens.
                    if let (Some(old), Some(s)) = (self.convo.take(), self.script.as_mut()) {
                        s.conversation_over(old.way);
                    }
                    self.wide = None;
                    // It's filmed only if both of them are there and fit to talk
                    // (`person_ok_for_conversation`); otherwise it's just said.
                    let who = |w: usize| self.by_way.get(&w).copied().filter(|&i| self.can_talk(i));
                    let pair = who(a).zip(who(b)).filter(|(i, j)| i != j);
                    if let (Some((i, j)), false) = (pair, ambient) {
                        // `PCOM_make_people_talk_to_eachother`: they turn to each other.
                        let (pi, pj) = (self.people[i].fig.pos, self.people[j].fig.pos);
                        self.people[i].face_towards(pj);
                        self.people[j].face_towards(pi);
                    }
                    self.convo = Some(Convo { way, speech, lines, line: usize::MAX, left: 0.0, pair, ambient, skip: 1.0, cam: None });
                }
                Command::Message { way, speech, text, secs, speaker } => {
                    let _ = way;
                    let face = match speaker {
                        script::Speaker::Street | script::Speaker::Tutorial => None,
                        script::Speaker::Radio => Some(talk::face::RADIO),
                        script::Speaker::Person(w) => match self.by_way.get(&w).copied() {
                            // Dead people can't talk.
                            Some(i) if self.people[i].is_dead() => continue,
                            Some(i) => {
                                // Someone the player is talking to turns to them.
                                if let (Some(p), true) = (self.player, self.can_talk(i)) {
                                    let at = self.people[p].fig.pos;
                                    self.people[i].face_towards(at);
                                }
                                Some(self.face(i))
                            }
                            None => Some(talk::face::RADIO),
                        },
                    };
                    let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                    self.sounds.push((Sfx::Voice(speech), at));
                    self.messages.retain(|m| m.text != text);
                    self.messages.push(Message { text, left: secs, face, filmed: false });
                }
                Command::Transfer { way } => self.transfer(way, level),
                Command::Platform { way, colour, group, speed, flags } => {
                    if let Some(p) = self.platforms.iter_mut().find(|p| p.way == way) {
                        p.start(colour, group, speed, flags);
                    }
                }
                Command::Vehicle { way, kind, pos, angle, key } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    // A vehicle's angle is its heading (its nose along (sin, cos)).
                    self.vehicles.push(Vehicle::new(way, kind, Vec3::new(pos.x, y, pos.z), angle, key));
                    self.veh_by_way.insert(way, self.vehicles.len() - 1);
                }
                Command::Chopper { way, pos, victim } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    self.choppers.push(chopper::Chopper::new(way, Vec3::new(pos.x, y, pos.z), victim));
                }
                Command::Stall { way } => {
                    if let Some(&v) = self.veh_by_way.get(&way) {
                        self.vehicles[v].stalled = true;
                    }
                }
                Command::Lock { way, locked } => {
                    if let Some(&v) = self.veh_by_way.get(&way) {
                        self.vehicles[v].key = if locked { vehicle::Key::Never } else { vehicle::Key::None };
                    }
                }
                // A `visual_effect` point's bang. The original's shockwave here never goes
                // off: `if (ew->ed.subtype=!16)` in `EWAY_DO_EXPLODE` (eway.cpp) assigns
                // instead of comparing, so the test is always false. Nobody is hurt.
                Command::Explode { way, pos, kinds, scale } => {
                    let _ = way;
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    let at = Vec3::new(pos.x, y.max(pos.y), pos.z);
                    self.pyro.burst(at, kinds, if scale > 0.0 { scale } else { 400.0 });
                    self.sounds.push((Sfx::Explosion, at));
                }
                Command::Barrel { way, kind, pos } => {
                    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 32.0);
                    let yaw = (self.rand() % 628) as f32 / 100.0;
                    let semtex = level.name.eq_ignore_ascii_case("semtex");
                    let (prim, explodes) = Barrel::of_kind(kind, semtex);
                    self.add_barrel(Barrel { kind, prim, explodes, way, gone: false, pos: Vec3::new(pos.x, y, pos.z), yaw, tilt: 0.0, on: None, spheres: None });
                }
                Command::Sign { which, flip } => {
                    let text = match (which, flip) {
                        (0, _) => "U-TURN",
                        (1, false) => "TURN RIGHT",
                        (1, true) => "TURN LEFT",
                        (2, false) => "NO RIGHT TURN",
                        (2, true) => "NO LEFT TURN",
                        _ => "STOP",
                    };
                    self.sign = Some((text.to_string(), 2.5));
                }
                Command::Mist { pos } => self.mists.create(pos.x, pos.z),
                Command::Sound { id, pos, replace } => self.sounds.push((Sfx::ById { id, replace }, pos)),
                Command::Electrify { pos, on } => self.electrify(level, pos, on),
                Command::NoFloor => self.no_floor = true,
                Command::OpenDoor { pos } => {
                    if !level.ground.open_door_near(pos.x, pos.z) {
                        // The original says so too, on its console.
                        eprintln!("mission: no door near {:.0},{:.0} to open", pos.x, pos.z);
                    }
                }
                Command::Win => self.finish(Outcome::Won),
                Command::Lose => self.finish(Outcome::Lost("Mission failed".into())),
            }
        }
    }

    /// Keeps the script's tally of cops killed up to date: the original bumps counter 7
    /// wherever a `PERSON_COP` dies, and a mission can ask for a number of them.
    pub(crate) fn count_dead_cops(&mut self) {
        let newly: Vec<usize> = self
            .people
            .iter()
            .enumerate()
            .filter(|(i, p)| p.kind == ptype::COP && p.knocked_out && !self.tally.dead_cops.contains(i))
            .map(|(i, _)| i)
            .collect();
        for i in newly {
            self.tally.dead_cops.insert(i);
            if let Some(s) = self.script.as_mut() {
                s.note_cop_killed();
            }
        }
    }

    /// The crime rate moves with deaths and arrests (fallen/Source/Person.cpp): killing a
    /// guilty person takes 2 off and arresting one 4, and killing a civilian out for a stroll
    /// puts 5 on. Someone who kills himself (`PCOM_AI_SUICIDE`) doesn't count.
    ///
    /// The original also meant objectives to bring it down (`EWAY_DO_OBJECTIVE`), but the
    /// loader only makes those under `if (0)` (elev.cpp:1434), so they never do.
    pub(crate) fn tally_crime(&mut self) {
        let mut rate = self.crime_rate as i32;
        for (i, p) in self.people.iter().enumerate() {
            let arrested = p.arrested || matches!(p.state, State::Cuffed);
            // A passer-by the crowd brings back (`respawn`) counts again next time.
            if !(p.knocked_out || arrested) {
                self.tally.crimes.remove(&i);
                continue;
            }
            if p.player || !self.tally.crimes.insert(i) {
                continue;
            }
            if arrested {
                if p.is_guilty() {
                    rate -= 4;
                }
            } else if p.ai != person::AI_SUICIDE {
                if p.is_guilty() {
                    rate -= 2;
                } else if p.ai == person::AI_CIV && p.mv == person::MOVE_WANDER {
                    rate += 5;
                }
                // The player killing an innocent, a civilian out for a stroll or a cop, and not
                // marked guilty (`FLAG2_PERSON_GUILTY`), earns a red mark (`RedMarks`,
                // Person.cpp:2340).
                let civ = p.ai == person::AI_CIV && p.mv == person::MOVE_WANDER;
                let cop = p.kind == uc_world::character::person::COP;
                if p.hurt_by.is_some() && p.hurt_by == self.player && (civ || cop) && !p.guilty {
                    self.red_marks = self.red_marks.saturating_add(1);
                }
            }
            rate = rate.clamp(0, 100);
        }
        self.crime_rate = rate as u8;
    }

    pub(crate) fn finish(&mut self, o: Outcome) {
        if self.outcome.is_none() {
            let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
            self.sounds.push((if o == Outcome::Won { Sfx::Won } else { Sfx::Lost }, at));
            self.outcome = Some(o);
        }
    }

    /// Whether mission event point `i` is active (for tests and debugging).
    pub fn point_active(&self, i: usize) -> bool {
        self.script.as_ref().is_some_and(|s| s.point_active(i))
    }

    /// The visible countdown, in seconds, while one runs.
    pub fn countdown(&self) -> Option<f32> {
        self.script.as_ref()?.countdown()
    }

    /// `EWAY_DO_TRANSFER_PLAYER`: the player becomes someone else. Whoever they were stays
    /// behind as a civilian who minds their own business, fights back if set on and can't be
    /// hurt; the new one loses any protection and is the camera's from now on.
    pub(crate) fn transfer(&mut self, way: usize, level: &Level) {
        // The original says nothing when there's nobody (its message is commented out).
        let Some(&new) = self.by_way.get(&way) else { return };
        if let Some(old) = self.player.filter(|&o| o != new) {
            let p = &mut self.people[old];
            p.player = false;
            (p.ai, p.bent, p.skill) = (1, person::BENT_ROBOT | person::BENT_FIGHT_BACK, 15);
            (p.mv, p.goal) = (1, None);
            p.protected = true;
            if matches!(p.state, State::Attack { .. } | State::Block | State::Shoot { .. }) {
                p.state = State::Move;
            }
        }
        let p = &mut self.people[new];
        p.player = true;
        (p.ai, p.skill) = (0, 0);
        p.bent &= !person::BENT_PLAYERKILL;
        p.protected = false;
        (p.goal, p.foe) = (None, None);
        self.player = Some(new);
        self.player_name = player_name(&level.mission.mission_name, p.kind);
        (self.target, self.aiming.creature) = (None, None);
    }
}

/// The game with its level, for script conditions that need the walls.
struct Scene<'a> {
    game: &'a Game,
    level: &'a Level,
}

impl script::Query for Scene<'_> {
    fn player_pos(&self) -> Option<Vec3> {
        self.game.player_pos()
    }
    fn person(&self, way: usize) -> Option<script::PersonState> {
        self.game.person(way)
    }
    fn thing(&self, way: usize) -> Option<script::ThingState> {
        self.game.thing(way)
    }
    fn in_vehicle(&self, person: usize, vehicle: usize) -> bool {
        self.game.in_vehicle(person, vehicle)
    }
    fn player_grabbing(&self) -> bool {
        self.game.player_grabbing()
    }
    fn player_used(&self) -> bool {
        self.game.player_used()
    }
    fn got_item(&self, way: usize) -> bool {
        self.game.got_item(way)
    }
    fn prim_damaged(&self, pos: Vec3) -> bool {
        self.game.prim_damaged(self.level, pos)
    }
    fn crime_rate(&self) -> u8 {
        self.game.crime_rate
    }
    fn tripped(&self, way: usize) -> bool {
        self.game.trips.tripped(way)
    }
    fn player_fired(&self) -> bool {
        self.game.player_fired()
    }

    /// `can_a_see_b` (fallen/Source/Person.cpp): up to 8 squares, within about 74 degrees of
    /// straight ahead (much wider up close), with a clear line between their heads: 0x60 up,
    /// or 0x20 for someone crouching, who can hide behind a low wall.
    fn sees(&self, a: usize, b: usize) -> bool {
        let g = self.game;
        let (Some(&a), Some(&b)) = (g.by_way.get(&a), g.by_way.get(&b)) else { return false };
        let (pa, pb) = (&g.people[a], &g.people[b]);
        if pa.removed || pb.removed || pa.is_dead() {
            return false;
        }
        let (from, to) = (pa.fig.pos, g.thing_pos(b));
        let d = Vec2::new(to.x - from.x, to.z - from.z);
        let dist = d.length();
        if dist > 8.0 * 256.0 {
            return false;
        }
        let ahead = pa.facing_dir().dot(d / dist.max(1.0));
        let cone = if dist < 192.0 { -0.54 } else { 0.28 };
        let head = |p: &Person| Vec3::new(0.0, if p.crouching() { 0x20 as f32 } else { 0x60 as f32 }, 0.0);
        ahead > cone && self.level.ground.line_of_sight(from + head(pa), to + head(pb))
    }
}

impl script::Query for Game {
    fn player_pos(&self) -> Option<Vec3> {
        self.player().map(|p| p.fig.pos)
    }

    fn person(&self, way: usize) -> Option<script::PersonState> {
        if let Some(&k) = self.creature_by_way.get(&way) {
            let c = &self.creatures[k];
            return Some(script::PersonState { dead: c.dead, down: c.dead, arrested: false, half_dead: c.health * 2 < c.max_health() });
        }
        if let Some(&v) = self.veh_by_way.get(&way) {
            let w = self.vehicles[v].wrecked();
            return Some(script::PersonState { dead: w, down: w, arrested: false, half_dead: self.vehicles[v].health < 150 });
        }
        // A barrel is "killed" once it has been made and blown up (eway.cpp:2704).
        if let Some(b) = self.barrels.iter().find(|b| way != 0 && b.way == way) {
            return Some(script::PersonState { dead: b.gone, down: b.gone, arrested: false, half_dead: false });
        }
        let p = &self.people[*self.by_way.get(&way)?];
        Some(script::PersonState {
            // Someone a kill_waypoint took away isn't dead: the original just moves them off
            // the map (Driving Silver swaps its instructor this way).
            // One of the Men in Black who went up in lightning is gone but still dead.
            dead: p.is_dead() && (!p.removed || p.blown_up),
            down: p.is_down() || p.removed,
            // Being cuffed counts: otherwise "killed, not arrested" fires mid-arrest.
            arrested: p.arrested || matches!(p.state, State::Cuffed),
            half_dead: p.health * 2 < p.max_health,
        })
    }

    fn thing(&self, way: usize) -> Option<script::ThingState> {
        if let Some(&k) = self.creature_by_way.get(&way) {
            let c = &self.creatures[k];
            return Some(script::ThingState { pos: c.fig.pos, angle: c.fig.angle, speed: 0.0 });
        }
        if let Some(&v) = self.veh_by_way.get(&way) {
            let v = &self.vehicles[v];
            return Some(script::ThingState { pos: v.pos, angle: v.script_angle(), speed: v.speed });
        }
        let p = &self.people[*self.by_way.get(&way)?];
        if let State::InCar { vehicle } = p.state {
            let v = &self.vehicles[vehicle];
            return Some(script::ThingState { pos: v.pos, angle: v.script_angle(), speed: v.speed });
        }
        Some(script::ThingState { pos: p.fig.pos, angle: p.fig.angle, speed: 0.0 })
    }

    fn in_vehicle(&self, person: usize, vehicle: usize) -> bool {
        let Some(&p) = self.by_way.get(&person) else { return false };
        match self.people[p].state {
            State::InCar { vehicle: v } => vehicle == 0 || self.vehicles[v].way == vehicle,
            _ => false,
        }
    }

    fn player_used(&self) -> bool {
        self.used
    }

    fn player_grabbing(&self) -> bool {
        self.player().is_some_and(|p| p.grabbing())
    }

    fn got_item(&self, way: usize) -> bool {
        self.tally.taken.contains(&way)
    }

    fn player_fired(&self) -> bool {
        self.tally.player_fired
    }
}
