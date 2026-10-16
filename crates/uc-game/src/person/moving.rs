//! Getting about on foot: walking, running and sprinting, crouching and crawling, jumping, falling and landing.

use super::*;

/// How close Darci has to be to kick between the legs (`dist < 100`, Person.cpp:9909), a
/// little more as our fighters stand a touch further apart.
const NAD_RANGE: f32 = 120.0;

impl Person {
    pub(super) fn step_crouch(&mut self, phase: u8, c: &Controls, ground: &Ground, map: &Map, dt: f32) {
        let moving = c.wish.length_squared() > 0.01;
        match phase {
            0..=2 if !c.sprint && !c.crouch => {
                let a = match self.weapon {
                    _ if !self.gun_out() => anim::CROUTCH_GETUP,
                    Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper => anim::UNCROUTCH_AK,
                    _ => anim::UNCROUTCH_PISTOL,
                };
                if self.fig.restart(a, Play::Once) || self.fig.restart(anim::CROUTCH_GETUP, Play::Once) {
                    self.state = State::Crouch { phase: 3 };
                } else {
                    self.state = State::Move;
                    self.idle(false);
                }
            }
            0 => {
                if self.fig.finished() {
                    self.state = State::Crouch { phase: 1 };
                }
            }
            1 | 2 if moving => {
                let dir = c.wish.normalize();
                self.fig.turn_towards(dir, 6.0, dt);
                let crawl = if self.gun_out() { anim::SNIPER_CRAWL } else { anim::CRAWL };
                if !self.fig.play(crawl, Play::Loop) {
                    self.fig.play(anim::CRAWL, Play::Loop);
                }
                self.state = State::Crouch { phase: 2 };
                // No crawling off an edge (`slope_ahead`).
                self.walk(self.fig.facing() * CRAWL_SPEED * dt, ground, map, false);
            }
            2 => {
                self.fig.restart(anim::IDLE_CROUTCH, Play::Once);
                self.state = State::Crouch { phase: 1 };
            }
            3 if self.fig.finished() => {
                self.state = State::Move;
                self.idle(false);
            }
            _ => {}
        }
    }

    /// Crouched or crawling (`is_person_crouching`).
    pub fn crouching(&self) -> bool {
        matches!(self.state, State::Crouch { phase: 0..=2 })
    }

    /// Getting breath back, a point a tick while action isn't held, jumping from 5 to 20 as
    /// it comes back from nothing; bleeding (under a quarter of health) loses a point a tick
    /// down to 50 (Person.cpp:4315, 4469).
    pub(super) fn breathe(&mut self, c: &Controls, dt: f32) {
        const TICK: f32 = 20.0;
        if !c.sprint {
            self.winded = false;
            if self.stamina < self.max_stamina {
                let before = self.stamina;
                self.stamina = (self.stamina + TICK * dt).min(self.max_stamina);
                if before < 5.0 && self.stamina >= 5.0 {
                    self.stamina = 20.0f32.min(self.max_stamina);
                }
            }
        }
        if self.health < self.max_health / 4 && self.stamina > 50.0 {
            self.stamina -= TICK * dt;
        }
    }

    /// Whether the player sprints: running with action held, starting only with more than 3
    /// breath, a point every fourth tick while at it, and out of breath under 10 (back to a
    /// run, panting, `S_DARCI_OUTOFBREATH`; Person.cpp:15700). Roper never sprints
    /// (interfac.cpp: `PersonType != PERSON_ROPER`). The AI's sprinting is left as it was.
    pub(super) fn sprint(&mut self, c: &Controls, moving: bool, dt: f32) -> bool {
        use uc_world::character::person as ptype;
        if !self.player {
            return c.sprint;
        }
        if !(c.sprint && c.run && moving) || self.winded || self.kind == ptype::ROPER || self.stamina <= 3.0 {
            return false;
        }
        if self.stamina < 10.0 {
            self.stamina = 0.0;
            self.winded = true;
            let id = if self.kind == ptype::ROPER { 203 } else { 176 };
            self.cues.push(Sfx::ById { id, replace: false });
            return false;
        }
        self.stamina -= 20.0 / 4.0 * dt;
        self.sprinting = true;
        true
    }

    pub(super) fn idle(&mut self, fight: bool) {
        let a = match (fight, self.weapon) {
            (_, w) if w.is_gun() && self.gun_out() => w.aim_anim(),
            (true, Weapon::Knife) => anim::KNIFE_FIGHT_READY,
            (true, Weapon::Bat) => anim::BAT_IDLE,
            (true, _) => anim::FIGHT,
            (false, _) => anim::BREATHE,
        };
        self.fig.play(a, Play::Loop);
        self.fig.set_speed(1.0);
    }

    /// Moves horizontally by `delta`, sliding along walls and following the floor. Returns
    /// false if it stepped off an edge (the caller falls). Without `drop_ok`, edges stop the
    /// person instead: the AI, and anyone fighting, never walks off a roof.
    pub(super) fn walk(&mut self, delta: Vec2, ground: &Ground, map: &Map, drop_ok: bool) -> bool {
        let p = &mut self.fig.pos;
        let to = ground.slide(vec2(p.x, p.z), p.y, delta, RADIUS);
        let floor = ground.surface(map, to.x, to.y, p.y);
        if floor - p.y > STEP_HEIGHT || (!drop_ok && p.y - floor > STEP_HEIGHT) {
            return true; // a wall without collision data, or an edge: just stop
        }
        *p = vec3(to.x, p.y, to.y);
        if p.y - floor > STEP_HEIGHT {
            return false;
        }
        p.y = floor;
        true
    }

    pub(super) fn jump(&mut self, vel: Vec2) {
        self.cues.push(Sfx::Jump { kind: self.kind });
        self.state = State::Fall { vy: JUMP_SPEED, vel };
        self.fig.restart(if vel.length() > 1.0 { anim::RUN_JUMP } else { anim::STANDING_JUMP }, Play::Once);
        self.fig.set_speed(1.5);
    }

    pub(super) fn fall(&mut self, vel: Vec2) {
        self.state = State::Fall { vy: 0.0, vel };
        self.fig.play(anim::FALLING, Play::Loop);
    }

    /// Advances the animation, then places anything that follows it: a climber's height on a
    /// ladder goes up a rung cycle as the climbing animation loops back down.
    pub fn advance(&mut self, dt: f32, ground: &Ground) {
        self.fig.advance(dt);
        self.flash = (self.flash - dt).max(0.0);
        // Footsteps: two per walk or run cycle.
        let walking = matches!(self.state, State::Move | State::Land { .. })
            && matches!(self.fig.anim(), anim::WALK | anim::RUN | anim::PISTOL_JOG | anim::AK_JOG | anim::FIGHT_STEP_N | anim::FIGHT_STEP_S | anim::FIGHT_STEP_E | anim::FIGHT_STEP_W);
        let phase = self.fig.loops().floor() + self.fig.progress();
        if walking && (phase * 2.0).floor() != (self.step_phase * 2.0).floor() {
            self.cues.push(Sfx::Footstep { player: self.player });
        }
        self.step_phase = phase;
        if let State::Climb { ladder, y0, loops0 } = self.state {
            self.fig.pos = ground.on_ladder(ladder, y0 + (self.fig.loops() - loops0) * RUNG_CYCLE);
        }
    }

    /// `step` in [`State::Land`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_land(&mut self, mut vel: Vec2, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        // Keep running out of a landing; otherwise skid to a stop.
        let held = c.wish.length_squared() > 0.25;
        vel *= if held { 1.0 } else { (-6.0 * dt).exp() };
        if held && vel.length() > 150.0 {
            self.fig.turn_towards(c.wish, 6.0, dt);
            vel = c.wish.normalize() * vel.length();
        }
        if !self.walk(vel * dt, ground, map, self.player) {
            self.fall(vel);
            return;
        }
        self.state = State::Land { vel };
        if self.fig.finished() || (held && self.fig.progress() > 0.5) {
            self.state = State::Move;
            self.step_move(me, c, snaps, ground, map, dt, events);
        }
    }

    /// `step` in [`State::Fall`].
    pub(super) fn step_fall(&mut self, mut vy: f32, mut vel: Vec2, c: &Controls, ground: &Ground, map: &Map, dt: f32) {
        vy -= GRAVITY * dt;
        let running = vel.length() > 100.0;
        if self.fig.finished() {
            self.fig.play(if running { anim::MID_AIR } else { anim::FALLING }, Play::Loop);
        }
        // A little air control.
        if self.player && c.wish.length_squared() > 0.25 {
            let want = c.wish.normalize() * vel.length().max(WALK_SPEED);
            vel = vel.lerp(want, (2.0 * dt).min(1.0));
        }
        // Grab a cable overhead.
        self.zip_cooldown -= dt;
        let grab = if self.zip_cooldown <= 0.0 { ground.cable_near(self.fig.pos + vec3(0.0, HANG - 60.0, 0.0), 110.0) } else { None };
        // Slide towards the lower end; anywhere but right at that end will do (The
        // Jump's zip line can only be reached at its very top).
        let lower = |cable: usize| if ground.cables[cable].at(1.0).y < ground.cables[cable].at(0.0).y { 1.0 } else { -1.0 };
        let room = |cable: usize, t: f32| if lower(cable) > 0.0 { 1.0 - t } else { t };
        if let Some((cable, t)) = grab.filter(|&(cable, t)| room(cable, t) > 0.03 && t > 0.0 && t < 1.0) {
            let dir = lower(cable);
            self.fig.restart(anim::DEATH_SLIDE, Play::Once);
            self.cues.push(Sfx::Zip);
            self.state = State::Zip { cable, t, dir, speed: vel.length() * 0.5, ridden: 0.0 };
            return;
        }
        let dir = if c.wish.length_squared() > 0.25 { c.wish.normalize() } else { vel.normalize_or_zero() };
        // Catch a ladder ahead on the way down, as `grab_ledge` does when the face it
        // finds is a ladder (fallen/Source/Person.cpp). A ladder that starts above the
        // ground, like the one up to The Fallen's first clue, can only be got on this way.
        if vy <= 0.0 {
            let p = self.fig.pos;
            if let Some(l) = ground.ladder_to_catch(p, 90.0, GRAB_REACH).filter(|&l| dir.dot(-ground.ladders[l].out) > 0.5) {
                // Hang so the hips stay where the jump had them, going straight into the
                // climb rather than blending from the jump's raised hips.
                let hips = self.fig.drawn_pos().y + self.fig.to_world(self.fig.root()).y;
                let y = hips - self.fig.anim_root(anim::ON_LADDER, false).map_or(self.fig.stand_root().y, |r| r.y);
                self.fig.pos = ground.on_ladder(l, y);
                self.fig.lift = 0.0;
                self.fig.cancel_rise = false;
                self.fig.restart_now(anim::ON_LADDER, Play::Loop);
                self.state = State::Climb { ladder: l, y0: y, loops0: self.fig.loops() };
                return;
            }
        }
        // Grab a ledge in front while in the air.
        if dir != Vec2::ZERO {
            // Only ledges too high to jump onto are grabbed; lower ones are landed on.
            let p = self.fig.pos;
            let floor = ground.surface(map, p.x, p.z, p.y);
            if let Some(to) = self.ledge_ahead(dir, GRAB_REACH, ground, map).filter(|to| to.y - floor > JUMPABLE) {
                self.grab_dangle(to, dir);
                return;
            }
        }
        let p = self.fig.pos;
        let to = ground.slide(vec2(p.x, p.z), p.y, vel * dt, RADIUS);
        let y = p.y + vy * dt;
        let floor = ground.surface(map, to.x, to.y, p.y);
        if y <= floor {
            self.fig.pos = vec3(to.x, floor, to.y);
            self.fig.lift = (floor - p.y).max(0.0);
            self.state = State::Land { vel };
            self.cues.push(Sfx::Land);
            self.fig.restart(if running { anim::LAND_RIGHT } else { anim::LAND_VERT }, Play::Once);
            self.fig.set_speed(if running { 1.5 } else { 2.0 });
            // Landing hard hurts, and from high enough kills (fallen/Source/Darci.cpp).
            let damage = fall_damage(-vy);
            if damage > 0 {
                self.fig.lift = 0.0;
                let ko = if damage >= self.health { ko::KNOCKDOWN } else { 0 };
                self.take_hit(self.fig.pos + vec3(0.0, 0.0, 1.0), damage, ko, false, true, Weapon::None);
            }
        } else {
            self.fig.pos = vec3(to.x, y, to.y);
            self.state = State::Fall { vy, vel };
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_move(&mut self, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        if c.switch && self.player {
            self.next_weapon();
            self.cues.push(Sfx::Pickup);
        }
        let moving = c.wish.length_squared() > 0.01;
        let sprint = self.sprint(c, moving, dt);
        let target = self.nearest_opponent(me, snaps, FIGHT_RANGE);
        // A gun drawn: no fighting stance (punch fires).
        let fight = target.is_some() && !c.run && !self.weapon.is_gun();
        let f = self.fig.facing();
        // Wish direction relative to facing: x = right, y = forward.
        let rel = vec2(c.wish.dot(vec2(-f.y, f.x)), c.wish.dot(f));

        // Roper picks up the injured: the nearest person within 256, if they're lying
        // injured with their middle within 128 (`find_corpse`, Person.cpp:11545).
        if c.action && self.player && self.kind == uc_world::character::person::ROPER {
            let nearest = snaps.iter().enumerate()
                .filter(|&(j, s)| j != me && s.present && !s.in_car)
                .map(|(j, s)| (j, s, flat(s.pos, self.fig.pos).length()))
                .filter(|&(_, s, d)| d < 256.0 && (s.pos.y - self.fig.pos.y).abs() < 256.0)
                .min_by(|a, b| a.2.total_cmp(&b.2));
            if let Some((j, s, d)) = nearest {
                if s.injured && d < 128.0 {
                    self.fig.restart(anim::PICKUP_CARRY, Play::Once);
                    self.state = State::Carry { victim: j, phase: 0 };
                    events.push(Event::Carry { carrier: me, victim: j });
                    return;
                }
            }
        }
        // Only Darci arrests people: the original asks for `PERSON_DARCI` before it even
        // looks for someone to cuff (`set_person_croutch`, fallen/Source/Person.cpp), so
        // Nitro Car's thug can't.
        if c.action && self.can_arrest() {
            // Arrest a downed opponent.
            // The best of those in reach: the nearest, but Roper and hostages only if there's
            // nobody else (`find_arrestee` takes 250 off their score of 500 less the distance).
            let downed = snaps.iter().enumerate()
                .filter(|&(j, s)| {
                    j != me && s.present && s.arrestable && !s.player
                        && flat(s.pos, self.fig.pos).length() < ARREST_RANGE && (s.pos.y - self.fig.pos.y).abs() < 64.0
                })
                .map(|(j, s)| {
                    use uc_world::character::person as ptype;
                    let last = matches!(s.kind, ptype::ROPER | ptype::HOSTAGE);
                    (j, s, 500.0 - flat(s.pos, self.fig.pos).length() - if last { 250.0 } else { 0.0 })
                })
                .max_by(|a, b| a.2.total_cmp(&b.2))
                .map(|(j, s, _)| (j, s));
            if let Some((j, s)) = downed {
                self.fig.angle = uc_world::character::angle_of(flat(s.pos, self.fig.pos));
                self.fig.restart(anim::ARREST_CROUTCH, Play::Once);
                self.state = State::Arrest { victim: j };
                self.cues.push(Sfx::Arrest);
                events.push(Event::Arrest { attacker: me, victim: j });
                return;
            }
        }
        // Climb a ladder by walking into it, or with action next to it.
        if let Some(l) = ground.ladder_near(self.fig.pos, 90.0) {
            let into = c.wish.dot(-ground.ladders[l].out) > 0.7;
            if into || c.action {
                self.fig.pos = ground.on_ladder(l, self.fig.pos.y);
                self.fig.restart(anim::ON_LADDER, Play::Loop);
                self.state = State::Climb { ladder: l, y0: self.fig.pos.y, loops0: self.fig.loops() };
                return;
            }
        }
        if c.block {
            self.fig.restart(anim::BLOCK_HIGH, Play::Once);
            self.state = State::Block;
            return;
        }
        if let (true, false, Some(at)) = (c.taunt, moving, c.target) {
            if self.fig.restart(anim::WANKER, Play::Once) {
                self.fig.angle = uc_world::character::angle_of(flat(snaps[at].pos, self.fig.pos));
                self.state = State::Taunt { at };
                return;
            }
        }
        // Action standing still with nothing else to do: crouch, gun and all.
        if self.player && c.punch && self.takedown(me, snaps, events) {
            return;
        }
        if self.player && ((c.action && !moving) || c.crouch) {
            let a = match self.weapon {
                _ if !self.gun_out() => anim::CROUTCH_DOWN,
                Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper => anim::SHOTGUN_DUCK,
                _ => anim::PISTOL_DUCK,
            };
            if self.fig.restart(a, Play::Once) || self.fig.restart(anim::CROUTCH_DOWN, Play::Once) {
                self.state = State::Crouch { phase: 0 };
                return;
            }
        }
        if c.punch && self.weapon.is_gun() {
            if self.rounds() == Some(0) {
                self.cues.push(Sfx::Click);
                return;
            }
            let target = c.target.filter(|&t| t != me && snaps[t].present);
            if let Some(t) = target {
                self.face(snaps[t].pos);
            } else if let Some(at) = c.aim {
                self.face(at);
            } else if !self.player {
                return;
            }
            let looped = self.weapon == Weapon::Ak47;
            if !self.fig.restart(self.weapon.fire_anim(), if looped { Play::Loop } else { Play::Once }) {
                self.fig.restart(anim::PISTOL_SHOOT, if looped { Play::Loop } else { Play::Once });
            }
            self.state = State::Shoot { target, shots: 0 };
            return;
        }
        if c.punch {
            // Punch while moving towards a close opponent: grab his arm.
            if let Some(t) = target {
                let d = flat(snaps[t].pos, self.fig.pos);
                if self.player && d.length() < GRAB_RANGE && moving && c.wish.dot(d.normalize_or_zero()) > 0.6 {
                    // Roper strangles or headbutts instead (`grapples[]`, combat.cpp:179: the
                    // arm grab is everyone else's): the strangle from up to 70 away, the
                    // headbutt from further. Neither on Roper himself, nor with too little
                    // room in front, as the victim is thrown forward at the end
                    // (`is_there_room_in_front_of_me(150)`, combat.cpp:789).
                    use uc_world::character::person as ptype;
                    let grab = if self.kind != ptype::ROPER {
                        Some(Grab::Grab)
                    } else {
                        let up = vec3(0.0, 64.0, 0.0);
                        let ahead = self.fig.pos + vec3(d.x, 0.0, d.y).normalize_or_zero() * 150.0;
                        let room = ground.line_of_sight(self.fig.pos + up, ahead + up);
                        let roper = snaps[t].kind == ptype::ROPER;
                        (room && !roper).then_some(if d.length() < 70.0 { Grab::Strangle } else { Grab::Headbutt })
                    };
                    if let Some(grab) = grab {
                        self.fig.angle = uc_world::character::angle_of(d);
                        self.fig.restart(grab.anims().0, Play::Once);
                        self.state = State::Grapple { victim: t, grab, hit: false, timer: 0.0, frame: None };
                        self.cues.push(Sfx::Grab);
                        events.push(Event::Grab { attacker: me, victim: t, grab });
                        if grab == Grab::Grab {
                            events.push(Event::Punch { attacker: me });
                        }
                        return;
                    }
                }
            }
            if let Some(t) = target {
                self.face(snaps[t].pos);
            }
            self.start_attack(me, Move::punch_with(self.weapon), events);
            return;
        }
        if c.kick {
            if moving && c.run {
                let speed = if sprint { SPRINT_SPEED * 1.15 } else { RUN_SPEED * 1.1 };
                self.fig.restart(anim::SLIDER_START, Play::Once);
                self.state = State::Slide { phase: 0, speed, hit: 0 };
                events.push(Event::Kick { attacker: me });
                return;
            }
            // Darci stepping in on someone close kicks between the legs, unless they're still
            // doubled up from the last one (`set_person_kick_near`).
            let close = target.filter(|&t| flat(snaps[t].pos, self.fig.pos).length() < NAD_RANGE && !snaps[t].doubled);
            let mv = if let (true, true, true, Some(t)) = (self.kind == uc_world::character::person::DARCI, fight && moving, rel.y > 0.5, close) {
                self.face(snaps[t].pos);
                Move::KickNad
            } else if fight && moving && rel.y < -0.5 {
                Move::KickBack
            } else if fight && moving && rel.x.abs() > 0.5 {
                if rel.x > 0.0 { Move::KickRight } else { Move::KickLeft }
            } else {
                if let Some(t) = target {
                    self.face(snaps[t].pos);
                }
                Move::Kick1
            };
            self.start_attack(me, mv, events);
            return;
        }

        if fight {
            // Fight stance: face the opponent and step without turning.
            let t = snaps[target.unwrap()].pos;
            let d = flat(t, self.fig.pos);
            self.fig.turn_towards(d, 8.0, dt);
            if moving {
                let step = if rel.y.abs() >= rel.x.abs() {
                    if rel.y > 0.0 { anim::FIGHT_STEP_N } else { anim::FIGHT_STEP_S }
                } else if rel.x > 0.0 { anim::FIGHT_STEP_E } else { anim::FIGHT_STEP_W };
                self.fig.play(step, Play::Loop);
                if !self.walk(c.wish.normalize_or_zero() * FIGHT_STEP_SPEED * dt, ground, map, false) {
                    self.fall(c.wish * FIGHT_STEP_SPEED);
                }
            } else {
                self.idle(true);
            }
        } else if moving {
            let dir = c.wish.normalize();
            self.fig.turn_towards(dir, 10.0, dt);
            let (speed, a, rate) = match (c.run, sprint) {
                (true, true) => (SPRINT_SPEED, anim::RUN, 1.35),
                (true, false) => (RUN_SPEED, anim::RUN, 1.0),
                _ => (WALK_SPEED, anim::WALK, 1.0),
            };
            if c.jump {
                self.jump(dir * speed);
                return;
            }
            // Running with a gun: the jog that keeps it up.
            let jog = match self.weapon {
                Weapon::Pistol => Some(anim::PISTOL_JOG),
                Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper => Some(anim::AK_JOG),
                _ => None,
            };
            if !(c.run && !c.sprint && jog.is_some_and(|j| self.fig.play(j, Play::Loop))) {
                self.fig.play(a, Play::Loop);
            }
            self.fig.set_speed(rate);
            let before = self.fig.pos;
            // Patrols go where their points are, even off a roof (The Jump's jumper does).
            // Following a route may mean dropping off an edge (`MAV_ACTION_FALL_OFF`).
            if !self.walk(dir * speed * dt, ground, map, self.player || self.goal.is_some() || self.via.is_some()) {
                self.fall(dir * speed);
                return;
            }
            // Blocked while walking into a fence: climb over. (Ledges are jumped onto.)
            let moved = vec2(self.fig.pos.x - before.x, self.fig.pos.z - before.z).length();
            if moved < speed * dt * 0.5 && self.fig.facing().dot(dir) > 0.7 {
                if let Some((i, at, over)) = ground.fence_ahead(self.fig.pos, dir, RADIUS + 30.0) {
                    let f = ground.fences[i];
                    self.climb_fence(at, over, f.y1);
                }
            }
        } else if c.jump {
            self.jump(Vec2::ZERO);
        } else {
            self.idle(false);
            // A gun drawn: keep it on the target.
            if let (true, Some(t)) = (self.weapon.is_gun(), c.target) {
                self.fig.turn_towards(flat(snaps[t].pos, self.fig.pos), 8.0, dt);
            }
            // Stand on whatever is underneath (something may have moved us).
            let p = self.fig.pos;
            let floor = ground.surface(map, p.x, p.z, p.y);
            if p.y - floor > STEP_HEIGHT {
                self.fall(Vec2::ZERO);
            } else {
                self.fig.pos.y = floor;
            }
        }
    }
}
