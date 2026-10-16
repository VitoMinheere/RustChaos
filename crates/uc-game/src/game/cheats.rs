//! The developer console's commands and the original's cheats (#106): teleporting, ending
//! the mission, playing as someone else, and the full-kit button combination
//! (`parse_console`, Controls.cpp:238; interfac.cpp:6993).

use crate::*;

impl Game {
    /// Puts the player on foot at `at` (world units), on whatever is there to stand on from
    /// a little above it. Not while driving or riding. Whether it did.
    pub fn teleport(&mut self, at: Vec3, level: &Level) -> bool {
        let Some(pi) = self.player else { return false };
        if matches!(self.people[pi].state, State::InCar { .. } | State::Bike { .. } | State::Enter { .. }) {
            return false;
        }
        let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
        let p = &mut self.people[pi];
        p.fig.pos = Vec3::new(at.x, y, at.z);
        if !matches!(p.state, State::Move) {
            p.stand_still();
        }
        p.fig.lift = 0.0;
        self.search = None;
        true
    }

    /// Where event point `n` is (world units), for `telw`.
    pub fn point_pos(level: &Level, n: usize) -> Option<Vec3> {
        let e = level.mission.event_points.iter().find(|e| e.index == n)?;
        Some(Vec3::new(e.x as f32, e.y as f32, e.z as f32))
    }

    /// The event point nearest the player, and how far it is (world units), for `wpt`.
    pub fn nearest_point(&self, level: &Level) -> Option<(usize, f32)> {
        let at = self.player()?.fig.pos;
        level.mission.event_points.iter()
            .map(|e| (e.index, (Vec3::new(e.x as f32, e.y as f32, e.z as f32) - at).length()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// `win` and `lose`: ends the mission now.
    pub fn end_mission(&mut self, won: bool) {
        self.finish(if won { Outcome::Won } else { Outcome::Lost("Lost by the console".into()) });
    }

    /// `roper` and `darci`: the player becomes a person of `PERSON_*` type `kind`, with that
    /// person's body, where they stand (the console's swap of type, anims and mesh). Roper
    /// only where the mission has loaded his body. Whether it could.
    pub fn play_as(&mut self, kind: u8, level: &Level) -> bool {
        let Some(pi) = self.player else { return false };
        let Some(people) = level.people.as_ref() else { return false };
        if kind == ptype::ROPER && people.roper.is_none() {
            return false;
        }
        let p = &self.people[pi];
        let Some(fig) = people.figure(kind, 0, p.fig.pos, p.fig.angle) else { return false };
        let p = &mut self.people[pi];
        p.fig = fig;
        p.kind = kind;
        p.stand_still();
        self.player_name = player_name(&level.mission.mission_name, kind);
        true
    }

    /// punch, kick, action, jump and right held together (interfac.cpp:6993): full health,
    /// 240 rounds for every gun, and an AK-47, a shotgun and a grenade at her feet.
    pub fn full_kit(&mut self, level: &Level) {
        let Some(pi) = self.player else { return };
        let p = &mut self.people[pi];
        p.health = p.max_health;
        p.ammo = [240; 4];
        let (at, f) = (p.fig.pos, p.facing_dir());
        let side = Vec2::new(-f.y, f.x);
        for (n, kind) in [6, 4, game::grenades::IT_GRENADE].into_iter().enumerate() {
            let d = f * 80.0 + side * (n as f32 - 1.0) * 60.0;
            let (x, z) = (at.x + d.x, at.z + d.y);
            let y = level.ground.surface(&level.map, x, z, at.y + 64.0);
            self.items.push(Item { way: 0, kind, pos: Vec3::new(x, y, z) });
        }
    }

    /// `boo`: the game-over explosion (`PYRO_GAMEOVER`) where the player stands, to look at.
    pub fn boo(&mut self) {
        if let Some(at) = self.player().map(|p| p.fig.pos) {
            self.pyro.burst(at, pyro::FLASH | pyro::DUST | pyro::STREAMERS | pyro::FIREBOMB, 512.0);
            self.sounds.push((Sfx::Explosion, at));
        }
    }
}
