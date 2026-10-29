//! Gun sights (`track_gun_sight`, fallen/Source/overlay.cpp:605, drawn by
//! `PANEL_draw_gun_sight`, DDEngine/Source/panel.cpp:728): on whatever the player's gun is
//! on, closing in as her aim steadies, and on her when someone's about to shoot her.

use crate::*;

/// `MAX_GUN_AIM` (Headers/guns.h): three seconds' worth.
const MAX_GUN_AIM: f32 = 3000.0;

/// A sight to draw: where (world units), how far off steady (0 steady to 100), and its
/// size (256 for a person).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sight {
    pub at: Vec3,
    pub accuracy: f32,
    pub scale: f32,
}

impl Game {
    /// Whose health to show over them (`OVERLAY_draw_enemy_health`, overlay.cpp:813): in a
    /// fight, the one she's squared up to; with a gun out, the one it's on. Where they stand
    /// and how much health they have left, 0 to 1, against their type's (the Men in Black's
    /// against 700, whatever they started with), and no more than all of it.
    pub fn enemy_health(&self) -> Option<(Vec3, f32)> {
        use uc_world::character::anim;
        let me = self.player()?;
        let fighting = matches!(me.state, State::Attack { .. } | State::Block { .. })
            || matches!(me.fig.anim(), anim::FIGHT | anim::FIGHT_STEP_N | anim::FIGHT_STEP_S | anim::FIGHT_STEP_E | anim::FIGHT_STEP_W | anim::KNIFE_FIGHT_READY | anim::BAT_IDLE);
        let t = if me.weapon.is_gun() && me.gun_out() {
            self.target?
        } else if fighting {
            self.people.iter().enumerate()
                .filter(|(_, q)| !q.removed && !q.player && q.hostile() && !q.is_dead() && !q.is_down())
                .map(|(i, q)| (i, (q.fig.pos - me.fig.pos).length()))
                .filter(|&(_, d)| d < 400.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))?.0
        } else {
            return None;
        };
        let q = &self.people[t];
        let full = if q.is_mib() { 700 } else { uc_world::character::person::HEALTH[q.kind.min(14) as usize] };
        Some((q.fig.pos, (q.health.max(0) as f32 / full.max(1) as f32).min(1.0)))
    }

    /// What the player's gun is on.
    fn aimed(&self) -> Option<Aim> {
        match (self.target, self.aiming.creature, self.aiming.barrel, self.aiming.mine, self.aiming.vehicle) {
            (Some(t), ..) => Some(Aim::Person(t)),
            (None, Some(k), ..) => Some(Aim::Creature(k)),
            (None, None, Some(k), ..) => Some(Aim::Barrel(k)),
            (None, None, None, Some(k), _) => Some(Aim::Mine(k)),
            (None, None, None, None, Some(k)) => Some(Aim::Vehicle(k)),
            _ => None,
        }
    }

    /// The player's aim steadying (Person.cpp:18260): back to `MAX_GUN_AIM` on a new target,
    /// then less by `calc_dist_benefit_to_gun` a tick (1324 less twice the distance up close,
    /// 500 less an eighth of it further off, in 1/256 squares; nothing past 16 squares),
    /// but never steadier than a moving target allows.
    pub(crate) fn steady_aim(&mut self, dt: f32) {
        let aim = self.aimed().filter(|_| self.player().is_some_and(|p| p.weapon.is_gun()));
        if aim != self.aiming.gun.0 {
            self.aiming.gun = (aim, MAX_GUN_AIM);
            return;
        }
        let (Some(aim), Some(me)) = (aim, self.player().map(|p| p.fig.pos)) else { return };
        let dist = (self.aim_point(aim) - me).length();
        let benefit = if dist < 512.0 { 1024.0 + 300.0 - dist * 2.0 } else { 500.0 - dist / 8.0 }.max(0.0);
        let moving = match aim {
            Aim::Person(t) => !person::idle_anim(self.people[t].fig.anim()) && matches!(self.people[t].state, State::Move),
            _ => false,
        };
        let floor = if moving { 600.0 } else { 0.0 };
        self.aiming.gun.1 = (self.aiming.gun.1 - benefit * 20.0 * dt * self.difficulty().steadying()).max(floor);
    }

    fn aim_point(&self, aim: Aim) -> Vec3 {
        match aim {
            Aim::Person(t) => self.head(t),
            Aim::Creature(k) => self.creatures[k].middle(),
            Aim::Barrel(k) => self.barrels[k].pos + Vec3::new(0.0, 80.0, 0.0),
            Aim::Mine(k) => self.mines[k].pos + Vec3::new(0.0, 30.0, 0.0),
            Aim::Vehicle(k) => self.vehicles[k].pos + Vec3::new(0.0, 64.0, 0.0),
        }
    }

    /// A person's head (sub-object 11), or near enough.
    fn head(&self, i: usize) -> Vec3 {
        let p = &self.people[i];
        p.fig.part_world(11).map_or(p.fig.pos + Vec3::new(0.0, 150.0, 0.0), |(_, at)| at)
    }

    /// The sights to draw this frame (`OVERLAY_draw_gun_sights`, overlay.cpp:693): the one
    /// on what the player's gun is on, and the one on her while anyone is about to shoot
    /// her (the nearest to firing).
    pub fn gun_sights(&self) -> Vec<Sight> {
        let mut out = Vec::new();
        if let Some(aim) = self.aiming.gun.0 {
            let scale = match aim {
                Aim::Person(_) => 256.0,
                Aim::Creature(_) | Aim::Mine(_) => 128.0,
                Aim::Barrel(_) => 200.0,
                Aim::Vehicle(_) => 400.0,
            };
            out.push(Sight { at: self.aim_point(aim), accuracy: self.aiming.gun.1 * 100.0 / MAX_GUN_AIM, scale });
        }
        if let Some(pi) = self.player {
            let soonest = self.people.iter()
                .filter(|q| q.foe == Some(pi) && !q.removed && !q.is_dead())
                .filter_map(|q| q.about_to_shoot())
                .min_by(|a, b| a.total_cmp(b));
            if let Some(accuracy) = soonest {
                out.push(Sight { at: self.head(pi), accuracy, scale: 256.0 });
            }
        }
        out
    }
}
