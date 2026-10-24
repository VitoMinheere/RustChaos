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
    pub(crate) ways: HashMap<usize, Route>,
}

impl Routes {
    pub(crate) fn new(level: &Level) -> Routes {
        Routes { mav: uc_world::mav::Mav::new(level), ways: HashMap::new() }
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
                    self.items.push(Item { way: 0, kind: crate::SNIPER, pos: Vec3::new(at.x, y, at.z) });
                }
            }
        }
    }

    /// Round what's in the way (`PCOM_set_person_move_mav_to_xz` and `MAV_do`): anyone whose
    /// goal, or the person they're fighting, can't be walked to in a straight line heads
    /// for the furthest square of the navigation map's route that can be. On foot and over
    /// fences ([`WALKER`]).
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
        if a == b || self.routes.mav.can_walk(a, b) {
            return None;
        }
        let route = match self.routes.ways.get(&i) {
            Some(&r) if r.from == a && (r.to == b || self.time - r.at < REPLAN) && self.time - r.at < RENAV => r,
            _ => {
                let mut rng = self.rngs.mav;
                let caps = caps_of(&self.people[i], &self.streets.wand);
                let (act, _) = self.routes.mav.route(a, b, caps, &mut || rand(&mut rng));
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
    /// there (within a square), the next one by number, and so on (it keeps to the one it's
    /// heading for, not the nearest, which would turn it back once it's left a point); `PCOM_MOVE_FOLLOW` (move 5) keeps up
    /// with the person their `follow` point created, the player by default.
    pub(crate) fn walk_goals(&mut self) {
        let Some(script) = &self.script else { return };
        // Where everyone stands this frame, so they all aim at the same thing.
        let at: Vec<Vec3> = self.people.iter().map(|p| p.fig.pos).collect();
        let mut goals: Vec<(Option<Vec3>, person::Pace)> = vec![(None, person::Pace::Walk); self.people.len()];
        let mut patrols: Vec<Option<usize>> = self.people.iter().map(|p| p.patrol).collect();
        for (i, p) in self.people.iter().enumerate() {
            if p.player || p.aggro() || p.is_dead() {
                continue;
            }
            // QDIST3: the largest axis plus a quarter of each of the other two.
            let near = |q: Vec3| {
                let d = (q - at[i]).abs();
                d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0
            };
            // A driver out of a car walks to the door of the nearest one free.
            if p.drives() {
                if let (true, Some(vi)) = (matches!(p.state, State::Move), self.car_for(i)) {
                    let door = self.vehicles[vi].door();
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
                    let going = p.goal.is_some();
                    let to = j
                        .filter(|&j| !self.people[j].removed && !self.people[j].is_dead())
                        .filter(|&j| {
                            let d = (at[j] - at[i]).length();
                            let running = self.people[j].fig.anim() == uc_world::character::anim::RUN;
                            if going { d >= 0x60 as f32 } else { d > if running { 0x40 } else { 0xa0 } as f32 }
                        })
                        .map(|j| at[j]);
                    let hurrying = j == self.player && self.sprinting;
                    (to, if hurrying { person::Pace::Sprint } else { person::Pace::Run })
                }
                // `PCOM_process_patrol`: a walk, unless they're `PCOM_BENT_DILIGENT`.
                2 | 3 => {
                    let points = script.patrol_points(p.colour, p.group);
                    let heading = p.patrol.and_then(|w| points.iter().position(|q| q.0 == w));
                    let nearest = (0..points.len()).min_by(|&a, &b| near(points[a].1).total_cmp(&near(points[b].1)));
                    let to = heading.or(nearest).map(|k| {
                        let k = if near(points[k].1) < 256.0 { (k + 1) % points.len() } else { k };
                        patrols[i] = Some(points[k].0);
                        points[k].1
                    });
                    (to, if p.bent & 2 != 0 { person::Pace::Run } else { person::Pace::Walk })
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
            if p.bent & person::BENT_GANG == 0 || !p.aggro() || fighting.get(i).copied().unwrap_or(false) {
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
}
