//! The level's moving parts: platforms, warehouses, tripwires and steam vents.

use crate::*;

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
        self.plat_clock += dt;
        while self.plat_clock >= plat::TICK {
            self.plat_clock -= plat::TICK;
            let Some(script) = self.script.take() else { break };
            for (k, t) in tops.iter().enumerate() {
                let Some((_, _, bot, top)) = *t else { continue };
                let before = self.platforms[k].pos;
                // Who is standing on it, before it goes.
                let riders: Vec<usize> = self.people.iter().enumerate()
                    .filter(|(_, p)| !p.removed && !matches!(p.state, State::InCar { .. }))
                    .filter(|(_, p)| (p.fig.pos.y - (before.y + top)).abs() < 16.0 && level.ground.over_platform(k, p.fig.pos.x, p.fig.pos.z, 0.0))
                    .map(|(i, _)| i)
                    .collect();
                let by = self.platforms[k].tick(&Route(&script));
                if by == Vec3::ZERO {
                    continue;
                }
                for &i in &riders {
                    self.people[i].fig.pos += by;
                }
                if self.platforms[k].rocket() && self.rand() & 1 == 0 {
                    self.pyro.lick(self.platforms[k].pos + Vec3::new(0.0, bot, 0.0), 70.0);
                }
                // Underneath it as it comes down: dead (`set_person_dead`).
                let under = (self.platforms[k].pos.y + bot - 0x80 as f32, self.platforms[k].pos.y + bot - 0x40 as f32);
                for i in 0..self.people.len() {
                    let p = &self.people[i];
                    if p.removed || p.is_dead() || riders.contains(&i) || !level.ground.over_platform(k, p.fig.pos.x, p.fig.pos.z, 10.0) {
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
            self.sync_platforms(level);
        }
    }

    /// Tells the ground where the platforms' tops are now.
    pub(crate) fn sync_platforms(&self, level: &Level) {
        level.ground.set_platforms(self.platforms.iter().zip(&self.plat_prims).filter_map(|(p, prim)| {
            let (lo, hi, _, top) = plat::top_box(&prim.as_ref()?.points);
            Some((Vec2::new(p.pos.x, p.pos.z), p.yaw, lo, hi, p.pos.y + top))
        }));
    }

    /// Whether the player has gone into a warehouse or come out of one. The ambience is the
    /// one it had as they came in, as `process_weather` picks it on the way in.
    pub(crate) fn go_indoors(&mut self, level: &Level) {
        let Some(p) = self.player() else { return };
        let at = p.fig.pos;
        let ware = self.wares.which_contains(&level.map, (at.x as i32) >> 8, (at.z as i32) >> 8).filter(|_| {
            // Under its roof, not on it.
            let roof = level.ground.surface(&level.map, at.x, at.z, 1.0e5);
            roof - at.y > 128.0
        });
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
