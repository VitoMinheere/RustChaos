//! Cars: who drives which, the AI at the wheel, and the player driving.

use crate::*;

/// The streets: the roads cars wander along, who's driving them, the pavements the city's
/// extra people walk and what they say when spoken to.
pub(crate) struct Streets {
    pub(crate) roads: road::Roads,
    /// What each person at the wheel is doing, by person.
    pub(crate) drivers: HashMap<usize, Driving>,
    pub(crate) wand: wand::Wander,
    pub(crate) chatter: city::Chatter,
}

impl Streets {
    pub(crate) fn new(level: &Level) -> Streets {
        Streets {
            roads: road::Roads::new(&level.map, &level.mission.map),
            drivers: HashMap::new(),
            wand: {
                let mut w = wand::Wander::new(&level.map, &level.game);
                w.forbid(&level.map, &level.mission.zones);
                w
            },
            chatter: city::Chatter::load(level),
        }
    }
}

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

    /// Person `i` gets into car `vi` (`set_person_enter_vehicle`, Person.cpp:8254): put at
    /// the nearer door (`position_person_for_vehicle`) facing a quarter turn off the car's
    /// heading, playing `ANIM_ENTER_CAR` at the driver's door and `ANIM_ENTER_TAXI` at the
    /// passenger's, with the door's slam. The car is theirs from now on; they're in it when
    /// the animation ends ([`Game::finish_entering`]).
    pub(crate) fn enter_car(&mut self, i: usize, vi: usize, level: &Level) {
        let v = &mut self.vehicles[vi];
        v.driver = Some(i);
        let pos = self.people[i].fig.pos;
        let doors = [v.door(), v.to_world(-200.0, 20.0)];
        let near = |d: Vec2| (d - Vec2::new(pos.x, pos.z)).length();
        let door = usize::from(near(doors[1]) < near(doors[0]));
        let at = doors[door];
        let quarter = std::f32::consts::FRAC_PI_2;
        let angle = v.driver_angle() + if door == 0 { -quarter } else { quarter };
        let y = level.ground.surface(&level.map, at.x, at.y, pos.y + 64.0);
        let p = &mut self.people[i];
        p.fig.pos = Vec3::new(at.x, y, at.y);
        p.fig.angle = angle;
        let anim = if door == 0 { uc_world::character::anim::ENTER_CAR } else { uc_world::character::anim::ENTER_TAXI };
        p.state = if p.fig.restart_now(anim, uc_world::character::Play::Once) {
            self.entering.push((i, vi));
            State::Enter { vehicle: vi }
        } else {
            State::InCar { vehicle: vi }
        };
        p.cues.push(Sfx::CarDoor);
    }

    /// `SUB_STATE_ENTERING_VEHICLE`: in once the animation's played out
    /// (`set_person_in_vehicle`). Anyone knocked out of it leaves the car free again.
    pub(crate) fn finish_entering(&mut self) {
        let entering = std::mem::take(&mut self.entering);
        for (i, vi) in entering {
            let p = &mut self.people[i];
            match p.state {
                State::Enter { vehicle } if p.fig.finished() => p.state = State::InCar { vehicle },
                State::Enter { .. } => self.entering.push((i, vi)),
                _ => {
                    if self.vehicles[vi].driver == Some(i) {
                        self.vehicles[vi].driver = None;
                    }
                }
            }
        }
    }

    /// Drivers who have walked up to their car's door get in (`PCOM_process_findcar`).
    /// Followers ride along (`PCOM_AI_STATE_HITCH`, `PCOM_process_hitch`, pcom.cpp:9358):
    /// one who reaches the car of whoever they follow gets in as a passenger, three at most
    /// to a car, and rides with it, out of sight; when the one they follow gets out, or the
    /// car is wrecked, they get out too, at the passenger door (`PCOM_process_following`).
    pub(crate) fn hitch_rides(&mut self, level: &Level) {
        let leader = |g: &Game, i: usize| {
            let p = &g.people[i];
            if p.follow == 0 { g.player } else { g.by_way.get(&p.follow).copied() }
        };
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.player || p.removed || p.is_dead() || p.mv != person::MOVE_FOLLOW {
                continue;
            }
            let theirs = leader(self, i).and_then(|j| match self.people[j].state {
                State::InCar { vehicle } => Some(vehicle),
                _ => None,
            });
            match (p.state.clone(), theirs) {
                // Riding along: with the car.
                (State::InCar { vehicle }, Some(v)) if vehicle == v && !self.vehicles[v].wrecked() && self.vehicles[v].driver != Some(i) => {
                    self.people[i].fig.pos = self.vehicles[v].pos;
                }
                // Their leader's out, or the car's gone: out they get.
                (State::InCar { vehicle }, _) if self.vehicles[vehicle].driver != Some(i) => {
                    let door = self.vehicles[vehicle].to_world(-200.0, 20.0);
                    let y = level.ground.surface(&level.map, door.x, door.y, self.vehicles[vehicle].pos.y + 64.0);
                    let p = &mut self.people[i];
                    p.fig.pos = Vec3::new(door.x, y, door.y);
                    p.state = State::Move;
                    p.fig.restart_now(uc_world::character::anim::BREATHE, uc_world::Play::Loop);
                }
                (State::Move, Some(v)) if !self.vehicles[v].wrecked() => {
                    let car = &self.vehicles[v];
                    let near = [car.door(), car.to_world(-200.0, 20.0)].iter().any(|d| Vec2::new(p.fig.pos.x - d.x, p.fig.pos.z - d.y).length() < 150.0);
                    let riding = self.people.iter().enumerate()
                        .filter(|(k, q)| matches!(q.state, State::InCar { vehicle } if vehicle == v) && car.driver != Some(*k))
                        .count();
                    if near && riding < 3 {
                        let p = &mut self.people[i];
                        p.state = State::InCar { vehicle: v };
                        p.fig.pos = car.pos;
                        p.cues.push(Sfx::CarDoor);
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn board_cars(&mut self, level: &Level) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if !p.drives() || p.player || p.removed || p.is_dead() || p.is_down() || p.aggro() || !matches!(p.state, State::Move) {
                continue;
            }
            let Some(vi) = self.car_for(i) else { continue };
            // At either door: `enter_car` takes the nearer (`position_person_for_vehicle`).
            let v = &self.vehicles[vi];
            if [v.door(), v.to_world(-200.0, 20.0)].iter().any(|d| Vec2::new(p.fig.pos.x - d.x, p.fig.pos.z - d.y).length() < 150.0) {
                self.enter_car(i, vi, level);
                self.streets.drivers.insert(i, Driving::default());
            }
        }
    }

    /// The controls of a car someone other than the player is driving. Wandering
    /// (`PCOM_MOVE_WANDER`) they drive the roads, picking a way at random at each junction
    /// (`PCOM_process_driving_wander`) and keeping to their lane (`DriveCar`, both in
    /// fallen/Source/pcom.cpp); on patrol they drive their route ([`Game::drive_patrol`]);
    /// otherwise they pull up and stay put (`PCOM_process_driving_still`).
    pub(crate) fn npc_drive(&mut self, vi: usize, pi: usize, dt: f32, level: &Level) -> Drive {
        const TURN: f32 = 896.0; // `LEFT_TURN_DIST`: how near the junction before turning
        let v = &self.vehicles[vi];
        let (at, speed) = (Vec2::new(v.pos.x, v.pos.z), v.speed);
        let stop = if speed.abs() > 30.0 { Drive { throttle: -speed.signum(), ..Default::default() } } else { Drive::default() };
        let mv = self.people[pi].mv;
        let mut st = self.streets.drivers.get(&pi).copied().unwrap_or_default();
        if matches!(mv, 2 | 3) {
            return self.drive_patrol(vi, pi, st, dt, level);
        }
        if mv != 4 || self.streets.roads.nodes.len() < 2 {
            st.road = None;
            self.streets.drivers.insert(pi, st);
            return stop;
        }
        // Onto the nearest road, going the way of the lane it's in.
        let (n1, n2) = match st.road {
            Some(r) => r,
            None => {
                let Some((mut a, mut b)) = self.streets.roads.find(at) else { return stop };
                if self.streets.roads.signed_dist(a, b, at) < 0.0 {
                    std::mem::swap(&mut a, &mut b);
                }
                st.next = 0;
                (a, b)
            }
        };
        // Near the junction: pick the next road, and turn into it (a left turn a little later).
        let mut road = (n1, n2);
        let to_end = creature::qdist2(self.streets.roads.dest(n1, n2) - at);
        if to_end < TURN {
            if st.next == 0 {
                let mut rng = self.rngs.main;
                st.next = self.streets.roads.whereto_now(n1, n2, &mut || rand(&mut rng));
                self.rngs.main = rng;
            }
            let late = if st.next != 0 && self.streets.roads.bend(n1, n2, st.next) < 0 { 256.0 } else { 0.0 };
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
        let mut dest = self.streets.roads.dest(n1, n2);
        let reversing = st.reversing > 0.0;
        if !reversing && (384.0 - self.streets.roads.signed_dist(n1, n2, at)).abs() > 128.0 {
            let (r1, r2) = (self.streets.roads.pos(n1), self.streets.roads.pos(n2));
            let ab = r2 - r1;
            let t = ((at - r1).dot(ab) / ab.length_squared().max(1.0)).clamp(0.0, 1.0);
            let sign = |v: f32| if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
            let dr = Vec2::new(sign(ab.x), sign(ab.y)) * 256.0;
            dest = r1 + ab * t + dr * 2.0 + Vec2::new(-dr.y, dr.x);
        }
        self.steer_car(vi, pi, dest, st, dt, level)
    }

    /// A patrol at the wheel (`PCOM_process_driving_patrol`, pcom.cpp:6376): to the nearest
    /// point of their colour and group, then on to the next (on a random patrol, any other,
    /// `EWAY_find_waypoint_rand`), each reached within 0x300; at one with the longest wait
    /// (`EWAY_get_delay` of ten seconds) they park, and stay parked
    /// (`PCOM_set_person_move_park_car`). Gang Order 1 and 2, MIB and Stealth Test have them.
    fn drive_patrol(&mut self, vi: usize, pi: usize, mut st: Driving, dt: f32, level: &Level) -> Drive {
        let v = &self.vehicles[vi];
        let (at, speed) = (Vec2::new(v.pos.x, v.pos.z), v.speed);
        let stop = if speed.abs() > 30.0 { Drive { throttle: -speed.signum(), ..Default::default() } } else { Drive::default() };
        let (colour, group, rand_patrol) = (self.people[pi].colour, self.people[pi].group, self.people[pi].mv == 3);
        let Some(script) = self.script.as_ref() else { return stop };
        let points = script.patrol_points(colour, group);
        if st.parked || points.is_empty() {
            self.streets.drivers.insert(pi, st);
            return stop;
        }
        let flat = |p: Vec3| Vec2::new(p.x, p.z);
        let mut k = st.patrol.and_then(|w| points.iter().position(|q| q.0 == w)).unwrap_or_else(|| {
            (0..points.len()).min_by(|&a, &b| (flat(points[a].1) - at).length().total_cmp(&(flat(points[b].1) - at).length())).unwrap_or(0)
        });
        if creature::qdist2(flat(points[k].1) - at) < 0x300 as f32 {
            if script.point_delay(points[k].0, 0.0) == 10.0 * 1000.0 {
                st.parked = true;
                self.streets.drivers.insert(pi, st);
                return stop;
            }
            k = if rand_patrol && points.len() > 1 {
                let mut rng = self.rngs.main;
                let n = rand(&mut rng) as usize;
                self.rngs.main = rng;
                (k + 1 + n % (points.len() - 1)) % points.len()
            } else {
                (k + 1) % points.len()
            };
        }
        st.patrol = Some(points[k].0);
        self.steer_car(vi, pi, flat(points[k].1), st, dt, level)
    }

    /// Steering car `vi`, driven by `pi`, for `dest`: slowing for a bend, backing off what it
    /// ran into, and stopping for anyone in the road ahead.
    fn steer_car(&mut self, vi: usize, pi: usize, dest: Vec2, mut st: Driving, dt: f32, level: &Level) -> Drive {
        let v = &self.vehicles[vi];
        let (at, speed, yaw, y) = (Vec2::new(v.pos.x, v.pos.z), v.speed, v.yaw, v.pos.y);
        let stop = if speed.abs() > 30.0 { Drive { throttle: -speed.signum(), ..Default::default() } } else { Drive::default() };
        let bent = self.people[pi].bent;
        let reversing = st.reversing > 0.0;
        let to = dest - at;
        let diff = (to.x.atan2(to.y) - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        // Slow for a bend, down to a third of the limit; diligent drivers go twice as fast.
        let dangle = diff.abs() * 2048.0 / std::f32::consts::TAU;
        let mut want = if dangle > 100.0 { (1.0 - (dangle - 100.0) / 2.0 / 750.0).max(250.0 / 750.0) } else { 1.0 } * CRUISE;
        if bent & 2 != 0 || st.reckless {
            want *= 2.0;
        }
        // (The car turns away from its steering: steer left for a larger yaw.)
        let mut steer = (-diff * 2.0).clamp(-1.0, 1.0);
        if st.drunk {
            // Weaving: a slow sway one way and the other, now and then a lurch.
            let t = self.time + pi as f32;
            steer = (steer + (t * 1.3).sin() * 0.6 + (t * 3.1).sin() * 0.25).clamp(-1.0, 1.0);
        }
        if reversing {
            st.reversing -= dt;
            want = -CRUISE * 0.4;
            steer = -steer;
        }
        // Stop for anyone in the road ahead (`PCOM_find_runover_thing`).
        let fwd = Vec2::new(yaw.sin(), yaw.cos());
        let side = Vec2::new(fwd.y, -fwd.x);
        let blocker = self.people.iter().enumerate().find(|&(j, o)| {
            let r = Vec2::new(o.fig.pos.x - at.x, o.fig.pos.z - at.y);
            j != pi && !o.removed && !matches!(o.state, State::InCar { .. }) && (o.fig.pos.y - y).abs() < 150.0
                && r.dot(fwd) > 0.0 && r.dot(fwd) < 250.0 + speed.max(0.0) * 0.5 && r.dot(side).abs() < 170.0
        }).map(|(_, o)| o.fig.pos)
            // And for a car in the way (`VEH_find_runover_things` takes `CLASS_VEHICLE` too,
            // vehicle.cpp:449).
            .or_else(|| self.vehicles.iter().enumerate().find(|&(k, o)| {
                let r = Vec2::new(o.pos.x - at.x, o.pos.z - at.y);
                k != vi && (o.pos.y - y).abs() < 150.0 && r.dot(fwd) > 0.0 && r.dot(fwd) < 450.0 + speed.max(0.0) * 0.5 && r.dot(side).abs() < 170.0
            }).map(|(_, o)| o.pos));
        self.streets.drivers.insert(pi, st);
        if let (Some(them), true) = (blocker, want > 0.0 && !st.drunk && !st.reckless) {
            // Off a zebra crossing, now and then a blast of the horn: on two turns in 128,
            // counted from the car's own number (pcom.cpp:12116, played at :14637).
            let zebra = crate::wand::is_zebra(&level.map, (them.x / 256.0) as i32, (them.z / 256.0) as i32);
            let (turn, was) = ((self.time * 20.0) as u32, ((self.time - dt) * 20.0) as u32);
            if !zebra && turn != was && matches!(turn.wrapping_add(vi as u32) & 0x7f, 0 | 25) {
                let pos = self.vehicles[vi].pos;
                self.sounds.push((Sfx::Horn, pos));
            }
            return stop;
        }
        Drive { throttle: Vehicle::throttle_for(want), steer, ..Default::default() }
    }

    /// Cars move; the player's car follows her controls. Cars knock people over and push
    /// cones and barrels around. `players` is each playing person's driving, for the car
    /// they're at the wheel of (one in single player, one a seat in co-op).
    pub(crate) fn drive(&mut self, dt: f32, players: &[(usize, Drive)], level: &Level) {
        let playing = |pi: usize| players.iter().find(|&&(p, _)| p == pi).map(|&(_, d)| d);
        // Anyone else at the wheel decides first. A driver who has gone (a kill_waypoint)
        // leaves the car where it is.
        let mut npc = vec![None; self.vehicles.len()];
        for (vi, d) in npc.iter_mut().enumerate() {
            if let Some(pi) = self.vehicles[vi].driver.filter(|&pi| playing(pi).is_none()) {
                if matches!(self.people[pi].state, State::Enter { .. }) {
                    // Still getting in.
                } else if self.people[pi].removed {
                    self.vehicles[vi].driver = None;
                } else if self.people[pi].scared.is_some() {
                    // Frightened: pull up, then get out and run (`PCOM_set_person_ai_leavecar`,
                    // parking first if the car is moving).
                    let v = &mut self.vehicles[vi];
                    if v.speed.abs() > 60.0 {
                        *d = Some((pi, Drive { throttle: -v.speed.signum(), ..Default::default() }));
                    } else {
                        v.driver = None;
                        let door = v.door();
                        let y = level.ground.surface(&level.map, door.x, door.y, v.pos.y + 64.0);
                        let p = &mut self.people[pi];
                        p.state = State::Move;
                        p.fig.pos = Vec3::new(door.x, y, door.y);
                        p.cues.push(Sfx::CarDoor);
                        self.streets.drivers.remove(&pi);
                    }
                } else {
                    *d = Some((pi, self.npc_drive(vi, pi, dt, level)));
                }
            }
        }
        for (vi, npc) in npc.into_iter().enumerate() {
            // Who's riding on top, to take along (`WMOVE`), and who's up there or coming down
            // onto it, for it to leave alone.
            let tops = self.vehicle_tops(vi, level);
            let riders = self.riders(&tops);
            let aboard: Vec<usize> = self.people.iter().enumerate()
                .filter(|(_, p)| tops.iter().any(|f| f.height(p.fig.pos.x, p.fig.pos.z, 0.0).is_some_and(|h| p.fig.pos.y > h - 16.0)))
                .map(|(i, _)| i)
                .chain(riders.iter().map(|r| r.0))
                .collect();
            let v = &mut self.vehicles[vi];
            let d = match v.driver.and_then(playing) {
                Some(d) => Drive { faster: true, ..d },
                None => npc.map_or(Drive::default(), |(_, d)| d),
            };
            v.work_siren(d);
            let sliding = v.slip.abs() > 300.0;
            let mut smashed = None;
            let (was_pos, was_yaw) = (v.pos, v.yaw);
            let v = &self.vehicles[vi];
            let touching: Vec<usize> = (0..self.vehicles.len()).filter(|&j| j != vi && v.overlaps(&self.vehicles[j])).collect();
            let v = &mut self.vehicles[vi];
            if let Some(speed) = v.update(d, dt, &level.ground, &level.map) {
                // `DoDamage` (vehicle.cpp:2094), in its units of 1/256 a tick at 20 ticks a
                // second: faster than 20000 smashes the prop it hit, as `OB_damage` does
                // (so breakable ones break, explosive ones go up, `prim_damaged` sees it),
                // and faster than 10000 only makes the noise.
                const SMASH: f32 = 20000.0 / 256.0 * 20.0;
                const NOISE: f32 = 10000.0 / 256.0 * 20.0;
                let front = v.front(speed);
                if speed > NOISE {
                    self.sounds.push((Sfx::Crash, v.pos));
                }
                if speed > SMASH {
                    let hit = touching_prop(level, front, v.pos.y);
                    smashed = hit;
                }
                // Ran into something: back off and come round (`PCOM_MOVE_SUBSTATE_3PTURN`).
                if let Some(dr) = npc.and_then(|(pi, _)| self.streets.drivers.get_mut(&pi)) {
                    if dr.reversing <= 0.0 {
                        dr.reversing = 1.5;
                    }
                }
            }
            // Into another car (`CollideCar` with the vehicles round it, vehicle.cpp:1656): back
            // where it was, bouncing off as off a wall, and both take the crash
            // (`DoDamage`: the other car what this one does, vehicle.cpp:2101), with a
            // smash if the player's in either (`VEH_co_damage`). Cars already touching can
            // part. (The dents of `VEH_add_damage` aren't drawn.)
            let hit = (0..self.vehicles.len()).find(|&j| j != vi && !touching.contains(&j) && self.vehicles[vi].overlaps(&self.vehicles[j]));
            if let Some(j) = hit {
                let v = &mut self.vehicles[vi];
                let speed = v.speed.abs();
                (v.pos, v.yaw) = (was_pos, was_yaw);
                v.speed = if speed > 200.0 { v.speed * -0.25 } else { 0.0 };
                v.slip = 0.0;
                let damage = Vehicle::crash_damage(speed);
                v.health -= damage;
                self.vehicles[j].health -= damage;
                let player = self.player.is_some() && (self.vehicles[vi].driver == self.player || self.vehicles[j].driver == self.player);
                if player && speed > 10000.0 / 256.0 * 20.0 {
                    self.sounds.push((Sfx::Crash, self.vehicles[vi].pos));
                }
                if let Some(dr) = npc.and_then(|(pi, _)| self.streets.drivers.get_mut(&pi)) {
                    if dr.reversing <= 0.0 {
                        dr.reversing = 1.5;
                    }
                }
            }
            if self.vehicles[vi].scrape() && self.turns.is_multiple_of(10) {
                // (The original's moving channel keeps one scrape going; a fresh one every
                // half second here.)
                self.sounds.push((Sfx::Scrape, self.vehicles[vi].pos));
            }
            let v = &self.vehicles[vi];
            if !sliding && v.slip.abs() > 300.0 {
                self.sounds.push((Sfx::Skid, v.pos));
            }
            // Brushing past one at speed counts too: the original damages whatever prop its
            // corners touch (`VEH_collide`), whether or not the car is stopped.
            if smashed.is_none() && self.vehicles[vi].speed.abs() > 20000.0 / 256.0 * 20.0 {
                let car = &self.vehicles[vi];
                smashed = touching_prop(level, car.front(car.speed), car.pos.y).filter(|i| !self.damaged.contains(i));
                if smashed.is_some() {
                    self.sounds.push((Sfx::Crash, car.pos));
                }
            }
            if let Some(i) = smashed {
                // (`VEH_collide` passes the car's x and y as where the knock came from,
                // vehicle.cpp:2123, not its x and z.)
                let p = self.vehicles[vi].pos;
                self.damage_prop(level, i, (p.x, p.y));
            }
            let after = self.vehicle_tops(vi, level);
            self.carry(&riders, &tops, &after);
            let v = self.vehicles[vi].clone();
            // (Not someone still getting in: they're at the door.)
            if let Some(d) = v.driver.filter(|&d| matches!(self.people[d].state, State::InCar { .. })) {
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
                    if p.player || Some(i) == v.driver || p.removed || p.is_down() || p.dodge.is_some() || aboard.contains(&i)
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
            // Going fast, it scares civilians near where it's headed: past `Velocity` 500
            // (each worth 1.25 world units a second), a noise two squares ahead (`VEH_driving`,
            // vehicle.cpp:3395).
            if v.speed.abs() > 625.0 {
                let ahead = fwd * v.speed.signum() * 512.0;
                self.noises.push((person::Noise::Van, v.pos + Vec3::new(ahead.x, 0.0, ahead.y), v.driver));
            }
            // People in the way.
            for (i, p) in self.people.iter_mut().enumerate() {
                if Some(i) == v.driver || p.removed || matches!(p.state, State::InCar { .. }) || (p.fig.pos.y - v.pos.y).abs() > 150.0 || aboard.contains(&i) {
                    continue;
                }
                let at = Vec2::new(p.fig.pos.x, p.fig.pos.z);
                if !v.covers(at, 30.0) {
                    continue;
                }
                if v.speed.abs() > 300.0 && !p.is_down() {
                    p.hurt_by = v.driver;
                    let behind = p.hit_from_behind(v.pos);
                    p.take_hit(v.pos, (v.speed.abs() / 12.0) as i32, person::ko::KNOCKDOWN, false, true, Weapon::None);
                    // "A nice sound!" (vehicle.cpp:3339).
                    self.sounds.push((Sfx::Squish, p.fig.pos));
                    // Killed by it, on the ground: `PERSON_DEATH_TYPE_OTHER` (vehicle.cpp:3329).
                    if p.is_dead() && !p.arrested {
                        p.knocked_dead(behind);
                    }
                }
                let away = if (at - Vec2::new(v.pos.x, v.pos.z)).dot(side) >= 0.0 { side } else { -side };
                let to = level.ground.slide(at, p.fig.pos.y, away * 40.0, 36.0);
                p.slid_odd = Some(true);
                p.fig.pos.x = to.x;
                p.fig.pos.z = to.y;
            }
            // Cones and barrels get knocked about (`BARREL_hit_with_prim` with the car's
            // body; here as two spheres, at its nose and its tail).
            if v.speed.abs() > 60.0 {
                let ends = [v.place(Vec3::new(0.0, 60.0, 100.0)), v.place(Vec3::new(0.0, 60.0, -100.0))];
                let mut seed = self.rngs.barrel;
                for at in ends {
                    for k in barrel::hit_with_sphere(&mut self.barrels, at, 0x60 as f32, &mut || rand(&mut seed)) {
                        let at = self.barrels[k].pos;
                        self.sounds.push((self.barrels[k].knock_sound(), at));
                    }
                }
                self.rngs.barrel = seed;
            }
        }
        self.sync_wmove(level);
    }

    /// The car the player is driving.
    /// The nearest car to `at` with its siren going, to hear it from.
    pub fn siren_noise(&self, at: Vec3) -> Option<Vec3> {
        self.vehicles.iter().filter(|v| v.siren).map(|v| v.pos).min_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at)))
    }

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
            if !self.vehicles[v].wrecked() || !self.tally.blown.insert(v) {
                continue;
            }
            let at = self.vehicles[v].pos;
            // Blowing up a police car or a meat wagon earns the player a red mark
            // (`VEH_reduce_health`, vehicle.cpp:4804).
            if self.vehicles[v].hurt_by.is_some() && self.vehicles[v].hurt_by == self.player && matches!(self.vehicles[v].kind, 3 | 6) {
                self.red_marks = self.red_marks.saturating_add(1);
            }
            self.pyro.burst(at + Vec3::new(0.0, 48.0, 0.0), pyro::FIREBOMB, 256.0);
            self.sounds.push((Sfx::Explosion, at));
            if self.aiming.vehicle == Some(v) {
                self.aiming.vehicle = None;
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

/// The breakable prop whose collision outline the round end of a car at `front` touches.
fn touching_prop(level: &Level, front: Vec2, y: f32) -> Option<usize> {
    let g = &level.ground;
    g.prop_walls.iter()
        .filter(|(i, _)| g.prop_damage.get(*i).is_some_and(|d| d.0 & DAMAGABLE != 0))
        .filter(|(_, r)| g.walls[r.start].y0 < y + 128.0 && g.walls[r.start].y1 > y)
        .map(|(i, r)| {
            let d = g.walls[r.clone()].iter().map(|w| {
                let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
                let t = ((front - a).dot(b - a) / (b - a).length_squared().max(1.0)).clamp(0.0, 1.0);
                (a + (b - a) * t - front).length()
            }).fold(f32::MAX, f32::min);
            (*i, d)
        })
        .filter(|&(_, d)| d < vehicle::REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}
