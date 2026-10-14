//! What people's steps did to each other: hits, shots, grabs and arrests.

use crate::*;

impl Game {
    pub(crate) fn apply(&mut self, events: Vec<person::Event>, level: &Level) {
        use person::{Event, Grab};
        for e in events {
            // Whether the player has ever fired, hit or miss: the original raises
            // `GF_PLAYER_FIRED_GUN` where the shot leaves the gun, not where it lands.
            if let Event::Shot { attacker, .. } = &e {
                self.player_fired |= Some(*attacker) == self.player;
                self.gunshot(*attacker);
            }
            match e {
                Event::Hit { attacker, victim, mut damage, ko, high, low, weapon } => {
                    // The player's strength badges add to every blow, and her reflexes to every
                    // shot (`damage += Strength` / `Skill`, combat.cpp:2070).
                    if Some(attacker) == self.player {
                        damage += self.stats[1] as i32;
                    }
                    let from = self.people[attacker].fig.pos;
                    if let State::Held { .. } = self.people[victim].state {
                        // Only the grappler's finishing move lands on a held victim.
                        if ko != person::ko::GRAPPLE {
                            continue;
                        }
                        self.people[victim].state = State::Move;
                    }
                    let was_down = self.people[victim].is_down();
                    self.people[victim].take_hit(from, damage, ko, high, low, weapon);
                    if !was_down && self.people[victim].is_down() && Some(attacker) == self.player {
                        let kind = self.people[attacker].kind;
                        self.people[attacker].cues.push(Sfx::Taunt { kind });
                    }
                    self.drop_weapon(victim);
                }
                Event::Shot { attacker, victim: None, weapon } if Some(attacker) == self.player && self.aim_creature.is_some() => {
                    let k = self.aim_creature.unwrap();
                    let (from, at) = (self.people[attacker].fig.pos, self.creatures[k].middle());
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    if !level.ground.line_of_sight(from + eye, at) {
                        continue;
                    }
                    let dist = (at - from).length();
                    let chance = (0.95 - dist / (40.0 * 256.0)).clamp(0.4, 0.95);
                    let damage = match weapon {
                        Weapon::Pistol => 70,
                        Weapon::Ak47 => 100,
                        Weapon::Shotgun => (250.0 * (1.0 - dist / (8.0 * 256.0))).max(30.0) as i32,
                        _ => 0,
                    };
                    if (self.rand() % 1000) as f32 / 1000.0 < chance {
                        let mut acts = Vec::new();
                        self.creatures[k].hit(Some((from, Some(attacker))), damage, &mut acts);
                        for a in acts {
                            if let creature::Act::Sound { id, at } = a {
                                self.sounds.push((Sfx::ById { id, replace: false }, at));
                            }
                        }
                    }
                }
                // A shot at a mine sets it off (Person.cpp:6474).
                Event::Shot { attacker, victim: None, .. } if Some(attacker) == self.player && self.aim_mine.is_some() => {
                    let k = self.aim_mine.unwrap();
                    let (from, at) = (self.people[attacker].fig.pos, self.mines[k].pos + Vec3::new(0.0, 8.0, 0.0));
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    if !level.ground.line_of_sight(from + eye, at) {
                        continue;
                    }
                    let dist = (at - from).length();
                    let chance = (0.95 - dist / (40.0 * 256.0)).clamp(0.4, 0.95);
                    if (self.rand() % 1000) as f32 / 1000.0 < chance {
                        self.set_off_mine(k, level);
                    }
                }
                Event::Shot { attacker, victim: None, .. } if Some(attacker) == self.player && self.aim_barrel.is_some() => {
                    let k = self.aim_barrel.unwrap();
                    let (from, at) = (self.people[attacker].fig.pos, self.barrels[k].pos + Vec3::new(0.0, 50.0, 0.0));
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    if !level.ground.line_of_sight(from + eye, at) {
                        continue;
                    }
                    let dist = (at - from).length();
                    let chance = (0.95 - dist / (40.0 * 256.0)).clamp(0.4, 0.95);
                    if (self.rand() % 1000) as f32 / 1000.0 < chance {
                        self.shoot_barrel(k, attacker, level);
                    }
                }
                Event::Shot { attacker, victim: Some(victim), weapon } => {
                    let (a, v) = (&self.people[attacker], &self.people[victim]);
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    if !level.ground.line_of_sight(a.fig.pos + eye, v.fig.pos + eye) {
                        continue;
                    }
                    // The player's aim is good; the AI's gets better with skill and worse with
                    // distance.
                    let dist = (a.fig.pos - v.fig.pos).length();
                    let chance = if a.player {
                        (0.95 - dist / (40.0 * 256.0)).clamp(0.4, 0.95)
                    } else {
                        (0.35 + a.skill as f32 * 0.03 - dist / (24.0 * 256.0)).clamp(0.1, 0.85)
                    };
                    // Damage from `get_shot_damage` (fallen/Source/person.cpp): the shotgun falls
                    // off with distance, the AI's AK-47 is weaker, the player takes half.
                    let mut damage = match weapon {
                        Weapon::Pistol => 70,
                        Weapon::Ak47 if a.player => 100,
                        Weapon::Ak47 => 40,
                        Weapon::Shotgun => (250.0 * (1.0 - dist / (8.0 * 256.0))).max(30.0) as i32,
                        _ => 0,
                    };
                    if a.player {
                        damage += self.stats[2] as i32;
                    }
                    if v.player {
                        damage /= 2;
                    }
                    let ko = if weapon == Weapon::Shotgun && damage >= 120 { person::ko::KNOCKDOWN } else { 0 };
                    let from = a.fig.pos;
                    if (self.rand() % 1000) as f32 / 1000.0 < chance {
                        let was_down = self.people[victim].is_down();
                        self.people[victim].take_hit(from, damage, ko, true, false, weapon);
                        if !was_down && self.people[victim].is_down() && Some(attacker) == self.player {
                            let kind = self.people[attacker].kind;
                            self.people[attacker].cues.push(Sfx::Taunt { kind });
                        }
                        self.drop_weapon(victim);
                    } else {
                        // A miss hits the nearest barrel within 0xa0 of who it was meant for
                        // (Person.cpp:6508).
                        let at = self.people[victim].fig.pos;
                        let near = self.barrels.iter().enumerate()
                            .filter(|(_, b)| !b.gone && (b.pos - at).length() < 0xa0 as f32)
                            .min_by(|a, b| (a.1.pos - at).length().total_cmp(&(b.1.pos - at).length()))
                            .map(|(k, _)| k);
                        if let Some(k) = near {
                            self.shoot_barrel(k, attacker, level);
                        }
                    }
                }
                Event::Grab { attacker, victim } => {
                    let v = &mut self.people[victim];
                    v.state = State::Held { by: attacker };
                    v.fig.restart(Grab::Grab.anims().1, uc_world::Play::Once);
                }
                Event::GrabMove { victim, grab } => {
                    self.people[victim].fig.restart(grab.anims().1, uc_world::Play::Once);
                }
                Event::Release { victim } => {
                    let from = self.people[victim].fig.pos;
                    let v = &mut self.people[victim];
                    if matches!(v.state, State::Held { .. }) {
                        v.state = State::Move;
                        v.take_hit(from, 0, 0, false, false, Weapon::None);
                    }
                }
                Event::Carry { carrier, victim } => {
                    self.people[victim].state = State::Carried { by: carrier };
                }
                Event::Arrest { attacker, victim } => {
                    let at = self.people[attacker].fig.pos;
                    let v = &mut self.people[victim];
                    v.state = State::Cuffed;
                    v.face_towards(at);
                    v.fig.restart(uc_world::character::anim::ARREST_BE_CUFFED, uc_world::Play::Once);
                }
                Event::Arrested { victim } => {
                    self.people[victim].arrested = true;
                    self.drop_weapon(victim);
                }
                Event::Punch { attacker } if Some(attacker) == self.player => {
                    if let Some(s) = &mut self.script {
                        s.note_punch();
                    }
                }
                Event::Kick { attacker } if Some(attacker) == self.player => {
                    if let Some(s) = &mut self.script {
                        s.note_kick();
                    }
                }
                _ => {}
            }
        }
    }

    /// Someone knocked out or arrested drops their weapon.
    pub(crate) fn drop_weapon(&mut self, i: usize) {
        let p = &mut self.people[i];
        if p.player || !(p.knocked_out || p.arrested) || p.weapon == Weapon::None {
            return;
        }
        if let Some(kind) = p.weapon.item() {
            let f = p.facing_dir();
            let pos = p.fig.pos + Vec3::new(f.x, 0.0, f.y) * 60.0;
            p.weapon = Weapon::None;
            self.items.push(Item { way: 0, kind, pos });
        }
    }

    /// A grappled victim stands in front of the attacker, facing him (`set_grapple_pos`,
    /// fallen/Source/combat.cpp: 90 units in front, turned 180 degrees).
    pub(crate) fn sync_grapples(&mut self) {
        for i in 0..self.people.len() {
            let State::Grapple { victim, .. } = self.people[i].state else {
                if let State::Held { by } = self.people[i].state {
                    if !matches!(self.people[by].state, State::Grapple { victim, .. } if victim == i) {
                        self.people[i].state = State::Move;
                    }
                }
                continue;
            };
            let (pos, angle, f) = (self.people[i].fig.pos, self.people[i].fig.angle, self.people[i].facing_dir());
            let v = &mut self.people[victim];
            if matches!(v.state, State::Held { .. }) {
                v.fig.pos = pos + Vec3::new(f.x, 0.0, f.y) * 90.0;
                v.fig.angle = angle + std::f32::consts::PI;
            }
        }
    }

    /// Pushes overlapping standing people apart.
    /// Whoever has the injured AI lies down injured (`PCOM_AI_HYPOCHONDRIA` puts them straight
    /// into `set_person_injured`). Someone being carried goes where the one carrying them puts
    /// them, animated to match; once they're let go of, put down or dropped because the one
    /// carrying them was hit (`emergency_uncarry`), they lie injured again where they are.
    pub(crate) fn sync_carrying(&mut self, level: &Level) {
        for p in &mut self.people {
            if p.ai == person::AI_INJURED && matches!(p.state, State::Move) && !p.player {
                p.state = State::Injured;
                p.fig.restart(uc_world::character::anim::INJURED_LOOP, uc_world::Play::Loop);
            }
        }
        for i in 0..self.people.len() {
            let State::Carried { by } = self.people[i].state else { continue };
            match self.people[by].carried_pose() {
                Some((victim, at, angle, a, play)) if victim == i => {
                    let v = &mut self.people[i];
                    v.fig.pos = at;
                    v.fig.angle = angle;
                    v.fig.play(a, play);
                }
                _ => {
                    let v = &mut self.people[i];
                    v.state = State::Injured;
                    v.fig.pos.y = level.ground.surface(&level.map, v.fig.pos.x, v.fig.pos.z, v.fig.pos.y + 64.0);
                    v.fig.restart(uc_world::character::anim::INJURED_LOOP, uc_world::Play::Loop);
                }
            }
        }
    }

    pub(crate) fn separate(&mut self, level: &Level) {
        const MIN: f32 = 64.0;
        let n = self.people.len();
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = (&self.people[i], &self.people[j]);
                let solid = |p: &Person| !p.removed && matches!(p.state, State::Move | State::Attack { .. } | State::Hurt | State::Land { .. });
                if !solid(a) || !solid(b) || (a.fig.pos.y - b.fig.pos.y).abs() > 100.0 {
                    continue;
                }
                let d = vec2(b.fig.pos.x - a.fig.pos.x, b.fig.pos.z - a.fig.pos.z);
                let len = d.length();
                if len >= MIN || len < 1e-3 {
                    continue;
                }
                let push = d / len * (MIN - len) * 0.5;
                for (k, s) in [(i, -1.0), (j, 1.0)] {
                    let p = &mut self.people[k].fig.pos;
                    let to = level.ground.slide(vec2(p.x, p.z), p.y, push * s, 36.0);
                    // Never shove anyone off an edge.
                    if (level.ground.surface(&level.map, to.x, to.y, p.y) - p.y).abs() <= uc_world::ground::STEP_HEIGHT {
                        p.x = to.x;
                        p.z = to.y;
                    }
                }
            }
        }
    }
}
