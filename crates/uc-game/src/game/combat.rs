//! What people's steps did to each other: hits, shots, grabs and arrests.

use crate::*;

impl Game {
    pub(crate) fn apply(&mut self, events: Vec<person::Event>, level: &Level) {
        use person::{Event, Grab};
        for e in events {
            // Whether the player has ever fired, hit or miss: the original raises
            // `GF_PLAYER_FIRED_GUN` where the shot leaves the gun, not where it lands.
            if let Event::Shot { attacker, .. } = &e {
                self.tally.player_fired |= Some(*attacker) == self.player;
                if Some(*attacker) == self.player {
                    self.record.shots += 1;
                }
                self.gunshot(*attacker);
                self.muzzle_fx(*attacker);
            }
            match e {
                // People in the same gang like each other: their blows and shots don't land
                // on each other (`people_allowed_to_hit_each_other`, combat.cpp:2700).
                Event::Hit { attacker, victim, .. } | Event::Shot { attacker, victim: Some(victim), .. } if self.same_gang(attacker, victim) => {}
                Event::Hit { attacker, victim, mut damage, ko, high, low, weapon } => {
                    // The player's strength badges add to every blow, and her reflexes to every
                    // shot (`damage += Strength` / `Skill`, combat.cpp:2070).
                    if Some(attacker) == self.player {
                        self.bored = 0.0;
                        damage += self.stats[1] as i32;
                    }
                    let from = self.people[attacker].fig.pos;
                    // Blows never land on the Men in Black: they jump clear.
                    if !weapon.is_gun() && self.people[victim].dodge() {
                        continue;
                    }
                    if let State::Held { .. } = self.people[victim].state {
                        // Only the grappler's finishing move lands on a held victim.
                        if ko != person::ko::GRAPPLE {
                            continue;
                        }
                        self.people[victim].state = State::Move;
                    }
                    let was_down = self.people[victim].is_down();
                    if self.people[attacker].fig.anim() == uc_world::character::anim::FIGHT_STOMP {
                        self.people[victim].hurt_by = Some(attacker);
                        let damage = self.hurt(victim, damage);
                        self.people[victim].take_stomp(damage);
                        let at = self.people[victim].body_pos();
                        self.gunfx.bleed(at, 1);
                        continue;
                    }
                    // Between the legs (combat.cpp:1894): 50 more for a man, 10 for a woman.
                    let nad = self.people[attacker].fig.anim() == uc_world::character::anim::KICK_NAD;
                    let male = self.people[victim].male();
                    let damage = damage + if !nad { 0 } else if male { 50 } else { 10 };
                    self.people[victim].hurt_by = Some(attacker);
                    let damage = self.hurt(victim, damage);
                    let before = self.people[victim].health;
                    self.people[victim].take_hit(from, damage, ko, high, low, weapon);
                    // A blow that lands draws blood: a drop, or five from a knife.
                    if self.people[victim].health < before {
                        let at = self.people[victim].fig.part_world(4).map_or(self.people[victim].fig.pos, |(_, at)| at);
                        self.gunfx.bleed(at, if weapon == Weapon::Knife { 5 } else { 1 });
                    }
                    // He doubles over (`ANIM_KICK_NAD_TAKE`), and she crows about it more
                    // often than not (`taunt_prob` 75).
                    let v = &mut self.people[victim];
                    if nad && matches!(v.state, State::Hurt) && v.fig.restart(uc_world::character::anim::KICK_NAD_TAKE, uc_world::Play::Once) {
                        v.fig.set_speed(1.0);
                        if male && (self.rand() & 0xff) < 75 {
                            let kind = self.people[attacker].kind;
                            self.people[attacker].cues.push(Sfx::Taunt { kind });
                        }
                    }
                    if !was_down && self.people[victim].is_down() {
                        self.count_knockout(attacker, weapon);
                    }
                    if !was_down && self.people[victim].is_down() && Some(attacker) == self.player {
                        let kind = self.people[attacker].kind;
                        self.people[attacker].cues.push(Sfx::Taunt { kind });
                    }
                }
                Event::Shot { attacker, victim: None, weapon } if Some(attacker) == self.player && self.aiming.creature.is_some() => {
                    let k = self.aiming.creature.unwrap();
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
                        Weapon::Sniper => SNIPER_DAMAGE,
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
                Event::Shot { attacker, victim: None, .. } if Some(attacker) == self.player && self.aiming.mine.is_some() => {
                    let k = self.aiming.mine.unwrap();
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
                Event::Shot { attacker, victim: None, weapon } if Some(attacker) == self.player && self.aiming.vehicle.is_some() => {
                    let k = self.aiming.vehicle.unwrap();
                    let (from, at) = (self.people[attacker].fig.pos, self.vehicles[k].pos + Vec3::new(0.0, 64.0, 0.0));
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
                        self.gunfx.spang(at);
                        self.vehicles[k].reduce_health(damage);
                        self.vehicles[k].hurt_by = Some(attacker);
                    }
                }
                Event::Shot { attacker, victim: None, .. } if Some(attacker) == self.player && self.aiming.barrel.is_some() => {
                    let k = self.aiming.barrel.unwrap();
                    let (from, at) = (self.people[attacker].fig.pos, self.barrels[k].pos + Vec3::new(0.0, 50.0, 0.0));
                    let eye = Vec3::new(0.0, 140.0, 0.0);
                    if !level.ground.line_of_sight(from + eye, at) {
                        continue;
                    }
                    let dist = (at - from).length();
                    let chance = (0.95 - dist / (40.0 * 256.0)).clamp(0.4, 0.95);
                    if (self.rand() % 1000) as f32 / 1000.0 < chance {
                        self.gunfx.spang(at);
                        self.shoot_barrel(k, attacker, level);
                    }
                }
                Event::Shot { attacker, victim: Some(victim), weapon } => {
                    // Firing at a target (Person.cpp:6219).
                    if Some(attacker) == self.player {
                        self.bored = 0.0;
                    }
                    // Darci firing at someone with their hands up: "Get Down." and down they
                    // go, unhurt, for her to cuff (Person.cpp:6828). She looks for someone else.
                    if self.people[attacker].kind == ptype::DARCI && matches!(self.people[victim].state, State::HandsUp { .. }) {
                        self.people[victim].get_down();
                        let face = Some(self.face(attacker));
                        self.messages.push(Message { text: "Get Down.".into(), left: 8.0, face, filmed: false });
                        if Some(attacker) == self.player {
                            self.target = None;
                        }
                        continue;
                    }
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
                        // `SNIPE_shoot`: `HIT_TYPE_GUN_SHOT_M` for 260.
                        Weapon::Sniper => SNIPER_DAMAGE,
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
                        // Skilled people (not players) can dodge a bullet with a flip
                        // (Person.cpp:6340): skill 7 or more, less 5 (10 for a shotgun's spread),
                        // out of 32. The original adds 5 when the *shooter* is a Man in Black,
                        // likely meant for the one shot at; that's kept as it shipped.
                        let v = &self.people[victim];
                        if !v.player && v.health > 0 && !v.is_down() && matches!(v.state, State::Move) {
                            let mib = self.people[attacker].is_mib();
                            let skill = v.skill as i32 + if mib { 5 } else { 0 };
                            if skill >= 7 {
                                let skill = skill - 5 - if weapon == Weapon::Shotgun { 5 } else { 0 };
                                if ((self.rand() & 0x1f) as i32) < skill {
                                    let left = self.rand() & 1 == 0;
                                    // `PCOM_attack_happened`: a fighter goes for them, anyone
                                    // else runs.
                                    let from = self.people[attacker].fig.pos;
                                    let v = &mut self.people[victim];
                                    v.hurt_by = Some(attacker);
                                    if v.fights() { v.go_for() } else { v.scare(Some(attacker), from) }
                                    v.flip(left);
                                    continue;
                                }
                            }
                        }
                        let was_down = self.people[victim].is_down();
                        self.people[victim].hurt_by = Some(attacker);
                        let damage = self.hurt(victim, damage);
                        let behind = self.people[victim].hit_from_behind(from);
                        self.people[victim].take_hit(from, damage, ko, true, false, weapon);
                        // Shot dead on the street, not on a roof (`person_on_floor`,
                        // `OnFace == 0`), they die as the shot had it (combat.cpp:2433): the
                        // pavement counts, anything a storey up doesn't. Which of the three
                        // from behind goes by who and when, not the game's chance.
                        let v = &self.people[victim];
                        let street = (v.fig.pos.y - level.map.ground_height(v.fig.pos.x, v.fig.pos.z)).abs() < 96.0;
                        if !was_down && v.is_dead() && street {
                            let pick = victim + (self.time * 20.0) as usize;
                            self.people[victim].shot_dead(behind, pick);
                        }
                        // A spark where it lands, and two drops of blood from the head.
                        let v = &self.people[victim];
                        let head = v.fig.part_world(11).map_or(v.fig.pos + Vec3::new(0.0, 150.0, 0.0), |(_, at)| at);
                        let chest = v.fig.part_world(4).map_or(head, |(_, at)| at);
                        self.gunfx.spang(chest);
                        self.gunfx.bleed(head, 2);
                        if Some(attacker) == self.player {
                            self.record.hits += 1;
                        }
                        if !was_down && self.people[victim].is_down() {
                            self.count_knockout(attacker, weapon);
                        }
                        if !was_down && self.people[victim].is_down() && Some(attacker) == self.player {
                            let kind = self.people[attacker].kind;
                            self.people[attacker].cues.push(Sfx::Taunt { kind });
                        }
                    } else {
                        // A miss hits the nearest barrel within 0xa0 of who it was meant for
                        // (Person.cpp:6508), or the ground beside them (Person.cpp:6587).
                        let at = self.people[victim].fig.pos;
                        self.gunfx.spang_beside(at, |x, z| level.ground.surface(&level.map, x, z, at.y + 64.0));
                        let near = self.barrels.iter().enumerate()
                            .filter(|(_, b)| !b.gone && (b.pos - at).length() < 0xa0 as f32)
                            .min_by(|a, b| (a.1.pos - at).length().total_cmp(&(b.1.pos - at).length()))
                            .map(|(k, _)| k);
                        if let Some(k) = near {
                            self.shoot_barrel(k, attacker, level);
                        }
                    }
                }
                Event::Grab { attacker, victim, grab } => {
                    let v = &mut self.people[victim];
                    v.state = State::Held { by: attacker };
                    v.taunt = None;
                    v.fig.restart(grab.anims().1, uc_world::Play::Once);
                    // Roper's grapples come with one of his taunts, a fifth of the time
                    // (`taunt_prob` 50 of 256, `set_grapple`).
                    if matches!(grab, Grab::Strangle | Grab::Headbutt) && (self.rand() & 0xff) < 50 {
                        let kind = self.people[attacker].kind;
                        self.people[attacker].cues.push(Sfx::Taunt { kind });
                    }
                }
                Event::Throw { thrower } => {
                    if !self.throw_can(thrower) {
                        self.release_grenade(thrower);
                    }
                }
                Event::PickUp { who } => self.pick_up_can(who),
                Event::Hook { thrower } => self.release_hook(thrower),
                Event::OutCold { attacker, victim } => {
                    if Some(attacker) == self.player {
                        self.record.takedowns += 1;
                    }
                    let from = self.people[attacker].fig.pos;
                    let v = &mut self.people[victim];
                    if matches!(v.state, State::Held { .. }) {
                        v.state = State::Move;
                    }
                    v.take_hit(from, 0, person::ko::GRAPPLE, false, false, Weapon::None);
                    if v.is_down() {
                        v.out_cold = true;
                        v.cues.push(Sfx::BatHit);
                    }
                }
                Event::Squeeze { victim, headbutt } => {
                    // `40 - GET_SKILL(p_target) >> 3` plus one a strangling frame, which C
                    // reads as (40 - skill) >> 3; 40 - skill for the headbutt, with a thud.
                    let v = &mut self.people[victim];
                    if !v.invulnerable {
                        let skill = v.skill as i32;
                        v.health -= if headbutt { 40 - skill } else { ((40 - skill) >> 3) + 1 };
                    }
                    if headbutt {
                        v.cues.push(Sfx::BatHit);
                    }
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
                    if Some(attacker) == self.player {
                        self.record.arrests += 1;
                    }
                    // "Hey!" (Person.cpp:9422).
                    self.noises.push((person::Noise::Hey, self.people[attacker].fig.pos, Some(attacker)));
                    let at = self.people[attacker].fig.pos;
                    let v = &mut self.people[victim];
                    v.state = State::Cuffed;
                    v.face_towards(at);
                    v.fig.restart(uc_world::character::anim::ARREST_BE_CUFFED, uc_world::Play::Once);
                }
                Event::Arrested { victim } => {
                    self.people[victim].arrested = true;
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

    /// A grappled victim stands in front of the attacker, facing him (`set_grapple_pos`,
    /// fallen/Source/combat.cpp: 90 units in front, 100 for Roper's, turned 180 degrees).
    pub(crate) fn sync_grapples(&mut self) {
        for i in 0..self.people.len() {
            let State::Grapple { victim, grab, .. } = self.people[i].state else {
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
                v.fig.pos = pos + Vec3::new(f.x, 0.0, f.y) * grab.reach();
                v.fig.angle = angle + if grab.face_to_face() { std::f32::consts::PI } else { 0.0 };
            }
        }
    }

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

    /// Pushes overlapping standing people apart (which counts towards their `SlideOdd`, as
    /// `collide_against_things` does).
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
                    self.people[k].slid_odd = Some(true);
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

    /// Whether `a` and `b` are in the same gang (`PCOM_BENT_GANG`, and the same colour).
    fn same_gang(&self, a: usize, b: usize) -> bool {
        let (a, b) = (&self.people[a], &self.people[b]);
        a.bent & b.bent & person::BENT_GANG != 0 && a.colour == b.colour
    }

    /// A gun going off in person `i`'s hand: sparks from the muzzle and casings out of the
    /// hand, three at a time from an AK-47 or a Man in Black (Person.cpp:6261).
    fn muzzle_fx(&mut self, i: usize) {
        let p = &self.people[i];
        let hand = p.fig.part_world(7).map_or(p.fig.pos + Vec3::new(0.0, 110.0, 0.0), |(_, at)| at);
        let dir = p.facing_dir();
        let muzzle = hand + Vec3::new(dir.x, 0.0, dir.y) * 40.0;
        let casings = if p.weapon == Weapon::Ak47 || p.is_mib() { 3 } else { 1 };
        self.gunfx.fire(muzzle, hand, dir, casings);
    }
}
