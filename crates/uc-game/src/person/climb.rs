//! Climbing: ledges, fences, hanging and pulling up, ladders and zip wires.

use super::*;

impl Person {
    /// Where a person would stand after climbing a ledge in direction `dir`, if there is one
    /// between a step and `reach` above the feet (a raised walkway, a platform rim, a roof,
    /// a van's top; not a car's, which has no edges to hold).
    pub(super) fn ledge_ahead(&self, dir: Vec2, reach: f32, ground: &Ground, map: &Map) -> Option<Vec3> {
        Self::ledge_from(self.fig.pos, dir, reach, ground, map)
    }

    /// [`Person::ledge_ahead`] from feet at `p`.
    pub(super) fn ledge_from(p: Vec3, dir: Vec2, reach: f32, ground: &Ground, map: &Map) -> Option<Vec3> {
        let probe = vec2(p.x, p.z) + dir * (RADIUS + 40.0);
        let top = ground.ledge(map, probe.x, probe.y, p.y + reach - STEP_HEIGHT);
        if top - p.y <= STEP_HEIGHT || top - p.y > reach {
            return None;
        }
        // Room to stand: no wall between the edge and the spot, at the new height.
        let spot = ground.slide(probe, top, dir * 30.0, RADIUS);
        Some(vec3(spot.x, ground.surface(map, spot.x, spot.y, top), spot.y))
    }

    /// Plays a climbing animation `a` that ends standing on `to`. The animation carries the
    /// body up by itself (a pull-up raises the hips 263 units), so the figure's position is
    /// moved so that the hips start where they are and end at standing height on `to`.
    /// `from_standing`: turn to face `to` first (else keep facing, e.g. on a ladder).
    pub(super) fn climb_onto(&mut self, a: usize, to: Vec3, from_standing: bool) {
        let f = &mut self.fig;
        if from_standing {
            f.angle = uc_world::character::angle_of(vec2(to.x - f.pos.x, to.z - f.pos.z));
        }
        let stand = f.stand_root();
        let (Some(first), Some(last)) = (f.anim_root(a, false), f.anim_root(a, true)) else { return };
        // Switch poses at once (blending in from the old pose would leave the body behind),
        // keeping the hips where they are.
        // From where the hips are drawn now (a jump's height correction included).
        let start = f.drawn_pos() + f.to_world(f.root() - first);
        f.cancel_rise = false;
        f.lift = 0.0;
        let end = to - f.to_world(last - stand);
        f.restart_now(a, Play::Once);
        f.set_speed(1.3);
        f.pos = start;
        self.state = State::PullUp { start, end, to };
    }

    /// Starts climbing over a fence whose near face is at `at` (xz), top at `top`.
    pub(super) fn climb_fence(&mut self, at: Vec2, over: Vec2, top: f32) {
        let from = self.fig.hips();
        self.fig.angle = uc_world::character::angle_of(over);
        self.fig.pin_root = true;
        self.fig.restart_now(anim::LAND_ON_FENCE, Play::Once);
        self.fig.set_speed(1.5);
        self.state = State::Fence { phase: 0, at, over, base: self.fig.pos.y, top, from };
        self.place_hips(from);
    }

    /// With the root pinned, puts the figure so its hips are drawn at `h`.
    pub(super) fn place_hips(&mut self, h: Vec3) {
        let stand = self.fig.stand_root();
        self.fig.pos = h - self.fig.to_world(stand);
    }

    pub(super) fn pull_up(&mut self, to: Vec3) {
        self.climb_onto(anim::PULL_UP, to, true);
    }

    /// Catches the ledge whose top is `to` and hangs from it (`SUB_STATE_GRAB_TO_DANGLE`,
    /// then `SUB_STATE_DANGLING`): turned to face it, with her left hand on its edge
    /// (`draw_info->Locked = SUB_OBJECT_LEFT_HAND`, Person.cpp:12980). She's held a body's
    /// radius off the wall, so the edge is that far ahead of her, at the top.
    pub(super) fn grab_dangle(&mut self, to: Vec3, dir: Vec2) {
        let p = self.fig.pos;
        let edge = vec3(p.x + dir.x * RADIUS, to.y, p.z + dir.y * RADIUS);
        self.fig.angle = uc_world::character::angle_of(dir);
        self.fig.lift = 0.0;
        self.fig.cancel_rise = false;
        // Catching hold first (`ANIM_FLY_GRABBING_LEDGE`), then hanging.
        if !self.fig.restart_now(anim::FLY_GRABBING_LEDGE, Play::Once) && !self.fig.restart_now(anim::DANGLE, Play::Loop) {
            return self.pull_up(to);
        }
        let Some((_, hand)) = self.fig.part_world(LEFT_HAND) else { return self.pull_up(to) };
        self.fig.pos += edge - hand;
        self.cues.push(Sfx::Grab);
        self.state = State::Dangle { to, dir, shift: None };
    }

    /// Where someone hanging from a ledge or pulling up over it will stand: on the face they
    /// hold, which is their `OnFace` from the grab on (Person.cpp:13001).
    pub(crate) fn holding(&self) -> Option<Vec3> {
        match self.state {
            State::Dangle { to, .. } | State::PullUp { to, .. } => Some(to),
            _ => None,
        }
    }

    /// Takes someone along with the moving face they're on, or whose edge they hold
    /// (`WMOVE_relative_pos`, Person.cpp:4239, whatever they're doing): `f` maps where a point
    /// on it was to where it is now.
    pub(crate) fn carry_by(&mut self, f: impl Fn(Vec3) -> Vec3) {
        let p = self.fig.pos;
        self.fig.pos = f(p);
        match &mut self.state {
            State::Dangle { to, dir, shift } => {
                let d = f(p + vec3(dir.x, 0.0, dir.y)) - self.fig.pos;
                *dir = vec2(d.x, d.z).normalize_or(*dir);
                *to = f(*to);
                if let Some((a, b)) = shift {
                    (*a, *b) = (f(*a), f(*b));
                }
            }
            State::PullUp { start, end, to } => (*start, *end, *to) = (f(*start), f(*end), f(*to)),
            State::OntoLadder { start, end, .. } => (*start, *end) = (f(*start), f(*end)),
            _ => {}
        }
    }

    /// Hanging from a ledge (`fn_person_dangling`, and `action_dangling`'s keys,
    /// fallen/Source/interfac.cpp:450): forwards pulls up (`set_person_pulling_up`),
    /// backwards lets go (`set_person_drop_down`), and left or right goes a hand along it,
    /// 74 units, if there's still ledge to hold there (`set_person_traverse`,
    /// `traverse_pos`). Anyone but the player pulls straight up.
    pub(super) fn step_dangle(&mut self, to: Vec3, dir: Vec2, shift: Option<(Vec3, Vec3)>, c: &Controls, ground: &Ground, map: &Map) {
        // Caught hold: hang once it's done, the hand staying on the edge.
        if self.fig.anim() == anim::FLY_GRABBING_LEDGE {
            if self.fig.finished() {
                let before = self.fig.part_world(LEFT_HAND).map(|(_, h)| h);
                self.fig.restart_now(anim::DANGLE, Play::Loop);
                if let (Some(b), Some((_, a))) = (before, self.fig.part_world(LEFT_HAND)) {
                    self.fig.pos += b - a;
                }
            }
            return;
        }
        if let Some((_, end)) = shift {
            // The animation carries her along itself; at its end, hang on from where it left
            // her hips (`locked_anim_change(..., ANIM_DANGLE)`, Person.cpp:15004), at the new
            // ledge's height.
            if self.fig.finished() {
                self.fig.restart_locked(anim::DANGLE, Play::Loop);
                self.fig.pos.y = end.y;
                self.state = State::Dangle { to, dir, shift: None };
            }
            return;
        }
        if !self.player {
            return self.climb_onto(anim::PULL_UP, to, false);
        }
        // `angle - 512` is her right (`traverse_pos`).
        let right = vec2(-dir.y, dir.x);
        let (ahead, side) = (c.wish.dot(dir), c.wish.dot(right));
        if ahead > 0.5 {
            self.climb_onto(anim::PULL_UP, to, false);
        } else if ahead < -0.5 {
            // Hips kept where they are (`set_person_drop_down` changes animation locked).
            self.fig.restart_locked(anim::FALLING, Play::Loop);
            self.state = State::Fall { vy: 0.0, vel: Vec2::ZERO };
        } else if side.abs() > 0.5 {
            const STEP: f32 = 74.0;
            let along = right * side.signum() * STEP;
            // Feet below the wall's foot there, where a jump would have reached it from.
            let foot = vec3(to.x + along.x, to.y - GRAB_REACH * 0.75, to.z + along.y) - vec3(dir.x, 0.0, dir.y) * (RADIUS + 60.0);
            let next = Self::ledge_from(foot, dir, GRAB_REACH, ground, map).filter(|t| (t.y - to.y).abs() < 24.0);
            if let Some(next) = next {
                let p = self.fig.pos;
                let end = vec3(p.x + along.x, p.y + (next.y - to.y), p.z + along.y);
                let a = if side > 0.0 { anim::TRAVERSE_RIGHT } else { anim::TRAVERSE_LEFT };
                self.fig.restart_now(a, Play::Once);
                self.state = State::Dangle { to: next, dir, shift: Some((p, end)) };
            }
        }
    }

    /// `step` in [`State::Fence`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_fence(&mut self, mut phase: u8, at: Vec2, over: Vec2, base: f32, top: f32, from: Vec3, dt: f32) {
        // Hips just in front of the fence, hands up on it.
        let near = vec3(at.x, 0.0, at.y) - vec3(over.x, 0.0, over.y) * 28.0;
        let grip = |y: f32| vec3(near.x, y, near.z);
        let (low, high) = (base + 150.0, (top - 30.0).max(base + 150.0));
        match phase {
            0 => {
                // Onto it, settling there (`ANIM_LANDED_ON_FENCE`), then up.
                let settling = self.fig.anim() == anim::LANDED_ON_FENCE;
                let t = if settling { 1.0 } else { self.fig.progress() };
                self.place_hips(from.lerp(grip(low), t));
                if self.fig.finished() {
                    if !settling && self.fig.restart(anim::LANDED_ON_FENCE, Play::Once) {
                        self.fig.set_speed(1.5);
                    } else {
                        phase = 1;
                        self.fig.restart(anim::CLIMB_UP_FENCE, Play::Loop);
                    }
                }
            }
            1 => {
                let y = (self.fig.hips().y + 220.0 * dt).min(high);
                self.place_hips(grip(y));
                if y >= high {
                    phase = 2;
                    self.fig.restart(anim::CLIMB_OVER_FENCE, Play::Once);
                }
            }
            _ => {
                // Over the top in an arc, ending just past the fence.
                let t = self.fig.progress();
                let far = vec3(at.x, 0.0, at.y) + vec3(over.x, 0.0, over.y) * 40.0;
                let xz = near.lerp(far, t);
                let y = high + (top + 60.0 - high) * (t * std::f32::consts::PI).sin() - (high - top + 60.0) * t * t;
                self.place_hips(vec3(xz.x, y, xz.z));
                if self.fig.finished() {
                    // Drop down on the far side.
                    let h = self.fig.hips();
                    self.fig.pin_root = false;
                    self.fig.pos = h - vec3(0.0, self.fig.stand_root().y, 0.0);
                    self.fall(over * 60.0);
                    return;
                }
            }
        }
        self.state = State::Fence { phase, at, over, base, top, from };
    }

    /// `step` in [`State::Zip`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_zip(&mut self, cable: usize, mut t: f32, dir: f32, mut speed: f32, mut ridden: f32, c: &Controls, ground: &Ground, map: &Map, dt: f32) {
        let cb = &ground.cables[cable];
        let len = cb.length().max(1.0);
        // Speed up downhill to a top speed.
        let ahead = cb.at(t + dir * 0.01);
        let slope = (cb.at(t).y - ahead.y) / (ahead - cb.at(t)).length().max(1.0);
        speed = (speed + GRAVITY * 0.6 * slope.max(0.1) * dt).min(1100.0);
        t += dir * speed * dt / len;
        ridden += speed * dt;
        let tangent = (cb.at(t + dir * 0.01) - cb.at(t)).normalize_or_zero();
        self.fig.angle = uc_world::character::angle_of(vec2(tangent.x, tangent.z));
        let end = !(0.0..=1.0).contains(&t);
        self.fig.pos = cb.at(t) - vec3(0.0, HANG, 0.0);
        // The grab, then a pull up on the cable, then hanging there.
        if self.fig.finished() {
            match self.fig.anim() {
                anim::DEATH_SLIDE => { self.fig.restart(anim::WIRE_SLIDE_PULLUP, Play::Once); }
                anim::WIRE_SLIDE_PULLUP => { self.fig.restart(anim::WIRE_SLIDE_HANG, Play::Loop); }
                _ => {}
            }
        }
        // Near the bottom, sliding into a roof (its top between the feet and the waist):
        // step off onto it. The Jump's zip line ends just above a roof that's above
        // Darci's feet. (Earlier on, the legs pass over whatever's below.)
        let p = self.fig.pos;
        let left = if dir > 0.0 { 1.0 - t } else { t } * len;
        let roof = ground.surface(map, p.x, p.z, p.y + HANG * 0.5);
        if roof > p.y - 8.0 && ridden > 300.0 && left < 600.0 {
            self.fig.pos.y = roof;
            self.zip_cooldown = 1.0;
            self.state = State::Land { vel: vec2(tangent.x, tangent.z) * speed * 0.3 };
            self.cues.push(Sfx::Land);
            self.fig.restart_now(anim::LAND_VERT, Play::Once);
            self.fig.set_speed(2.0);
            return;
        }
        if end || c.jump || c.action {
            // Let go: drop from where the hanging body is, and don't grab straight back.
            let hips = self.fig.hips();
            self.zip_cooldown = 1.0;
            self.fall(vec2(tangent.x, tangent.z) * speed * 0.6);
            self.fig.restart_now(anim::FALLING, Play::Loop);
            // Falling draws the hips at standing height above the feet.
            self.fig.pos.y = hips.y - self.fig.stand_root().y;
            return;
        }
        self.state = State::Zip { cable, t, dir, speed, ridden };
    }

    /// `step` in [`State::Climb`].
    /// Onto the grappling hook's line at `at`, the foot of the wall, facing `dir` into it,
    /// to climb up to the edge at `top` and onto the roof at `to` (#109).
    pub fn climb_rope(&mut self, at: Vec2, dir: Vec2, top: f32, to: Vec3) {
        let y = self.fig.pos.y;
        self.fig.pos = vec3(at.x, y, at.y);
        self.fig.angle = uc_world::character::angle_of(dir);
        self.fig.lift = 0.0;
        self.fig.cancel_rise = false;
        self.fig.restart_now(anim::ON_LADDER, Play::Loop);
        self.state = State::Rope { at, dir, top, to, y0: y, loops0: self.fig.loops() };
    }

    /// Up the hook's line as up a ladder ([`Person::step_climb`]): towards the wall climbs,
    /// away lets go, and at the top she climbs off onto the roof.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_rope(&mut self, at: Vec2, dir: Vec2, top: f32, to: Vec3, y0: f32, loops0: f32, c: &Controls) {
        self.fig.angle = uc_world::character::angle_of(dir);
        if c.wish.dot(-dir) > 0.7 || c.action {
            self.fall(-dir * 60.0);
            return;
        }
        // The player climbs while pushing towards the wall; anyone else just climbs.
        let moving = !self.player || c.wish.dot(dir) > 0.3;
        self.fig.set_speed(if moving { 1.3 } else { 0.0 });
        let y = y0 + (self.fig.loops() - loops0) * RUNG_CYCLE;
        self.fig.pos = vec3(at.x, y, at.y);
        if y + self.off_rise() >= top - 24.0 && self.fig.root().y >= 220.0 {
            self.climb_onto(anim::OFF_LADDER_TOP, to, false);
        }
    }

    /// How far up climbing off the top of a ladder takes her feet.
    fn off_rise(&self) -> f32 {
        self.fig.anim_root(anim::OFF_LADDER_TOP, true).map_or(261.0, |r| r.y - self.fig.stand_root().y)
    }

    /// Where climbing off the top of `ladder` leaves her standing.
    fn off_top(ladder: usize, ground: &Ground, map: &Map) -> Vec3 {
        let l = ground.ladders[ladder];
        let spot = ground.on_ladder(ladder, l.top) - vec3(l.out.x, 0.0, l.out.y) * 90.0;
        vec3(spot.x, ground.surface(map, spot.x, spot.z, l.top), spot.z)
    }

    /// The ladder whose top she's standing by, to climb down onto with action
    /// (interfac.cpp:1420): its top within 64 of her feet, and she within 75 of where
    /// climbing off it leaves her (`set_person_climb_down_onto_ladder`, collide.cpp:2575).
    pub(super) fn ladder_top_near(&self, ground: &Ground, map: &Map) -> Option<usize> {
        let p = self.fig.pos;
        (0..ground.ladders.len()).find(|&l| {
            (ground.ladders[l].top - p.y).abs() < 64.0 && flat(Self::off_top(l, ground, map), p).length() < 75.0
        })
    }

    /// Down onto `ladder` from its top (`set_person_climb_down_onto_ladder`): put where
    /// climbing off it leaves her, back to the edge, she climbs off it backwards
    /// (`ANIM_OFF_LADDER_TOP`, `person_backwards_animate`), onto it where climbing up it
    /// would have climbed off.
    pub(super) fn climb_down_onto(&mut self, ladder: usize, ground: &Ground, map: &Map) -> bool {
        let l = ground.ladders[ladder];
        let to = Self::off_top(ladder, ground, map);
        let rungs = ((l.top - 24.0 - self.off_rise() - l.base) / RUNG_CYCLE).ceil().max(0.0);
        let start = ground.on_ladder(ladder, l.base + rungs * RUNG_CYCLE);
        let f = &mut self.fig;
        f.angle = uc_world::character::angle_of(-l.out);
        let (stand, Some(last)) = (f.stand_root(), f.anim_root(anim::OFF_LADDER_TOP, true)) else { return false };
        if !f.restart_reversed(anim::OFF_LADDER_TOP) {
            return false;
        }
        f.set_speed(-1.3);
        (f.cancel_rise, f.lift) = (false, 0.0);
        let end = to - f.to_world(last - stand);
        f.pos = end;
        self.state = State::OntoLadder { ladder, start, end };
        true
    }

    /// `step` in [`State::OntoLadder`]: on the ladder at the end, in the climb's pose that
    /// climbing off starts from (its sixth).
    pub(super) fn step_onto_ladder(&mut self, ladder: usize, start: Vec3, end: Vec3) {
        self.fig.pos = start.lerp(end, self.fig.progress());
        if self.fig.finished() {
            self.fig.restart_now(anim::ON_LADDER, Play::Loop);
            self.fig.skip_keyframes(5);
            self.fig.set_speed(0.0);
            self.fig.pos = start;
            self.state = State::Climb { ladder, y0: start.y, loops0: self.fig.loops() };
        }
    }

    /// `step` in [`State::Mount`]: on once the step on's done, in the climb's first pose.
    pub(super) fn step_mount(&mut self, ladder: usize, ground: &Ground) {
        if self.fig.finished() {
            self.fig.restart_now(anim::ON_LADDER, Play::Loop);
            self.fig.set_speed(0.0);
            self.state = State::Climb { ladder, y0: self.fig.pos.y, loops0: self.fig.loops() };
        }
        self.fig.pos = ground.on_ladder(ladder, self.fig.pos.y);
    }

    pub(super) fn step_climb(&mut self, ladder: usize, y0: f32, loops0: f32, c: &Controls, ground: &Ground, map: &Map) {
        let l = ground.ladders[ladder];
        self.fig.angle = uc_world::character::angle_of(-l.out);
        // Jump lets go (`action_ladder`'s `ACTION_DROP_DOWN`, interfac.cpp:544), as does action
        // off the ground.
        if c.jump || c.action && self.fig.pos.y > l.base + 64.0 {
            self.fall(l.out * 60.0);
            return;
        }
        // The animation moves the body up a rung cycle and loops back, so the feet move up a
        // cycle each time it loops. Pulling back climbs down: the same, backwards
        // (`SUB_STATE_CLIMB_DOWN_LADDER`, `person_backwards_animate`).
        let up = c.wish.dot(-l.out) > 0.3;
        let down = c.wish.dot(l.out) > 0.3;
        self.fig.set_speed(if up { 1.3 } else if down { -1.3 } else { 0.0 });
        let y = y0 + (self.fig.loops() - loops0) * RUNG_CYCLE;
        // Down at the foot: off it onto the ground (`ANIM_OFF_LADDER_BOT`) once the climb's
        // back to its first pose, which is where stepping off starts (or just past it, into
        // the rung below). A ladder that stops short of the ground is let go of instead.
        if down && y <= l.base + 1.0 && (self.fig.frame() == Some(0) || y < l.base - 1.0) {
            let at = ground.on_ladder(ladder, l.base);
            let floor = ground.surface(map, at.x, at.z, l.base + 8.0);
            if (floor - l.base).abs() < 16.0 && self.fig.restart_now(anim::OFF_LADDER_BOT, Play::Once) {
                self.fig.pos = vec3(at.x, floor, at.z);
                self.state = State::Press { then: None };
            } else {
                self.fall(Vec2::ZERO);
            }
            return;
        }
        self.fig.pos = ground.on_ladder(ladder, y);
        // Near the top, climbing up, at the point of the cycle where the hands reach the
        // edge, climb off onto the roof.
        if up && y + self.off_rise() >= l.top - 24.0 && self.fig.root().y >= 220.0 {
            self.climb_onto(anim::OFF_LADDER_TOP, Self::off_top(ladder, ground, map), false);
        }
    }
}
