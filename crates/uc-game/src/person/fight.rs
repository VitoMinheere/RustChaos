//! Fighting: attacks, the slide, shooting, grapples and the takedown, and taking hits.

use super::*;

impl Person {
    /// Crouching and crawling. Letting go of action stands her up; pushing a direction
    /// crawls that way at the original's velocity 15 to walking's 16, on all fours (or on
    /// her belly with a gun drawn: `ANIM_SNIPER_CRAWL`); stopping goes back to crouching.
    /// The takedown from behind (`find_best_grapple` trying `grapples[0]`, combat.cpp:181:
    /// `{ANIM_PISTOL_WHIP, 75, 65, 1024, 0, 1}`, the player's only): someone 10 to 140 off, in
    /// front of her, with their back to her (facing her way within 160 of 2048), and not
    /// one of the Men in Black, Roper, or a fight-test dummy (combat.cpp:752, 777). Whipped,
    /// they go down out cold whatever their health. Returns whether she did it.
    pub(super) fn takedown(&mut self, me: usize, snaps: &[Snap], events: &mut Vec<Event>) -> bool {
        use uc_world::character::person as ptype;
        let f = self.fig.facing();
        let back_to_her = (160.0 / 2048.0 * std::f32::consts::TAU).cos();
        let t = snaps.iter().enumerate().filter(|&(j, s)| {
            let d = flat(s.pos, self.fig.pos);
            j != me && s.present && s.standing && !s.player && !s.in_car && !s.dummy
                && !matches!(s.kind, ptype::MIB1 | ptype::MIB2 | ptype::MIB3 | ptype::ROPER)
                && (10.0..140.0).contains(&d.length()) && (s.pos.y - self.fig.pos.y).abs() < 50.0
                && f.dot(d.normalize_or_zero()) > 0.7 && s.facing.dot(f) > back_to_her
        }).min_by(|a, b| flat(a.1.pos, self.fig.pos).length().total_cmp(&flat(b.1.pos, self.fig.pos).length()))
            .map(|(j, _)| j);
        let Some(t) = t else { return false };
        if !self.fig.restart(anim::PISTOL_WHIP, Play::Once) {
            return false;
        }
        self.fig.angle = uc_world::character::angle_of(flat(snaps[t].pos, self.fig.pos));
        self.state = State::Grapple { victim: t, grab: Grab::PistolWhip, hit: false, timer: 0.0, frame: None };
        events.push(Event::Grab { attacker: me, victim: t, grab: Grab::PistolWhip });
        true
    }

    pub(super) fn start_attack(&mut self, me: usize, mv: Move, events: &mut Vec<Event>) {
        self.fig.restart(mv.def().anim, Play::Once);
        self.state = State::Attack { mv, hit: false, queued: None };
        self.cues.push(Sfx::Swing);
        events.push(if mv.is_punch() { Event::Punch { attacker: me } } else { Event::Kick { attacker: me } });
    }

    /// `step` in [`State::Shoot`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_shoot(&mut self, target: Option<usize>, mut shots: u32, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        let target = target.filter(|&t| snaps[t].present && !snaps[t].down);
        if let Some(t) = target {
            self.fig.turn_towards(flat(snaps[t].pos, self.fig.pos), 12.0, dt);
        } else if let Some(at) = c.aim {
            self.fig.turn_towards(flat(at, self.fig.pos), 12.0, dt);
        }
        let w = self.weapon;
        if w == Weapon::Ak47 {
            // Automatic: a round each time round the firing loop, while the trigger is
            // held (the AI fires bursts of three).
            let more = if self.player { c.fire } else { shots < 3 };
            if self.fig.loops() >= shots as f32 {
                if more && self.fire_round(me, target, events) {
                    shots += 1;
                } else {
                    // (Unless out of rounds she's getting out something else.)
                    if matches!(self.state, State::Shoot { .. }) {
                        self.state = State::Move;
                        self.idle(false);
                    }
                    return;
                }
            }
        } else if shots == 0 && self.fig.progress() > 0.25 {
            shots = 1;
            self.fire_round(me, target, events);
        } else if shots > 0 && (self.fig.finished() || (c.punch && self.fig.progress() > 0.6)) {
            // Done (or fire again straight away).
            self.state = State::Move;
            self.idle(false);
            if c.punch {
                self.step_move(me, c, snaps, ground, map, dt, events);
            }
            return;
        }
        self.state = State::Shoot { target, shots };
    }

    /// `step` in [`State::Attack`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_attack(&mut self, mv: Move, mut hit: bool, mut queued: Option<Move>, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        let def = mv.def();
        let p = self.fig.progress();
        if !hit && p >= def.hit_at {
            hit = true;
            let dir = match def.dir {
                Dir::Front => self.fig.facing(),
                Dir::Back => -self.fig.facing(),
                Dir::Left => { let f = self.fig.facing(); vec2(f.y, -f.x) }
                Dir::Right => { let f = self.fig.facing(); vec2(-f.y, f.x) }
            };
            // A stamp lands only on someone lying down, and nothing else does.
            let stomp = mv == Move::Stomp;
            for (j, s) in snaps.iter().enumerate() {
                let open = if stomp { s.floored } else { s.standing };
                if j == me || !open || !self.opponents(j, s) || (s.pos.y - self.fig.pos.y).abs() > 100.0 {
                    continue;
                }
                let d = flat(s.pos, self.fig.pos);
                if d.length() < def.reach && d.normalize_or_zero().dot(dir) > 0.5 {
                    // The AI hits less hard, more so the less skilled.
                    let damage = if self.player { def.damage } else { def.damage * (8 + self.skill as i32) / 30 };
                    let weapon = if matches!(mv, Move::Knife1 | Move::Knife2 | Move::Knife3 | Move::Bat1 | Move::Bat2) { self.weapon } else { Weapon::None };
                    // The foot sweep goes in under a guard.
                    let low = mv == Move::LegSweep;
                    events.push(Event::Hit { attacker: me, victim: j, damage, ko: def.ko, high: def.high, low, weapon });
                }
            }
        }
        // Combos: the same button again in the second half of the move continues the
        // combo; the other button (or the same one after a finisher) starts afresh.
        if queued.is_none() && p > 0.4 && (c.punch || c.kick) {
            let same = if mv.is_punch() { c.punch } else { c.kick };
            queued = match (same, def.next) {
                (true, Some(next)) => Some(next),
                _ if c.punch => Some(Move::punch_with(self.weapon)),
                _ => Some(Move::Kick1),
            };
        }
        self.state = State::Attack { mv, hit, queued };
        if self.fig.finished() {
            match queued {
                // A punch while walking at someone close is a grab, not another punch.
                Some(Move::Punch1) if c.wish.length_squared() > 0.25 => {
                    self.state = State::Move;
                    let c = Controls { punch: true, kick: false, ..*c };
                    self.step_move(me, &c, snaps, ground, map, dt, events);
                }
                Some(next) => self.start_attack(me, next, events),
                None if mv.ret().is_some_and(|a| self.fig.restart(a, Play::Once)) => {
                    self.state = State::Recover { queued: None };
                }
                None => {
                    self.state = State::Move;
                    self.idle(self.nearest_opponent(me, snaps, FIGHT_RANGE).is_some());
                }
            }
        }
    }

    /// `step` in [`State::Slide`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_slide(&mut self, mut phase: u8, mut speed: f32, mut hit: u64, me: usize, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        let f = self.fig.facing();
        if phase < 2 {
            for (j, s) in snaps.iter().enumerate() {
                if j == me || j >= 64 || hit & (1 << j) != 0 || !s.standing || !self.opponents(j, s) {
                    continue;
                }
                let d = flat(s.pos, self.fig.pos);
                if d.length() < 110.0 && d.normalize_or_zero().dot(f) > 0.3 {
                    hit |= 1 << j;
                    events.push(Event::Hit { attacker: me, victim: j, damage: 30, ko: ko::SLIDE, high: false, low: true, weapon: Weapon::None });
                }
            }
        }
        speed = (speed - 500.0 * dt).max(0.0);
        if !self.walk(f * speed * dt, ground, map, self.player) {
            self.fall(f * speed);
            return;
        }
        match phase {
            0 if self.fig.finished() => {
                phase = 1;
                self.fig.play(anim::SLIDER_HOLD, Play::Loop);
            }
            1 if speed < 120.0 => {
                phase = 2;
                self.fig.restart(anim::SLIDER_END, Play::Once);
            }
            2 if self.fig.finished() => {
                self.state = State::Move;
                self.idle(false);
                return;
            }
            _ => {}
        }
        self.state = State::Slide { phase, speed, hit };
    }

    /// Shot dead standing on the street (`PERSON_DEATH_TYPE_SHOT_*`, chosen in combat.cpp:2433
    /// and played by `set_person_dead`, Person.cpp:2504): from the front they're thrown onto
    /// their back (`ANIM_SHOT_DEAD_BACK`); from behind it's the head, the gut or the chest,
    /// `pick` choosing which. In place of the knockdown they were given; they don't get up.
    pub(crate) fn shot_dead(&mut self, behind: bool, pick: usize) {
        let a = if behind { [anim::SHOT_DEAD_HEAD, anim::SHOT_DEAD_GUT, anim::SHOT_DEAD_CHEST][pick % 3] } else { anim::SHOT_DEAD_BACK };
        self.fig.restart(a, Play::Once);
    }

    /// Fired at with their hands up: they lie face down, helpless, to be cuffed
    /// (`PERSON_DEATH_TYPE_GET_DOWN`, Person.cpp:2620: `ANIM_HANDS_UP_LIE`, with no fall and no
    /// cry). Nothing hurts them, and they stay down.
    pub(crate) fn get_down(&mut self) {
        self.state = State::Down;
        self.down_timer = 0.0;
        self.fig.restart(anim::HANDS_UP_LIE, Play::Once);
    }

    /// Knocked dead by something that isn't a fight, a shot or a fall, standing on the ground:
    /// a blast or a car (`knock_person_down` with `PERSON_DEATH_TYPE_OTHER`, Person.cpp:2585).
    /// Onto the back from the front (`ANIM_KO_BACK`), flat out from behind (`ANIM_KO_BEHIND_BIG`).
    pub(crate) fn knocked_dead(&mut self, behind: bool) {
        self.fig.restart(if behind { anim::KO_BEHIND_BIG } else { anim::KO_BACK }, Play::Once);
    }

    /// Stamped on lying down (`ANIM_FIGHT_STOMP`). Killed by it, they die where they lie
    /// (`PERSON_DEATH_TYPE_COMBAT_PRONE`, Person.cpp:2468): `ANIM_FIGHT_STOMPED_BACK` on their
    /// front, `ANIM_FIGHT_STOMPED_FRONT` on their back.
    pub fn take_stomp(&mut self, damage: i32) {
        if !self.floored() || self.invulnerable {
            return;
        }
        self.health -= damage;
        self.aggro = true;
        self.cues.push(Sfx::Hit { heavy: true });
        if self.health > 0 {
            self.cues.push(Sfx::Hurt { kind: self.kind });
            return;
        }
        self.health = 0;
        self.knocked_out = true;
        self.cues.push(Sfx::KnockedOut { kind: self.kind });
        let front = self.lying_on_front();
        self.fig.restart_locked(if front { anim::FIGHT_STOMPED_BACK } else { anim::FIGHT_STOMPED_FRONT }, Play::Once);
    }

    /// Lying on their front, by what put them down (`person_is_lying_on_what`,
    /// Person.cpp:1157): a blow from behind, a stamp, a takedown, lying down with their hands
    /// up. On their back otherwise (a thrown victim among them).
    pub(crate) fn lying_on_front(&self) -> bool {
        matches!(self.fig.anim(), anim::KD_FRONT_LOW | anim::KD_FRONT_MID | anim::KD_FRONT_HI | anim::KD_FRONT_LAND
            | anim::KO_BEHIND_BIG | anim::PISTOL_WHIP_TAKE | anim::FIGHT_STOMPED_BACK | anim::HANDS_UP_LIE)
    }

    /// Being cuffed (`set_person_arrest`, Person.cpp:9322): rolled over onto the front first
    /// if on the back (`ANIM_ARREST_ROLL`), else cuffed as they lie, the body kept where it is
    /// (`set_locked_anim`). From now on they struggle now and then, each on a beat of their
    /// own (`PTIME`, a byte that starts at random and counts game turns).
    pub(crate) fn be_cuffed(&mut self) {
        self.state = State::Cuffed;
        self.struggle = (self.fidget_roll >> 8 & 0xff) as f32 / 20.0;
        let a = if self.lying_on_front() { anim::ARREST_BE_CUFFED } else { anim::ARREST_ROLL };
        self.fig.restart_locked(a, Play::Once);
    }

    /// `step` in [`State::Cuffed`] (`fn_person_dead`, Person.cpp:17419). Rolled over, they're
    /// turned round with it and cuffed from three keyframes in (`SUB_STATE_DEAD_ARREST_TURN_
    /// OVER`). Cuffed, every 256 turns, once still, a struggle (`SUB_STATE_DEAD_ARRESTED`).
    pub(super) fn step_cuffed(&mut self, dt: f32) {
        self.struggle -= dt;
        let beat = self.struggle <= 0.0;
        if beat {
            self.struggle += 256.0 / 20.0;
        }
        if !self.fig.finished() {
            return;
        }
        if self.fig.anim() == anim::ARREST_ROLL {
            let hips = self.fig.hips();
            self.fig.angle += std::f32::consts::PI;
            self.fig.restart_now(anim::ARREST_BE_CUFFED, Play::Once);
            self.fig.skip_keyframes(3);
            let now = self.fig.hips();
            self.fig.pos += vec3(hips.x - now.x, 0.0, hips.z - now.z);
        } else if beat {
            self.fig.restart_locked(anim::ARREST_STRUGGLE, Play::Once);
        }
    }

    /// Whether a blow or a shot from `from` comes from behind them.
    pub(crate) fn hit_from_behind(&self, from: Vec3) -> bool {
        flat(from, self.fig.pos).dot(self.fig.facing()) <= 0.0
    }

    /// `step` in [`State::Down`].
    pub(super) fn step_down(&mut self, ground: &Ground, map: &Map, dt: f32) {
        // Knocked down, not out: get up after a while (unless arrested meanwhile).
        self.down_timer += dt;
        let lying = matches!(self.fig.anim(), anim::KD_FRONT_LAND | anim::KD_BACK_LAND | anim::GRAB_ARM_THROWV);
        if !self.knocked_out && !self.out_cold && lying && self.fig.finished() && self.down_timer > 2.5 {
            // Off the front, the get-up from it; off the back, Darci and the gangs spring up,
            // everyone else gets up slowly (`SUB_STATE_DYING_KNOCK_DOWN_WAIT`, Person.cpp:17173).
            use uc_world::character::person as ptype;
            let a = if self.lying_on_front() {
                anim::KO_BEHIND_BIG_GU
            } else if matches!(self.kind, ptype::DARCI | ptype::THUG_RASTA | ptype::THUG_RED | ptype::THUG_GREY) {
                anim::QUICK_GET_UP
            } else {
                anim::KO_BACK_GU
            };
            self.fig.restart_locked(a, Play::Once);
            self.fig.lift = 0.0;
            self.state = State::GetUp;
            return;
        }
        // A knockdown is two animations: the fall, then hitting the floor (on the back
        // if knocked over from the front). Chain them keeping the hips in place.
        let land = match self.fig.anim() {
            anim::KD_FRONT_MID | anim::KD_FRONT_LOW | anim::KD_FRONT_HI | anim::HIT_FRONT_HI | anim::PLUNGE_BACKWARD => Some(anim::KD_BACK_LAND),
            anim::KD_BACK_MID | anim::KD_BACK_LOW | anim::KD_BACK_HI | anim::PLUNGE_FORWARDS => Some(anim::KD_FRONT_LAND),
            _ => None,
        };
        if let Some(vy) = self.topple {
            self.step_topple(vy, land, ground, map, dt);
            return;
        }
        if let (Some(a), true) = (land, self.fig.finished()) {
            // Gone over an edge, the hips more than 8 above what's under them: on down
            // (`SUB_STATE_DYING_PRONE`, Person.cpp:16928), plunging if it's more than 128,
            // else hitting the floor as they go.
            let h = self.fig.hips();
            let p = self.fig.pos;
            let drop = p.y - ground.surface(map, h.x, h.z, p.y + 8.0);
            if drop > 8.0 {
                let plunge = if a == anim::KD_BACK_LAND { anim::PLUNGE_BACKWARD } else { anim::PLUNGE_FORWARDS };
                if !(drop > 128.0 && self.fig.restart_locked(plunge, Play::Once)) {
                    self.fig.restart_locked(a, Play::Once);
                }
                // `DY = -(4 << 8)` a tick.
                self.topple = Some(-4.0 * 20.0);
                return;
            }
            self.fig.restart_locked(a, Play::Once);
            self.fig.pos.y = ground.surface(map, p.x, p.z, p.y);
        }
    }

    /// Knocked down over an edge, falling at `vy` until they hit what's under their hips:
    /// then the floor's hit lying as they fell (Person.cpp:16990), and the fall hurts as a
    /// landing does, which may finish them.
    fn step_topple(&mut self, vy: f32, land: Option<usize>, ground: &Ground, map: &Map, dt: f32) {
        let vy = vy - GRAVITY * dt;
        let h = self.fig.hips();
        let p = self.fig.pos;
        let floor = ground.surface(map, h.x, h.z, p.y);
        let y = p.y + vy * dt;
        if y > floor {
            self.fig.pos.y = y;
            self.topple = Some(vy);
            return;
        }
        self.fig.pos.y = floor;
        self.topple = None;
        if let (Some(a), true) = (land, matches!(self.fig.anim(), anim::PLUNGE_BACKWARD | anim::PLUNGE_FORWARDS)) {
            self.fig.restart_locked(a, Play::Once);
        }
        self.cues.push(Sfx::Land);
        let damage = fall_damage(-vy);
        if damage > 0 && !self.invulnerable && !self.knocked_out {
            self.health -= damage;
            if self.health <= 0 {
                self.health = 0;
                self.knocked_out = true;
                self.cues.push(Sfx::KnockedOut { kind: self.kind });
            }
        }
    }

    /// `step` in [`State::Grapple`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_grapple(&mut self, victim: usize, grab: Grab, mut hit: bool, mut timer: f32, frame: Option<usize>, me: usize, c: &Controls, snaps: &[Snap], dt: f32, events: &mut Vec<Event>) {
        let v = snaps[victim];
        if !v.present {
            self.state = State::Move;
            return;
        }
        timer += dt;
        let next = match grab {
            Grab::Grab if self.fig.finished() => Some(Grab::Hold),
            Grab::Hold if c.punch => Some(Grab::Knee1),
            Grab::Hold if c.kick => Some(Grab::Throw),
            Grab::Knee1 if c.punch && self.fig.progress() > 0.4 => Some(Grab::Knee2),
            Grab::Knee1 if self.fig.finished() => Some(Grab::Hold),
            _ => None,
        };
        if grab == Grab::Hold && timer > 5.0 {
            // The victim wriggles free, and the hold's broken (`ANIM_GRAB_ARM_ESCAPE`).
            events.push(Event::Release { victim });
            if self.fig.restart(anim::GRAB_ARM_ESCAPE, Play::Once) {
                self.state = State::Recover { queued: None };
            } else {
                self.state = State::Move;
                self.idle(true);
            }
            return;
        }
        if let Some(g) = next {
            if g != Grab::Hold {
                self.fig.restart(g.anims().0, Play::Once);
                events.push(Event::GrabMove { victim, grab: g });
                events.push(if g == Grab::Throw { Event::Kick { attacker: me } } else { Event::Punch { attacker: me } });
            }
            self.state = State::Grapple { victim, grab: g, hit: false, timer: 0.0, frame };
            return;
        }
        if matches!(grab, Grab::Knee2 | Grab::Throw) {
            if !hit && self.fig.progress() > 0.55 {
                hit = true;
                events.push(Event::Hit { attacker: me, victim, damage: 60, ko: ko::GRAPPLE, high: false, low: false, weapon: Weapon::None });
            }
            if self.fig.finished() {
                self.state = State::Move;
                self.idle(true);
                return;
            }
        }
        self.state = State::Grapple { victim, grab, hit, timer, frame };
    }

    /// Whether the player's gun will fire (`shoot_get_ammo_sound_anim_time` and
    /// `set_person_shoot`, Person.cpp:6085 and 6898): a clip used up is changed instead, to a
    /// dry click and no shot; out altogether, a click, and on to her best weapon with rounds
    /// left, or put away. The AI has its one load ([`Person::out_of_rounds`]).
    pub(super) fn trigger(&mut self) -> bool {
        let Some(s) = self.weapon.slot().filter(|_| self.player) else { return true };
        if self.ammo[s] == 0 {
            self.cues.push(Sfx::Click);
            let was = self.weapon;
            self.weapon = self.best_with_ammo().unwrap_or(Weapon::None);
            if self.weapon != was {
                self.draw_weapon();
            }
            return false;
        }
        if self.spent[s] >= self.weapon.clip() {
            self.spent[s] = 0;
            self.cues.push(Sfx::Click);
            return false;
        }
        true
    }

    /// Fires one round from the gun in hand; false when it's empty or the clip's changed.
    pub(super) fn fire_round(&mut self, me: usize, target: Option<usize>, events: &mut Vec<Event>) -> bool {
        let Some(s) = self.weapon.slot() else { return false };
        if !self.trigger() {
            return false;
        }
        if self.player {
            self.ammo[s] -= 1;
            self.spent[s] += 1;
        } else if self.out_of_rounds() {
            self.cues.push(Sfx::Click);
            return false;
        } else {
            self.ai_fired += 1;
        }
        self.cues.push(Sfx::Shot(self.weapon));
        self.flash = 0.07;
        events.push(Event::Shot { attacker: me, victim: target, weapon: self.weapon });
        true
    }

    /// Takes a blow from someone at `from`. Fight-test dummies only go down to the moves in
    /// their `ko_moves`; everyone else loses health.
    #[allow(clippy::too_many_arguments)]
    pub fn take_hit(&mut self, from: Vec3, damage: i32, ko_flag: u8, high: bool, low: bool, weapon: Weapon) {
        // The injured lie as the dead do (`STATE_DEAD`): nothing more happens to them.
        if self.removed || self.invulnerable || self.is_down()
            || matches!(self.state, State::InCar { .. } | State::Injured | State::Carried { .. })
            // Nobody hits someone in the middle of a foot sweep or a jump kick (combat.cpp:2929).
            || matches!(self.state, State::Attack { mv: Move::LegSweep | Move::KickJump, .. })
        {
            return;
        }
        let front = flat(from, self.fig.pos).dot(self.fig.facing()) > 0.0;
        // A guard stops blows from the front; weapons still hurt a little.
        let blocking = matches!(self.state, State::Block { .. }) && !weapon.is_gun();
        if blocking && front && !low {
            self.cues.push(Sfx::Block);
            if weapon != Weapon::None {
                self.health -= damage / 4;
            }
            if self.health > 0 {
                return;
            }
        }
        // Any sort of come-on, and a taunt is over: they go for whoever it was.
        self.taunt = None;
        let damage = if blocking { damage / 4 } else { damage };
        // Fight-test dummies only go down to their moves; everyone else loses health, and
        // finishing moves knock them down even with some left.
        let (knocked_out, knocked_down) = if self.is_dummy() {
            let ko = self.ko_moves & ko_flag != 0;
            (ko, ko)
        } else {
            self.health -= damage;
            (self.health <= 0, self.health <= 0 || ko_flag & ko::FINISHERS != 0)
        };
        if self.ai == 16 {
            self.ai = 6; // a cornered fleer fights back
        }
        // A civilian set on runs (`PCOM_attack_happened`'s `goto flee`).
        if !knocked_out {
            self.scare(None, from);
        }
        self.aggro = true;
        self.attack_timer = self.attack_timer.max(0.8);
        // A bullet landing makes no thump of its own: all the original adds is a spark
        // (`PYRO_hitspang`), and it keeps the ricochets for shots that miss. The gun going
        // off and the victim's own voice carry it, so a shot mustn't sound like a punch.
        if !weapon.is_gun() {
            self.cues.push(match weapon {
                Weapon::Knife => Sfx::Stab,
                Weapon::Bat => Sfx::BatHit,
                _ => Sfx::Hit { heavy: knocked_down || damage >= 25 },
            });
        }
        self.cues.push(if knocked_out { Sfx::KnockedOut { kind: self.kind } } else { Sfx::Hurt { kind: self.kind } });
        if knocked_down {
            if knocked_out {
                self.health = 0;
                self.knocked_out = true;
            }
            self.down_timer = 0.0;
            self.state = State::Down;
            // Thrown, the throw lays them out as it goes, where it puts them
            // (`PERSON_DEATH_TYPE_STAY_ALIVE_PRONE`, which starts no animation of its own).
            // Otherwise by where the blow lands, low, middle or high (`knock_down`,
            // Person.cpp:2409, by the attack's `Height`).
            if self.fig.anim() != anim::GRAB_ARM_THROWV {
                let a = match (front, low, high) {
                    (true, true, _) => anim::KD_FRONT_LOW,
                    (false, true, _) => anim::KD_BACK_LOW,
                    (true, false, true) => anim::KD_FRONT_HI,
                    (false, false, true) => anim::KD_BACK_HI,
                    (true, false, false) => anim::KD_FRONT_MID,
                    (false, false, false) => anim::KD_BACK_MID,
                };
                self.fig.restart(a, Play::Once);
            }
        } else {
            self.state = State::Hurt;
            let a = match (front, high) {
                (true, true) => anim::HIT_FRONT_HI,
                (true, false) => anim::HIT_FRONT_MID,
                (false, true) => anim::HIT_BACK_HI,
                (false, false) => anim::HIT_BACK_MID,
            };
            self.fig.restart(a, Play::Once);
            self.fig.set_speed(if self.player { 1.8 } else { 1.2 });
        }
    }
}
