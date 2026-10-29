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
    /// Whether player `pi` has someone to arrest within reach (`find_arrestee`, as their
    /// step looks): arresting comes before everything else the action button does
    /// (interfac.cpp:1407), talking to a bodyguard stood over the one she's knocked down
    /// included. Nobody arrests their own bodyguards.
    fn arrestee_near(&self, pi: usize) -> bool {
        let p = &self.people[pi];
        if !p.can_arrest() {
            return false;
        }
        let own_guard = |o: &Person| o.ai == person::AI_BODYGUARD
            && (if o.other == 0 { self.player } else { self.by_way.get(&o.other).copied() }) == Some(pi);
        self.people.iter().enumerate().any(|(j, o)| {
            let s = o.snap();
            let d = Vec2::new(s.pos.x - p.fig.pos.x, s.pos.z - p.fig.pos.z).length();
            j != pi && s.present && s.arrestable && !s.player && !own_guard(o)
                && d < person::ARREST_RANGE && (s.pos.y - p.fig.pos.y).abs() < 64.0
        })
    }

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
            // Someone down to arrest: the button's for that (the player's own step does it).
            State::Move if self.arrestee_near(pi) => false,
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
                        self.search = Some(Search { what: Searched::Prop(ob), time: 0.0 });
                        return true;
                    }
                }
                // Search someone at her feet (`find_searchable_person`): dead, out cold or
                // arrested, not searched yet, with their pelvis within 64 of her.
                let flat = |h: Vec3| Vec2::new(h.x - pos.x, h.z - pos.z).length();
                let body = self.people.iter().enumerate()
                    .filter(|&(j, o)| j != pi && !o.player && !o.removed && !o.searched
                        && (o.is_dead() || o.knocked_out || o.arrested) && !matches!(o.state, State::Carried { .. }))
                    .map(|(j, o)| (j, o.fig.hips()))
                    .filter(|&(_, h)| flat(h) < 64.0 && (h.y - pos.y).abs() < 128.0)
                    .min_by(|a, b| flat(a.1).total_cmp(&flat(b.1)));
                if let Some((j, _)) = body {
                    self.search = Some(Search { what: Searched::Body(j), time: 0.0 });
                    return true;
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
        // She calls out to them (Person.cpp:18200): "Freeze!" either way one time in four,
        // "Stop, police!" the other half.
        if self.people[cop].kind == ptype::DARCI {
            let line = (self.rand() & 3).min(2) as u8;
            self.people[cop].cues.push(Sfx::Freeze(line));
        }
        let q = &self.people[t];
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

    ///
    /// Looking round at `look` (up, radians), those at that height are favoured: someone
    /// 128 of 2048 off it or more isn't picked, and nearer it, as if nearer
    /// (`calc_target_score_new`, guns.cpp:379).
    pub(crate) fn update_target(&mut self, cycle: bool, look: Option<f32>, level: &Level) {
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
            let height = look.map_or(1.0, |up| {
                let off = ((at.y - pos.y).atan2(dist.max(1.0)) - up).abs() / std::f32::consts::TAU * 2048.0;
                (128.0 - off.min(128.0)) / 128.0
            });
            (dist < TARGET_RANGE && height > 0.0 && (at.y - pos.y).abs() < 4.0 * 256.0 && level.ground.line_of_sight(pos + eye, at + eye))
                .then(|| (aim, dist / height, 1.0 - f.dot(d / dist.max(1.0))))
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
    /// What searching person `j` turns up (`drop_all_items` with `is_being_searched`): the
    /// weapons they had and their `drop`, put on the ground round their pelvis
    /// (`find_nice_place_near_person`). There's nothing more on them from then on.
    fn empty_pockets(&mut self, j: usize, level: &Level) -> Vec<Item> {
        let p = &mut self.people[j];
        p.searched = true;
        // And a grenade he was given and never threw.
        let grenade = (!p.player && p.grenades > 0).then_some(game::grenades::IT_GRENADE);
        if grenade.is_some() {
            p.grenades = 0;
        }
        let kinds: Vec<u8> = [p.weapon.item(), p.melee.item(), p.drop.take(), grenade].into_iter().flatten().collect();
        (p.weapon, p.melee) = (Weapon::None, Weapon::None);
        let hips = p.fig.hips();
        kinds.into_iter().enumerate().map(|(n, kind)| {
            let a = n as f32 * 2.4;
            let (x, z) = (hips.x + 24.0 * a.sin(), hips.z + 24.0 * a.cos());
            let y = level.ground.surface(&level.map, x, z, hips.y + 32.0);
            Item { way: 0, kind, pos: Vec3::new(x, y, z), rounds: None }
        }).collect()
    }

    /// Anyone who'd fight, empty-handed, goes for a weapon someone dropped lying near
    /// (`PCOM_is_there_an_item_i_should_get`, pcom.cpp:4969): a gun, shotgun, AK, bat or knife
    /// within 0x300, which they can get to straight (`there_is_a_los_mav`). They look every
    /// 128 ticks standing about (pcom.cpp:8176), and every 256 going for someone (7568, 8578),
    /// then run there, pick it up and get back to it. Not civilians, the men in black (whose
    /// arm is an AK), anyone in a warehouse, or anyone carrying anything at all; the player's
    /// bodyguards have [`Game::arm_bodyguards`]. Only what was dropped: what the mission put
    /// down stays the player's to find (as the bodyguards have it).
    pub(crate) fn fetch_weapons(&mut self, level: &Level) {
        let tick = (self.time * 20.0) as usize;
        let eye = Vec3::new(0.0, 64.0, 0.0);
        for i in 0..self.people.len() {
            let p = &self.people[i];
            let armed = p.weapon != Weapon::None || p.melee != Weapon::None || p.guns != 0;
            if let Some(at) = p.fetch {
                let lying = self.items.iter().any(|it| it.way == 0 && it.pos.distance(at) < 1.0);
                if armed || !lying || p.is_down() || p.is_dead() || p.removed {
                    self.people[i].fetch = None;
                } else if Vec2::new(at.x - p.fig.pos.x, at.z - p.fig.pos.z).length() < 60.0 {
                    self.pick_up_for(i);
                }
                continue;
            }
            let every = if p.aggro() { 256 } else { 128 };
            if !(tick + i).is_multiple_of(every) || armed || p.player || p.removed || p.is_down() || p.is_dead()
                || p.ai == person::AI_CIV || p.ai == person::AI_BODYGUARD || p.is_mib() || !matches!(p.state, State::Move)
                || self.wares.under_roof(&level.map, &level.ground, p.fig.pos).is_some()
            {
                continue;
            }
            let at = p.fig.pos;
            let near = self.items.iter()
                .filter(|it| it.way == 0 && matches!(it.kind, 2 | 4 | 5 | 6 | 8))
                .filter(|it| it.pos.distance(at) < 0x300 as f32)
                .min_by(|a, b| a.pos.distance(at).total_cmp(&b.pos.distance(at)))
                .map(|it| it.pos)
                .filter(|&to| level.ground.line_of_sight(at + eye, to + eye));
            self.people[i].fetch = near;
        }
    }

    /// Killed or arrested, someone may leave something behind (`generate_bonus_item`,
    /// Person.cpp:16763, called at 17052 and 17436): anyone of skill 12 or more, and one in
    /// 256/34 of the city's extra people bar civilians. The extras leave a health pack;
    /// the skilled a health pack or pistol, shotgun or AK ammo, one in four each. It goes
    /// on the ground by them (`find_nice_place_near_person`).
    pub(crate) fn bonus_items(&mut self, level: &Level) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            let dead = p.knocked_out && matches!(p.state, State::Down) && p.fig.finished();
            if p.bonus_rolled || p.player || p.removed || !(dead || p.arrested) {
                continue;
            }
            self.people[i].bonus_rolled = true;
            let p = &self.people[i];
            let (skilled, fake) = (p.skill >= 12, p.fake && p.ai != person::AI_CIV);
            if !skilled && !(fake && self.rand() & 0xff < 34) {
                continue;
            }
            // `IT_HEALTH`, `IT_AMMO_PISTOL`, `_SHOTGUN`, `_AK47`.
            let kind = if fake { 3 } else { [3, 10, 11, 12][(self.rand() & 3) as usize] };
            let hips = self.people[i].fig.hips();
            let (x, z) = (hips.x + 24.0, hips.z);
            let y = level.ground.surface(&level.map, x, z, hips.y + 32.0);
            self.items.push(Item { way: 0, kind, pos: Vec3::new(x, y, z), rounds: None });
        }
    }

    /// Down and lying there, someone lets go of the weapon in their hand (`drop_current_gun`,
    /// Person.cpp:7130, called on going down at 17133 and on dying at 17064), Darci too: a
    /// gun with what's in it, a pistol of the AI's with 3 to 6 rounds, a knife or bat as
    /// it is. It's no longer theirs until picked up again. What they carry and weren't
    /// using stays on them until they're searched (`drop_all_items`).
    pub(crate) fn drop_held(&mut self, level: &Level) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            let w = p.weapon;
            let lying = matches!(p.state, State::Down | State::Injured) && p.fig.finished();
            // A pistol only if it was out (`FLAG_PERSON_GUN_OUT`); what else is in the hand is
            // in use (`SpecialUse`). Anything carried stays on them until they're searched.
            let held = match w {
                Weapon::Pistol => p.gun_out(),
                Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper | Weapon::Knife | Weapon::Bat => true,
                _ => false,
            };
            if p.removed || !lying || !held {
                continue;
            }
            let (Some(kind), player) = (w.item(), p.player) else { continue };
            let roll = self.rand();
            let p = &mut self.people[i];
            let rounds = w.slot().map(|s| {
                let n = if w == Weapon::Pistol && !player { 3 + (roll & 3) } else { p.ammo[s].min(w.clip()) };
                p.ammo[s] = p.ammo[s].saturating_sub(n);
                p.guns &= !(1 << s);
                n
            });
            if w.slot().is_none() {
                p.melee = Weapon::None;
            }
            p.weapon = Weapon::None;
            let hips = p.fig.hips();
            let (x, z) = (hips.x - 24.0, hips.z);
            let y = level.ground.surface(&level.map, x, z, hips.y + 32.0);
            self.items.push(Item { way: 0, kind, pos: Vec3::new(x, y, z), rounds });
        }
    }

    pub(crate) fn searching(&mut self, dt: f32, held: bool, level: &Level) {
        let Some(mut s) = self.search.take() else { return };
        let standing = self.player.is_some_and(|p| matches!(self.people[p].state, State::Move));
        if !held || !standing || self.outcome.is_some() {
            return;
        }
        let before = s.time;
        s.time += dt * self.difficulty().searching();
        if before < Search::FOUND && s.time >= Search::FOUND {
            let found: Vec<Item> = match s.what {
                Searched::Prop(ob) => {
                    let (found, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.hidden).into_iter().partition(|(o, _)| *o == ob);
                    self.hidden = kept;
                    found.into_iter().map(|(_, it)| it).collect()
                }
                Searched::Body(j) => {
                    self.record.searched += 1;
                    self.empty_pockets(j, level)
                }
            };
            if !found.is_empty() {
                let at = self.player().map_or(Vec3::ZERO, |p| p.fig.pos);
                self.sounds.push((Sfx::Found, at));
            }
            self.items.extend(found);
        }
        if s.time < Search::DONE {
            self.search = Some(s);
        }
    }

    /// The player's bodyguards with no gun go for one someone dropped, lying near, in sight
    /// and on their level, while there's nobody to fight (a bat or knife if their hands are
    /// empty), and pick it up. Only the player's, and only what's dropped: what the mission
    /// put down is the player's to find (Gang Order 2's Wildcat would otherwise leave the
    /// base for the shotgun by it), and anyone else's bodyguards stay at their post.
    pub(crate) fn arm_bodyguards(&mut self, level: &Level) {
        let eye = Vec3::new(0.0, 120.0, 0.0);
        let Some(me) = self.player else { return };
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.ai != person::AI_BODYGUARD || p.player || p.removed || p.is_down() || p.is_dead() || p.aggro() || p.weapon.is_gun() {
                continue;
            }
            let client = if p.other == 0 { Some(me) } else { self.by_way.get(&p.other).copied() };
            if client != Some(me) {
                continue;
            }
            if !matches!(p.state, State::Move) {
                continue;
            }
            let at = p.fig.pos;
            let flat = |v: Vec3| Vec2::new(v.x - at.x, v.z - at.z).length();
            let near = self.items.iter()
                .filter(|it| it.way == 0 && matches!(it.kind, 2 | 4 | 5 | 6 | 8 | SNIPER) && wanted(p, it.kind))
                .filter(|it| (it.pos.y - at.y).abs() < 100.0 && flat(it.pos) < crate::GUARDS * 256.0)
                // (Close to, a post or a bin can be in the way of the eye: no matter.)
                .filter(|it| flat(it.pos) < 256.0 || level.ground.line_of_sight(at + eye, it.pos + eye))
                .min_by(|a, b| flat(a.pos).total_cmp(&flat(b.pos)))
                .map(|it| it.pos);
            if let Some(to) = near {
                let p = &mut self.people[i];
                p.goal = Some(to);
                p.pace = person::Pace::Run;
                self.pick_up_for(i);
            }
        }
    }

    /// The player walks over items: health heals, weapons and ammo are picked up.
    pub(crate) fn pick_up(&mut self) {
        if let Some(pi) = self.player {
            self.pick_up_for(pi);
        }
    }

    /// Person `pi` walks over items. Anyone but the player (a capture-the-flag bot) only
    /// takes health, weapons and ammo.
    pub(crate) fn pick_up_for(&mut self, pi: usize) {
        let player = Some(pi) == self.player;
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
        let (kind, way, rounds) = (self.items[i].kind, self.items[i].way, self.items[i].rounds);
        let had = Weapon::from_item(kind).is_some_and(|w| w.is_gun() && self.people[pi].has(w));
        if !player && !matches!(kind, 2..=6 | 8 | 10..=12 | SNIPER) {
            return;
        }
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
                // One someone dropped has what was left in it.
                if let (Some(n), Some(s)) = (rounds, w.slot()) {
                    p.ammo[s] = p.ammo[s] - w.clip() + n;
                }
                // The AI draws it over a bat or knife.
                if p.weapon == Weapon::None || (!player && !p.weapon.is_gun()) {
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
            if player {
                self.info = info_text(kind, had).map(|t| (t.to_string(), 0.0));
            }
            self.items.remove(i);
            if way != 0 {
                self.tally.taken.insert(way);
            }
        }
    }

    /// What the music should be doing (`context_music`, Controls.cpp:1372): fight music while
    /// enemies are on the player, sprint music while sprinting, crawl music crawling, driving
    /// music at the wheel, and quiet otherwise. A mission with a theme of its own plays it
    /// throughout, and a visible countdown under 30 seconds plays the timer tune over
    /// anything (`music_mode_override`, eway.cpp:2605).
    pub fn mood(&self) -> Mood {
        let me = self.player();
        match &self.outcome {
            Some(Outcome::Won) => Mood::Won,
            Some(Outcome::Lost(_)) if me.is_some_and(|p| !p.is_dead() && p.health > 0) => Mood::Chaos,
            Some(Outcome::Lost(_)) => Mood::Lost,
            None if self.countdown().is_some_and(|t| t < 30.0) => Mood::Timer,
            None => match self.theme {
                Some(t) => Mood::Theme(t),
                None if self.fight_timer > 0.0 => Mood::Fight,
                None if self.sprinting => Mood::Sprint,
                None if me.is_some_and(|p| matches!(p.state, State::Crouch { phase: 2 })) => Mood::Crawl,
                None if self.player_vehicle().is_some() => Mood::Drive,
                None => Mood::Calm,
            },
        }
    }
}

/// What the panel says the player picked up (`special_pickup`'s `x_message`, Special.cpp:360,
/// in `lang_english.txt`): a gun she has already is ammo for it. The badges have their own
/// message. Health says +50 though it heals 100, as it did.
fn info_text(kind: u8, had: bool) -> Option<&'static str> {
    Some(match kind {
        2 | 4 | 6 if had => "Ammo",
        2 => "Gun",
        4 => "Shotgun",
        6 => "M16",
        10..=12 => "Ammo",
        3 => "Health +50",
        5 => "Knife",
        8 => "Baseball Bat",
        1 => "Keycard",
        19 => "Video",
        game::grenades::IT_GRENADE => "Grenade",
        EXPLOSIVES => "Explosives",
        _ => return None,
    })
}

/// A `SPECIAL_*` item type as the `IT_*` one the items here use: the reverse of
/// `ELEV_load_level`'s table (elev.cpp:1054). None for nothing, or a type with no item.
pub(crate) fn special_item(special: u8) -> Option<u8> {
    Some(match special {
        1 => 1,   // key
        2 => 2,   // pistol
        3 => 3,   // health
        5 => 4,   // shotgun
        6 => 5,   // knife
        7 => 23,  // explosives
        8 => 22,  // grenade
        9 => 6,   // AK47
        10 => 7,  // mine
        12 => 8,  // baseball bat
        13 => 10, // pistol ammo
        14 => 11, // shotgun ammo
        15 => 12, // AK47 ammo
        16 => 13, // keycard
        17 => 14, // file
        18 => 15, // floppy disk
        19 => 16, // crowbar
        20 => 19, // video
        21 => 20, // gloves
        22 => 21, // weed-away
        _ => return None,
    })
}

/// What someone the AI drives would pick up (a capture-the-flag bot, a bodyguard): health
/// when hurt, a gun when they have none, a bat or knife when their hands are empty, and
/// rounds for the gun they have.
pub(crate) fn wanted(p: &Person, kind: u8) -> bool {
    match kind {
        3 => p.health < p.max_health * 3 / 4,
        2 | 4 | 6 | SNIPER => !p.weapon.is_gun(),
        5 | 8 => p.weapon == Weapon::None,
        10..=12 => p.weapon == Weapon::GUNS[(kind - 10) as usize],
        _ => false,
    }
}
