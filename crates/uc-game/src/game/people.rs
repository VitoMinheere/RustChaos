//! Making people, and walking them to where their script wants them.

use crate::*;

/// The missions where the sniper rifle lies in front of the player's start: Officer Training,
/// RTA, The Jump, Seek Sneak and Seize, Assassin and The Fallen.
pub(crate) const SNIPER_AT_START: [&str; 6] = ["train11", "police1", "police2", "Stealtst1", "Gangorder1", "MIB"];

/// What a person on the AI's legs can do: walk, climb a fence (which the walker does by
/// itself on meeting one), and drop off an edge (`MAV_ACTION_FALL_OFF`: the original's people
/// may do all Darci can, `MAV_CAPS_DARCI`, pcom.cpp:1957; jumping, pulling up and ladders
/// aren't done here yet).
const WALKER: u8 = uc_world::mav::caps(uc_world::mav::GOTO) | uc_world::mav::caps(uc_world::mav::CLIMB_OVER)
    | uc_world::mav::caps(uc_world::mav::FALL_OFF);

/// How long a route is kept while its walker stays in the same square, though where they're
/// going moves (the original plans again only on reaching the route's next square).
const REPLAN: f32 = 1.0;

/// A person's last route: (from square, to square), the furthest square to walk to and the
/// first square along it, or the square over an edge to drop into; and when it was made.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Route {
    from: (u8, u8),
    to: (u8, u8),
    dest: Option<(u8, u8)>,
    first: Option<(u8, u8)>,
    drop: Option<(u8, u8)>,
    at: f32,
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
        // Points can sit a little under the surface they're on (The Jump's jumper is 64 below
        // his ledge): look from half a storey up.
        let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 128.0);
        let fig = people.figure(kind, variant, Vec3::new(pos.x, y, pos.z), angle)?;
        self.people.push(Person::new(fig, kind, player, way));
        let i = self.people.len() - 1;
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
            let p = &self.people[i];
            if p.player || p.removed || p.is_dead() || p.is_down() {
                self.people[i].via = None;
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
            let mut r = || (rand(&mut self.mav_rng) & 0x7ff) as f32 - 0x400 as f32;
            let mut gx = at.x + dx * 8192.0 / dist + r();
            let mut gz = at.z + dz * 8192.0 / dist + r();
            // Stay on the map, bouncing off its edges.
            (gx, gz) = (gx.abs(), gz.abs());
            if gx > size { gx = 2.0 * size - gx; }
            if gz > size { gz = 2.0 * size - gz; }
            let goal = Vec3::new(gx, at.y, gz);
            self.ways.remove(&i);
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
    /// old. A route that starts with a drop off an edge heads straight for the square over it.
    fn leg(&mut self, i: usize, from: Vec3, to: Vec3, level: &Level) -> Option<Vec3> {
        let sq = |v: Vec3| ((v.x / 256.0).clamp(0.0, 127.0) as u8, (v.z / 256.0).clamp(0.0, 127.0) as u8);
        let (a, b) = (sq(from), sq(to));
        if a == b || self.mav.can_walk(a, b) {
            return None;
        }
        let route = match self.ways.get(&i) {
            Some(&r) if r.from == a && (r.to == b || self.time - r.at < REPLAN) => r,
            _ => {
                let mut rng = self.mav_rng;
                let (act, _) = self.mav.route(a, b, WALKER, &mut || rand(&mut rng));
                self.mav_rng = rng;
                // Over a fence: straight at it, and the walker climbs it.
                let walk = act.action == uc_world::mav::GOTO && act.dest != a;
                let (dest, first) = if walk { (Some(act.dest), Some(act.first)) } else { (None, None) };
                let drop = (act.action == uc_world::mav::FALL_OFF).then(|| {
                    let (dx, dz) = uc_world::mav::DIRS[act.dir as usize & 3];
                    ((a.0 as i32 + dx).clamp(0, 127) as u8, (a.1 as i32 + dz).clamp(0, 127) as u8)
                });
                let r = Route { from: a, to: b, dest, first, drop, at: self.time };
                self.ways.insert(i, r);
                r
            }
        };
        let (dest, first) = (route.dest, route.first);
        let middle = |s: (u8, u8)| Vec3::new(s.0 as f32 * 256.0 + 128.0, from.y, s.1 as f32 * 256.0 + 128.0);
        if let Some(over) = route.drop {
            return Some(middle(over));
        }
        let dest = middle(dest?);
        // A body's width: the line from each side of them as well as the middle.
        let waist = Vec3::new(0.0, uc_world::ground::STEP_HEIGHT - 4.0, 0.0);
        let along = (dest - from).normalize_or_zero();
        let side = Vec3::new(-along.z, 0.0, along.x) * 40.0;
        let clear = [Vec3::ZERO, side, -side].iter().all(|&o| level.ground.line_of_sight(from + waist + o, dest + waist + o));
        Some(if clear { dest } else { middle(first.unwrap_or(sq(dest))) })
    }

    /// Where each person is walking to when they aren't fighting: `PCOM_process_patrol`
    /// (move 2 or 3) heads for the nearest active point of their colour and group, or once
    /// there (within a square), the next one by number; `PCOM_MOVE_FOLLOW` (move 5) keeps up
    /// with the person their `follow` point created, the player by default.
    pub(crate) fn walk_goals(&mut self) {
        let Some(script) = &self.script else { return };
        // Where everyone stands this frame, so they all aim at the same thing.
        let at: Vec<Vec3> = self.people.iter().map(|p| p.fig.pos).collect();
        let mut goals: Vec<(Option<Vec3>, person::Pace)> = vec![(None, person::Pace::Walk); self.people.len()];
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
                // who is sprinting. They stop a couple of squares short of treading on them.
                5 => {
                    let j = if p.follow == 0 { self.player } else { self.by_way.get(&p.follow).copied() };
                    let to = j
                        .filter(|&j| !self.people[j].removed && !self.people[j].is_dead())
                        .map(|j| at[j])
                        .filter(|&to| (to - at[i]).length() > 2.0 * 256.0);
                    let hurrying = j == self.player && self.sprinting;
                    (to, if hurrying { person::Pace::Sprint } else { person::Pace::Run })
                }
                // `PCOM_process_patrol`: a walk, unless they're `PCOM_BENT_DILIGENT`.
                2 | 3 => {
                    let points = script.patrol_points(p.colour, p.group);
                    let nearest = (0..points.len()).min_by(|&a, &b| near(points[a].1).total_cmp(&near(points[b].1)));
                    let to = nearest.map(|k| {
                        let k = if near(points[k].1) < 256.0 { (k + 1) % points.len() } else { k };
                        points[k].1
                    });
                    (to, if p.bent & 2 != 0 { person::Pace::Run } else { person::Pace::Walk })
                }
                _ => (None, person::Pace::Walk),
            };
        }
        for (p, (goal, pace)) in self.people.iter_mut().zip(goals) {
            (p.goal, p.pace) = (goal, pace);
        }
    }
}
