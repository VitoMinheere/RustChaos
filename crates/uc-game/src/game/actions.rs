//! What the player does with the action key, picks up and aims at.

use crate::*;

/// What the player's gun is pointed at when it isn't a person (`find_target_new` takes
/// creatures, barrels, mines and vehicles too), and how steady it is.
#[derive(Debug, Clone)]
pub struct Aiming {
    pub creature: Option<usize>,
    pub barrel: Option<usize>,
    pub mine: Option<usize>,
    pub vehicle: Option<usize>,
    /// The player's aim (`Genus.Person->Gunaim`): what at, and how far off steady, from
    /// `MAX_GUN_AIM` (3000, three seconds' worth) down to 0.
    pub(crate) gun: (Option<Aim>, f32),
    /// Who Darci was last aiming at, to notice when that changes.
    pub(crate) last: Option<usize>,
}

impl Default for Aiming {
    fn default() -> Aiming {
        Aiming { creature: None, barrel: None, mine: None, vehicle: None, gun: (None, 3000.0), last: None }
    }
}

impl Game {
    /// The player presses action: get out of the car, into a car next to her, or talk to the
    /// person next to her. Returns whether something used the key.
    pub(crate) fn use_something(&mut self, level: &Level) -> bool {
        let Some(pi) = self.player else { return false };
        match self.people[pi].state.clone() {
            State::InCar { vehicle } => {
                let v = &mut self.vehicles[vehicle];
                if v.speed.abs() > 250.0 {
                    return true;
                }
                v.driver = None;
                let door = v.door();
                let y = level.ground.surface(&level.map, door.x, door.y, v.pos.y + 64.0);
                let p = &mut self.people[pi];
                p.fig.pos = Vec3::new(door.x, y, door.y);
                p.state = State::Move;
                p.cues.push(Sfx::CarDoor);
                true
            }
            // Off the bike (`set_person_dismount_bike`), once it's stopped.
            State::Bike { bike, phase: 1 } => {
                self.dismount(pi, bike);
                true
            }
            State::Bike { .. } => true,
            State::Move if self.mount_near(pi) => true,
            State::Move => {
                let pos = self.people[pi].fig.pos;
                // Explosives in hand: put one down at her feet with its fuse burning
                // (`SPECIAL_set_explosives`). The last one leaves her hands empty.
                let p = &mut self.people[pi];
                if p.weapon == Weapon::Grenade && p.grenades > 0 {
                    self.use_grenade(pi);
                    return true;
                }
                if p.weapon == Weapon::Explosives && p.explosives > 0 {
                    p.explosives -= 1;
                    if p.explosives == 0 {
                        p.weapon = Weapon::None;
                    }
                    self.charges.push(Charge { pos, fuse: Charge::FUSE });
                    self.messages.push(Message { text: "Five second fuse set".into(), left: 3.0, face: None, filmed: false });
                    return true;
                }
                let near = |v: &Vehicle| !v.wrecked() && v.driver.is_none()
                    && v.covers(Vec2::new(pos.x, pos.z), 140.0) && (v.pos.y - pos.y).abs() < 150.0;
                let has_key = self.people[pi].has_key;
                if let Some(vi) = self.vehicles.iter().position(|v| near(v) && v.opens_for(has_key)) {
                    self.enter_car(pi, vi, level);
                    return true;
                }
                // Only locked ones: said below, if there's nothing else to do here.
                let locked_near = self.vehicles.iter().any(near);
                // Search a prop in front of her (`set_person_search`: the nearest searchable
                // one within a square, no more than an eighth of a turn off her facing).
                if let Some(ob) = find_ob(level, pos, 256.0, Some(&self.searchable)) {
                    let o = &level.map.objects[ob];
                    let at = Vec3::new(o.x as f32, pos.y, o.z as f32);
                    let to = Vec2::new(at.x - pos.x, at.z - pos.z).normalize_or_zero();
                    let p = &mut self.people[pi];
                    if to == Vec2::ZERO || p.facing_dir().dot(to) > std::f32::consts::FRAC_1_SQRT_2 {
                        if to != Vec2::ZERO {
                            p.face_towards(at);
                        }
                        self.search = Some(Search { ob, time: 0.0 });
                        return true;
                    }
                }
                // Talk to someone friendly close by, on their feet (not lying injured: those
                // are for Roper to pick up).
                let talk = self.people.iter().enumerate().find(|(j, o)| {
                    *j != pi && o.way != 0 && !o.removed && !o.hostile() && !o.is_down()
                        && !matches!(o.state, State::Injured | State::Carried { .. })
                        && (o.fig.pos - pos).length() < 170.0
                });
                if let Some((_, o)) = talk {
                    let way = o.way;
                    if let Some(mut script) = self.script.take() {
                        script.person_used(way, self);
                        self.script = Some(script);
                    }
                    return true;
                }
                if self.chat(pi) {
                    return true;
                }
                // `set_person_enter_vehicle`'s `X_CAR_LOCKED`, an info message on the panel.
                if locked_near {
                    self.messages.push(Message { text: "The car is locked.".into(), left: 3.0, face: None, filmed: false });
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Keeps the player's target: the enemy picked stays picked while it can be shot,
    /// otherwise the nearest one (preferring those in front). `cycle` moves on to the next.
    /// Darci's gun on someone new (`fn_person_gun`, Person.cpp:18180, and
    /// `player_running_aim_gun`): `PCOM_cop_aiming_at_you` (pcom.cpp:14092) for anyone who
    /// might be a villain (not a cop, Roper, Darci or a hostage). Robots and her own
    /// bodyguards pay no heed; the guilty go for her; anyone else doing nothing much puts
    /// their hands up. Twenty seconds on, those with their hands up put them down once
    /// she's aiming at somebody else or has put the gun away (`PCOM_process_hands_up`).
    pub(crate) fn aim_at_people(&mut self) {
        let Some(pi) = self.player else { return };
        let p = &self.people[pi];
        let aiming = p.kind == ptype::DARCI && p.weapon.is_gun() && !matches!(p.state, State::InCar { .. });
        let target = self.target.filter(|_| aiming);
        if target != self.aiming.last {
            if let Some(t) = target {
                self.cop_aiming_at(t, pi);
            }
        }
        self.aiming.last = target;
        for (i, q) in self.people.iter_mut().enumerate() {
            if let State::HandsUp { by, time } = q.state {
                if time > 20.0 && (by != pi || target != Some(i)) {
                    q.hands_down();
                }
            }
        }
    }

    fn cop_aiming_at(&mut self, t: usize, cop: usize) {
        let q = &self.people[t];
        if matches!(q.kind, ptype::DARCI | ptype::ROPER | ptype::COP | ptype::HOSTAGE) || q.removed || q.is_dead() || q.is_down() {
            return;
        }
        if q.bent & person::BENT_ROBOT != 0 {
            return;
        }
        // Bodyguards ignore the person they're guarding aiming at them.
        if q.ai == person::AI_BODYGUARD && (q.other == 0 || self.by_way.get(&q.other) == Some(&cop)) {
            return;
        }
        let at = self.people[cop].fig.pos;
        let q = &mut self.people[t];
        if q.is_guilty() {
            if q.scared.is_none() {
                q.go_for();
            }
        } else if q.doing_nothing_important() {
            q.hands_up(cop, at);
        }
    }

    pub(crate) fn update_target(&mut self, cycle: bool, level: &Level) {
        let Some(pi) = self.player else { return };
        let p = &self.people[pi];
        if !p.weapon.is_gun() || matches!(p.state, State::InCar { .. }) {
            self.target = None;
            self.aiming.creature = None;
            self.aiming.barrel = None;
            self.aiming.mine = None;
            self.aiming.vehicle = None;
            return;
        }
        let (pos, f) = (p.fig.pos, p.facing_dir());
        let eye = Vec3::new(0.0, 140.0, 0.0);
        let seen = |at: Vec3, aim: Aim| {
            let d = Vec2::new(at.x - pos.x, at.z - pos.z);
            let dist = d.length();
            (dist < TARGET_RANGE && (at.y - pos.y).abs() < 4.0 * 256.0 && level.ground.line_of_sight(pos + eye, at + eye))
                .then(|| (aim, dist, 1.0 - f.dot(d / dist.max(1.0))))
        };
        // (who, how far, how far round from straight ahead), nearest first. Creatures can be
        // shot as well as people (`BAT_apply_hit` from `shoot_get_ammo_sound_and_damage`).
        // Anyone at all, weighted as `calc_target_score_new` does (guns.cpp:413): those out to
        // get her four times as keenly, gangs, Men In Black and guards twice, cops half as
        // keenly, civilians 180/256, hostages 170/256, the tarts 210/256; and nobody lying
        // down with their hands up. A higher score counts here as being that much nearer.
        // Her own bodyguards aren't to be shot (`people_allowed_to_hit_each_other`). (AI 5 is
        // `PCOM_AI_COP`.)
        let darci = matches!(p.kind, ptype::DARCI | ptype::ROPER);
        let keen = |q: &Person| -> f32 {
            let mut k = 1.0;
            if q.aggro() && q.foe == Some(pi) {
                k *= 4.0;
            } else if darci {
                if q.kind == ptype::COP || matches!(q.ai, 5 | person::AI_COP_DRIVER) {
                    k *= 0.5;
                }
                if matches!(q.kind, ptype::THUG_RASTA | ptype::THUG_GREY | ptype::THUG_RED | ptype::MIB1 | ptype::MIB2 | ptype::MIB3)
                    || matches!(q.ai, person::AI_GANG | person::AI_GUARD | 12) {
                    k *= 2.0;
                }
            } else if q.kind == ptype::COP || matches!(q.ai, 5 | person::AI_COP_DRIVER) {
                k *= 2.0;
            }
            if q.kind == ptype::CIV || q.ai == person::AI_CIV {
                k *= 180.0 / 256.0;
            } else if q.kind == ptype::HOSTAGE {
                k *= 170.0 / 256.0;
            } else if matches!(q.kind, ptype::SLAG_TART | ptype::SLAG_FATUGLY) {
                k *= 210.0 / 256.0;
            }
            k
        };
        let guard_of = |q: &Person| q.ai == person::AI_BODYGUARD && (q.other == 0 || self.by_way.get(&q.other) == Some(&pi));
        let people = self.people.iter().enumerate()
            .filter(|&(i, q)| i != pi && !q.removed && !q.is_dead() && !q.is_down() && !q.player && !guard_of(q)
                && q.fig.anim() != uc_world::character::anim::HANDS_UP_LIE && !matches!(q.state, State::InCar { .. } | State::Carried { .. }))
            .filter_map(|(i, q)| seen(q.fig.pos, Aim::Person(i)).map(|(a, d, off)| (a, d / keen(q), off)));
        let creatures = self.creatures.iter().enumerate()
            .filter(|(_, c)| c.targetable())
            .filter_map(|(k, c)| seen(c.fig.pos, Aim::Creature(k)));
        // Barrels that go up, though less keenly than people: 220/256 of the score
        // (`calc_target_score_new`, guns.cpp:512), so as if that much further away.
        let barrels = self.barrels.iter().enumerate()
            .filter(|(_, b)| b.explodes && !b.gone)
            .filter_map(|(k, b)| seen(b.pos, Aim::Barrel(k)).map(|(a, d, off)| (a, d * 256.0 / 220.0, off)));
        // Mines, as keenly as people ("nothing bad/good about mines").
        let mines = self.mines.iter().enumerate()
            .filter(|(_, m)| !m.gone)
            .filter_map(|(k, m)| seen(m.pos, Aim::Mine(k)));
        // Cars too, scored as people are, but only those the original would pick: within 8
        // squares and 2048/7 of a turn of straight ahead (`MAX_NEW_TARGET_DANGLE`, guns.cpp),
        // and not one that's already blown up or the one she's in.
        let cone = (std::f32::consts::TAU / 7.0).cos();
        let cars = self.vehicles.iter().enumerate()
            .filter(|(_, v)| !v.wrecked())
            .filter_map(|(k, v)| seen(v.pos, Aim::Vehicle(k)))
            .filter(|&(_, d, off)| d < 8.0 * 256.0 && 1.0 - off > cone);
        let mut options: Vec<(Aim, f32, f32)> = people.chain(creatures).chain(barrels).chain(mines).chain(cars).collect();
        options.sort_by(|a, b| a.1.total_cmp(&b.1));
        let current = match (self.target, self.aiming.creature, self.aiming.barrel, self.aiming.mine, self.aiming.vehicle) {
            (Some(t), ..) => Some(Aim::Person(t)),
            (None, Some(k), ..) => Some(Aim::Creature(k)),
            (None, None, Some(k), ..) => Some(Aim::Barrel(k)),
            (None, None, None, Some(k), _) => Some(Aim::Mine(k)),
            (None, None, None, None, Some(k)) => Some(Aim::Vehicle(k)),
            _ => None,
        };
        let kept = current.filter(|t| options.iter().any(|o| o.0 == *t));
        let aim = if cycle && !options.is_empty() {
            let at = kept.and_then(|t| options.iter().position(|o| o.0 == t));
            Some(options[at.map_or(0, |k| (k + 1) % options.len())].0)
        } else if kept.is_some() {
            kept
        } else {
            // The nearest, favouring those ahead (someone right behind counts as 3x as far).
            options.iter().min_by(|a, b| (a.1 * (1.0 + a.2)).total_cmp(&(b.1 * (1.0 + b.2)))).map(|o| o.0)
        };
        (self.target, self.aiming.creature, self.aiming.barrel, self.aiming.mine, self.aiming.vehicle) = match aim {
            Some(Aim::Person(t)) => (Some(t), None, None, None, None),
            Some(Aim::Creature(k)) => (None, Some(k), None, None, None),
            Some(Aim::Barrel(k)) => (None, None, Some(k), None, None),
            Some(Aim::Mine(k)) => (None, None, None, Some(k), None),
            Some(Aim::Vehicle(k)) => (None, None, None, None, Some(k)),
            None => (None, None, None, None, None),
        };
    }

    /// The search goes on while action is held (`continue_pressing_action`); let go and it's
    /// abandoned. Part way through, whatever is stashed in the prop drops out where it was
    /// put, and the prop is empty from then on, though still searchable.
    pub(crate) fn searching(&mut self, dt: f32, held: bool) {
        let Some(mut s) = self.search.take() else { return };
        let standing = self.player.is_some_and(|p| matches!(self.people[p].state, State::Move));
        if !held || !standing || self.outcome.is_some() {
            return;
        }
        let before = s.time;
        s.time += dt;
        if before < Search::FOUND && s.time >= Search::FOUND {
            let (found, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.hidden).into_iter().partition(|(ob, _)| *ob == s.ob);
            self.hidden = kept;
            if !found.is_empty() {
                let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                self.sounds.push((Sfx::Found, at));
            }
            self.items.extend(found.into_iter().map(|(_, it)| it));
        }
        if s.time < Search::DONE {
            self.search = Some(s);
        }
    }

    /// The player walks over items: health heals, weapons and ammo are picked up.
    pub(crate) fn pick_up(&mut self) {
        let Some(pi) = self.player else { return };
        let p = &self.people[pi];
        if !matches!(p.state, State::Move | State::Land { .. }) {
            return;
        }
        let pos = p.fig.pos;
        // Badges are collected within 160 units, summing the three axes (as the original does).
        let near = |it: &Item| if (BADGE..BADGE + 4).contains(&it.kind) {
            let d = (it.pos - pos).abs();
            d.x + d.y + d.z < 160.0
        } else {
            Vec2::new(it.pos.x - pos.x, it.pos.z - pos.z).length() < 80.0 && (it.pos.y - pos.y).abs() < 100.0
        };
        let Some(i) = self.items.iter().position(near) else { return };
        let (kind, way) = (self.items[i].kind, self.items[i].way);
        let p = &mut self.people[pi];
        let took = match kind {
            3 if p.health < p.max_health => {
                p.health = (p.health + 100).min(p.max_health);
                true
            }
            // A badge raises one of Darci's stats. Constitution adds health; strength makes
            // her blows land harder (see `apply`).
            k if (BADGE..BADGE + 4).contains(&k) => {
                let stat = (k - BADGE) as usize;
                self.stats[stat] += 1;
                self.badges[stat] += 1;
                self.found.insert(way);
                if stat == 3 {
                    p.max_health += 10;
                    p.health += 10;
                }
                self.messages.push(Message { text: format!("{} increased", STAT_NAMES[stat]), left: 4.0, face: None, filmed: false });
                true
            }
            // A gun: carried (with its rounds), and drawn if the hands are empty.
            2 | 4 | 6 | SNIPER => {
                let w = Weapon::from_item(kind).unwrap();
                p.give(w);
                if p.weapon == Weapon::None {
                    p.weapon = w;
                }
                true
            }
            5 | 8 if p.melee == Weapon::None => {
                let w = Weapon::from_item(kind).unwrap();
                p.give(w);
                if p.weapon == Weapon::None {
                    p.weapon = w;
                }
                true
            }
            // The things a mission sends her to fetch: a key, a keycard, files, a floppy, a
            // crowbar, a gas mask, a wrench, a video, gloves, weedkiller. They do nothing in
            // the hand; what matters is that she has them, which a mission reads with
            // `specific_item_held` (The Fallen's five clues are picked up this way).
            // A key opens the cars that ask for one (`SPECIAL_KEY`).
            1 => {
                p.has_key = true;
                true
            }
            13..=21 => true,
            // Explosives are carried, four at most, and left lying once she has four
            // (`should_person_get_item`).
            // Grenades come three at a time, and are picked up while she has fewer than eight
            // (`should_person_get_item`, `person_get_item`).
            game::grenades::IT_GRENADE if p.grenades < 8 => {
                p.grenades += 3;
                true
            }
            EXPLOSIVES if p.explosives < 4 => {
                p.explosives += 1;
                true
            }
            // Ammo: pistol, shotgun, AK-47.
            10..=12 => {
                let g = Weapon::GUNS[(kind - 10) as usize];
                p.ammo[(kind - 10) as usize] += g.clip();
                true
            }
            _ => false,
        };
        if took {
            p.cues.push(Sfx::Pickup);
            self.items.remove(i);
            if way != 0 {
                self.tally.taken.insert(way);
            }
        }
    }

    /// What the music should be doing: fight music while enemies are on the player, sprint
    /// music while sprinting, quiet otherwise.
    pub fn mood(&self) -> Mood {
        match &self.outcome {
            Some(Outcome::Won) => Mood::Won,
            Some(Outcome::Lost(_)) => Mood::Lost,
            None if self.fight_timer > 0.0 => Mood::Fight,
            None if self.sprinting => Mood::Sprint,
            None => Mood::Calm,
        }
    }
}
