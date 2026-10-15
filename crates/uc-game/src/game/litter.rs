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
        self.dirt_clock -= dt;
        while self.dirt_clock <= 0.0 {
            self.dirt_clock += 1.0 / 20.0;
            if let Some(p) = self.player() {
                let at = p.fig.pos;
                self.dirt.set_focus(at.x as i32, at.z as i32, dirt::FOCUS_RADIUS, at.y as i32, &level.map);
            }
            let movers: Vec<Vec3> = self.people.iter()
                .map(|p| if p.removed || matches!(p.state, State::InCar { .. }) { Vec3::NAN } else { p.fig.pos })
                .chain(self.vehicles.iter().map(|v| v.pos))
                .collect();
            let mut kicked = Vec::new();
            for (i, &now) in movers.iter().enumerate() {
                let Some(&was) = self.dirt_marks.get(i) else { continue };
                if now.is_nan() || was.is_nan() || (now.x as i32, now.z as i32) == (was.x as i32, was.z as i32) {
                    continue;
                }
                kicked.extend(self.dirt.gust(was.x as i32, was.z as i32, now.x as i32, now.z as i32));
            }
            self.dirt_marks = movers;
            kicked.extend(self.dirt.process(&level.map));
            for (x, y, z) in kicked {
                self.sounds.push((Sfx::ById { id: dirt::KICK_CAN, replace: false }, Vec3::new(x as f32, y as f32, z as f32)));
            }
        }
    }
}
