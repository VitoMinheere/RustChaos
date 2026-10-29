//! The AI: noticing people, deciding what to do, wandering and running scared.

use super::*;

impl Person {
    /// A noise at `at` (`PCOM_oscillate_tympanum`): anyone hostile who isn't already after the
    /// player turns to look (`PCOM_set_person_ai_investigate`).
    pub fn hear(&mut self, at: Vec3) {
        if self.hostile() && !self.aggro && !self.removed && !self.is_dead() && !self.is_down() {
            if self.heard.is_none() && self.ai == AI_GUARD {
                self.cues.push(Sfx::Curious { kind: self.kind });
            }
            // The sound's square, its middle (`pcom_ai_arg` keeps the mapsquare).
            let at = Vec3::new((at.x / 256.0).floor() * 256.0 + 128.0, at.y, (at.z / 256.0).floor() * 256.0 + 128.0);
            self.heard = Some(Heard { at, phase: Investigate::Surprised, time: 0.0 });
        }
    }

    /// `PCOM_process_investigating` (pcom.cpp:7768): a second turned to the sound, then, unless
    /// they can see where it came from from here, a walk over to it (to within 0x40), and
    /// there a look round: a quarter turn left after one second, half a turn right after three,
    /// and after five, half the time back to what they were doing, which takes them home, and
    /// half the time on to somewhere someone might be hiding (pcom.cpp:7935), to look round
    /// there in turn.
    fn investigate(&mut self, mut h: Heard, mut c: Controls, ground: &Ground, dt: f32, rand: &mut impl FnMut() -> u32) -> Controls {
        let before = h.time;
        h.time += dt;
        let to = flat(h.at, self.fig.pos);
        match h.phase {
            Investigate::Surprised => {
                self.fig.turn_towards(to, 6.0, dt);
                if h.time > 1.0 {
                    // From head height to a little over where it came from (0x60 + 0x80 up).
                    let seen = ground.line_of_sight(self.fig.pos + vec3(0.0, 0x60 as f32, 0.0), h.at + vec3(0.0, 0xe0 as f32, 0.0));
                    if seen {
                        self.heard = None;
                        return c;
                    }
                    h = Heard { phase: Investigate::WalkOver, time: 0.0, ..h };
                }
            }
            Investigate::WalkOver => {
                if to.length() < 0x40 as f32 || h.time > 20.0 || self.blocked > 2.0 {
                    h = Heard { phase: Investigate::Look, time: 0.0, ..h };
                } else {
                    c.wish = to.normalize_or_zero();
                }
            }
            Investigate::Look => {
                let quarter = std::f32::consts::FRAC_PI_2;
                if before < 1.0 && h.time >= 1.0 {
                    self.fig.angle -= quarter;
                }
                if before < 3.0 && h.time >= 3.0 {
                    self.fig.angle += 2.0 * quarter;
                }
                if h.time > 5.0 {
                    if rand() & 1 == 0 {
                        self.heard = None;
                        return c;
                    }
                    h = Heard { phase: Investigate::Hide, time: 0.0, ..h };
                }
            }
            // Waiting on the game to find somewhere.
            Investigate::Hide => {}
        }
        self.heard = Some(h);
        c
    }

    /// `can_a_see_b` (Person.cpp:3198), with its range capped at `range`: within it, half that
    /// if they're crouching; in front within 700 of 2048 either way up close (0xc0), 420
    /// further off (`PEOPLE_FOV`); and in view head to head, a crouched head being lower.
    /// Seconds to the next shot: sooner the more skilled. `PCOM_AI_SHOOT_DEAD` waits for
    /// nothing (pcom.cpp:8622, 8973), so the gun goes off as fast as it can be fired.
    fn shot_interval(&self, skill: f32, rand: &mut impl FnMut() -> u32) -> f32 {
        if self.ai == 21 {
            return 0.3;
        }
        ((2.4 - skill * 0.1).max(0.8) + (rand() % 100) as f32 / 100.0) * self.shot_wait
    }

    /// Called in by a Man in Black: off after the player, if they're the sort who'd fight
    /// and aren't already (`PCOM_alert_nearby_mib_to_attack` sends the Men in Black, guards,
    /// gang members and fight-test dummies who are going about their business, home or
    /// looking round, to `PCOM_set_person_ai_navtokill`).
    pub(crate) fn called_in(&mut self) {
        if self.player || self.removed || self.is_down() || self.is_dead() || self.aggro || self.truce || self.scared.is_some()
            || !matches!(self.ai, AI_MIB | AI_GUARD | AI_GANG | AI_FIGHT_TEST)
        {
            return;
        }
        self.aggro = true;
        self.heard = None;
        self.cues.push(Sfx::Alert { kind: self.kind });
        self.attack_timer = self.attack_timer.max(0.8);
    }

    /// Sees `p` as `can_a_see_b` does with its own range, 8 squares.
    pub(crate) fn sees(&self, p: &Snap, ground: &Ground) -> bool {
        self.can_see(p, 8.0 * 256.0, ground)
    }

    /// A noise the game made (`PCOM_oscillate_tympanum`, pcom.cpp:13240): `seen` whether they
    /// can see where it was. Civilians run from a killer's "what you looking at"; guards,
    /// gang members and cops go to look at a "Hey!" or that; a grenade landing sends whoever
    /// sees it running (those that can), and the gang and cops who can't go to look.
    pub(crate) fn hear_noise(&mut self, noise: Noise, at: Vec3, by: Option<usize>, seen: bool) {
        if self.player || self.removed || self.is_dead() || self.is_down() {
            return;
        }
        match noise {
            Noise::GrenadeHit if seen => self.scare(None, at),
            Noise::GrenadeHit => {
                if matches!(self.ai, AI_GANG | AI_COP) {
                    self.hear(at);
                }
            }
            Noise::LookingAtMe if self.ai == AI_CIV => self.scare(by, at),
            Noise::Hey | Noise::LookingAtMe => {
                if matches!(self.ai, AI_GUARD | AI_GANG | AI_COP) {
                    self.hear(at);
                }
            }
        }
    }

    /// Turned to look at something they heard (`PCOM_AI_STATE_INVESTIGATING`).
    pub(crate) fn listening(&self) -> bool {
        self.heard.is_some()
    }

    /// A gang mate's fight: off after `victim` if they're free to (not down, frightened or
    /// held to a truce).
    pub(crate) fn join_fight(&mut self, victim: usize) {
        if self.player || self.removed || self.is_down() || self.is_dead() || self.truce || self.scared.is_some() || !self.hostile() {
            return;
        }
        self.bully_after(victim);
    }

    /// A bully off after `victim`: noticed, and out to get them.
    pub(crate) fn bully_after(&mut self, victim: usize) {
        self.victim = Some(victim);
        if !self.aggro {
            self.aggro = true;
            self.cues.push(Sfx::Alert { kind: self.kind });
            self.attack_timer = self.attack_timer.max(0.8);
        }
    }

    pub(super) fn can_see(&self, p: &Snap, range: f32, ground: &Ground) -> bool {
        let d = flat(p.pos, self.fig.pos);
        let dist = d.length();
        let view = if p.crouching { range / 2.0 } else { range };
        if dist > view || (p.pos.y - self.fig.pos.y).abs() > 256.0 {
            return false;
        }
        let ahead = self.fig.facing().dot(d / dist.max(1.0));
        // cos(700 / 2048 of a turn) and cos(420 / 2048).
        let cone = if dist < 0xc0 as f32 { -0.54 } else { 0.28 };
        let head = |crouch: bool| vec3(0.0, if crouch { 0x20 as f32 } else { 0x60 as f32 }, 0.0);
        ahead > cone && ground.line_of_sight(self.fig.pos + head(self.crouching()), p.pos + head(p.crouching))
    }

    /// Whose blows can land on whom: the player can hit anybody, the AI only its foe, and
    /// only once it's noticed them (else anyone hostile turned to face the player as she came
    /// near, back turned or not).
    pub(super) fn opponents(&self, j: usize, other: &Snap) -> bool {
        if self.player { !other.player } else { self.fights() && self.foe == Some(j) && (self.aggro || self.is_dummy()) }
    }

    /// The nearest standing opponent within `range`.
    pub(super) fn nearest_opponent(&self, me: usize, snaps: &[Snap], range: f32) -> Option<usize> {
        snaps
            .iter()
            .enumerate()
            .filter(|&(j, s)| j != me && s.standing && self.opponents(j, s) && (!self.player || s.fighter)
                && (s.pos.y - self.fig.pos.y).abs() < 128.0)
            .map(|(j, s)| (j, flat(s.pos, self.fig.pos).length()))
            .filter(|&(_, d)| d < range)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(j, _)| j)
    }

    /// AI: decide this frame's controls, against its `foe` (see `Person::foe`).
    pub fn think(&mut self, me: usize, snaps: &[Snap], ground: &Ground, dt: f32,
                 rand: &mut impl FnMut() -> u32) -> Controls {
        // Sitting down, something needing them (a fight, a fright) gets them up.
        // A pose ends too when their move changes to one without it (a hostage set to follow).
        let posing = matches!(self.mv, MOVE_DANCE | MOVE_HANDS_UP | MOVE_TIED_UP);
        if (matches!(self.state, State::Sit | State::Pose { .. }) && (self.aggro || self.scared.is_some()))
            || (matches!(self.state, State::Pose { .. }) && !posing)
        {
            self.fidget = None;
            self.state = State::Move;
            self.idle(false);
        }
        let mut c = self.decide(me, snaps, ground, dt, rand);
        // At a route's jump: run at the edge and jump; at a ladder, walk into it.
        if let Some((at, dir, jump)) = self.leap {
            let near = flat(at, self.fig.pos).length() < 64.0;
            // Even once they're as good as at the edge, where they'd stop walking.
            let going = self.goal.is_some() || self.via.is_some();
            if near && going && matches!(self.state, State::Move) {
                c.wish = dir;
                (c.run, c.jump) = (jump, jump);
            }
        }
        // `SlideOdd`: slid along a prop, a person or a car for 50 turns in a row on the way
        // somewhere. "EMERGENCY! Take evasive action": a running jump the way they're going
        // (pcom.cpp:12428). Wandering civilians don't jump.
        let civ = self.ai == AI_CIV && self.mv == MOVE_WANDER && !self.aggro && self.scared.is_none();
        let going = self.goal.is_some() || self.via.is_some();
        if self.slide_odd >= 50 && going && !civ && c.wish.length_squared() > 0.01 && matches!(self.state, State::Move) {
            (c.run, c.jump) = (true, true);
            self.slide_odd = 0;
        }
        // On a ladder: up it and off the top, rather than hang about halfway (routes only
        // ever go up ladders, `MAV_ACTION_LADDER_UP`).
        if let State::Climb { ladder, .. } = self.state {
            c.wish = -ground.ladders[ladder].out;
        }
        // "MIB are too cool to run anywhere" (Person.cpp:19913).
        if self.is_mib() {
            (c.run, c.sprint) = (false, false);
        }
        c
    }

    /// Someone with a gun out about to shoot their foe, who they can see within range: how
    /// near the shot is, 0 (now) to 100 (`track_gun_sight(p_target, shoot_time - counter)`,
    /// pcom.cpp:8946, with our countdown at twenty to the second).
    pub fn about_to_shoot(&self) -> Option<f32> {
        (self.aggro && self.weapon.is_gun() && !self.player && matches!(self.state, State::Move | State::Shoot { .. }))
            .then(|| (self.attack_timer.max(0.0) * 20.0).min(100.0))
    }

    /// One of the Men in Black (`PersonIsMIB`, Person.cpp:498).
    pub fn is_mib(&self) -> bool {
        use uc_world::character::person as ptype;
        matches!(self.kind, ptype::MIB1 | ptype::MIB2 | ptype::MIB3)
    }

    /// One of the Men in Black dodging a blow (Person.cpp:1970): "Men In Black jump to avoid
    /// the attack like ninjas!" On their feet, they jump straight up and it misses; returns
    /// whether they did.
    pub fn dodge(&mut self) -> bool {
        if !self.is_mib() {
            return false;
        }
        match self.state {
            // Already in the air: it passes beneath.
            State::Fall { .. } | State::Land { .. } => true,
            State::Move | State::Attack { .. } | State::Shoot { .. } | State::Block { .. } => {
                self.jump(Vec2::ZERO);
                true
            }
            _ => false,
        }
    }

    pub(super) fn decide(&mut self, me: usize, snaps: &[Snap], ground: &Ground, dt: f32,
              rand: &mut impl FnMut() -> u32) -> Controls {
        let player = self.foe;
        // Only someone who has noticed their foe has them as a target: a gun drawn is kept on
        // the target, so otherwise anyone armed would turn to face the player before they
        // had seen her.
        let mut c = Controls { target: player.filter(|_| self.aggro || self.is_dummy()), ..Default::default() };
        // A car coming: run out of its way (`PCOM_find_runover_thing`).
        if let Some((dir, left)) = self.dodge {
            self.dodge = (left > dt).then_some((dir, left - dt));
            c.wish = dir;
            c.run = true;
            return c;
        }
        // Frightened: run until far enough from it, fifteen squares x plus z, or three once
        // they've been running half a minute (`PCOM_AI_STATE_FLEE_PERSON` in
        // `PCOM_process_state_change`, fallen/Source/pcom.cpp). Then back to what they were
        // doing.
        if let Some(s) = self.scared.as_mut() {
            s.time += dt;
            let d = flat(self.fig.pos, s.at);
            let far = if s.time >= 30.0 { 3.0 * 256.0 } else { 15.0 * 256.0 };
            if d.x.abs() + d.y.abs() > far {
                self.scared = None;
                self.wander = (None, 0.0);
                self.sized_up = false;
            } else {
                // Along the way the game found to somewhere away from it, or just away.
                let to = self.via.or(s.goal).map(|v| flat(v, self.fig.pos)).unwrap_or(d);
                c.wish = to.try_normalize().unwrap_or(self.fig.facing());
                c.run = true;
                return c;
            }
        }
        // Off for a weapon lying near (`PCOM_AI_STATE_GETITEM`, `PCOM_process_getitem`): there
        // at a run, fight or no fight, and then back to what they were doing.
        if let Some(at) = self.fetch {
            let d = flat(at, self.fig.pos);
            if d.length() > 40.0 {
                c.wish = d.normalize();
                c.run = true;
            }
            return c;
        }
        // Nobody to fight: walk the patrol, follow whoever is being followed, or stroll.
        // Dropping `aggro` matters as much as the walking does: the game only gives someone
        // somewhere to go while they aren't in a fight, so a bodyguard whose man went down
        // would otherwise stand there for good.
        let Some(p) = player.map(|p| snaps[p]) else {
            self.aggro = false;
            return self.wander(c, ground, dt, rand);
        };
        let d = flat(p.pos, self.fig.pos);
        let dist = d.length();
        let dy = (p.pos.y - self.fig.pos.y).abs();
        // Which way to go to close in: round whatever is in the way, if there's a way round.
        let towards = self.via.map_or(d, |v| flat(v, self.fig.pos)).normalize_or_zero();
        // PCOM_AI_FLEE_PLAYER keeps its distance (until hit: then it fights, see take_hit).
        if self.ai == 16 && p.present && !p.down {
            if dist < 6.0 * 256.0 {
                c.wish = -d.normalize_or_zero();
                c.run = true;
            }
            return c;
        }
        // No point having a fist fight with a car: someone without a gun taunts its driver
        // instead (`PCOM_process_navtokill`, pcom.cpp:7518 and 8814).
        if self.fights() && self.aggro && p.present && p.in_car && !self.weapon.is_gun() && dist < self.aggro_range() && self.can_taunt() {
            self.taunt.get_or_insert(Taunt::Now);
        } else if !self.fights() || !p.present || p.down || p.in_car {
            self.aggro = false;
            self.taunt = None;
            return self.wander(c, ground, dt, rand);
        }
        let range = self.aggro_range();
        let hunter = range > 32.0 * 256.0;
        // A gang member notices the player by seeing them (`PCOM_can_i_see_person_to_taunt`
        // is `can_a_see_b`), or turns to look on hearing them (running feet, a landing, a
        // fight) and may see them then; others still by being near.
        let gang = self.ai == AI_GANG && !hunter;
        // A Man in Black sees the player, out to 8 squares (`can_a_see_b`'s own range), and
        // whoever's near comes for her; he hears her too, and calls in those near *her*
        // (`PCOM_AI_MIB`, pcom.cpp:11703 and 13487).
        let mib = self.ai == AI_MIB;
        if mib && !self.aggro && !self.truce {
            if p.noise > 0.0 && dist < p.noise {
                self.alert = Some(p.pos);
            } else if self.can_see(&p, range, ground) {
                self.alert = Some(self.fig.pos);
            }
        }
        // Someone kept to a zone only goes after a person standing in it, and only hears
        // what happens there; once they're out of it, the chase is over
        // (`PCOM_can_i_see_person_to_attack`, pcom.cpp:5775; pcom.cpp:8534 and 13244).
        let in_zone = self.zone == 0 || p.zone & self.zone != 0;
        if !in_zone {
            self.aggro = false;
            self.taunt = None;
            self.heard = None;
            return self.wander(c, ground, dt, rand);
        }
        // Gang members (and cops) pay Darci's footsteps no heed; guards listen to everything
        // (`PCOM_oscillate_tympanum`, pcom.cpp:13431).
        let darci_feet = p.footsteps && p.kind == uc_world::character::person::DARCI;
        if ((gang && !darci_feet) || self.ai == AI_GUARD) && !self.aggro && p.noise > 0.0 && dist < p.noise {
            self.hear(p.pos);
        }
        let notices = if gang || mib { self.can_see(&p, range, ground) } else { dist < range && (dy < 128.0 || hunter) };
        if !self.aggro && !notices {
            if let Some(h) = self.heard {
                return self.investigate(h, c, ground, dt, rand);
            }
        }
        if !self.aggro && notices && !self.truce {
            self.heard = None;
            self.aggro = true;
            self.cues.push(Sfx::Alert { kind: self.kind });
            self.attack_timer = 0.8;
            // A gang member who sees the player taunts them first (`PCOM_AI_GANG` in
            // `PCOM_process_person`, pcom.cpp:11180, `PCOM_can_i_see_person_to_taunt`).
            if self.ai == AI_GANG && !hunter && self.can_taunt() {
                self.taunt = Some(Taunt::Now);
            }
        } else if self.aggro && !hunter && self.victim.is_none() && (dist > range * 2.0 + AGGRO_RANGE || dy > 256.0) {
            // Out to get someone in particular (a gang mate's fight, a bully's pick), they keep
            // on after them however far: `PCOM_AI_STATE_NAVTOKILL`.
            self.aggro = false;
        }
        if !self.aggro {
            self.taunt = None;
            return self.wander(c, ground, dt, rand);
        }
        if let Some(t) = self.taunt {
            // `PCOM_process_taunt` (pcom.cpp:9467): out of sight, they give it up; done with
            // a taunt, they go for them half the time and otherwise pause (up to a second
            // and a half) and taunt again. A driver they can only taunt.
            // Losing sight of them ends the taunt, not the fight: they come looking.
            let eye = vec3(0.0, 140.0, 0.0);
            self.taunt = match t {
                _ if !ground.line_of_sight(self.fig.pos + eye, p.pos + eye) => None,
                Taunt::Now => {
                    c.taunt = true;
                    Some(Taunt::Doing)
                }
                Taunt::Doing if matches!(self.state, State::Taunt { .. }) => Some(Taunt::Doing),
                Taunt::Doing if rand() & 1 == 1 && !p.in_car => None,
                Taunt::Doing => Some(Taunt::Pause((rand() & 0xf) as f32 / 10.0)),
                Taunt::Pause(left) if left > dt => Some(Taunt::Pause(left - dt)),
                // "Lazy people dont taunt" (pcom.cpp:9546): once is enough.
                Taunt::Pause(_) if self.bent & BENT_LAZY != 0 => None,
                Taunt::Pause(_) => Some(Taunt::Now),
            };
            if self.taunt.is_some() {
                return c;
            }
        }
        // An assassin told to stand still is a sniper (`PCOM_AI_STATE_SNIPE`, pcom.cpp:11098 and
        // `PCOM_process_snipe`): he stays where he is, gun out, and shoots his mark whenever he
        // can see them (`can_a_see_b`, out to 8 squares). With no gun there's not much he can
        // do but stand there.
        if matches!(self.ai, 3 | 21) && self.mv == 1 && self.foe.is_some() {
            if self.weapon.is_gun() && self.can_see(&p, 8.0 * 256.0, ground) {
                self.fig.turn_towards(d, 8.0, dt);
                self.attack_timer -= dt;
                if self.attack_timer <= 0.0 {
                    c.punch = true;
                    let skill = self.skill as f32;
                    self.attack_timer = self.shot_interval(skill, rand);
                }
            }
            return c;
        }
        // Going for someone with a gun on them (`PCOM_set_person_ai_navtokill`, pcom.cpp:4294:
        // they can see them, a gun in hand, pointing within 256 of 2048 of them): without a
        // gun, and not a thug who fights back, half the time they run for it instead. Cops
        // and the city's extra people don't. They look again once they've stopped running.
        if !self.aggro {
            self.sized_up = false;
        } else if !self.sized_up {
            self.sized_up = true;
            let aimed = p.gun_out && (-d).normalize_or_zero().dot(p.facing) > (256.0 / 2048.0 * std::f32::consts::TAU).cos()
                && ground.line_of_sight(p.pos + vec3(0.0, 140.0, 0.0), self.fig.pos + vec3(0.0, 140.0, 0.0));
            use uc_world::character::person as ptype;
            let thug = matches!(self.kind, ptype::THUG_RASTA | ptype::THUG_GREY | ptype::THUG_RED);
            let fights_back = thug && self.bent & BENT_FIGHT_BACK != 0;
            if aimed && !self.weapon.is_gun() && !fights_back && !self.fake && self.kind != ptype::COP && rand() & 4 != 0 {
                self.scared = Some(Scare { by: player, at: p.pos, time: 0.0, goal: None });
                return c;
            }
        }
        // Block a blow that's coming (better fighters more often).
        self.block_timer -= dt;
        if self.block_timer > 0.0 {
            c.block = true;
            return c;
        }
        let swinging = snaps.iter().enumerate().any(|(j, s)| (s.player || Some(j) == player) && s.attacking) && dist < 220.0;
        if swinging && self.block_timer <= -0.4 && rand() % 100 < self.skill as u32 * 5 {
            self.block_timer = 0.6;
            c.block = true;
            return c;
        }
        // Guns shoot from range when they can see the player, and close in otherwise.
        let skill = self.skill as f32;
        if self.weapon.is_gun() && dist > 260.0 {
            self.attack_timer -= dt;
            let eye = vec3(0.0, 140.0, 0.0);
            let seen = dist < SHOOT_RANGE && ground.line_of_sight(self.fig.pos + eye, p.pos + eye);
            if seen && self.attack_timer <= 0.0 {
                c.punch = true;
                self.attack_timer = self.shot_interval(skill, rand);
            } else if !seen || dist > SHOOT_RANGE * 0.8 {
                c.wish = towards;
                c.run = dist > SHOOT_RANGE;
            }
            return c;
        }
        // Keep out of each other's way: hold back while another opponent is closer to the player
        // (not the one they're fighting, who's hostile too when two of the AI fight). Only
        // the hostile make way for each other: a bodyguard wades in among them.
        let crowded = self.hostile() && snaps.iter().enumerate().any(|(j, s)| {
            j != me && Some(j) != player && s.standing && s.hostile && flat(p.pos, s.pos).length() + 20.0 < dist
                && flat(s.pos, self.fig.pos).normalize_or_zero().dot(d.normalize_or_zero()) > 0.8
        });
        // The player comes the rest of the way; another of the AI wouldn't, and from 140 no
        // punch (130) reaches.
        let close = if p.player { 140.0 } else { 110.0 };
        if dist > close && !crowded {
            c.wish = towards;
        } else if dist < 80.0 {
            c.wish = -d / dist.max(1.0) * 0.5;
        }
        // After someone running, a hunter runs too, and once both are running within 0xc0 of
        // each other goes in with a sliding tackle (pcom.cpp:8681): if they can see them, not
        // if they're a fight-test dummy, and at the player only one turn in eight.
        c.run = hunter && (dist > 600.0 || p.footsteps);
        let flat_out = matches!(self.fig.anim(), anim::RUN | anim::YOMP);
        if c.run && p.footsteps && flat_out && dist < 0xc0 as f32 && !self.is_dummy() && (!p.player || rand().is_multiple_of(8)) {
            let eye = vec3(0.0, 140.0, 0.0);
            if ground.line_of_sight(self.fig.pos + eye, p.pos + eye) {
                c.wish = towards;
                c.kick = true;
                return c;
            }
        }
        self.attack_timer -= dt;
        // Take turns, like the original's gang attacks: nobody swings while another does. A
        // bodyguard doesn't wait for the gang's turn to be over.
        let busy = self.hostile() && snaps.iter().enumerate().any(|(j, s)| j != me && Some(j) != player && s.hostile && s.attacking);
        // Swinging as the player steps in reaches her; another of the AI stays put, so they
        // wait until a punch would reach.
        let strike = if p.player { 170.0 } else { 125.0 };
        if dist < strike && self.attack_timer <= 0.0 && p.standing && !busy {
            // Armed people use their weapon; others punch or kick.
            if self.weapon != Weapon::None || rand().is_multiple_of(2) { c.punch = true } else { c.kick = true }
            self.attack_timer = (1.8 - skill * 0.07).max(0.6) + (rand() % 100) as f32 / 70.0;
            // A bodyguard keeps at it, half again as often.
            if self.ai == AI_BODYGUARD {
                self.attack_timer *= 0.5;
            }
        }
        c
    }

    /// `PCOM_MOVE_WANDER` (move 4): strolls from spot to spot nearby, pausing in between.
    /// Spots are in plain sight on the same level; walking never steps off a ledge.
    pub(super) fn wander(&mut self, mut c: Controls, ground: &Ground, dt: f32, rand: &mut impl FnMut() -> u32) -> Controls {
        // Off to the goal the game set: a patrol's next point, or whoever is being followed.
        if let (Some(goal), false) = (self.goal, self.is_down()) {
            let d = flat(goal, self.fig.pos);
            if d.length() > 40.0 {
                c.wish = d.normalize();
                (c.run, c.sprint) = self.pace.controls();
                c.crouch = self.pace == Pace::Crawl;
            }
            return c;
        }
        if self.mv != 4 || self.is_down() {
            return c;
        }
        let here = vec2(self.fig.pos.x, self.fig.pos.z);
        let (spot, left) = self.wander;
        // Held up on the way: somewhere else (straight away, for the city's extra people),
        // turned round, as the next place is picked ahead of them.
        let left = if spot.is_some() && self.blocked > 1.0 {
            self.blocked = 0.0;
            self.fig.angle += std::f32::consts::PI;
            0.0
        } else {
            left - dt
        };
        match spot {
            Some(s) if left > 0.0 && (s - here).length() > 40.0 => {
                c.wish = (s - here).normalize_or_zero();
                self.wander = (Some(s), left);
            }
            // The city's extra people go straight on to the next place, which the game
            // picks from the pavements ([`Person::wander_to`]).
            _ if self.fake => self.wander = (None, 0.0),
            // There (or stuck): pause for a few seconds.
            Some(_) => self.wander = (None, 1.0 + (rand() % 400) as f32 / 100.0),
            None if left > 0.0 => self.wander = (None, left),
            None => {
                let angle = (rand() % 628) as f32 / 100.0;
                let dist = 2.0 * 256.0 + (rand() % 1024) as f32;
                let s = here + vec2(angle.sin(), angle.cos()) * dist;
                let waist = vec3(0.0, 80.0, 0.0);
                if ground.line_of_sight(self.fig.pos + waist, vec3(s.x, self.fig.pos.y, s.y) + waist) {
                    // Long enough to get there at a walk, with some to spare.
                    self.wander = (Some(s), dist / WALK_SPEED * 1.5 + 1.0);
                }
            }
        }
        c
    }

    /// Whether a wanderer has nowhere to go just now.
    pub fn wants_somewhere(&self) -> bool {
        self.mv == 4 && self.wander.0.is_none() && self.wander.1 <= 0.0
    }

    /// Sends a wanderer to `spot`, picking another if not there in five seconds, so they
    /// don't stand stuck against something (`PCOM_process_wander`).
    pub fn wander_to(&mut self, spot: Vec2) {
        self.wander = (Some(spot), 5.0);
    }

    /// Frightened by `by`, or by something at `at`: run from it, unless they're the sort to
    /// stand their ground (`PCOM_set_person_ai_flee_person`). Only civilians scare, and
    /// they remember it.
    ///
    /// In the original the mission's civilians scare too. Here only the city's extra people
    /// do for now: with no route-finding, someone running from gunfire can end up behind a
    /// wall where nobody can reach them, and a mission's witness or mark doing that can stall
    /// its script (RTA's drunk driver hides from the very cops sent to stop him).
    pub fn scare(&mut self, by: Option<usize>, at: Vec3) {
        // Drivers too: `PCOM_set_person_ai_flee_person` has them leave their car first
        // (`PCOM_EXCAR_FLEE_PERSON`). Cop drivers don't run.
        if self.player || !matches!(self.ai, AI_CIV | AI_DRIVER) || self.bent & BENT_FIGHT_BACK != 0 || self.removed || self.is_dead() {
            return;
        }
        self.annoyed = true;
        match self.scared.as_mut() {
            Some(s) => (s.by, s.at) = (by, at),
            None => self.scared = Some(Scare { by, at, time: 0.0, goal: None }),
        }
    }
}
