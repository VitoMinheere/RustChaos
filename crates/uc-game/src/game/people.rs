//! Making people, and walking them to where their script wants them.

use crate::*;

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
            }
        }
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
