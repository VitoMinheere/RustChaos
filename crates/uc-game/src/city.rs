//! The city's own people and traffic, besides whoever the mission makes: the extra civilians
//! a mission asks for (`fake_civs`) strolling the pavements, and the cars it asks for
//! (`fake_cars`) driving the roads (`ELEV_load_level`, fallen/Source/elev.cpp). Neither is
//! ever really far away: once out of sight for a moment, they're moved to just out of sight
//! ahead of the player (`PCOM_do_regen`, `PCOM_process_driving_wander`,
//! fallen/Source/pcom.cpp).
//!
//! These civilians run from gunfire, from someone drawing a gun in sight of them and from
//! being hit, and don't forget it (the mission's own don't yet: see
//! [`crate::Person::scare`]). Walk up to one and press action and they have something to
//! say, from the mission's own chatter file (`EWAY_get_fake_wander_message`,
//! fallen/Source/eway.cpp).

use uc_formats::units::radians;
use glam::{vec2, Vec2, Vec3};
use uc_world::character::person as ptype;
use uc_world::Level;

use crate::person::{AI_CIV, AI_COP, AI_GANG, BENT_FIGHT_BACK};
use crate::vehicle::Vehicle;
use crate::{rand, Driving, Game, Message, State, Weapon, AI_DRIVER};

/// `DRAW_DIST` on the PC: 22 squares, how far the original draws, so how far away the extra
/// people have to be before they can be moved without anyone seeing.
pub const DRAW_DIST: f32 = 22.0 * 256.0;
/// Somebody put down at the edge of sight is still in it: the original's are drawn the
/// moment they appear ahead of the player, so they stay where they're put. Without this
/// margin half of them would be out of sight again at once and moved on, until they all
/// gathered at the few spots where a pavement crosses the line.
const MARGIN: f32 = 2.0 * 256.0;
/// Seconds out of sight before being moved: fifty of the original's turns at twenty a
/// second (`should_person_regen`, and `sewerbits` for drivers).
const UNSEEN: f32 = 2.5;
/// How far gunfire carries, and how far someone drawing a gun is noticed
/// (`PCOM_oscillate_tympanum`'s `PCOM_SOUND_GUNSHOT` and `_DRAW_GUN`).
const GUNSHOT: f32 = 0xa00 as f32;
const DRAW_GUN: f32 = 0x600 as f32;
/// `vehicle_random` (fallen/Source/vehicle.cpp): the extra cars are vans, cars, taxis, jeeps
/// and sedans.
const VEHICLE_RANDOM: [u8; 16] = [0, 1, 2, 5, 7, 0, 1, 2, 5, 7, 0, 1, 2, 0, 1, 2];

/// What the extra people say when spoken to: any line of the mission's chatter file, or
/// `annoyed.txt`'s once someone has frightened them.
#[derive(Debug, Clone, Default)]
pub struct Chatter {
    pub normal: Vec<String>,
    pub annoyed: Vec<String>,
}

/// The numbered lines of a message file (`EWAY_load_message_file`): `n. text`. Section
/// headings and anything else are skipped, so `citsez.txt`'s annoyed lines are ordinary
/// chatter too, as they are in the original.
pub fn message_lines(bytes: &[u8]) -> Vec<String> {
    // The files are Latin-1.
    let text: String = bytes.iter().map(|&b| b as char).collect();
    text.lines()
        .filter_map(|line| {
            let t = line.trim_start();
            let digits = t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let rest = t[digits..].strip_prefix('.').filter(|_| digits > 0)?;
            let msg = rest.trim();
            (!msg.is_empty()).then(|| msg.to_string())
        })
        .collect()
}

impl Chatter {
    /// `EWAY_load_fake_wander_text`: the mission's file, else `citsez.txt`.
    pub fn load(level: &Level) -> Chatter {
        let read = |name: &str| {
            (!name.is_empty())
                .then(|| uc_formats::resolve(&level.game, name))
                .flatten()
                .and_then(|p| std::fs::read(p).ok())
                .map(|b| message_lines(&b))
        };
        Chatter {
            normal: read(&level.mission.citsez).or_else(|| read("text\\citsez.txt")).unwrap_or_default(),
            annoyed: read("text\\annoyed.txt").unwrap_or_default(),
        }
    }
}

impl Game {
    fn city_rand(&mut self) -> u32 {
        rand(&mut self.rngs.city)
    }

    /// Makes the mission's extra civilians and cars, all over the map.
    pub(crate) fn populate(&mut self, level: &Level) {
        if level.people.is_none() || !self.streets.wand.any() {
            return;
        }
        for _ in 0..level.mission.fake_civs {
            // Somewhere at random, well in from the edges.
            let mut at = None;
            for _ in 0..1000 {
                let (x, z) = ((self.city_rand() % 100 + 14) as i32, (self.city_rand() % 100 + 14) as i32);
                if let Some(p) = self.streets.wand.start_near(x, z, 2) {
                    at = Some(p);
                    break;
                }
            }
            let Some(at) = at else { break };
            let angle = radians((self.city_rand() & 2047) as f32);
            let variant = self.city_rand();
            let Some(i) = self.spawn_variant(level, ptype::CIV, false, 0, Vec3::new(at.x, 0.0, at.y), angle, variant) else { break };
            // Partway to being moved already (`InsideRoom` starts at 20 to 39 of the 50).
            let unseen = (self.city_rand() % 20 + 20) as f32 / 20.0;
            let p = &mut self.people[i];
            (p.ai, p.mv, p.fake, p.unseen) = (AI_CIV, 4, true, unseen);
        }
        if self.streets.roads.nodes.len() < 2 {
            return;
        }
        for _ in 0..level.mission.fake_cars {
            let Some((at, yaw)) = (0..16).find_map(|_| self.car_start(level, true)) else { continue };
            let kind = VEHICLE_RANDOM[((self.city_rand() >> 4) & 15) as usize];
            let y = level.ground.surface(&level.map, at.x, at.y, 1.0e4);
            let variant = self.city_rand();
            let Some(i) = self.spawn_variant(level, ptype::CIV, false, 0, Vec3::new(at.x, y, at.y), 0.0, variant) else { break };
            self.vehicles.push(Vehicle::new(0, kind, Vec3::new(at.x, y, at.y), yaw, crate::vehicle::Key::None));
            let vi = self.vehicles.len() - 1;
            self.vehicles[vi].driver = Some(i);
            let unseen = (self.city_rand() % 20 + 20) as f32 / 20.0;
            let p = &mut self.people[i];
            (p.ai, p.mv, p.fake, p.unseen) = (AI_DRIVER, 4, true, unseen);
            p.state = State::InCar { vehicle: vi };
            self.streets.drivers.insert(i, Driving::default());
        }
    }

    /// `WAND_find_good_start_point_for_car`: a spot in a lane of the road nearest a random
    /// point, anywhere on the map or `view_dist` from the player, with the way the lane runs.
    /// None if it's near a junction, near another car, or (from the player) in sight.
    fn car_start(&mut self, level: &Level, anywhere: bool) -> Option<(Vec2, f32)> {
        let heading = (self.city_rand() >> 9) & 1 == 1;
        let player = self.player.map(|p| self.thing_pos(p));
        let p = if anywhere {
            vec2(((self.city_rand() % 112 + 8) << 8) as f32, ((self.city_rand() % 112 + 8) << 8) as f32)
        } else {
            let a = radians((self.city_rand() & 2047) as f32);
            let pl = player?;
            vec2(pl.x, pl.z) + vec2(a.sin(), a.cos()) * self.view_dist
        };
        let (n1, n2) = self.streets.roads.find(p)?;
        let (r1, r2) = (self.streets.roads.pos(n1), self.streets.roads.pos(n2));
        let side = if heading { -384.0 } else { 384.0 };
        let at = if r1.x == r2.x {
            // Not off the end of the road, and not on a junction.
            if p.y < r1.y.min(r2.y) || p.y > r1.y.max(r2.y) || (p.y - r1.y).abs() < 640.0 || (p.y - r2.y).abs() < 640.0 {
                return None;
            }
            vec2(r1.x + side, p.y)
        } else {
            if p.x < r1.x.min(r2.x) || p.x > r1.x.max(r2.x) || (p.x - r1.x).abs() < 640.0 || (p.x - r2.x).abs() < 640.0 {
                return None;
            }
            vec2(p.x, r1.y + side)
        };
        if let (false, Some(pl)) = (anywhere, player) {
            if crate::creature::qdist2(at - vec2(pl.x, pl.z)) < self.view_dist {
                return None;
            }
        }
        if self.vehicles.iter().any(|v| (vec2(v.pos.x, v.pos.z) - at).length() < 1024.0) {
            return None;
        }
        let (mx, mz) = ((at.x as i32) >> 8, (at.y as i32) >> 8);
        if !(0..128).contains(&mx) || !(0..128).contains(&mz) || level.map.square(mx as usize, mz as usize).has(uc_formats::iam::flags::HIDDEN) {
            return None;
        }
        // Facing down the lane it's in, so it drives off the way the traffic goes.
        let dir = if self.streets.roads.signed_dist(n1, n2, at) >= 0.0 { r2 - r1 } else { r1 - r2 };
        Some((at, dir.x.atan2(dir.y)))
    }

    /// Each frame: the extra people's next place to walk to, keeping up with whoever they're
    /// running from, and moving those long out of sight to somewhere near the player.
    pub(crate) fn city_life(&mut self, dt: f32, level: &Level) {
        for i in 0..self.people.len() {
            if let Some(by) = self.people[i].scared.and_then(|s| s.by) {
                let at = self.thing_pos(by);
                if let Some(s) = self.people[i].scared.as_mut() {
                    s.at = at;
                }
            }
        }
        // The player drawing a gun, where people can see it.
        if let Some(pi) = self.player {
            let drawn = self.people[pi].weapon.is_gun() && !matches!(self.people[pi].state, State::InCar { .. });
            if drawn && !self.gun_drawn {
                let at = self.people[pi].fig.pos;
                let eye = Vec3::new(0.0, 140.0, 0.0);
                for j in 0..self.people.len() {
                    let o = &self.people[j];
                    if j != pi && (o.fig.pos - at).length() < DRAW_GUN && level.ground.line_of_sight(o.fig.pos + eye, at + eye) {
                        self.people[j].scare(Some(pi), at);
                    }
                }
            }
            self.gun_drawn = drawn;
        }
        let Some(pi) = self.player else { return };
        let me = self.thing_pos(pi);
        let facing = match self.people[pi].state {
            State::InCar { vehicle } => {
                let y = self.vehicles[vehicle].yaw;
                vec2(y.sin(), y.cos())
            }
            _ => self.people[pi].facing_dir(),
        };
        // Nobody is moved while the player is indoors or watching a scene.
        let paused = self.indoors.is_some() || self.cutscene.is_some() || self.convo.is_some();
        for i in 0..self.people.len() {
            if !self.people[i].fake || i == pi {
                continue;
            }
            let at = self.thing_pos(i);
            let d = at - me;
            let seen = crate::creature::qdist2(vec2(d.x, d.z)) < self.view_dist + MARGIN;
            let p = &mut self.people[i];
            p.unseen = if seen { 0.0 } else { p.unseen + dt };
            match p.state {
                State::InCar { vehicle } => {
                    if p.unseen > UNSEEN && !paused && self.vehicles[vehicle].driver == Some(i) && !self.vehicles[vehicle].wrecked() {
                        if let Some((to, yaw)) = self.car_start(level, false) {
                            let y = level.ground.surface(&level.map, to.x, to.y, 1.0e4);
                            self.vehicles[vehicle].reset(Vec3::new(to.x, y, to.y), yaw);
                            self.streets.drivers.insert(i, Driving::default());
                            self.people[i].unseen = (self.city_rand() & 15) as f32 / 20.0;
                        }
                    }
                }
                _ if p.ai == AI_CIV && p.kind != ptype::THUG_RASTA => {
                    if p.unseen > UNSEEN && !paused && p.scared.is_none() {
                        let mut rng = self.rngs.city;
                        let to = self.streets.wand.start_ahead(me, facing, self.view_dist, &mut || rand(&mut rng));
                        self.rngs.city = rng;
                        // Not on top of another of them. The original takes the first pavement
                        // it finds near a spot ahead of the player, so while the player stands
                        // still, everyone moved ends up at the same few corners; this engine
                        // tries again next frame instead.
                        let crowded = |to: &Vec2| self.people.iter().any(|o| o.fake && vec2(o.fig.pos.x - to.x, o.fig.pos.z - to.y).length() < 3.0 * 256.0);
                        let to = to.filter(|to| !crowded(to));
                        if let Some(to) = to {
                            let y = level.ground.surface(&level.map, to.x, to.y, 1.0e4);
                            self.people[i].respawn(Vec3::new(to.x, y, to.y));
                            self.turn_bored(i, pi, level);
                        }
                    }
                    let p = &mut self.people[i];
                    if p.wants_somewhere() && p.scared.is_none() && !p.is_down() {
                        let mut rng = self.rngs.city;
                        let to = self.streets.wand.next_place(p.fig.pos, p.fig.facing(), &mut || rand(&mut rng));
                        self.rngs.city = rng;
                        p.wander_to(to);
                    }
                }
                // A passer-by boredom turned into a thug: in on the player only while no real
                // gang member or Man In Black is within six squares of her, and otherwise
                // keeping clear, running if within fifteen (`PCOM_process_wander`,
                // `PCOM_should_fake_person_attack_darci`). Out of sight thirty squares off,
                // they're moved back near her like anyone in the crowd (pcom.cpp:8661).
                _ if p.kind == ptype::THUG_RASTA => {
                    let far = (at - me).length() > 30.0 * 256.0;
                    if far && p.unseen > UNSEEN && !paused {
                        let mut rng = self.rngs.city;
                        let to = self.streets.wand.start_ahead(me, facing, self.view_dist, &mut || rand(&mut rng));
                        self.rngs.city = rng;
                        if let Some(to) = to {
                            let y = level.ground.surface(&level.map, to.x, to.y, 1.0e4);
                            self.people[i].respawn(Vec3::new(to.x, y, to.y));
                            self.turn_bored(i, pi, level);
                        }
                        continue;
                    }
                    if p.is_down() || p.is_dead() {
                        continue;
                    }
                    let real_thugs = self.people.iter().any(|o| {
                        !o.fake && !o.removed && !o.is_dead()
                            && matches!(o.kind, ptype::THUG_RASTA | ptype::THUG_GREY | ptype::THUG_RED | ptype::MIB1 | ptype::MIB2 | ptype::MIB3)
                            && (o.fig.pos - me).length() < 0x600 as f32
                    });
                    let p = &mut self.people[i];
                    if real_thugs || paused {
                        p.ai = AI_CIV;
                        p.foe = None;
                        if (at - me).length() < 15.0 * 256.0 {
                            p.scare(Some(pi), me);
                        }
                    } else {
                        p.ai = AI_GANG;
                        p.foe = Some(pi);
                    }
                }
                _ => {}
            }
        }
    }

    /// `PCOM_do_regen`'s boredom (pcom.cpp:7066): once nothing has happened for the mission's
    /// `BOREDOM_RATE` times five seconds (at least four, and 255 for never), the passer-by
    /// just moved back near the player comes back as a rasta thug out to get her, or as a cop
    /// if she's one of the gang. Either way with 200 health, sometimes a gun.
    pub(crate) fn turn_bored(&mut self, i: usize, pi: usize, level: &Level) {
        let rate = level.mission.boredom_rate;
        let rate = if level.mission.version < 10 { 4 } else { rate.max(4) };
        if rate == 255 || self.bored <= rate as f32 * 5.0 {
            return;
        }
        let gang_player = !matches!(self.people[pi].kind, ptype::DARCI | ptype::ROPER);
        let (kind, ai, bent) = if gang_player { (ptype::COP, AI_COP, 0) } else { (ptype::THUG_RASTA, AI_GANG, BENT_FIGHT_BACK) };
        let Some(people) = level.people.as_ref() else { return };
        let variant = self.city_rand() % 3;
        let (pos, angle) = (self.people[i].fig.pos, self.people[i].fig.angle);
        let Some(fig) = people.figure(kind, variant, pos, angle) else { return };
        // `timer_bored & 16`, in milliseconds.
        let gun = (self.bored * 1000.0) as u32 & 16 != 0;
        let p = &mut self.people[i];
        p.fig = fig;
        p.kind = kind;
        (p.ai, p.bent, p.mv) = (ai, bent, 4);
        (p.health, p.max_health) = (200, 200);
        p.weapon = Weapon::None;
        if gun {
            p.give(Weapon::Pistol);
            p.weapon = Weapon::Pistol;
        }
        p.foe = Some(pi);
        p.scared = None;
        self.bored = 0.0;
    }

    /// Gunfire: every civilian within earshot of `shooter` runs from them.
    pub(crate) fn gunshot(&mut self, shooter: usize) {
        let at = self.people[shooter].fig.pos;
        self.bang(shooter, at);
    }

    /// A loud noise at `at` that `shooter` made (`PCOM_SOUND_BANG`): whoever is near is scared.
    pub(crate) fn bang(&mut self, shooter: usize, at: Vec3) {
        for j in 0..self.people.len() {
            if j != shooter && (self.people[j].fig.pos - at).length() < GUNSHOT {
                self.people[j].scare(Some(shooter), at);
                self.people[j].hear(at);
            }
        }
    }

    /// One of the extra people within reach, for the player to talk to: only Darci asks
    /// them anything (`PERSON_DARCI`, in `INTERFAC`'s action handling).
    pub(crate) fn chat(&mut self, pi: usize) -> bool {
        if self.people[pi].kind != ptype::DARCI || self.streets.chatter.normal.is_empty() {
            return false;
        }
        let pos = self.people[pi].fig.pos;
        let Some(j) = (0..self.people.len()).find(|&j| {
            let o = &self.people[j];
            j != pi && o.fake && o.ai == AI_CIV && o.scared.is_none() && self.can_talk(j) && (o.fig.pos - pos).length() < 170.0
        }) else {
            return false;
        };
        let r = self.city_rand() as usize;
        let lines = if self.people[j].annoyed && !self.streets.chatter.annoyed.is_empty() { &self.streets.chatter.annoyed } else { &self.streets.chatter.normal };
        let text = lines[r % lines.len()].clone();
        self.people[j].face_towards(pos);
        let face = Some(self.face(j));
        self.messages.push(Message { text, left: 4.0, face, filmed: false });
        true
    }
}
