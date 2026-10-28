//! Getting about on foot: walking, running and sprinting, crouching and crawling, jumping, falling and landing.

use super::*;
use crate::tracks::Foot;

/// How close Darci has to be to kick between the legs (`dist < 100`, Person.cpp:9909), a
/// little more as our fighters stand a touch further apart.
const NAD_RANGE: f32 = 120.0;

/// How steep the ground is at (x, z), as `PAP_on_slope` has it (pap.cpp:459, halved as
/// `check_on_slippy_slope` takes it): the two height differences of the triangle of the
/// square that (x, z) is in, over 8, and the way down it. None under a building.
pub(crate) fn slope_at(map: &Map, x: f32, z: f32) -> Option<(f32, Vec2)> {
    let (mx, mz) = ((x / 256.0).floor() as i32, (z / 256.0).floor() as i32);
    if !(0..127).contains(&mx) || !(0..127).contains(&mz) {
        return None;
    }
    let (mx, mz) = (mx as usize, mz as usize);
    if map.square(mx, mz).has(uc_formats::iam::flags::HIDDEN) {
        return None;
    }
    let h = |a: usize, b: usize| map.corner_alt(a, b) as f32;
    let (h0, h1, h2, h3) = (h(mx, mz), h(mx, mz + 1), h(mx + 1, mz), h(mx + 1, mz + 1));
    let (fx, fz) = (x - mx as f32 * 256.0, z - mz as f32 * 256.0);
    let (slope, grad) = if fx + fz < 256.0 {
        (((h2 - h0).abs() + (h1 - h0).abs()) / 8.0, vec2(h2 - h0, h1 - h0))
    } else {
        (((h1 - h3).abs() + (h2 - h3).abs()) / 8.0, vec2(h3 - h1, h3 - h2))
    };
    Some((slope, -grad.normalize_or_zero()))
}

/// Ground steeper than this can't be stood on (`check_on_slippy_slope`'s `size`).
const SLIPPY: f32 = 50.0;

/// How fast someone slides down it: `Velocity` 50, at walking's 16 to `WALK_SPEED`.
const SLIP_SPEED: f32 = WALK_SPEED * 50.0 / 16.0;

impl Person {
    /// Standing on open ground too steep to stand on: the way down it.
    pub(super) fn slipping(&self, ground: &Ground, map: &Map) -> Option<Vec2> {
        let p = self.fig.pos;
        if (p.y - map.ground_height(p.x, p.z)).abs() > 16.0 || ground.surface(map, p.x, p.z, p.y + 8.0) > p.y + 16.0 {
            return None;
        }
        slope_at(map, p.x, p.z).filter(|(s, d)| *s > SLIPPY && d.length_squared() > 0.5).map(|(_, d)| d)
    }

    /// Flips to the side out of a bullet's way (`set_person_flip`, Person.cpp:7875).
    pub(crate) fn flip(&mut self, left: bool) {
        if self.fig.restart(if left { anim::FLIP_LEFT } else { anim::FLIP_RIGHT }, Play::Once) {
            self.state = State::Flip { left };
        }
    }

    /// `SUB_STATE_FLIPING` (Person.cpp:16273): sideways, at `Velocity` 50 from the second frame
    /// and 20 after the fifth, until the eleventh; then back on their feet.
    pub(super) fn step_flip(&mut self, left: bool, ground: &Ground, map: &Map, dt: f32) {
        if self.fig.finished() {
            self.state = State::Move;
            self.idle(true);
            return;
        }
        let frame = self.fig.frame().unwrap_or(0);
        let velocity = match frame {
            0 | 1 => 0.0,
            2..=5 => 50.0,
            _ => 20.0,
        };
        if frame < 11 && velocity > 0.0 {
            let f = self.fig.facing();
            let side = if left { vec2(-f.y, f.x) } else { vec2(f.y, -f.x) };
            self.walk(side * velocity * WALK_SPEED / 16.0 * dt, ground, map, false);
        }
    }

    /// `SUB_STATE_SLIPPING`: down the slope, facing down it, falling, until the ground is
    /// gentle enough to stand on (`SUB_STATE_SLIPPING_END`).
    pub(super) fn step_slip(&mut self, ground: &Ground, map: &Map, dt: f32) {
        let Some(down) = self.slipping(ground, map) else {
            self.state = State::Move;
            self.idle(false);
            return;
        };
        self.fig.turn_towards(down, 8.0, dt);
        let p = self.fig.pos;
        let to = ground.slide(vec2(p.x, p.z), p.y, down * SLIP_SPEED * dt, RADIUS);
        let floor = ground.surface(map, to.x, to.y, p.y + STEP_HEIGHT);
        self.fig.pos = vec3(to.x, floor, to.y);
    }
}

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

    /// A run's loop, and the push off into it. With a gun, short of sprinting, the jog that
    /// keeps it up. The player's run is the yomp, a bat's its own; sprinting it's the run,
    /// which is the AI's whatever the speed (`set_anim_running`, Person.cpp:8749), and the
    /// AI doesn't push off. Roper has no sprint and always yomps. `sprint_held` is whether
    /// they'd sprint, `sprint` whether they are (the player can be out of breath).
    pub(super) fn run_anims(&self, sprint_held: bool, sprint: bool) -> (usize, Option<usize>) {
        use uc_world::character::person as ptype;
        let roper = self.kind == ptype::ROPER;
        let yomps = self.player || roper;
        let (run, start) = match self.weapon {
            Weapon::Pistol if !sprint_held => (anim::PISTOL_JOG, anim::YOMP_START_PISTOL),
            Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper if !sprint_held => (anim::AK_JOG, anim::YOMP_START_AK),
            _ if !yomps || (sprint && !roper) => return (anim::RUN, None),
            Weapon::Bat => (anim::YOMP_BAT, anim::YOMP_START_BAT),
            _ => (anim::YOMP, anim::YOMP_START),
        };
        (run, yomps.then_some(start))
    }

    /// Standing about (`set_anim_idle`, Person.cpp:8800): with a two-handed gun, holding it,
    /// unless they're Roper; the women hand on hip; everyone else breathing.
    fn stand_anim(&self) -> usize {
        use uc_world::character::person as ptype;
        if matches!(self.weapon, Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper) && self.kind != ptype::ROPER {
            anim::SHOTGUN_IDLE
        } else if matches!(self.kind, ptype::SLAG_TART | ptype::SLAG_FATUGLY) || (self.kind == ptype::CIV && self.fig.model.body == 9) {
            anim::STAND_HIP
        } else {
            anim::BREATHE
        }
    }

    /// Walking (`set_anim_walking`, Person.cpp:8694): with a two-handed gun anyone but Darci
    /// and Roper carries it, and the gangs have a walk of their own.
    fn walk_anim(&self) -> usize {
        use uc_world::character::person as ptype;
        let two_handed = matches!(self.weapon, Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper);
        match self.kind {
            ptype::DARCI | ptype::ROPER => anim::WALK,
            _ if two_handed => anim::THUG_WALK_SHOTGUN,
            ptype::THUG_RASTA | ptype::THUG_RED | ptype::THUG_GREY => anim::THUG_WALK,
            _ => anim::WALK,
        }
    }

    pub(super) fn idle(&mut self, fight: bool) {
        let a = match (fight, self.weapon) {
            (_, w) if w.is_gun() && self.gun_out() => w.aim_anim(),
            (true, Weapon::Knife) => anim::KNIFE_FIGHT_READY,
            (true, Weapon::Bat) => anim::BAT_IDLE,
            (true, _) => anim::FIGHT,
            (false, _) => self.stand_anim(),
        };
        if !self.fig.play(a, Play::Loop) {
            self.fig.play(anim::BREATHE, Play::Loop);
        }
        self.fig.set_speed(1.0);
    }

    /// The aim pose turned `twist` (-255 to 255 of 2048) to the left or right: between the
    /// first frames of aiming ahead and pointing that way. Pistols and the rest point their
    /// own ways (the shotgun's left and right are the other way round).
    fn twist_to(&mut self, twist: f32) {
        let pistol = !matches!(self.weapon, Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper);
        let (ahead, left, right) = if pistol {
            (anim::PISTOL_AIM, anim::POINT_GUN_LEFT, anim::POINT_GUN_RIGHT)
        } else {
            (anim::SHOTGUN_AIM, anim::POINT_SHOTGUN_RIGHT, anim::POINT_SHOTGUN_LEFT)
        };
        let side = if twist < 0.0 { left } else { right };
        if let (Some(a), Some(b)) = (self.fig.first_keyframe(ahead), self.fig.first_keyframe(side)) {
            self.fig.show_keyframes(a, b, twist.abs().min(255.0));
        }
    }

    /// Standing still out of a fight: breathing, and every 400 to 655 turns a scratch or a
    /// deeper breath (`set_person_random_idle`, Person.cpp:11669; `fn_person_idle` counts
    /// down `Timer1`), unless something's in hand (`SpecialUse`). Roper scratches or
    /// breathes, half and half; the cops' own idles are in Roper's set of moves, which isn't
    /// loaded, so they just breathe.
    pub(super) fn stand_about(&mut self, dt: f32) {
        use uc_world::character::person as ptype;
        let fidgeting = matches!(self.fig.anim(), anim::IDLE_SCRATCH1 | anim::IDLE_SCRATCH2 | anim::TALK_ASK | anim::TALK_TELL | anim::TALK_LISTEN | anim::UNFIGHT);
        if fidgeting && !self.fig.finished() {
            return;
        }
        self.idle(false);
        if self.weapon != Weapon::None || self.kind == ptype::COP {
            return;
        }
        let mut roll = || {
            self.fidget_roll = self.fidget_roll.wrapping_mul(69069).wrapping_add(1);
            self.fidget_roll >> 16
        };
        // `set_person_idle` starts the count at 400 to 911, each fidget at 400 to 655.
        let left = match self.fidget {
            Some(t) => t - dt,
            None => ((roll() & 0x1ff) + 400) as f32 / 20.0,
        };
        if left > 0.0 {
            self.fidget = Some(left);
            return;
        }
        let r = roll();
        self.fidget = Some(((roll() & 0xff) + 400) as f32 / 20.0);
        let a = match self.kind {
            ptype::ROPER => if r & 1 != 0 { anim::BREATHE } else { anim::IDLE_SCRATCH1 },
            _ => [anim::IDLE_SCRATCH1, anim::IDLE_SCRATCH2, anim::BREATHE][(r % 3) as usize],
        };
        if a != anim::BREATHE {
            self.fig.restart(a, Play::Once);
        }
    }

    /// Saying a line of a conversation (`ask`ing or not), or hearing one, standing about
    /// (`PCOM_set_person_ai_talk_to`, pcom.cpp:4706). With a two-handed gun, unless they're
    /// Roper, they just hold it, as they stand anyway (`ANIM_SHOTGUN_IDLE`).
    pub(crate) fn talk(&mut self, speaking: bool, ask: bool) {
        use uc_world::character::person as ptype;
        let now = self.fig.anim();
        if !matches!(self.state, State::Move) || super::walking_anim(now) || super::running_anim(now) {
            return;
        }
        if matches!(self.weapon, Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper) && self.kind != ptype::ROPER {
            return;
        }
        let a = match (speaking, ask) {
            (false, _) => anim::TALK_LISTEN,
            (true, true) => anim::TALK_ASK,
            (true, false) => anim::TALK_TELL,
        };
        self.fig.restart(a, Play::Once);
    }

    /// Moves horizontally by `delta`, sliding along walls and following the floor. Returns
    /// false if it stepped off an edge (the caller falls). Without `drop_ok`, edges stop the
    /// person instead: the AI, and anyone fighting, never walks off a roof.
    pub(super) fn walk(&mut self, delta: Vec2, ground: &Ground, map: &Map, drop_ok: bool) -> bool {
        let p = &mut self.fig.pos;
        let (to, odd) = ground.slide_odd(vec2(p.x, p.z), p.y, delta, RADIUS);
        let to = uc_world::mav::keep_off_nogo(map, to);
        self.slid_odd = Some(self.slid_odd == Some(true) || odd);
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
        let (gun, two_handed) = self.armed();
        let a = match (vel.length() > 1.0, gun, two_handed) {
            (true, true, _) => anim::RUN_JUMP_LEFT_AK,
            (true, false, _) => anim::RUN_JUMP,
            (false, _, true) => anim::STANDING_JUMP_AK,
            (false, _, false) => anim::STANDING_JUMP,
        };
        if !self.fig.restart(a, Play::Once) {
            self.fig.restart(if vel.length() > 1.0 { anim::RUN_JUMP } else { anim::STANDING_JUMP }, Play::Once);
        }
        self.fig.set_speed(1.5);
    }

    pub(crate) fn fall(&mut self, vel: Vec2) {
        self.state = State::Fall { vy: 0.0, vel };
        self.fig.play(self.falling_anim(), Play::Loop);
    }

    /// Kick in the air off a running jump, and land it on the first one in the way
    /// (`SUB_STATE_FLYING_KICK`, Person.cpp:13610): 50 on top of a finisher's blow, and down
    /// they go (combat.cpp:1910).
    fn fly_kick(&mut self, running: bool, me: usize, c: &Controls, snaps: &[Snap], events: &mut Vec<Event>) {
        let jumping = matches!(self.fig.anim(), anim::RUN_JUMP | anim::RUN_JUMP_LEFT_AK | anim::MID_AIR | anim::MID_AIR_TWEEN_LEFT_AK);
        if c.kick && running && jumping && self.fly_kick.is_none() && self.fig.restart(anim::FLYKICK_START, Play::Once) {
            self.fly_kick = Some(false);
            self.cues.push(Sfx::Swing);
            events.push(Event::Kick { attacker: me });
        }
        if self.fly_kick != Some(false) {
            return;
        }
        let f = self.fig.facing();
        let hit = snaps.iter().enumerate().find(|&(j, s)| {
            let d = flat(s.pos, self.fig.pos);
            j != me && s.standing && self.opponents(j, s) && (s.pos.y - self.fig.pos.y).abs() < 120.0
                && d.length() < 150.0 && d.normalize_or_zero().dot(f) > 0.5
        });
        if let Some((j, _)) = hit {
            self.fly_kick = Some(true);
            let damage = if self.player { 80 } else { 80 * (8 + self.skill as i32) / 30 };
            events.push(Event::Hit { attacker: me, victim: j, damage, ko: ko::KNOCKDOWN, high: true, low: false, weapon: Weapon::None });
        }
    }

    /// A gun drawn, and whether it's a two-handed one (`person_has_gun_out`,
    /// `person_holding_2handed`).
    pub(super) fn armed(&self) -> (bool, bool) {
        let gun = self.weapon.is_gun() && self.gun_out();
        (gun, gun && matches!(self.weapon, Weapon::Shotgun | Weapon::Ak47 | Weapon::Sniper))
    }

    /// Falling, with a two-handed gun its own.
    pub(super) fn falling_anim(&self) -> usize {
        if self.armed().1 { anim::FALLING_AK } else { anim::FALLING }
    }

    /// Advances the animation, then places anything that follows it: a climber's height on a
    /// ladder goes up a rung cycle as the climbing animation loops back down.
    pub fn advance(&mut self, dt: f32, ground: &Ground) {
        self.landed_for = (self.landed_for - dt).max(0.0);
        self.fig.advance(dt);
        self.flash = (self.flash - dt).max(0.0);
        // Footsteps: two per walk or run cycle.
        let walking = matches!(self.state, State::Move | State::Land { .. })
            && matches!(self.fig.anim(), anim::WALK | anim::THUG_WALK | anim::THUG_WALK_SHOTGUN | anim::RUN | anim::YOMP | anim::YOMP_BAT | anim::PISTOL_JOG | anim::AK_JOG | anim::FIGHT_STEP_N | anim::FIGHT_STEP_S | anim::FIGHT_STEP_E | anim::FIGHT_STEP_W
                | anim::FIGHT_STEP_N_BAT | anim::FIGHT_STEP_S_BAT | anim::FIGHT_STEP_E_BAT | anim::FIGHT_STEP_W_BAT);
        let phase = self.fig.loops().floor() + self.fig.progress();
        if walking && (phase * 2.0).floor() != (self.step_phase * 2.0).floor() {
            self.cues.push(Sfx::Footstep { player: self.player, sound: None });
            // Walking and running leave prints, the right foot then the left (Person.cpp:15457),
            // but not the steps of a fight.
            if matches!(self.fig.anim(), anim::WALK | anim::THUG_WALK | anim::THUG_WALK_SHOTGUN | anim::RUN | anim::YOMP | anim::YOMP_BAT | anim::PISTOL_JOG | anim::AK_JOG) {
                let right = (phase * 2.0).floor() as i64 & 1 == 0;
                self.footfall = Some(if right { Foot::Right } else { Foot::Left });
            }
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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_fall(&mut self, mut vy: f32, mut vel: Vec2, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        vy -= GRAVITY * dt;
        let running = vel.length() > 100.0;
        self.fly_kick(running, me, c, snaps, events);
        if self.fly_kick.is_some() {
            if self.fig.anim() == anim::FLYKICK_START && self.fig.finished() {
                self.fig.play(anim::FLYKICK_FALL, Play::Loop);
            }
        } else if self.fig.finished() && !matches!(self.fig.anim(), anim::PLUNGE_START | anim::PLUNGE_FORWARDS) {
            let a = match (running, self.armed().0) {
                (true, true) => anim::MID_AIR_TWEEN_LEFT_AK,
                (true, false) => anim::MID_AIR,
                (false, _) if self.fig.anim() == anim::STANDING_JUMP => anim::FALLING_QUEUED,
                (false, _) if self.fig.anim() == anim::STANDING_JUMP_AK => anim::FALLING_QUEUED_AK,
                (false, _) => self.falling_anim(),
            };
            self.fig.play(a, Play::Loop);
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
                // A moving face's edge is pulled up over at once (Person.cpp:13009).
                if matches!(self.state, State::Dangle { .. }) && ground.moving_at(to).is_some() {
                    self.climb_onto(anim::PULL_UP, to, false);
                }
                return;
            }
        }
        // Falling to their death (Darci.cpp:997): falling faster than 12000 a tick for the
        // player, 6000 for anyone else, with no ground within 0x600 under three points along
        // the way they're going, they dive head first (`ANIM_PLUNGE_START`) and drop
        // straight down, screaming a couple of frames in.
        let plunging = matches!(self.fig.anim(), anim::PLUNGE_START | anim::PLUNGE_FORWARDS);
        let death = if self.player { 12000.0 } else { 6000.0 } / 256.0 * 20.0;
        if -vy > death {
            let p = self.fig.pos;
            if plunging {
                if self.fig.anim() == anim::PLUNGE_START && self.fig.finished() {
                    self.fig.play(anim::PLUNGE_FORWARDS, Play::Loop);
                }
                if !self.screamed && self.fig.frame().unwrap_or(0) >= 2 {
                    self.screamed = true;
                    self.cues.push(Sfx::ScreamFall { kind: self.kind });
                }
                vel = Vec2::ZERO;
            } else {
                let ahead = vel / 20.0 * 8.0;
                let safe = (1..4).any(|i| {
                    let q = vec2(p.x, p.z) + ahead * i as f32;
                    ground.surface(map, q.x, q.y, p.y) >= p.y - 0x600 as f32
                });
                if !safe {
                    self.fig.restart_locked(anim::PLUNGE_START, Play::Once);
                    self.screamed = false;
                }
            }
        }
        let p = self.fig.pos;
        let to = uc_world::mav::keep_off_nogo(map, ground.slide(vec2(p.x, p.z), p.y, vel * dt, RADIUS));
        let y = p.y + vy * dt;
        let floor = ground.surface(map, to.x, to.y, p.y);
        if y <= floor {
            self.fig.pos = vec3(to.x, floor, to.y);
            self.fig.lift = (floor - p.y).max(0.0);
            self.state = State::Land { vel };
            // `PCOM_SOUND_DROP_BIG`, `_MED` or `_DROP` by how hard they hit (`DY` under -20000,
            // -10000 or -5000 a tick, Darci.cpp:444), or nothing.
            let dy = -vy / 20.0 * 256.0;
            self.landed = if dy > 20000.0 { 0x600 as f32 } else if dy > 10000.0 { 0x400 as f32 } else if dy > 5000.0 { 0x200 as f32 } else { 0.0 };
            self.landed_for = 0.2;
            self.cues.push(Sfx::Land);
            // Both feet come down (`person_splash(p_person, -1)`).
            self.footfall = Some(Foot::Both);
            let held = c.wish.length_squared() > 0.25;
            let a = match (running, self.armed().0) {
                _ if self.fly_kick.take().is_some() => anim::FLYKICK_LAND,
                (true, _) if !held && self.player => anim::LAND_STAND,
                (true, true) => anim::LAND_RIGHT_AK,
                (true, false) => anim::LAND_RIGHT,
                (false, _) if dy > 15000.0 => anim::BIG_LAND,
                (false, _) => anim::LAND_VERT,
            };
            self.fig.restart(a, Play::Once);
            self.fig.set_speed(if running { 1.5 } else { 2.0 });
            // Landing hard hurts, and from high enough kills (fallen/Source/Darci.cpp).
            let damage = fall_damage(-vy);
            if damage > 0 {
                self.fig.lift = 0.0;
                let ko = if damage >= self.health { ko::KNOCKDOWN } else { 0 };
                self.take_hit(self.fig.pos + vec3(0.0, 0.0, 1.0), damage, ko, false, true, Weapon::None);
                // A fall that kills (`PERSON_DEATH_TYPE_LAND`, Person.cpp:2559): face down
                // in a splash of blood from a plunge (`ANIM_PLUNGE_FRONT_SLAM`,
                // `PYRO_SPLATTERY`), else `ANIM_BIGLAND_DIE`.
                if self.knocked_out {
                    if plunging {
                        self.fig.restart_locked(anim::PLUNGE_FRONT_SLAM, Play::Once);
                        self.splattered = true;
                    } else {
                        self.fig.restart(anim::BIGLAND_DIE, Play::Once);
                    }
                }
            }
        } else {
            self.fig.pos = vec3(to.x, y, to.y);
            self.state = State::Fall { vy, vel };
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_move(&mut self, me: usize, c: &Controls, snaps: &[Snap], ground: &Ground, map: &Map, dt: f32, events: &mut Vec<Event>) {
        if c.switch && self.player && self.switch_weapon() {
            return;
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
        // Climb a ladder by walking into it, or with action next to it: stepping on first
        // at its foot (`ANIM_MOUNT_LADDER`).
        if let Some(l) = ground.ladder_near(self.fig.pos, 90.0) {
            let into = c.wish.dot(-ground.ladders[l].out) > 0.7;
            if into || c.action {
                self.fig.pos = ground.on_ladder(l, self.fig.pos.y);
                self.fig.angle = uc_world::character::angle_of(-ground.ladders[l].out);
                if self.fig.restart(anim::MOUNT_LADDER, Play::Once) {
                    self.state = State::Mount { ladder: l };
                } else {
                    self.fig.restart(anim::ON_LADDER, Play::Loop);
                    self.state = State::Climb { ladder: l, y0: self.fig.pos.y, loops0: self.fig.loops() };
                }
                return;
            }
        }
        if c.block {
            // A bat or a shotgun in hand guards with it.
            let a = match self.weapon {
                Weapon::Bat => anim::BAT_BLOCK,
                Weapon::Shotgun => anim::SHOTGUN_BLOCK,
                _ => anim::BLOCK_HIGH,
            };
            if !self.fig.restart(a, Play::Once) {
                self.fig.restart(anim::BLOCK_HIGH, Play::Once);
            }
            self.state = State::Block { auto: false };
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
        // Action standing in front of a building's wall: back to it (interfac.cpp:2063,
        // before crouching).
        if self.player && c.action && !moving && !c.crouch {
            if let Some((a, b, out)) = ground.wall_to_hug(map, self.fig.pos, self.fig.facing()) {
                if self.start_hug(a, b, out) {
                    return;
                }
            }
        }
        // Action standing bare-handed by a can: pick it up (interfac.cpp:2074).
        if self.player && c.action && !moving && !c.crouch && self.can_near && self.can.is_none() && self.weapon == Weapon::None
            && self.fig.restart(anim::CAN_PICKUP, Play::Once)
        {
            self.state = State::PickUp { got: false };
            return;
        }
        // (The AI only ever crouches to crawl after someone who is.)
        if (self.player && c.action && !moving) || c.crouch {
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
        // Punch with a can in hand throws it (`player_activate_in_hand`, interfac.cpp:668).
        if c.punch && self.can.is_some() {
            if self.fig.restart(anim::CAN_RELEASE, Play::Once) {
                self.state = State::Throw { released: false };
            } else {
                events.push(Event::Throw { thrower: me });
            }
            return;
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
            // Someone lying knocked down in front is stamped on (`turn_to_target_and_kick`,
            // combat.cpp:3488).
            let f = self.fig.facing();
            let lying = snaps.iter().enumerate()
                .filter(|&(j, s)| j != me && s.floored && self.opponents(j, s) && (s.pos.y - self.fig.pos.y).abs() < 100.0)
                .map(|(j, s)| (j, flat(s.pos, self.fig.pos)))
                .filter(|(_, d)| d.length() < 130.0 && d.normalize_or_zero().dot(f) > 0.3)
                .min_by(|a, b| a.1.length().total_cmp(&b.1.length()))
                .map(|(j, _)| j);
            if let Some(t) = lying {
                self.face(snaps[t].pos);
                self.start_attack(me, Move::Stomp, events);
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
                // Someone within 120 gets the near kick, not a combo's (`turn_to_target_and_kick`,
                // combat.cpp:3493).
                let near = target.is_some_and(|t| flat(snaps[t].pos, self.fig.pos).length() < 120.0);
                if near { Move::KickNear } else { Move::Kick1 }
            };
            self.start_attack(me, mv, events);
            return;
        }

        if fight {
            // Fight stance: face the opponent and step without turning.
            let t = snaps[target.unwrap()].pos;
            let d = flat(t, self.fig.pos);
            self.fig.turn_towards(d, 8.0, dt);
            // Jump: to the side a flip, with no way held a jump kick (interfac.cpp:5070, 5096).
            if c.jump && self.player {
                if moving && rel.x.abs() > rel.y.abs() {
                    // `rel.x` is along the way a flip to the left goes (`step_flip`).
                    self.flip(rel.x > 0.0);
                    return;
                }
                if !moving {
                    self.start_attack(me, Move::KickJump, events);
                    return;
                }
            }
            // Stepping back from someone swinging at her: the auto-block (`should_i_block`,
            // combat.cpp:951: only on the back step, so not with a bat, whose step is another
            // animation). Backing away from them, that is, as the manual has it: not stepping
            // towards someone behind her to kick them.
            let swung_at = snaps.iter().enumerate().any(|(j, s)| {
                let d = flat(self.fig.pos, s.pos);
                j != me && s.attacking && self.opponents(j, s) && d.length() < 220.0 && s.facing.dot(d.normalize_or_zero()) > 0.5
                    && c.wish.normalize_or_zero().dot(d.normalize_or_zero()) > 0.5
            });
            if self.player && moving && rel.y < -0.5 && rel.y.abs() >= rel.x.abs() && swung_at && self.weapon != Weapon::Bat
                && self.fig.restart(anim::BLOCK_HIGH, Play::Once)
            {
                self.state = State::Block { auto: true };
                return;
            }
            if moving {
                let bat = self.weapon == Weapon::Bat;
                let step = match (rel.y.abs() >= rel.x.abs(), rel.y > 0.0, rel.x > 0.0, bat) {
                    (true, true, _, false) => anim::FIGHT_STEP_N,
                    (true, false, _, false) => anim::FIGHT_STEP_S,
                    (false, _, true, false) => anim::FIGHT_STEP_E,
                    (false, _, false, false) => anim::FIGHT_STEP_W,
                    (true, true, _, true) => anim::FIGHT_STEP_N_BAT,
                    (true, false, _, true) => anim::FIGHT_STEP_S_BAT,
                    (false, _, true, true) => anim::FIGHT_STEP_E_BAT,
                    (false, _, false, true) => anim::FIGHT_STEP_W_BAT,
                };
                self.fig.play(step, Play::Loop);
                if !self.walk(c.wish.normalize_or_zero() * FIGHT_STEP_SPEED * dt, ground, map, false) {
                    self.fall(c.wish * FIGHT_STEP_SPEED);
                }
            } else {
                // The player squares up from standing about (`ANIM_FIGHT_READY`).
                let now = self.fig.anim();
                if now == anim::FIGHT_READY && !self.fig.finished() {
                    return;
                }
                let about = super::idle_anim(now) || matches!(now, anim::IDLE_SCRATCH1 | anim::IDLE_SCRATCH2 | anim::UNFIGHT);
                if !(self.player && about && self.fig.restart(anim::FIGHT_READY, Play::Once)) {
                    self.idle(true);
                }
            }
        } else if moving {
            let dir = c.wish.normalize();
            self.fig.turn_towards(dir, 10.0, dt);
            let (speed, rate) = match (c.run, sprint) {
                (true, true) => (SPRINT_SPEED, 1.35),
                (true, false) => (RUN_SPEED, 1.0),
                _ => (WALK_SPEED, 1.0),
            };
            if c.jump {
                self.jump(dir * speed);
                return;
            }
            // The AI held up a while stands there for up to three seconds, then tries again.
            if !self.player && self.blocked > 0.5 {
                self.blocked += dt;
                if self.blocked < 3.0 {
                    self.idle(false);
                    return;
                }
                (self.blocked, self.walk_check) = (0.0, (self.fig.pos, 0.0));
            }
            if c.run {
                // Pushing off from standing or walking, not out of a running landing or a
                // vault, which keep going.
                let (run, start) = self.run_anims(c.sprint, sprint);
                let now = self.fig.anim();
                let pushing = start == Some(now) && !self.fig.finished();
                let push = !running_anim(now) && !matches!(now, anim::LAND_RIGHT | anim::LAND_RIGHT_AK | anim::VAULT);
                if !pushing && !(push && start.is_some_and(|s| self.fig.restart(s, Play::Once))) && !self.fig.play(run, Play::Loop) {
                    self.fig.play(anim::RUN, Play::Loop);
                }
            } else if !self.fig.play(self.walk_anim(), Play::Loop) {
                self.fig.play(anim::WALK, Play::Loop);
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
                // The player running into a wall half a storey (128) high gets up onto it
                // (`is_facet_half_step`, Person.cpp:11281; `set_person_climb_half`).
                let half = self.ledge_ahead(dir, 136.0, ground, map).filter(|to| (to.y - self.fig.pos.y - 128.0).abs() < 8.0);
                if let (true, true, Some(to)) = (self.player, c.run, half) {
                    self.climb_onto(anim::GET_UP_HALF_BLOCK, to, true);
                    if matches!(self.state, State::PullUp { .. }) {
                        return;
                    }
                }
                if let Some((i, at, over)) = ground.fence_ahead(self.fig.pos, dir, RADIUS + 30.0) {
                    let f = ground.fences[i];
                    // A fence two blocks of 16 high, at a run: vaulted, not climbed
                    // (`is_facet_vaultable`, Person.cpp:11265; `set_person_vault`), unless the
                    // far side is somewhere nobody may go.
                    let far = at + over * (RADIUS + 40.0);
                    let nogo = map.square((far.x / 256.0) as usize % 128, (far.y / 256.0) as usize % 128).has(uc_formats::iam::flags::NOGO);
                    if c.run && ((f.y1 - f.y0) - 128.0).abs() < 8.0 && (self.fig.pos.y - f.y0).abs() < 32.0 && !nogo
                        && self.fig.restart(anim::VAULT, Play::Once)
                    {
                        self.fig.angle = uc_world::character::angle_of(over);
                        self.state = State::Vault { from: self.fig.pos, to: vec3(far.x, self.fig.pos.y, far.y) };
                        return;
                    }
                    self.climb_fence(at, over, f.y1);
                    return;
                }
            }
            // Getting nowhere (the AI): under a quarter of the way they'd walk, over half a
            // second, counting what pushed them back (a car they're walking into shoves
            // them out of it). Strolling to a spot of their own, it's measured along the way
            // they're going, so being shoved back and popping out again doesn't count as
            // getting anywhere; on a route or after someone, sliding along what's in the way
            // counts (they jump it after a while).
            if !self.player {
                let (from, t) = self.walk_check;
                let t = t + dt;
                if t >= 0.5 {
                    let d = flat(self.fig.pos, from);
                    let net = if self.goal.is_none() && self.via.is_none() { d.dot(dir.normalize_or_zero()) } else { d.length() };
                    self.blocked = if net < speed * t * 0.25 { self.blocked + t } else { 0.0 };
                    self.walk_check = (self.fig.pos, 0.0);
                } else {
                    self.walk_check = (from, t);
                }
            }
        } else if c.jump {
            self.jump(Vec2::ZERO);
        } else {
            // The player drops out of a fight stance with nobody left to fight (`ANIM_UNFIGHT`).
            if self.player && matches!(self.fig.anim(), anim::FIGHT | anim::FIGHT_READY | anim::KNIFE_FIGHT_READY | anim::BAT_IDLE) {
                self.fig.restart(anim::UNFIGHT, Play::Once);
            }
            self.stand_about(dt);
            // A gun drawn: keep it on the target. The player's upper body twists to it
            // (`twist_darci_body_to_angle`, Person.cpp:17836), seven eighths of the way, up to
            // 255 of 2048 either side; she only turns to it when it's further round than that.
            if let (true, Some(t)) = (self.weapon.is_gun(), c.target) {
                let to = flat(snaps[t].pos, self.fig.pos);
                let off = (uc_world::character::angle_of(to) - self.fig.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                let twist = off * 2048.0 / std::f32::consts::TAU * 7.0 / 8.0;
                if !self.player || twist.abs() > 255.0 {
                    self.fig.turn_towards(to, 8.0, dt);
                } else {
                    self.twist_to(twist);
                }
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
