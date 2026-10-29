//! Making people, and walking them to where their script wants them.

use crate::*;

/// The missions where the sniper rifle lies in front of the player's start: Officer Training,
/// RTA, The Jump, Seek Sneak and Seize, Assassin and The Fallen.
pub(crate) const SNIPER_AT_START: [&str; 6] = ["train11", "police1", "police2", "Stealtst1", "Gangorder1", "MIB"];

/// What person `p` may do on a route: everything Darci can (walk, jump, pull up, climb
/// fences and ladders, drop off edges), but for `PCOM_BENT_RESTRICTED` people, and for
/// civilians wandering or running scared from somewhere a civilian may wander, who only walk
/// (pcom.cpp:1957).
fn caps_of(p: &Person, wand: &wand::Wander) -> u8 {
    let walk = uc_world::mav::caps(uc_world::mav::GOTO);
    if p.bent & person::BENT_RESTRICTED != 0 {
        return walk;
    }
    let wandering = (p.mv == person::MOVE_WANDER && !p.aggro()) || p.scared.is_some();
    let (x, z) = ((p.fig.pos.x / 256.0) as i32, (p.fig.pos.z / 256.0) as i32);
    if p.ai == person::AI_CIV && wandering && wand.is_wander(x, z) {
        return walk;
    }
    uc_world::mav::CAPS_DARCI
}

/// How long a route is kept while its walker stays in the same square, though where they're
/// going moves (the original plans again only on reaching the route's next square).
const REPLAN: f32 = 1.0;

/// How long a route is kept at most: then it's planned again, in case its walker is stuck
/// (`pcom_move_counter > PCOM_get_duration(20)`, pcom.cpp:12452).
const RENAV: f32 = 2.0;

/// How near the middle of the square a walk goes to counts as there (`PCOM_ARRIVE_DIST`).
const ARRIVE: f32 = 64.0;

/// A person's last route: (from square, to square), the furthest square to walk to and the
/// first square along it, or the square over an edge to drop into; and when it was made.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Route {
    from: (u8, u8),
    to: (u8, u8),
    dest: Option<(u8, u8)>,
    first: Option<(u8, u8)>,
    drop: Option<(u8, u8)>,
    /// A jump, pull-up or ladder to do first (`MAV_ACTION_*`), in direction `MAV_DIR_*`.
    leap: Option<(u8, u8)>,
    at: f32,
}

/// Route-finding: the navigation map (`MAV_precalculate`) and each person's last route.
pub(crate) struct Routes {
    pub(crate) mav: uc_world::mav::Mav,
    /// The one for inside the warehouses ([`uc_world::mav::Mav::inside`]), if there are any.
    pub(crate) inside: Option<uc_world::mav::Mav>,
    pub(crate) ways: HashMap<usize, Route>,
}

impl Routes {
    pub(crate) fn new(level: &Level) -> Routes {
        Routes { mav: uc_world::mav::Mav::new(level), inside: uc_world::mav::Mav::inside(level), ways: HashMap::new() }
    }
}

impl Game {
    pub(crate) fn spawn(&mut self, level: &Level, kind: u8, player: bool, way: usize, pos: Vec3, angle: f32) -> Option<usize> {
        let variant = self.rand();
        self.spawn_variant(level, kind, player, way, pos, angle, variant)
    }

    /// [`Game::spawn`], with the look picked by `variant`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_variant(&mut self, level: &Level, kind: u8, player: bool, way: usize, pos: Vec3, angle: f32, variant: u32) -> Option<usize> {
        let people = level.people.as_ref()?;
        // On the surface nearest the point (`create_person`, Person.cpp:761). Points can sit a
        // little under the surface they're on: The Jump's jumper is 64 below his ledge. One
        // in the air with nothing under it is left there, and falls (bball2's warehouse).
        let y = level.ground.spawn_height(&level.map, pos.x, pos.z, pos.y);
        let fig = people.figure(kind, variant, Vec3::new(pos.x, y, pos.z), angle)?;
        self.people.push(Person::new(fig, kind, player, way));
        let i = self.people.len() - 1;
        if y - level.ground.surface(&level.map, pos.x, pos.z, y) > uc_world::ground::STEP_HEIGHT {
            self.people[i].fall(Vec2::ZERO);
        }
        if way != 0 {
            self.by_way.insert(way, i);
        }
        Some(i)
    }

    pub(crate) fn spawn_player(&mut self, level: &Level, pos: Vec3, angle: f32, way: usize, kind: u8, has: u8) {
        if self.player.is_none() {
            self.player = self.spawn(level, kind, true, way, pos, angle);
            self.player_name = player_name(&level.mission.mission_name, kind);
            // Carried, not drawn: `PCOM_create_player` only raises `FLAGS_HAS_GUN`, which puts
            // the pistol in the weapon list (`PANEL_ADDWEAPON`) for the player to pick.
            if let Some(i) = self.player {
                self.people[i].give(Weapon::from_has(has));
                // Stern Revenge hands her a shotgun too, hard-coded (`playing_level`,
                // pcom.cpp:5601).
                if level.mission.mission_name.eq_ignore_ascii_case("skymiss2") {
                    self.people[i].give(Weapon::Shotgun);
                }
                // Not in the original: a few missions have the sniper rifle lying just in
                // front of where the player starts, to try it out.
                if SNIPER_AT_START.iter().any(|m| level.mission.mission_name.eq_ignore_ascii_case(m)) {
                    let f = self.people[i].facing_dir();
                    let at = pos + Vec3::new(f.x, 0.0, f.y) * 128.0;
                    let y = level.ground.surface(&level.map, at.x, at.z, pos.y + 64.0);
                    self.items.push(Item { way: 0, kind: crate::SNIPER, pos: Vec3::new(at.x, y, at.z), rounds: None });
                }
            }
        }
    }

    /// Round what's in the way (`PCOM_set_person_move_mav_to_xz` and `MAV_do`): anyone whose
    /// goal, or the person they're fighting, can't be walked to in a straight line heads
    /// for the furthest square of the navigation map's route that can be. On foot and over
    /// fences ([`WALKER`]).
    /// Somewhere someone might be hiding, for whoever investigated a noise and is going on
    /// to look there (`PCOM_find_hiding_place`, pcom.cpp:1589): a square within five
    /// (each way) not in a building, two or more away (x plus z), that can't be seen from
    /// their head (0x60 up) to a head standing on it, with a route there that doesn't start
    /// off a long way round (its first leg at most two further, `MAV_do`); of those, at
    /// random, the nearer the likelier (`0x1000 - Random() % (dist << 4)`). Nowhere, and
    /// home they go.
    pub(crate) fn hiding_places(&mut self, level: &Level) {
        for i in 0..self.people.len() {
            if !self.people[i].heard.is_some_and(|h| h.phase == person::Investigate::Hide) {
                continue;
            }
            let at = self.people[i].fig.pos;
            let (mx0, mz0) = ((at.x / 256.0) as i32, (at.z / 256.0) as i32);
            let eye = at + Vec3::new(0.0, 0x60 as f32, 0.0);
            let mut seed = self.rngs.mav;
            let mut best: Option<(i32, Vec3)> = None;
            for dx in -5..=5 {
                for dz in -5..=5 {
                    let (mx, mz) = (mx0 + dx, mz0 + dz);
                    let dist = dx.abs() + dz.abs();
                    if !(0..128).contains(&mx) || !(0..128).contains(&mz) || dist < 2
                        || level.map.square(mx as usize, mz as usize).has(uc_formats::iam::flags::HIDDEN)
                    {
                        continue;
                    }
                    let (x, z) = (mx as f32 * 256.0 + 128.0, mz as f32 * 256.0 + 128.0);
                    let spot = Vec3::new(x, level.map.ground_height(x, z), z);
                    if level.ground.line_of_sight(eye, spot + Vec3::new(0.0, 0x60 as f32, 0.0)) {
                        continue;
                    }
                    let (step, found) = self.routes.mav.route((mx0 as u8, mz0 as u8), (mx as u8, mz as u8), uc_world::mav::CAPS_DARCI, &mut || rand(&mut seed));
                    let leg = (step.dest.0 as i32 - mx0).abs() + (step.dest.1 as i32 - mz0).abs();
                    if !found || leg > dist + 2 {
                        continue;
                    }
                    let score = 0x1000 - (rand(&mut seed) % (dist << 4) as u32) as i32;
                    if best.is_none_or(|(b, _)| score > b) {
                        best = Some((score, spot));
                    }
                }
            }
            self.rngs.mav = seed;
            self.people[i].heard = best.map(|(_, spot)| person::Heard { at: spot, phase: person::Investigate::WalkOver, time: 0.0 });
        }
    }

    pub(crate) fn find_ways(&mut self, level: &Level) {
        for i in 0..self.people.len() {
            self.people[i].leap = None;
            let p = &self.people[i];
            if p.player || p.removed || p.is_dead() || p.is_down() {
                self.people[i].via = None;
                continue;
            }
            if let Some(s) = p.steer {
                self.people[i].via = Some(s);
                continue;
            }
            let at = p.fig.pos;
            if let Some(goal) = p.goal {
                if let Some(leg) = self.leg(i, at, goal, level) {
                    self.people[i].goal = Some(leg);
                }
            }
            if self.people[i].scared.is_some() {
                let via = self.run_away(i, at, level);
                self.people[i].via = via;
                continue;
            }
            let p = &self.people[i];
            let foe = p.foe.or(self.player).filter(|_| p.aggro()).map(|f| self.people[f].fig.pos);
            let via = foe.and_then(|to| self.leg(i, at, to, level));
            self.people[i].via = via;
        }
    }

    /// Running from something (`PCOM_process_fleeing`): a place 32 squares or so straight away
    /// from the danger, give or take four, gone to by the navigation map, and another once
    /// there (`PCOM_set_person_move_runaway`, pcom.cpp:2466). A place whose way starts off
    /// towards the danger is tried again, twice. Returns the next leg.
    fn run_away(&mut self, i: usize, at: Vec3, level: &Level) -> Option<Vec3> {
        let s = self.people[i].scared?;
        let danger = s.at;
        let arrived = s.goal.is_none_or(|g| Vec2::new(g.x - at.x, g.z - at.z).length() < 1.5 * 256.0);
        if !arrived {
            return self.leg(i, at, s.goal?, level);
        }
        let size = (uc_formats::iam::MAP_SIZE * 256) as f32;
        let mut pick = None;
        for _ in 0..3 {
            let (dx, dz) = (at.x - danger.x, at.z - danger.z);
            let dist = dx.abs() + dz.abs() + 1.0;
            let mut r = || (rand(&mut self.rngs.mav) & 0x7ff) as f32 - 0x400 as f32;
            let mut gx = at.x + dx * 8192.0 / dist + r();
            let mut gz = at.z + dz * 8192.0 / dist + r();
            // Stay on the map, bouncing off its edges.
            (gx, gz) = (gx.abs(), gz.abs());
            if gx > size { gx = 2.0 * size - gx; }
            if gz > size { gz = 2.0 * size - gz; }
            let goal = Vec3::new(gx, at.y, gz);
            self.routes.ways.remove(&i);
            let leg = self.leg(i, at, goal, level);
            let dest = leg.unwrap_or(goal);
            let flat = |a: Vec3, b: Vec3| Vec2::new(a.x - b.x, a.z - b.z).length();
            pick = Some((goal, leg));
            if flat(dest, danger) >= flat(dest, at) {
                break;
            }
        }
        let (goal, leg) = pick?;
        if let Some(s) = self.people[i].scared.as_mut() {
            s.goal = Some(goal);
        }
        leg
    }

    /// The next leg of the way from `from` to `to`: `None` to go straight (it can be walked,
    /// or there's no knowing better), else the middle of the furthest square along the route
    /// that can be walked to, or of the route's first square if a body's width of line from
    /// where they stand to that one catches a wall (the corner of a building). A person's
    /// route is kept until they change square, or where they're going does and it's a second
    /// old, or it's two seconds old. A route that starts with a drop off an edge heads
    /// straight for the square over it.
    ///
    /// Someone who has just stepped into the square they were walking to keeps on for its
    /// middle before going on (round a corner, rather than straight at the wall or the lamp
    /// post on it).
    fn leg(&mut self, i: usize, from: Vec3, to: Vec3, level: &Level) -> Option<Vec3> {
        let sq = |v: Vec3| ((v.x / 256.0).clamp(0.0, 127.0) as u8, (v.z / 256.0).clamp(0.0, 127.0) as u8);
        let (a, b) = (sq(from), sq(to));
        let middle = |s: (u8, u8)| Vec3::new(s.0 as f32 * 256.0 + 128.0, from.y, s.1 as f32 * 256.0 + 128.0);
        // Arrived only within `PCOM_ARRIVE_DIST` of the middle of the walk's square
        // (`PCOM_process_movement`, pcom.cpp:12340).
        if let Some(r) = self.routes.ways.get(&i) {
            let walking = r.drop.is_none() && r.leap.is_none() && r.dest == Some(a) && a != b;
            if walking && self.time - r.at < RENAV && Vec2::new(from.x - middle(a).x, from.z - middle(a).z).length() >= ARRIVE {
                return Some(middle(a));
            }
        }
        // In a warehouse, or going into one: by the map of their floors (`WARE_mav_inside`,
        // `WARE_mav_enter`).
        let indoors = |at: Vec3| self.wares.under_roof(&level.map, &level.ground, at).is_some();
        let mav = self.routes.inside.as_ref().filter(|_| indoors(from) || indoors(to)).unwrap_or(&self.routes.mav);
        if a == b || mav.can_walk(a, b) {
            return None;
        }
        let route = match self.routes.ways.get(&i) {
            Some(&r) if r.from == a && (r.to == b || self.time - r.at < REPLAN) && self.time - r.at < RENAV => r,
            _ => {
                let mut rng = self.rngs.mav;
                let caps = caps_of(&self.people[i], &self.streets.wand);
                let (act, _) = mav.route(a, b, caps, &mut || rand(&mut rng));
                self.rngs.mav = rng;
                // Over a fence: to the square it's climbed from first (`dest`), then straight
                // at it, and the walker climbs it.
                let climb = act.action == uc_world::mav::CLIMB_OVER;
                let walk = (act.action == uc_world::mav::GOTO || climb) && act.dest != a;
                let (dest, first) = if walk { (Some(act.dest), Some(act.first)) } else { (None, None) };
                // Dropping off an edge, or over a fence from where it's climbed: into the next
                // square that way, square on, so the walker drops or climbs (a fence met at a
                // slant isn't climbed).
                let drop = (act.action == uc_world::mav::FALL_OFF || climb && act.dest == a).then(|| {
                    let (dx, dz) = uc_world::mav::DIRS[act.dir as usize & 3];
                    ((a.0 as i32 + dx).clamp(0, 127) as u8, (a.1 as i32 + dz).clamp(0, 127) as u8)
                });
                use uc_world::mav::{JUMP, JUMPPULL, JUMPPULL2, LADDER_UP, PULLUP};
                let leap = matches!(act.action, JUMP | JUMPPULL | JUMPPULL2 | PULLUP | LADDER_UP).then_some((act.action, act.dir));
                let r = Route { from: a, to: b, dest, first, drop, leap, at: self.time };
                self.routes.ways.insert(i, r);
                r
            }
        };
        let (dest, first) = (route.dest, route.first);
        if let Some(over) = route.drop {
            return Some(middle(over));
        }
        // A jump or a pull-up: to the edge of the square the way it goes, and jump there
        // (the walker grabs a ledge too high to land on, and pulls up by itself). A ladder:
        // to its foot, and walk into it.
        if let Some((action, dir)) = route.leap {
            let (dx, dz) = uc_world::mav::DIRS[dir as usize & 3];
            let d = Vec2::new(dx as f32, dz as f32);
            let mid = middle(a);
            let edge = mid + Vec3::new(d.x, 0.0, d.y) * 112.0;
            if action == uc_world::mav::LADDER_UP {
                let ladder = level.ground.ladders.iter()
                    .filter(|l| l.out.dot(-d) > 0.7 && (l.foot - Vec2::new(edge.x, edge.z)).length() < 256.0)
                    .min_by(|p, q| (p.foot - Vec2::new(from.x, from.z)).length().total_cmp(&(q.foot - Vec2::new(from.x, from.z)).length()));
                if let Some(l) = ladder {
                    let foot = l.foot + l.out * 60.0;
                    let at = Vec3::new(foot.x, from.y, foot.y);
                    self.people[i].leap = Some((at, -l.out, false));
                    return Some(at);
                }
                return Some(edge);
            }
            self.people[i].leap = Some((edge, d, true));
            return Some(edge);
        }
        let dest = middle(dest?);
        // A body's width: the line from each side of them as well as the middle.
        let waist = Vec3::new(0.0, uc_world::ground::STEP_HEIGHT - 4.0, 0.0);
        let along = (dest - from).normalize_or_zero();
        let side = Vec3::new(-along.z, 0.0, along.x) * 40.0;
        let clear = [Vec3::ZERO, side, -side].iter().all(|&o| level.ground.line_of_sight(from + waist + o, dest + waist + o));
        Some(if clear { dest } else { middle(first.unwrap_or(sq(dest))) })
    }

    /// `PCOM_MOVE_FOLLOW_ON_SEE` (pcom.cpp:10479): someone within 0x200 of the person their
    /// `follow` point made (the player by default) who can see them starts following them
    /// (`PCOM_MOVE_FOLLOW`). Takeover1's hostages wait for Darci this way.
    pub(crate) fn follow_on_see(&mut self, level: &Level) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.mv != person::MOVE_FOLLOW_ON_SEE || p.player || p.is_dead() || p.removed {
                continue;
            }
            let j = if p.follow == 0 { self.player } else { self.by_way.get(&p.follow).copied() };
            let Some(j) = j.filter(|&j| j != i) else { continue };
            if (self.people[j].fig.pos - p.fig.pos).length() < 0x200 as f32 && self.can_a_see_b(i, j, level) {
                self.people[i].mv = person::MOVE_FOLLOW;
            }
        }
    }

    /// Where each person is walking to when they aren't fighting: `PCOM_process_patrol`
    /// (move 2 or 3) heads for the nearest active point of their colour and group, and once
    /// there, the next one by number, and so on (it keeps to the one it's
    /// heading for, not the nearest, which would turn it back once it's left a point); `PCOM_MOVE_FOLLOW` (move 5) keeps up
    /// with the person their `follow` point created, the player by default.
    pub(crate) fn walk_goals(&mut self) {
        let Some(script) = &self.script else { return };
        // Where everyone stands this frame, so they all aim at the same thing.
        let at: Vec<Vec3> = self.people.iter().map(|p| p.fig.pos).collect();
        let mut goals: Vec<(Option<Vec3>, person::Pace)> = vec![(None, person::Pace::Walk); self.people.len()];
        let mut patrols: Vec<Option<usize>> = self.people.iter().map(|p| p.patrol).collect();
        for (i, p) in self.people.iter().enumerate() {
            // `PCOM_AI_NONE` does nothing at all (pcom.cpp:10851): no walking anywhere.
            if p.player || p.aggro() || p.is_dead() || p.ai == 0 {
                continue;
            }
            // QDIST3: the largest axis plus a quarter of each of the other two.
            let near = |q: Vec3| {
                let d = (q - at[i]).abs();
                d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0
            };
            // A driver out of a car walks to the nearer door of the nearest one free (not
            // round the car to the driver's: whichever, as `enter_car` has it).
            if p.drives() {
                if let (true, Some(vi)) = (matches!(p.state, State::Move), self.car_for(i)) {
                    let v = &self.vehicles[vi];
                    let here = Vec2::new(at[i].x, at[i].z);
                    let door = [v.door(), v.to_world(-200.0, 20.0)].into_iter()
                        .min_by(|a, b| (*a - here).length().total_cmp(&(*b - here).length()))
                        .unwrap_or(v.door());
                    goals[i] = (Some(Vec3::new(door.x, at[i].y, door.y)), person::Pace::Walk);
                }
                continue;
            }
            goals[i] = match p.mv {
                // Followers run after whoever they're following, as `PCOM_set_person_ai_follow`
                // asks (`PERSON_SPEED_RUN`), and break into a sprint to keep up with a player
                // who is sprinting. They stop on getting within 0x60 of them
                // (`PCOM_finished_nav`), and set off again once they're more than 0xa0 away,
                // or 0x40 if they're running (`PCOM_process_following`, pcom.cpp:8104).
                5 => {
                    let j = if p.follow == 0 { self.player } else { self.by_way.get(&p.follow).copied() };
                    // In a car, they climb in with them (`PCOM_set_person_ai_hitch`, pcom.cpp:4758):
                    // off to its passenger door at a run (a walk for the lazy).
                    if let Some(State::InCar { vehicle: v }) = j.map(|j| self.people[j].state.clone()) {
                        if !matches!(p.state, State::InCar { .. }) && !self.vehicles[v].wrecked() {
                            let door = self.vehicles[v].to_world(-200.0, 20.0);
                            let pace = if p.bent & person::BENT_LAZY != 0 { person::Pace::Walk } else { person::Pace::Run };
                            goals[i] = (Some(Vec3::new(door.x, at[i].y, door.y)), pace);
                        }
                        continue;
                    }
                    let going = p.goal.is_some();
                    let to = j
                        .filter(|&j| !self.people[j].removed && !self.people[j].is_dead())
                        .filter(|&j| {
                            let d = (at[j] - at[i]).length();
                            let running = person::running_anim(self.people[j].fig.anim());
                            if going { d >= 0x60 as f32 } else { d > if running { 0x40 } else { 0xa0 } as f32 }
                        })
                        .map(|j| at[j]);
                    let hurrying = j == self.player && self.sprinting;
                    // Keeping down with them when they crouch or crawl (`PERSON_SPEED_CRAWL`,
                    // pcom.cpp:8016).
                    let crawling = j.is_some_and(|j| matches!(self.people[j].state, State::Crouch { .. }));
                    (to, if crawling { person::Pace::Crawl } else if hurrying { person::Pace::Sprint } else { person::Pace::Run })
                }
                // `PCOM_process_patrol`: a walk, unless they're `PCOM_BENT_DILIGENT`. On a
                // random patrol (`PCOM_MOVE_PATROL_RAND`), the next point is any of the others
                // (`EWAY_find_waypoint_rand`), picked by who and when rather than the game's
                // chance.
                2 | 3 => {
                    let points = script.patrol_points(p.colour, p.group);
                    let heading = p.patrol.and_then(|w| points.iter().position(|q| q.0 == w));
                    let nearest = (0..points.len()).min_by(|&a, &b| near(points[a].1).total_cmp(&near(points[b].1)));
                    let pick = i.wrapping_mul(7919) + (self.time * 20.0) as usize;
                    let to = heading.or(nearest).map(|k| {
                        let next = if p.mv == 3 && points.len() > 1 { (k + 1 + pick % (points.len() - 1)) % points.len() } else { (k + 1) % points.len() };
                        // On the way to a point: there within `PCOM_ARRIVE_DIST`, or a square if
                        // they've been sliding along a wall a while (`PCOM_finished_nav`, which
                        // measures from the middle of its square, where the original's walk
                        // ends; here the walk is to the point). Setting out: one within a
                        // square of the nearest is passed over for the next.
                        let arrived = if heading.is_some() {
                            let d = Vec2::new(points[k].1.x - at[i].x, points[k].1.z - at[i].z).length();
                            d < ARRIVE || p.slide_odd > 20 && d < 256.0
                        } else {
                            near(points[k].1) < 256.0
                        };
                        let k = if arrived { next } else { k };
                        patrols[i] = Some(points[k].0);
                        points[k].1
                    });
                    (to, if p.bent & 2 != 0 { person::Pace::Run } else { person::Pace::Walk })
                }
                // `PCOM_MOVE_WARM_HANDS` (pcom.cpp:10461): to the nearest fire within 0x300, to
                // stand at it, an arm's length off.
                // Standing still (or posing) at home: more than 256 off it, back they go
                // (`PCOM_set_person_ai_homesick`, pcom.cpp:10213), at a walk. Someone with a
                // fight on (a robot assassin holding his mark at gunpoint) stays where he is.
                1 | person::MOVE_DANCE | person::MOVE_HANDS_UP | person::MOVE_TIED_UP if !(p.fights() && p.foe.is_some()) => {
                    // How far, along the ground (`PCOM_get_dist_from_home`).
                    // Once on the way, all the way there (`PCOM_ARRIVE_DIST`, 0x40).
                    let far = if p.goal.is_some() { 0x40 as f32 } else { 256.0 };
                    let home = p.home.map(|(h, _)| h).filter(|h| Vec2::new(h.x - at[i].x, h.z - at[i].z).length() > far);
                    (home, person::Pace::Walk)
                }
                person::MOVE_WARM_HANDS => {
                    let fire = self.pyro.fires().map(|(f, _)| f).filter(|f| (*f - at[i]).length() < 0x300 as f32)
                        .min_by(|a, b| (*a - at[i]).length().total_cmp(&(*b - at[i]).length()));
                    let to = fire.map(|f| f + (at[i] - f).normalize_or(Vec3::X) * 96.0).filter(|t| (*t - at[i]).length() > 48.0);
                    (to, person::Pace::Walk)
                }
                _ => (None, person::Pace::Walk),
            };
        }
        for ((p, (goal, pace)), patrol) in self.people.iter_mut().zip(goals).zip(patrols) {
            (p.goal, p.pace, p.patrol) = (goal, pace, patrol);
        }
    }

    /// A Man in Black has seen or heard the player: everyone within 0x500 of where he raised
    /// it who'd fight comes for her, himself included (`PCOM_alert_nearby_mib_to_attack`,
    /// pcom.cpp:10678).
    pub(crate) fn raise_alerts(&mut self) {
        let alerts: Vec<Vec3> = self.people.iter_mut().filter_map(|p| p.alert.take()).collect();
        for at in alerts {
            for p in self.people.iter_mut().filter(|p| (p.fig.pos - at).length() < 0x500 as f32) {
                p.called_in();
            }
        }
    }

    /// The bullies pick their victims (`PCOM_AI_BULLY`, pcom.cpp:11514): once one's down,
    /// they're free again, and while free, every 8 ticks each looks round for someone to pick
    /// on (`PCOM_can_i_see_person_to_bully`, pcom.cpp:5811): anyone they can see within 0x600
    /// who isn't down, a passer-by strolling about or the player (only the player if they're
    /// `PCOM_BENT_ONLY_KILL_PLAYER`). The nearest wins, the height counting double and the
    /// player at half the distance.
    pub(crate) fn bullies(&mut self, snaps: &[person::Snap], dt: f32, level: &Level) {
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.ai != person::AI_BULLY || p.player || p.is_down() || p.is_dead() {
                continue;
            }
            if p.victim.is_some() {
                continue;
            }
            let clock = p.bully_clock - dt;
            if clock > 0.0 {
                self.people[i].bully_clock = clock;
                continue;
            }
            self.people[i].bully_clock = clock + 8.0 / 20.0;
            let p = &self.people[i];
            let only_player = p.bent & person::BENT_ONLY_KILL_PLAYER != 0;
            let victim = self.people.iter().enumerate()
                .filter(|&(j, q)| {
                    j != i && !q.removed && !q.is_down() && !q.is_dead()
                        && (q.player || (!only_player && q.ai == person::AI_CIV && q.mv == person::MOVE_WANDER))
                        && (q.fig.pos - p.fig.pos).length() < 0x600 as f32
                        && p.sees(&snaps[j], &level.ground)
                })
                .map(|(j, q)| {
                    let d = (q.fig.pos - p.fig.pos).abs();
                    let score = d.x + d.y * 2.0 + d.z;
                    (j, if q.player { score / 2.0 } else { score })
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(j, _)| j);
            if let Some(v) = victim {
                self.people[i].bully_after(v);
            }
        }
    }

    /// `PCOM_call_cop_to_arrest_me(.., 1)`: `i` is up to no good; a cop may come for them
    /// once the turn is over.
    pub(crate) fn call_cop(&mut self, i: usize) {
        /// `MAX_ARREST_ME` (PC).
        const MAX_ARREST_ME: usize = 5;
        if self.arrest_me.len() < MAX_ARREST_ME && !self.arrest_me.contains(&i) {
            self.arrest_me.push(i);
        }
    }

    /// `do_arrests` (Thing.cpp:1224, pcom.cpp:5963): for each villain called in, every cop
    /// within 0x800 who is doing nothing much and whom the villain can see marks them a
    /// felon and comes after them, calling it out. A tramp isn't worth it. A cop at the wheel
    /// would first get out of the car; here only cops on foot come.
    pub(crate) fn do_arrests(&mut self, level: &Level) {
        for v in std::mem::take(&mut self.arrest_me) {
            let p = &self.people[v];
            if p.removed || p.is_dead() || p.kind == uc_world::character::person::TRAMP {
                continue;
            }
            let at = p.fig.pos;
            let cops: Vec<usize> = (0..self.people.len())
                .filter(|&j| {
                    let c = &self.people[j];
                    j != v && c.ai == person::AI_COP && !matches!(c.state, State::InCar { .. }) && c.doing_nothing_important()
                        && c.victim.is_none() && (c.fig.pos - at).length() < 0x800 as f32
                })
                .filter(|&j| self.can_a_see_b(v, j, level))
                .collect();
            for j in cops {
                self.people[v].felon = true;
                self.people[j].bully_after(v);
                self.people[j].cues.push(Sfx::CopArrest);
            }
        }
    }

    /// A gang member who has just gone into a fight brings in the rest of their gang: every
    /// one of their colour who isn't busy fighting goes for the same person, however far off
    /// (`PCOM_alert_my_gang_to_a_fight`, pcom.cpp:1781, from `PCOM_set_person_ai_kill_person`).
    /// And anyone out to get someone who's down is free again.
    pub(crate) fn back_up_gangs(&mut self) {
        let fighting = std::mem::take(&mut self.fighting);
        for i in 0..self.people.len() {
            if let Some(v) = self.people[i].victim {
                let q = &self.people[v];
                if q.removed || q.is_down() || q.is_dead() {
                    self.people[i].victim = None;
                    self.people[i].calm_down();
                }
            }
        }
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if !p.aggro() || fighting.get(i).copied().unwrap_or(false) {
                continue;
            }
            // A guard who's spotted someone shouts (`PCOM_SOUND_HEY`, pcom.cpp:11030).
            if p.ai == person::AI_GUARD {
                self.noises.push((person::Noise::Hey, p.fig.pos, Some(i)));
            }
            let p = &self.people[i];
            if p.bent & person::BENT_GANG == 0 {
                continue;
            }
            let Some(target) = p.victim.or(p.foe) else { continue };
            let colour = p.colour;
            for (j, q) in self.people.iter_mut().enumerate() {
                if j != i && j != target && q.bent & person::BENT_GANG != 0 && q.colour == colour && !q.aggro() {
                    q.join_fight(target);
                }
            }
        }
        self.fighting = self.people.iter().map(|p| p.aggro()).collect();
    }

    /// The lazy sit down (`PCOM_BENT_LAZY` standing still at home, pcom.cpp:10286): every 64
    /// ticks each looks for the nearest bench, sofa or armchair within 0x200 (x and z plus
    /// twice the height) and sits on it, put straight there
    /// (`PCOM_position_person_to_sit_on_prim`), unless the player is near enough to see them
    /// jump there: then only if it's within 0x50 already.
    pub(crate) fn sit_down(&mut self) {
        let tick = (self.time * 20.0) as usize;
        let darci = self.player().map(|p| p.fig.pos);
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if !(tick + i).is_multiple_of(64) || p.bent & person::BENT_LAZY == 0 || p.player || p.removed || p.is_dead() || p.is_down()
                || p.mv != 1 || p.aggro() || p.scared.is_some() || !matches!(p.state, State::Move)
            {
                continue;
            }
            let at = p.fig.pos;
            let score = |s: &Vec3| (s.x - at.x).abs() + (s.z - at.z).abs() + 2.0 * (s.y - at.y).abs();
            let Some(&(seat, angle)) = self.seats.iter().filter(|(s, _)| score(s) < 0x200 as f32).min_by(|a, b| score(&a.0).total_cmp(&score(&b.0))) else {
                continue;
            };
            let seen = darci.is_some_and(|d| (d - at).length() < 20.0 * 256.0);
            if seen && (seat.x - at.x).abs() + (seat.z - at.z).abs() > 0x50 as f32 {
                continue;
            }
            let p = &mut self.people[i];
            p.fig.pos = seat;
            p.fig.angle = angle;
            p.state = State::Sit;
            p.fig.restart(uc_world::character::anim::SIT_DOWN, uc_world::Play::Once);
        }
    }
}

/// The player's sit on a bench (`ACTION_SIT_BENCH`): health given so far (`Timer1`), the
/// time to the next point, and whether it's been had lately (`GF_DISABLE_BENCH_HEALTH`).
#[derive(Debug, Clone, Default)]
pub(crate) struct Bench {
    given: u32,
    clock: f32,
    spent: bool,
    /// The original's turns (20 a second) counted, for when it can be had again.
    turns: u64,
}

impl Game {
    /// The player sits down on a bench, a sofa or an armchair, and gets some health back.
    /// The original sits Darci when she walks backwards into one (collide.cpp:5640), but
    /// here she always turns to face the way she walks: so it's action stood at one, with
    /// nothing else to do (nobody to talk to, no car to get in). Every fifth of a second sat
    /// there she gets a point of health, 39 at most (Person.cpp:16459); and that only once
    /// in every 1024 of the original's turns (Game.cpp:2682), else she just sits. Pushing
    /// any way gets her up (interfac.cpp:4727).
    pub(crate) fn sit_on_benches(&mut self, playing: &[(usize, Controls, bool)], dt: f32) {
        // `GAME_TURN & 0x3ff == 314` clears the flag.
        let turns = (self.time * 20.0) as u64;
        if (turns + 1024 - 314) / 1024 != (self.bench.turns + 1024 - 314) / 1024 {
            self.bench.spent = false;
        }
        self.bench.turns = turns;
        for &(pi, c, _) in playing {
            let p = &self.people[pi];
            if matches!(p.state, State::Sit) {
                if matches!(p.fig.anim(), uc_world::character::anim::SIT_DOWN | uc_world::character::anim::SIT_IDLE) && p.fig.finished() {
                    self.bench.clock += dt;
                    while self.bench.clock >= 0.2 {
                        self.bench.clock -= 0.2;
                        self.bench.given += 1;
                        if self.bench.given < 40 {
                            let p = &mut self.people[pi];
                            p.health = (p.health + 1).min(p.max_health);
                        }
                    }
                }
                continue;
            }
            // Action with nothing else to do crouches her (to pick something up): that's
            // where the sit takes over.
            let free = matches!(p.state, State::Move) || matches!(p.state, State::Crouch { phase: 0 }) && !c.crouch;
            if !c.action || c.wish.length() > 0.1 || !free || p.is_dead() || self.convo.is_some() {
                continue;
            }
            let at = p.fig.pos;
            let near = self.seats.iter()
                .filter(|(s, _)| Vec2::new(s.x - at.x, s.z - at.z).length() < 0x60 as f32 && (s.y - at.y).abs() < 0x40 as f32)
                .min_by(|a, b| (a.0 - at).length().total_cmp(&(b.0 - at).length()));
            let Some(&(seat, angle)) = near else { continue };
            let p = &mut self.people[pi];
            p.fig.pos = seat;
            p.fig.angle = angle;
            p.state = State::Sit;
            p.fig.restart(uc_world::character::anim::SIT_DOWN, uc_world::Play::Once);
            // `set_person_sit_down`: a fresh sit gives health, one too soon after the last
            // none (`Timer1` 100).
            self.bench.given = if self.bench.spent { 100 } else { 0 };
            self.bench.clock = 0.0;
            self.bench.spent = true;
        }
    }
}

/// Where someone can sit on the level's benches (`PRIM_OBJ_PARK_BENCH`), armchairs and sofas
/// (`_ARMCHAIR`, `_SOFA`): in front of each by 0x20 less than its depth, facing the way it
/// does, at its height (`PCOM_position_person_to_sit_on_prim`, pcom.cpp:1020).
pub(crate) fn seats(level: &Level) -> Vec<(Vec3, f32)> {
    let mut depth: std::collections::HashMap<u8, Option<f32>> = std::collections::HashMap::new();
    level.map.objects.iter()
        .filter(|o| matches!(o.prim, 89 | 101 | 105))
        .filter_map(|o| {
            let min_z = *depth.entry(o.prim).or_insert_with(|| uc_world::level::load_prim(&level.game, o.prim).map(|p| uc_world::level::prim_bounds(&p).0.z));
            let away = min_z? - 0x20 as f32;
            let a = uc_formats::units::radians(o.yaw as f32);
            Some((Vec3::new(o.x as f32 + a.sin() * away, o.y as f32, o.z as f32 + a.cos() * away), a))
        })
        .collect()
}

impl Game {
    /// Those whose move has them pose at home strike it while they've nothing else to do
    /// (pcom.cpp:10245): dancing, one of three dances by who they are (`dance_anim`); hands up
    /// or tied up, hands up.
    pub(crate) fn strike_poses(&mut self) {
        use uc_world::character::anim;
        let darci = self.player().map(|p| p.fig.pos);
        for (i, p) in self.people.iter_mut().enumerate() {
            if p.player || p.removed || p.is_dead() || p.is_down() || p.aggro() || p.scared.is_some() || !matches!(p.state, State::Move) {
                continue;
            }
            // Within 256 of home, standing still: turned the way they started, and put back
            // if they're off it and nobody can see (`PCOM_teleport_home`, pcom.cpp:10230).
            // Their pose waits until they're there (`PCOM_ARRIVE_DIST`, 0x40).
            if let (Some((home, angle)), true) = (p.home, matches!(p.mv, 1 | 8 | 9 | 10) && !p.listening() && !(p.fights() && p.foe.is_some())) {
                let d = Vec2::new(home.x - p.fig.pos.x, home.z - p.fig.pos.z).length();
                if d > 256.0 || p.goal.is_some() {
                    continue;
                }
                p.fig.angle = angle;
                if d > 0x40 as f32 {
                    if darci.is_some_and(|me| (me - p.fig.pos).length() > 20.0 * 256.0) {
                        p.fig.pos.x = home.x;
                        p.fig.pos.z = home.z;
                    } else {
                        continue;
                    }
                }
            }
            let a = match p.mv {
                person::MOVE_DANCE => [anim::DANCE_BOOGIE, anim::DANCE_WOOGIE, anim::DANCE_HEADBANG, anim::DANCE_BOOGIE][i & 3],
                person::MOVE_HANDS_UP | person::MOVE_TIED_UP => anim::HANDS_UP,
                _ => continue,
            };
            // A dance on a loop; hands up once, then held (`ANIM_HANDS_UP_LOOP`).
            let play = if a == anim::HANDS_UP { uc_world::Play::Once } else { uc_world::Play::Loop };
            if p.fig.restart(a, play) {
                p.state = State::Pose { anim: a };
            }
        }
    }

    /// A player stood about for three seconds (`PCOM_get_duration(30)`) with the nearest
    /// person within 0x200 dancing dances along: turned to them, the same dance
    /// (`PCOM_process_person`, pcom.cpp:12866). `playing` is each player with their controls.
    pub(crate) fn join_dance(&mut self, dt: f32, playing: &[(usize, Controls, bool)]) {
        use uc_world::character::anim;
        for &(i, ref c, _) in playing {
            let me = &mut self.people[i];
            if !me.player || !matches!(me.state, State::Move) || c.any() {
                me.idle_for = 0.0;
                continue;
            }
            me.idle_for += dt;
            if me.idle_for <= 3.0 {
                continue;
            }
            me.idle_for = 0.0;
            let at = me.fig.pos;
            let near = self.people.iter().enumerate()
                .filter(|&(j, o)| j != i && !o.removed && (o.fig.pos - at).length() < 0x200 as f32)
                .min_by(|a, b| (a.1.fig.pos - at).length().total_cmp(&(b.1.fig.pos - at).length()));
            let Some((_, o)) = near else { continue };
            let State::Pose { anim: a } = o.state else { continue };
            if !matches!(a, anim::DANCE_BOOGIE | anim::DANCE_WOOGIE | anim::DANCE_HEADBANG) {
                continue;
            }
            let to = o.fig.pos;
            let me = &mut self.people[i];
            me.fig.angle = uc_world::character::angle_of(Vec2::new(to.x - at.x, to.z - at.z));
            if me.fig.restart(a, uc_world::Play::Loop) {
                me.state = State::Pose { anim: a };
            }
        }
    }

    /// The noises the game made this turn reach everyone within earshot but whoever made them
    /// (`PCOM_oscillate_tympanum`): whether they can see where it was is a clear line from
    /// their head. And every 64 ticks, someone after a person who's paying them no attention
    /// says "what you looking at" (pcom.cpp:7536).
    pub(crate) fn hear_noises(&mut self, level: &Level) {
        let tick = (self.time * 20.0) as usize;
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.player || !p.aggro() || !(tick + i).is_multiple_of(64) {
                continue;
            }
            let ignored = p.foe.is_some_and(|t| !self.people[t].player && !self.people[t].aggro() && self.people[t].scared.is_none());
            if ignored {
                self.noises.push((person::Noise::LookingAtMe, p.fig.pos, Some(i)));
            }
        }
        let eye = Vec3::new(0.0, 0x60 as f32, 0.0);
        for (noise, at, by) in std::mem::take(&mut self.noises) {
            for j in 0..self.people.len() {
                if Some(j) == by || (self.people[j].fig.pos - at).length() >= noise.radius() {
                    continue;
                }
                let seen = level.ground.line_of_sight(self.people[j].fig.pos + eye, at + Vec3::new(0.0, 16.0, 0.0));
                self.people[j].hear_noise(noise, at, by, seen);
            }
        }
    }
}
