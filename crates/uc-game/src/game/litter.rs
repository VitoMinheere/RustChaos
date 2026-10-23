//! The leaves, cans and snow lying about: kept round the player and blown about by whoever
//! moves through them (`DIRT_set_focus`, `DIRT_gust`, `DIRT_process`).

use crate::*;

/// `PRIM_FLAG_TREE`.
const PRIM_FLAG_TREE: u8 = 1 << 4;

impl Game {
    /// `DIRT_init(100, 1, 0, ...)` (elev.cpp:2356): a can to every hundred leaves, with the
    /// map's trees (`PRIM_FLAG_TREE` props) shedding their own.
    pub(crate) fn new_dirt(level: &Level) -> dirt::Dirt {
        let mut tree: HashMap<u8, bool> = HashMap::new();
        let trees: Vec<(i32, i32)> = level.map.objects.iter()
            .filter(|o| *tree.entry(o.prim).or_insert_with(|| {
                uc_world::level::load_prim(&level.game, o.prim).is_some_and(|p| p.flag & PRIM_FLAG_TREE != 0)
            }))
            .map(|o| (o.x, o.z))
            .collect();
        dirt::Dirt::new(dirt::Ground::of(level.map.texture_set), 100, 1, trees)
    }

    /// The dirt's turns (20 a second): the focus follows the player, everyone and every car
    /// that moved since the last turn blows the dirt about (`move_thing`, collide.cpp:7082;
    /// vehicle.cpp:3352), and it all moves on. Kicked and clattering cans are heard.
    pub(crate) fn blow_dirt(&mut self, dt: f32, level: &Level) {
        // No dirt if there is no floor (`GF_NO_FLOOR`).
        if self.no_floor {
            return;
        }
        // Every frame, so a can in hand keeps up with it.
        self.hold_cans();
        self.clocks.dirt -= dt;
        while self.clocks.dirt <= 0.0 {
            self.clocks.dirt += 1.0 / 20.0;
            if let Some(p) = self.player() {
                let at = p.fig.pos;
                self.dirt.set_focus(at.x as i32, at.z as i32, dirt::FOCUS_RADIUS, at.y as i32, &level.map);
            }
            let movers: Vec<Vec3> = self.people.iter()
                .map(|p| if p.removed || matches!(p.state, State::InCar { .. }) { Vec3::NAN } else { p.fig.pos })
                .chain(self.vehicles.iter().map(|v| v.pos))
                .collect();
            // A can someone kicks is heard by the guards about (`DIRT_gust`, dirt.cpp:2194).
            let mut kicked = Vec::new();
            for (i, &now) in movers.iter().enumerate() {
                let Some(&was) = self.dirt_marks.get(i) else { continue };
                if now.is_nan() || was.is_nan() || (now.x as i32, now.z as i32) == (was.x as i32, was.z as i32) {
                    continue;
                }
                let by = (i < self.people.len()).then_some(i);
                kicked.extend(self.dirt.gust(was.x as i32, was.z as i32, now.x as i32, now.z as i32)
                    .into_iter().map(|at| dirt::Clatter { at, alerts: true, by }));
            }
            self.dirt_marks = movers;
            kicked.extend(self.dirt.process(&level.map));
            for c in kicked {
                let at = Vec3::new(c.at.0 as f32, c.at.1 as f32, c.at.2 as f32);
                self.sounds.push((Sfx::ById { id: dirt::KICK_CAN, replace: false }, at));
                if c.alerts {
                    self.unusual(at, c.by);
                }
            }
        }
    }

    /// A can in someone's hand stays in their right hand (`DIRT_TYPE_HELDCAN`, dirt.cpp:1819),
    /// till they're doing something a can can't be held through (knocked down, climbing, in a
    /// car), when it drops. And whether the player has a can in reach to pick up.
    fn hold_cans(&mut self) {
        let unit = 2048.0 / std::f32::consts::TAU;
        for i in 0..self.people.len() {
            let p = &mut self.people[i];
            if p.player {
                let at = p.fig.pos;
                p.can_near = self.dirt.nearest_can(at.x as i32, at.y as i32, at.z as i32).is_some_and(|(_, d)| d < 0x80);
            }
            let Some(can) = p.can else { continue };
            let angle = (p.fig.angle * unit) as i32;
            let holding = !p.removed && matches!(p.state, State::Move | State::PickUp { .. } | State::Throw { .. } | State::Crouch { .. } | State::Hurt);
            if !holding {
                self.dirt.release(can, angle, 0);
                p.can = None;
                continue;
            }
            let hand = p.fig.part_world(10).map_or(p.fig.pos + Vec3::new(0.0, 100.0, 0.0), |(_, at)| at);
            self.dirt.hold(can, hand.x as i32, hand.y as i32, hand.z as i32, angle);
        }
    }

    /// Something out of the ordinary heard at `at` (`PCOM_SOUND_UNUSUAL`, 0x600 off,
    /// pcom.cpp:13124), made by `by`: a can kicked or landing after it's thrown. Anyone about
    /// but whoever made it turns to look.
    pub(crate) fn unusual(&mut self, at: Vec3, by: Option<usize>) {
        for (j, p) in self.people.iter_mut().enumerate() {
            if Some(j) != by && (p.fig.pos - at).length() < 0x600 as f32 {
                p.hear(at);
            }
        }
    }

    /// The pickup's reached the can: the nearest one in reach goes into `who`'s hand
    /// (`DIRT_pick_up_can_or_head`).
    pub(crate) fn pick_up_can(&mut self, who: usize) {
        let p = &mut self.people[who];
        let at = p.fig.pos;
        if p.can.is_none() {
            p.can = self.dirt.pick_up(who, at.x as i32, at.y as i32, at.z as i32);
            if p.can.is_some() {
                p.cues.push(Sfx::Pickup);
            }
        }
    }

    /// The throw's let go: the can in `who`'s hand flies off the way she faces, at the
    /// original's power of 128 (`DIRT_release_can_or_head`). Returns whether there was one.
    pub(crate) fn throw_can(&mut self, who: usize) -> bool {
        let p = &mut self.people[who];
        let Some(can) = p.can.take() else { return false };
        let angle = (p.fig.angle * 2048.0 / std::f32::consts::TAU) as i32;
        self.dirt.release(can, angle, 128);
        true
    }
}
