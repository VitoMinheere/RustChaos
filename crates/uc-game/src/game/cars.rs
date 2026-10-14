//! Cars: who drives which, the AI at the wheel, and the player driving.

use crate::*;

impl Game {
    /// The car a driver out of one goes for (`PCOM_set_person_ai_findcar`): the nearest that
    /// nobody is driving, isn't wrecked and isn't locked, measured x plus z.
    pub(crate) fn car_for(&self, i: usize) -> Option<usize> {
        let at = self.people[i].fig.pos;
        self.vehicles.iter().enumerate()
            .filter(|(_, v)| v.driver.is_none() && !v.wrecked() && !v.locked())
            .map(|(k, v)| (k, (v.pos.x - at.x).abs() + (v.pos.z - at.z).abs()))
            .filter(|&(_, d)| d < FIND_CAR)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k)
    }

    /// Drivers who have walked up to their car's door get in (`PCOM_process_findcar`).
    pub(crate) fn board_cars(&mut self) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.ai != AI_DRIVER || p.player || p.removed || p.is_dead() || p.is_down() || p.aggro() || !matches!(p.state, State::Move) {
                continue;
            }
            let Some(vi) = self.car_for(i) else { continue };
            let door = self.vehicles[vi].door();
            if Vec2::new(p.fig.pos.x - door.x, p.fig.pos.z - door.y).length() < 150.0 {
                self.vehicles[vi].driver = Some(i);
                let p = &mut self.people[i];
                p.state = State::InCar { vehicle: vi };
                p.cues.push(Sfx::CarDoor);
                self.drivers.insert(i, Driving::default());
            }
        }
    }

    /// The controls of a car someone other than the player is driving. Wandering
    /// (`PCOM_MOVE_WANDER`) they drive the roads, picking a way at random at each junction
    /// (`PCOM_process_driving_wander`) and keeping to their lane (`DriveCar`, both in
    /// fallen/Source/pcom.cpp); otherwise they pull up and stay put
    /// (`PCOM_process_driving_still`).
    pub(crate) fn npc_drive(&mut self, vi: usize, pi: usize, dt: f32) -> Drive {
        const TURN: f32 = 896.0; // `LEFT_TURN_DIST`: how near the junction before turning
        let v = &self.vehicles[vi];
        let (at, speed, yaw, y) = (Vec2::new(v.pos.x, v.pos.z), v.speed, v.yaw, v.pos.y);
        let stop = if speed.abs() > 30.0 { Drive { throttle: -speed.signum(), ..Default::default() } } else { Drive::default() };
        let (mv, bent) = (self.people[pi].mv, self.people[pi].bent);
        let mut st = self.drivers.get(&pi).copied().unwrap_or_default();
        if mv != 4 || self.roads.nodes.len() < 2 {
            st.road = None;
            self.drivers.insert(pi, st);
            return stop;
        }
        // Onto the nearest road, going the way of the lane it's in.
        let (n1, n2) = match st.road {
            Some(r) => r,
            None => {
                let Some((mut a, mut b)) = self.roads.find(at) else { return stop };
                if self.roads.signed_dist(a, b, at) < 0.0 {
                    std::mem::swap(&mut a, &mut b);
                }
                st.next = 0;
                (a, b)
            }
        };
        // Near the junction: pick the next road, and turn into it (a left turn a little later).
        let mut road = (n1, n2);
        let to_end = creature::qdist2(self.roads.dest(n1, n2) - at);
        if to_end < TURN {
            if st.next == 0 {
                let mut rng = self.rng;
                st.next = self.roads.whereto_now(n1, n2, &mut || rand(&mut rng));
                self.rng = rng;
            }
            let late = if st.next != 0 && self.roads.bend(n1, n2, st.next) < 0 { 256.0 } else { 0.0 };
            if st.next != 0 && to_end + late < TURN {
                road = (n2, st.next);
                st.next = 0;
            }
        } else {
            st.next = 0;
        }
        st.road = Some(road);
        // Head for the far end of the road, or back into the lane first if well out of it.
        let (n1, n2) = road;
        let mut dest = self.roads.dest(n1, n2);
        let reversing = st.reversing > 0.0;
        if !reversing && (384.0 - self.roads.signed_dist(n1, n2, at)).abs() > 128.0 {
            let (r1, r2) = (self.roads.pos(n1), self.roads.pos(n2));
            let ab = r2 - r1;
            let t = ((at - r1).dot(ab) / ab.length_squared().max(1.0)).clamp(0.0, 1.0);
            let sign = |v: f32| if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
            let dr = Vec2::new(sign(ab.x), sign(ab.y)) * 256.0;
            dest = r1 + ab * t + dr * 2.0 + Vec2::new(-dr.y, dr.x);
        }
        let to = dest - at;
        let diff = (to.x.atan2(to.y) - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        // Slow for a bend, down to a third of the limit; diligent drivers go twice as fast.
        let dangle = diff.abs() * 2048.0 / std::f32::consts::TAU;
        let mut want = if dangle > 100.0 { (1.0 - (dangle - 100.0) / 2.0 / 750.0).max(250.0 / 750.0) } else { 1.0 } * CRUISE;
        if bent & 2 != 0 {
            want *= 2.0;
        }
        // (The car turns away from its steering: steer left for a larger yaw.)
        let mut steer = (-diff * 2.0).clamp(-1.0, 1.0);
        if reversing {
            st.reversing -= dt;
            want = -CRUISE * 0.4;
            steer = -steer;
        }
        // Stop for anyone in the road ahead (`PCOM_find_runover_thing`).
        let fwd = Vec2::new(yaw.sin(), yaw.cos());
        let side = Vec2::new(fwd.y, -fwd.x);
        let blocked = self.people.iter().enumerate().any(|(j, o)| {
            let r = Vec2::new(o.fig.pos.x - at.x, o.fig.pos.z - at.y);
            j != pi && !o.removed && !matches!(o.state, State::InCar { .. }) && (o.fig.pos.y - y).abs() < 150.0
                && r.dot(fwd) > 0.0 && r.dot(fwd) < 250.0 + speed.max(0.0) * 0.5 && r.dot(side).abs() < 170.0
        });
        self.drivers.insert(pi, st);
        if blocked && want > 0.0 {
            return stop;
        }
        Drive { throttle: Vehicle::throttle_for(want), steer, handbrake: false }
    }

    /// Cars move; the player's car follows her controls. Cars knock people over and push
    /// cones and barrels around.
    pub(crate) fn drive(&mut self, dt: f32, input: &Controls, level: &Level) {
        let player = self.player;
        // Anyone else at the wheel decides first. A driver who has gone (a kill_waypoint)
        // leaves the car where it is.
        let mut npc = vec![None; self.vehicles.len()];
        for (vi, d) in npc.iter_mut().enumerate() {
            if let Some(pi) = self.vehicles[vi].driver.filter(|&pi| Some(pi) != player) {
                if self.people[pi].removed {
                    self.vehicles[vi].driver = None;
                } else {
                    *d = Some((pi, self.npc_drive(vi, pi, dt)));
                }
            }
        }
        for (vi, npc) in npc.into_iter().enumerate() {
            let v = &mut self.vehicles[vi];
            let d = if v.driver.is_some() && v.driver == player { input.drive } else { npc.map_or(Drive::default(), |(_, d)| d) };
            let sliding = v.slip.abs() > 300.0;
            if let Some(speed) = v.update(d, dt, &level.ground, &level.map) {
                if speed > 300.0 {
                    self.sounds.push((Sfx::Crash, v.pos));
                }
                // Ran into something: back off and come round (`PCOM_MOVE_SUBSTATE_3PTURN`).
                if let Some(dr) = npc.and_then(|(pi, _)| self.drivers.get_mut(&pi)) {
                    if dr.reversing <= 0.0 {
                        dr.reversing = 1.5;
                    }
                }
            }
            if !sliding && v.slip.abs() > 300.0 {
                self.sounds.push((Sfx::Skid, v.pos));
            }
            let v = self.vehicles[vi].clone();
            if let Some(d) = v.driver {
                let p = &mut self.people[d];
                p.fig.pos = v.pos;
                p.fig.angle = v.driver_angle();
            }
            let fwd = v.forward();
            let side = Vec2::new(fwd.y, -fwd.x);
            // People ahead of a moving car run out of its way, as in the original. Those who
            // don't make it are knocked flying when it's fast and nudged aside when it's slow.
            if v.speed.abs() > 150.0 {
                let heading = fwd * v.speed.signum();
                for (i, p) in self.people.iter_mut().enumerate() {
                    if p.player || Some(i) == v.driver || p.removed || p.is_down() || p.dodge.is_some()
                        || !matches!(p.state, State::Move) || (p.fig.pos.y - v.pos.y).abs() > 150.0
                    {
                        continue;
                    }
                    let r = Vec2::new(p.fig.pos.x - v.pos.x, p.fig.pos.z - v.pos.z);
                    let (ahead, across) = (r.dot(heading), r.dot(side));
                    // Within the next second or so of the car's path.
                    if ahead > 0.0 && ahead < 400.0 + v.speed.abs() && across.abs() < 300.0 {
                        let away = if across >= 0.0 { side } else { -side };
                        p.dodge = Some((away, 0.9));
                    }
                }
            }
            // People in the way.
            for (i, p) in self.people.iter_mut().enumerate() {
                if Some(i) == v.driver || p.removed || matches!(p.state, State::InCar { .. }) || (p.fig.pos.y - v.pos.y).abs() > 150.0 {
                    continue;
                }
                let at = Vec2::new(p.fig.pos.x, p.fig.pos.z);
                if !v.covers(at, 30.0) {
                    continue;
                }
                if v.speed.abs() > 300.0 && !p.is_down() {
                    p.take_hit(v.pos, (v.speed.abs() / 12.0) as i32, person::ko::KNOCKDOWN, false, true, Weapon::None);
                }
                let away = if (at - Vec2::new(v.pos.x, v.pos.z)).dot(side) >= 0.0 { side } else { -side };
                let to = level.ground.slide(at, p.fig.pos.y, away * 40.0, 36.0);
                p.fig.pos.x = to.x;
                p.fig.pos.z = to.y;
            }
            // Cones and barrels get knocked about.
            for b in self.barrels.iter_mut().filter(|b| !b.gone) {
                let at = Vec2::new(b.pos.x, b.pos.z);
                if v.speed.abs() > 60.0 && v.covers(at, 30.0) && b.vel.length() < v.speed.abs() * 0.5 {
                    let side = (at - Vec2::new(v.pos.x, v.pos.z)).normalize_or_zero();
                    b.vel = fwd * v.speed * 0.9 + side * v.speed.abs() * 0.3;
                    self.sounds.push((Sfx::Cone, b.pos));
                }
            }
        }
        for b in &mut self.barrels {
            if b.vel.length_squared() < 1.0 {
                continue;
            }
            let at = Vec2::new(b.pos.x, b.pos.z);
            let to = level.ground.slide(at, b.pos.y, b.vel * dt, 30.0);
            b.pos = Vec3::new(to.x, level.ground.surface(&level.map, to.x, to.y, b.pos.y + 32.0), to.y);
            b.vel *= (-2.5 * dt).exp();
            b.yaw += b.vel.length() * dt / 60.0;
            if b.kind == 1 {
                b.tilt = (b.tilt + 8.0 * dt).min(std::f32::consts::FRAC_PI_2);
            }
        }
    }

    /// The car the player is driving.
    pub fn player_vehicle(&self) -> Option<&Vehicle> {
        match self.player().map(|p| &p.state) {
            Some(State::InCar { vehicle }) => self.vehicles.get(*vehicle),
            _ => None,
        }
    }

    /// A car whose health runs out blows up (`VEH_process`, vehicle.cpp:2963): a firebomb and
    /// a bang, and whoever is driving is thrown out and knocked over. Its flame "wave"
    /// (`PYRO_FLAGS_WAVE`) is only drawn, so there's no shockwave and no chain of cars.
    pub(crate) fn wreck_cars(&mut self) {
        for v in 0..self.vehicles.len() {
            if !self.vehicles[v].wrecked() || !self.blown.insert(v) {
                continue;
            }
            let at = self.vehicles[v].pos;
            self.pyro.burst(at + Vec3::new(0.0, 48.0, 0.0), pyro::FIREBOMB, 256.0);
            self.sounds.push((Sfx::Explosion, at));
            if self.aim_vehicle == Some(v) {
                self.aim_vehicle = None;
            }
            if let Some(d) = self.vehicles[v].driver.take() {
                let p = &mut self.people[d];
                p.state = State::Move;
                p.fig.pos = at;
                p.take_hit(at + Vec3::new(40.0, 0.0, 0.0), 0, person::ko::KNOCKDOWN, false, false, Weapon::None);
            }
        }
    }
}
