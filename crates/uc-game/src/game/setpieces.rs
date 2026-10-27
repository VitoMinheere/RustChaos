//! The level's moving parts: platforms, warehouses, tripwires and steam vents.

use crate::*;
use uc_world::ground::Wmove;

impl Game {
    /// Platforms move on the original's ticks, carrying whoever stands on them and killing
    /// whoever they come down on; then the ground learns where their tops are.
    pub(crate) fn move_platforms(&mut self, dt: f32, level: &Level) {
        if self.platforms.is_empty() {
            return;
        }
        let tops: Vec<Option<(Vec2, Vec2, f32, f32)>> = self.plat_prims.iter()
            .map(|prim| prim.as_ref().map(|prim| plat::top_box(&prim.points)))
            .collect();
        self.clocks.plat += dt;
        while self.clocks.plat >= plat::TICK {
            self.clocks.plat -= plat::TICK;
            let Some(script) = self.script.take() else { break };
            for (k, t) in tops.iter().enumerate() {
                let Some((_, _, bot, top)) = *t else { continue };
                let before = self.platforms[k].pos;
                // Who is standing on it (or holding its edge), before it goes.
                let riders: Vec<usize> = self.people.iter().enumerate()
                    .filter(|(_, p)| !p.removed && !matches!(p.state, State::InCar { .. }))
                    .filter(|(_, p)| {
                        let at = p.holding().unwrap_or(p.fig.pos);
                        (at.y - (before.y + top)).abs() < 16.0 && level.ground.over_moving(k, at.x, at.z, 0.0)
                    })
                    .map(|(i, _)| i)
                    .collect();
                let by = self.platforms[k].tick(&Route(&script));
                if by == Vec3::ZERO {
                    continue;
                }
                for &i in &riders {
                    self.people[i].carry_by(|q| q + by);
                }
                if self.platforms[k].rocket() && self.rand() & 1 == 0 {
                    self.pyro.lick(self.platforms[k].pos + Vec3::new(0.0, bot, 0.0), 70.0);
                }
                // Underneath it as it comes down: dead (`set_person_dead`).
                let under = (self.platforms[k].pos.y + bot - 0x80 as f32, self.platforms[k].pos.y + bot - 0x40 as f32);
                for i in 0..self.people.len() {
                    let p = &self.people[i];
                    if p.removed || p.is_dead() || riders.contains(&i) || !level.ground.over_moving(k, p.fig.pos.x, p.fig.pos.z, 10.0) {
                        continue;
                    }
                    let y = p.fig.pos.y;
                    if self.botanicc && matches!(p.state, State::Fall { .. }) && y >= before.y + bot && y <= before.y + top {
                        // Grim Gardens: jumping onto it puts you on it.
                        self.people[i].fig.pos.y = self.platforms[k].pos.y + top;
                    } else if by.y < 0.0 && y >= under.0 && y <= under.1 {
                        let from = self.platforms[k].pos;
                        let p = &mut self.people[i];
                        p.health = 0;
                        p.take_hit(from, 10_000, person::ko::FINISHERS, false, false, Weapon::None);
                    }
                }
            }
            self.script = Some(script);
            self.sync_wmove(level);
        }
    }

    /// Tells the ground where the moving faces are now (`WMOVE_process`): the platforms' tops,
    /// one each, then the vehicles'.
    pub(crate) fn sync_wmove(&self, level: &Level) {
        let plats = self.platforms.iter().zip(&self.plat_prims).map(|(p, prim)| {
            // (A platform without a prim still has its place, at the bottom of the world.)
            let (lo, hi, _, top) = prim.as_ref().map_or((Vec2::ZERO, Vec2::ZERO, 0.0, f32::MIN), |prim| plat::top_box(&prim.points));
            Wmove::flat(Vec2::new(p.pos.x, p.pos.z), p.yaw, lo, hi, p.pos.y + top)
        });
        let cars = (0..self.vehicles.len()).flat_map(|vi| self.vehicle_tops(vi, level));
        level.ground.set_moving(plats.chain(cars).collect::<Vec<_>>());
    }

    /// Who is standing on one of `faces` (`OnFace` a `WMOVE` face), or hanging from its edge
    /// or pulling up over it: (person, face, how far along it, feet above it).
    pub(crate) fn riders(&self, faces: &[Wmove]) -> Vec<(usize, usize, Vec2, f32)> {
        let mut out = Vec::new();
        for (i, p) in self.people.iter().enumerate() {
            if p.removed || matches!(p.state, State::InCar { .. } | State::Enter { .. }) {
                continue;
            }
            let at = p.holding().unwrap_or(p.fig.pos);
            let on = faces.iter().enumerate().find_map(|(f, face)| {
                let along = face.along(at.x, at.z)?;
                let dy = at.y - face.height(at.x, at.z, 0.0)?;
                (dy.abs() < 16.0).then_some((i, f, along, dy))
            });
            out.extend(on);
        }
        out
    }

    /// Takes riders along with the faces they're on as those move from `before` to `after`
    /// (`WMOVE_relative_pos`, wmove.cpp:726, as `Person.cpp:4239` uses it): to the same place
    /// on the face, turned as much as it has.
    pub(crate) fn carry(&mut self, riders: &[(usize, usize, Vec2, f32)], before: &[Wmove], after: &[Wmove]) {
        for &(i, f, along, dy) in riders {
            let (Some(was), Some(now)) = (before.get(f), after.get(f)) else { continue };
            // "Stop sliding up off the face."
            let dy = if dy.abs() < 2.0 { 0.0 } else { dy };
            let to = now.at(along) + Vec3::new(0.0, dy, 0.0);
            let turn = (now.angle() - was.angle() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let p = &mut self.people[i];
            if p.holding().is_some() {
                p.carry_by(|q| was.moved(now, q).unwrap_or(q));
                p.fig.angle += turn;
                continue;
            }
            // Not if it's jumped more than a square.
            if (to.x - p.fig.pos.x).abs() <= 256.0 {
                p.fig.pos.x = to.x;
            }
            if (to.z - p.fig.pos.z).abs() <= 256.0 {
                p.fig.pos.z = to.z;
            }
            p.fig.pos.y = to.y;
            p.fig.angle += turn;
        }
    }

    /// The walkable faces on top of vehicle `vi`, where it is now.
    pub(crate) fn vehicle_tops(&self, vi: usize, level: &Level) -> Vec<Wmove> {
        let v = &self.vehicles[vi];
        let (body, wheel, ..) = vehicle::TYPES[v.kind as usize];
        let Some(body) = level.item_prims.get(&body) else { return Vec::new() };
        v.tops(body, vehicle::wheel_radius(level.item_prims.get(&wheel))).collect()
    }

    /// Whether the player has gone into a warehouse or come out of one. The ambience is the
    /// one it had as they came in, as `process_weather` picks it on the way in.
    pub(crate) fn go_indoors(&mut self, level: &Level) {
        let Some(p) = self.player() else { return };
        let at = p.fig.pos;
        let ware = self.wares.under_roof(&level.map, &level.ground, at);
        self.indoors = match (ware, self.indoors) {
            (Some(w), Some((was, amb))) if w == was => Some((was, amb)),
            (Some(w), _) => Some((w, self.wares.wares[w].ambience)),
            (None, _) => None,
        };
    }

    /// While the player is inside a warehouse, its ambience (0 for none): the city goes
    /// quiet, and any ambience it has plays instead of the music.
    pub fn indoors(&self) -> Option<u8> {
        self.indoors.map(|(_, amb)| amb)
    }

    /// The tripwires watch for the player, from her lower foot to her head
    /// (`TRIP_process`, calc_sub_objects_position on SUB_OBJECT_LEFT_FOOT, RIGHT_FOOT and
    /// HEAD).
    pub(crate) fn trip_wires(&mut self, dt: f32) {
        let body = self.player().map(|p| {
            let part = |k: usize| p.fig.part_world(k).map(|(_, at)| at.y);
            let feet = part(3).zip(part(14)).map_or(p.fig.pos.y, |(l, r)| l.min(r));
            (p.fig.pos, feet, part(11).unwrap_or(p.fig.pos.y + 0xc0 as f32))
        });
        self.trips.update(dt, body);
    }

    /// The vents blow, and their steam scalds whoever stands in it: no stagger, just the
    /// health and a cry of pain, until it's too much (`PFLAG_HURTPEOPLE`). Nobody in a car is
    /// touched, nor anyone only the player can hurt (`PCOM_BENT_PLAYERKILL`).
    pub(crate) fn vent_steam(&mut self, dt: f32, level: &Level) {
        let Some(script) = &self.script else { return };
        let (vents, secs) = (script.vents(), script.time());
        let scald = self.steam.update(dt, secs, &vents, |at| level.ground.surface(&level.map, at.x, at.z, at.y + 8.0));
        if !scald || self.outcome.is_some() {
            return;
        }
        for p in &mut self.people {
            if p.removed || p.knocked_out || p.invulnerable || p.bent & 0x80 != 0 || matches!(p.state, State::InCar { .. }) {
                continue;
            }
            // From the feet to the top of the head.
            let height = 2.0 * (p.fig.hips().y - p.fig.pos.y).max(60.0);
            let hurt = self.steam.scalding(p.fig.pos, height);
            if hurt == 0 {
                continue;
            }
            p.health -= hurt;
            if p.health > 0 {
                p.cues.push(Sfx::Hurt { kind: p.kind });
            } else if p.is_down() {
                p.health = 0;
                p.knocked_out = true;
                p.cues.push(Sfx::KnockedOut { kind: p.kind });
            } else {
                let from = p.fig.pos;
                p.take_hit(from, 0, person::ko::FINISHERS, false, false, Weapon::None);
            }
        }
    }
}

/// The script as a platform's route.
struct Route<'a>(&'a Script);

impl plat::Route for Route<'_> {
    fn nearest(&self, at: Vec3, colour: u8, group: u8) -> Option<usize> {
        self.0.nearest_point(at, colour, group)
    }
    fn next(&self, after: usize, colour: u8, group: u8) -> Option<usize> {
        self.0.next_point(after, colour, group)
    }
    fn position(&self, way: usize) -> Option<Vec3> {
        self.0.point_pos(way)
    }
    fn delay(&self, way: usize, default: f32) -> f32 {
        self.0.point_delay(way, default)
    }

}

impl Game {
    /// `set_electric_fence_state` for the fence nearest `pos` (`find_electric_fence_dbuilding`,
    /// supermap.cpp:2407): the nearest facet's whole building, every facet of it.
    pub(crate) fn electrify(&mut self, level: &Level, pos: Vec3, on: bool) {
        let p = vec2(pos.x, pos.z);
        let nearest = level.map.facets.iter().skip(1)
            .map(|f| {
                let (a, b) = (vec2(f.x[0] as f32, f.z[0] as f32) * 256.0, vec2(f.x[1] as f32, f.z[1] as f32) * 256.0);
                let t = ((p - a).dot(b - a) / (b - a).length_squared().max(1.0)).clamp(0.0, 1.0);
                (f.building, (a + (b - a) * t - p).length())
            })
            // It looks through the map cells within 256 of the point, a square of 1024 apiece.
            .filter(|&(_, d)| d < 1024.0)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match nearest {
            Some((building, _)) if on => {
                self.electrified.insert(building);
            }
            Some((building, _)) => {
                self.electrified.remove(&building);
            }
            None => eprintln!("mission: no fence near {:.0},{:.0} to electrify", pos.x, pos.z),
        }
    }

    /// Electrified fences: whoever lands on one climbing it is thrown off it with sparks from
    /// their hands and feet, knocked down for 30 (Person.cpp:14232), and every 32 ticks each
    /// one sparks to the ground somewhere along it (`SPARK_process`, spark.cpp:1270).
    pub(crate) fn electric_fences(&mut self, dt: f32, level: &Level) {
        if self.electrified.is_empty() {
            return;
        }
        let live = |at: Vec2| {
            level.ground.fences.iter().zip(&level.ground.fence_buildings)
                .filter(|(w, _)| {
                    let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
                    let t = ((at - a).dot(b - a) / (b - a).length_squared().max(1.0)).clamp(0.0, 1.0);
                    (a + (b - a) * t - at).length() < 16.0
                })
                .any(|(_, b)| self.electrified.contains(b))
        };
        for i in 0..self.people.len() {
            let State::Fence { phase: 1, at, .. } = self.people[i].state else { continue };
            if !live(at) {
                continue;
            }
            let p = &mut self.people[i];
            // `SUB_OBJECT_LEFT_FOOT`, `_LEFT_HAND`, `_RIGHT_FOOT`, `_RIGHT_HAND`.
            for limb in [3, 7, 14, 10] {
                if let Some((_, from)) = p.fig.part_world(limb) {
                    let to = Vec3::new(at.x, from.y, at.y);
                    self.sparks.push(Spark { from, to, left: 1.0 });
                }
            }
            // Off the fence, and knocked down away from it.
            let h = p.fig.hips();
            p.fig.pin_root = false;
            p.fig.pos = h - Vec3::new(0.0, p.fig.stand_root().y, 0.0);
            p.state = State::Move;
            let f = p.fig.facing();
            let from = p.fig.pos + Vec3::new(f.x, 0.0, f.y) * 64.0;
            p.take_hit(from, 30, person::ko::KNOCKDOWN, false, false, Weapon::None);
            self.sounds.push((Sfx::Explosion, from));
        }
        self.clocks.fence_spark -= dt;
        if self.clocks.fence_spark > 0.0 {
            return;
        }
        self.clocks.fence_spark = 32.0 / 20.0;
        let fences: Vec<(Vec2, Vec2, f32)> = level.ground.fences.iter().zip(&level.ground.fence_buildings)
            .filter(|(_, b)| self.electrified.contains(b))
            .map(|(w, _)| (Vec2::from(w.a), Vec2::from(w.b), w.y0))
            .collect();
        for (a, b, bottom) in fences {
            let along = (self.rand() & 0xff) as f32 / 256.0;
            let at = a + (b - a) * along;
            let from = Vec3::new(at.x, bottom + (self.rand() & 0x7f) as f32 + 64.0, at.y);
            let (x, z) = (at.x + (self.rand() & 0x7f) as f32 - 64.0, at.y + (self.rand() & 0x7f) as f32 - 64.0);
            let ground = level.ground.surface(&level.map, x, z, bottom + 64.0);
            // Only where the fence stands on the ground there.
            if (bottom - ground).abs() < 64.0 {
                self.sparks.push(Spark { from, to: Vec3::new(x, ground, z), left: 50.0 / 20.0 });
            }
        }
    }

    /// The end of the world (`PYRO_GAMEOVER`, pyro.cpp:1067), round the player. Its counter
    /// climbs 3 a tick at 20 ticks a second: past 100 comes the big bang (`S_BIG_BANG_END`),
    /// and past 242 the mission is lost, the floor goes and the player is knocked down for
    /// good (500 of damage). The original lights it all with eight flickering lights round
    /// the player, going from dark to fiery orange; with no dynamic lights here (#240), the
    /// bang is the `boo` cheat's explosion instead.
    pub(crate) fn end_the_world(&mut self, dt: f32) {
        const BANG: f32 = 102.0 / 3.0 / 20.0;
        const OVER: f32 = 243.0 / 3.0 / 20.0;
        /// `S_BIG_BANG_END` in the sound list.
        const BIG_BANG: usize = 521;
        let Some(t) = self.end_of_world else { return };
        let now = t + dt;
        self.end_of_world = Some(now);
        let Some(pi) = self.player else { return };
        let at = self.people[pi].fig.pos;
        if t <= BANG && now > BANG {
            self.sounds.push((Sfx::ById { id: BIG_BANG, replace: false }, at));
            self.boo();
        }
        if t <= OVER && now > OVER {
            self.finish(crate::Outcome::Lost("Time ran out".into()));
            self.no_floor = true;
            let from = at + Vec3::new(16.0, 0.0, 16.0);
            let hit = self.hurt(pi, 500);
            self.people[pi].take_hit(from, hit, crate::person::ko::FINISHERS, false, false, crate::Weapon::None);
        }
    }

    /// With no floor, someone falling fast below the street stops moving sideways, so they
    /// can't come up under a building and pop out on top of it (Darci.cpp:1010).
    pub(crate) fn no_floor_falls(&mut self) {
        if !self.no_floor {
            return;
        }
        // `DY < -6000` a tick: in world units a second at 20 ticks.
        const FAST: f32 = -6000.0 / 256.0 * 20.0;
        for p in &mut self.people {
            if let State::Fall { vy, vel } = &mut p.state {
                if p.fig.pos.y < 0.0 && *vy < FAST {
                    *vel = Vec2::ZERO;
                }
            }
        }
    }
}
