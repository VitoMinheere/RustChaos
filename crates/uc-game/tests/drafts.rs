//! The drafts and prototypes the game never listed that play to an end (uc-viewer's
//! `DRAFTS`), each run to its win: the fighting is done by knocking people out (and the
//! Balrogs by hitting them hard), the walking by putting the player where they have to be.
//! Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Outcome, Weapon};

mod common;
use common::{Run, DT};

/// `PCOM_AI_CIVILIAN`.
const AI_CIVILIAN: u8 = 1;

impl Run {
    /// Talks to the person made by `way` from `off` of them, facing them, then plays `secs`.
    fn talk_to(&mut self, way: usize, off: Vec3, secs: f32) {
        let at = self.g.people[self.who(way)].fig.pos;
        self.stand(at + off);
        let me = self.player();
        self.g.people[me].fig.angle = uc_world::character::angle_of(vec2(-off.x, -off.z));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }

    /// Stands on point `way` and presses action there.
    fn press(&mut self, way: usize) {
        if !self.stand_at_switch(way) {
            let p = self.point(way);
            self.stand(p);
        }
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(2.0, Controls::default());
    }

    /// Picks up the item point `way` made.
    fn fetch(&mut self, way: usize) {
        let at = self.g.items.iter().find(|i| i.way == way).unwrap_or_else(|| panic!("no item from point {way}")).pos;
        self.stand(at);
        self.play(1.0, Controls::default());
    }

    /// Knocks the person made by `way` out.
    fn knock_out(&mut self, way: usize) {
        let i = self.who(way);
        let at = self.g.people[i].fig.pos;
        self.g.people[i].take_hit(at, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }

    /// Plays up to `secs`, once a second knocking out everyone but civilians and hitting the
    /// creatures too hard to live, until the mission ends.
    fn fight(&mut self, secs: f32) {
        for _ in 0..secs as usize {
            let p = self.player();
            let from = self.g.people[p].fig.pos;
            for e in self.g.people.iter_mut() {
                if !e.player && !e.removed && !e.is_dead() && !e.is_down() && e.ai != AI_CIVILIAN {
                    e.take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
                }
            }
            let mut acts = Vec::new();
            for c in self.g.creatures.iter_mut() {
                c.hit(None, 1_000_000, &mut acts);
            }
            self.play(1.0, Controls::default());
            if self.g.outcome.is_some() {
                return;
            }
        }
    }

    fn won(&self) {
        assert_eq!(self.g.outcome, Some(Outcome::Won), "{:?}", self.seen);
    }
}

/// Combat Platinum (`fight3`): the waves come and go down.
#[test]
fn combat_platinum() {
    let Some(mut r) = common::start("fight3") else { return };
    r.fight(300.0);
    assert!(r.said("GRADE COMPLETED"), "{:?}", r.seen);
    r.won();
}

/// Three Balrogs (`BAALROG1`): with all three dead, it's won; there are guns about to do it.
#[test]
fn three_balrogs() {
    let Some(mut r) = common::start("BAALROG1") else { return };
    r.play(1.0, Controls::default());
    assert_eq!(r.g.creatures.len(), 3, "three Balrogs");
    assert!(r.g.items.iter().any(|i| Weapon::from_item(i.kind).is_some_and(|w| w.is_gun())), "nothing to shoot them with");
    r.fight(60.0);
    r.won();
}

/// Mr Bates (`gang1`): Bates (point 113) down, and Mako back at the den, which has a beacon
/// from then on, in place of Bates's (`uc_world::level::finish_unfinished`).
#[test]
fn mr_bates() {
    let Some(mut r) = common::start("gang1") else { return };
    r.play(2.0, Controls::default());
    assert_eq!(r.labels(), ["Mr Bates."]);
    r.knock_out(113);
    assert!(r.until(5.0, |r| r.said("return to the den")), "{:?}", r.seen);
    assert_eq!(r.labels(), ["The den"]);
    let den = r.point(116);
    r.stand(den);
    assert!(r.until(40.0, |r| r.g.outcome.is_some()));
    r.won();
}

/// Trash the Place (`gang3`): the files off the man in the store and back to the den. It has
/// no ending of its own, nor any beacons (`uc_world::level::finish_unfinished`).
#[test]
fn trash_the_place() {
    let Some(mut r) = common::start("gang3") else { return };
    r.play(2.0, Controls::default());
    let beacons = |r: &Run| r.g.beacons().into_iter().map(|(l, _)| l).collect::<Vec<_>>();
    assert_eq!(beacons(&r), ["The store"]);
    r.talk_to(12, vec3(110.0, 0.0, 0.0), 10.0);
    r.talk_to(111, vec3(110.0, 0.0, 0.0), 15.0);
    r.talk_to(134, vec3(110.0, 0.0, 0.0), 30.0);
    assert!(r.said("here they are"), "{:?}", r.seen);
    r.fetch(136);
    assert_eq!(beacons(&r), ["The den"]);
    assert_eq!(r.g.outcome, None, "won before getting back");
    let den = r.point(137);
    r.stand(den);
    assert!(r.until(10.0, |r| r.g.outcome.is_some()));
    assert!(r.said("Good work"), "{:?}", r.seen);
    r.won();
}

/// Mr X's Getaway (`Getaway1`): the police contact (point 162) sends Darci after Mr X (131),
/// and taking him down wins.
#[test]
fn mr_xs_getaway() {
    let Some(mut r) = common::start("Getaway1") else { return };
    r.play(2.0, Controls::default());
    r.talk_to(162, vec3(110.0, 0.0, 0.0), 15.0);
    assert!(r.said("intercept him before he gets away"), "{:?}", r.seen);
    assert!(r.until(10.0, |r| r.g.people.iter().any(|p| p.way == 131)), "no Mr X");
    r.knock_out(131);
    assert!(r.until(10.0, |r| r.g.outcome.is_some()));
    r.won();
}

/// Transmitter (`Transmit1`): Roper's explosives (point 140, on his roof, not the street
/// below it) on the transmitter, which point 120 watches once it's moved onto it
/// (`uc_world::level::finish_unfinished`).
#[test]
fn transmitter() {
    let Some(mut r) = common::start("Transmit1") else { return };
    r.play(2.0, Controls::default());
    r.talk_to(138, vec3(110.0, 0.0, 0.0), 12.0);
    assert!(r.said("here it is"), "{:?}", r.seen);
    let roper = r.g.people[r.who(138)].fig.pos;
    let item = r.g.items.iter().find(|i| i.way == 140).expect("no explosives").pos;
    assert!((item.y - roper.y).abs() < 8.0, "the explosives are at {item}, Roper at {roper}");
    r.fetch(140);
    let me = r.player();
    assert_eq!(r.g.people[me].explosives, 1);
    let t = r.point(120);
    r.stand(t + vec3(150.0, 0.0, 0.0));
    r.take_out(Weapon::Explosives);
    r.play(DT, Controls { action: true, ..Default::default() });
    assert_eq!(r.g.charges.len(), 1, "no charge set");
    r.stand(t + vec3(0.0, 0.0, 1500.0));
    assert!(r.until(30.0, |r| r.g.outcome.is_some()));
    r.won();
}

/// Farraday's Files (`estate1`): the override in the pool house, the disk from Officer Jed at
/// the gates, the files copied off the main computer, and back to Jed with them.
#[test]
fn farradays_files() {
    let Some(mut r) = common::start("estate1") else { return };
    r.play(2.0, Controls::default());
    r.press(88);
    assert!(r.said("Security systems off"), "{:?}", r.seen);
    let gate = r.point(106);
    r.stand(gate);
    assert!(r.until(30.0, |r| r.g.items.iter().any(|i| i.way == 108)), "no disk: {:?}", r.seen);
    r.fetch(108);
    r.press(110);
    assert!(r.until(30.0, |r| r.said("Files Copied")), "{:?}", r.seen);
    r.fetch(121);
    let jed = r.point(118);
    r.stand(jed);
    assert!(r.until(40.0, |r| r.g.outcome.is_some()));
    r.won();
}

impl Run {
    /// Runs the player to `to` along the shortest route of plain walks (and drops) over the
    /// navigation map, for up to `secs`. Whether she got there.
    fn walk_to(&mut self, to: Vec3, secs: f32) -> bool {
        use uc_world::mav::{self, Mav};
        let m = Mav::new(&self.level);
        let me = self.player();
        let sq = |p: Vec3| ((p.x / 256.0) as i32, (p.z / 256.0) as i32);
        let (from, end) = (sq(self.g.people[me].fig.pos), sq(to));
        let mut came = std::collections::HashMap::from([(from, from)]);
        let mut queue = std::collections::VecDeque::from([from]);
        while let Some((x, z)) = queue.pop_front() {
            for (dir, &(dx, dz)) in mav::DIRS.iter().enumerate() {
                let next = (x + dx, z + dz);
                if m.opt(x, z, dir) & (mav::caps(mav::GOTO) | mav::caps(mav::FALL_OFF)) != 0 && !came.contains_key(&next) {
                    came.insert(next, (x, z));
                    queue.push_back(next);
                }
            }
        }
        if !came.contains_key(&end) {
            return false;
        }
        let mut route = vec![end];
        while route[route.len() - 1] != from {
            route.push(came[&route[route.len() - 1]]);
        }
        route.reverse();
        let mid = |(x, z): (i32, i32)| vec2(x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
        let mut next = 0;
        for _ in 0..(secs / DT) as usize {
            let at = self.g.people[me].fig.pos;
            let goal = if next + 1 < route.len() { mid(route[next]) } else { vec2(to.x, to.z) };
            let d = goal - vec2(at.x, at.z);
            if d.length() < 90.0 {
                if next + 1 >= route.len() {
                    return true;
                }
                next += 1;
                continue;
            }
            self.play(DT, Controls { wish: d.normalize(), run: true, ..Default::default() });
        }
        false
    }
}

/// ...and from where Darci starts she can run to Officer Jed at the gates (point 106). The
/// draft map has her start in a yard she can't get out of (`uc_world::level::finish_unfinished`).
#[test]
fn farradays_files_from_the_start() {
    let Some(mut r) = common::start("estate1") else { return };
    r.play(2.0, Controls::default());
    let gate = r.point(106);
    assert!(r.walk_to(gate, 120.0), "stuck at {}", r.g.people[r.player()].fig.pos);
}

impl Run {
    /// Knocks the person made by `way` down without hurting them, and cuffs them.
    fn arrest(&mut self, way: usize) {
        let i = self.who(way);
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(160.0, 0.0, 0.0));
        self.play(0.5, Controls::default());
        let at = self.g.people[i].fig.pos;
        self.g.people[i].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, Weapon::None);
        for _ in 0..60 {
            let me = self.player();
            if matches!(self.g.people[me].state, uc_game::State::Move) {
                break;
            }
            self.play(DT * 1.01, Controls::default());
        }
        let body = self.g.people[i].body_pos();
        self.stand(body + vec3(60.0, 0.0, 0.0));
        self.play(0.2, Controls { action: true, ..Default::default() });
        self.play(4.0, Controls::default());
    }

    fn labels(&self) -> Vec<String> {
        self.g.beacons().into_iter().map(|(l, _)| l).collect()
    }

    /// 24th Street Shooting up to Harley Blvd: the workman (point 58) leads Darci into an
    /// alley, where he and the brutes he springs on her (66 to 68) go down.
    fn follow_the_trail(&mut self) {
        self.play(2.0, Controls::default());
        assert!(!self.labels().iter().any(|l| l == "The geek"), "the geek's beacon too soon");
        self.talk_to(58, vec3(110.0, 0.0, 0.0), 6.0);
        let (w, alley) = (self.who(58), self.point(64));
        self.g.people[w].fig.pos = alley;
        let cuboid = self.point(71);
        self.stand(cuboid);
        assert!(self.until(15.0, |r| [66, 67, 68].iter().all(|&b| r.g.people.iter().any(|p| p.way == b))), "no brutes");
        for b in [58, 66, 67, 68] {
            self.knock_out(b);
        }
        assert!(self.until(10.0, |r| r.said("Harley Blvd")), "{:?}", self.seen);
        assert!(self.labels().iter().any(|l| l == "The geek"), "{:?}", self.labels());
    }
}

/// 24th Street Shooting (`Crimescene`): arresting the geek at Harley Blvd (point 79) wins.
/// The mission had no ending (`uc_world::level::finish_unfinished`).
#[test]
fn twenty_fourth_street_shooting() {
    let Some(mut r) = common::start("Crimescene") else { return };
    r.follow_the_trail();
    assert_eq!(r.g.outcome, None);
    r.arrest(79);
    assert!(r.until(10.0, |r| r.g.outcome.is_some()));
    assert!(r.said("talk about 24th Street"), "{:?}", r.seen);
    r.won();
}

/// ...and killing him, the only lead, loses.
#[test]
fn twenty_fourth_street_shooting_without_the_geek() {
    let Some(mut r) = common::start("Crimescene") else { return };
    r.follow_the_trail();
    r.knock_out(79);
    assert!(r.until(15.0, |r| r.g.outcome.is_some()));
    assert!(r.said("our only lead"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}

/// ...and the one on the roof (point 44), seen, runs for it over the roofs: across the gap
/// to the next roof (a route's jump and pull-up), not down into the street.
#[test]
fn twenty_fourth_street_shooting_roof_runner_makes_the_jump() {
    let Some(mut r) = common::start("Crimescene") else { return };
    r.play(1.0, Controls::default());
    let roof = r.who(44);
    r.stand(r.point(53));
    // Over the street at z 13824..14368, onto the roof beyond, 1216 up.
    assert!(r.until(30.0, |r| r.g.people[roof].fig.pos.z < 13700.0), "never got across: {:?}", r.g.people[roof].fig.pos);
    let p = &r.g.people[roof];
    assert!(!p.is_dead() && p.fig.pos.y > 1000.0, "fell: {:?}, health {}", p.fig.pos, p.health);
}

/// ...and once the one on the roof (point 44) has told what he saw, or is dead, a beacon
/// points Darci at the workman (58), until he's led her to the alley.
#[test]
fn twenty_fourth_street_shooting_points_to_the_workman() {
    for beaten in [true, false] {
        let Some(mut r) = common::start("Crimescene") else { return };
        r.play(2.0, Controls::default());
        assert!(!r.labels().iter().any(|l| l == "The workman"), "too soon: {:?}", r.labels());
        let roof = r.who(44);
        if beaten {
            // Half his health gone, and he talks (point 36).
            let at = r.g.people[roof].fig.pos;
            let half = r.g.people[roof].health / 2 + 1;
            r.g.people[roof].take_hit(at + vec3(50.0, 0.0, 0.0), half, 0, false, false, Weapon::None);
            r.stand(at + vec3(150.0, 0.0, 0.0));
        } else {
            r.knock_out(44);
        }
        assert!(r.until(30.0, |r| r.labels().iter().any(|l| l == "The workman")), "beaten {beaten}: {:?}", r.labels());
        r.talk_to(58, vec3(110.0, 0.0, 0.0), 6.0);
        let (w, alley) = (r.who(58), r.point(64));
        r.g.people[w].fig.pos = alley;
        assert!(r.until(5.0, |r| !r.labels().iter().any(|l| l == "The workman")), "{:?}", r.labels());
    }
}

/// ...and at Harley Blvd as it's played: Darci runs from the beacon to the geek (point 79),
/// at the gate of a yard his two friends (80 and 81) hang about in. He talks to her (86),
/// one of them calls to him from behind the gate (87), and she's told to bring him in. Set
/// on, he fights back and his friends, of his gang, come over the gate after her; with them
/// down, arresting him wins.
#[test]
fn twenty_fourth_street_shooting_at_harley_blvd() {
    let Some(mut r) = common::start("Crimescene") else { return };
    r.follow_the_trail();
    let blvd = r.point(77);
    r.stand(blvd);
    let g = r.who(79);
    let geek = r.g.people[g].fig.pos;
    assert!(r.walk_to(geek + vec3(0.0, 0.0, 150.0), 30.0), "stuck at {}", r.g.people[r.player()].fig.pos);
    r.talk_to(79, vec3(0.0, 0.0, 150.0), 1.0);
    assert!(r.until(30.0, |r| r.said("bring him in")), "{:?}", r.seen);
    assert!(r.said("That bitch sweatin' you?"), "{:?}", r.seen);
    // The yard: west of the gate (x 15360), north of its wall (z 1792).
    let in_yard = |r: &Run, way: usize| {
        let p = r.g.people[r.who(way)].fig.pos;
        p.x < 15360.0 && p.z < 1792.0
    };
    assert!(in_yard(&r, 80) && in_yard(&r, 81), "the friends aren't behind the gate");
    assert_eq!(r.g.outcome, None);
    // Punches until he fights back, then back to the street.
    r.talk_to(79, vec3(0.0, 0.0, 120.0), DT);
    for k in 0..90 {
        if r.g.people[g].hostile() {
            break;
        }
        r.play(0.1, Controls { punch: k % 2 == 0, ..Default::default() });
    }
    assert!(r.g.people[g].hostile(), "he never fought back");
    let street = r.point(90);
    r.stand(street);
    assert!(r.until(30.0, |r| !in_yard(r, 80) && !in_yard(r, 81)), "still behind the gate: {} {}",
            r.g.people[r.who(80)].fig.pos, r.g.people[r.who(81)].fig.pos);
    for friend in [80, 81] {
        r.knock_out(friend);
    }
    r.arrest(79);
    assert!(r.until(10.0, |r| r.g.outcome.is_some()));
    assert!(r.said("talk about 24th Street"), "{:?}", r.seen);
    r.won();
}

/// Station Bombed (`police5`): the five Wildcats arrested wins, each with a beacon until
/// he's in. The mission had no ending (`uc_world::level::finish_unfinished`).
#[test]
fn station_bombed() {
    let Some(mut r) = common::start("police5") else { return };
    r.play(2.0, Controls::default());
    assert_eq!(r.labels(), ["Wildcat"; 5]);
    for (k, way) in [93, 94, 137, 138, 139].into_iter().enumerate() {
        assert_eq!(r.g.outcome, None, "won with {k} in");
        r.arrest(way);
        assert_eq!(r.labels().len(), 4 - k, "beacons after {k}");
    }
    assert!(r.until(10.0, |r| r.g.outcome.is_some()));
    assert!(r.said("That's the last of 'em"), "{:?}", r.seen);
    r.won();
}

/// ...and killing one of them loses.
#[test]
fn station_bombed_with_a_dead_wildcat() {
    let Some(mut r) = common::start("police5") else { return };
    r.play(2.0, Controls::default());
    r.knock_out(137);
    assert!(r.until(15.0, |r| r.g.outcome.is_some()));
    assert!(r.said("We needed those Wildcats alive"), "{:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}

/// The Final Reckoning (`Finale1b`), on Day Of Reckoning's map as its own is missing
/// (`uc_world::level::stand_in_map`): the three survivors brought to safety; Darci over the
/// drop to the pumps' roof on Day Of Reckoning's platforms, which the gate's switch (point
/// 125) sets going (`uc_world::level::finish_unfinished`); both fuel pumps on, the
/// explosives on the prop point 62 watches, and into the safe zone, on that roof.
#[test]
fn the_final_reckoning() {
    let Some(mut r) = common::start("Finale1b") else { return };
    assert!(r.level.mission.map.ends_with("Rooftest2.iam"), "{}", r.level.mission.map);
    // Bane's men are taken off: the fighting isn't what's tested.
    r.each_turn = Some(|g| {
        for p in g.people.iter_mut().filter(|p| p.hostile()) {
            p.removed = true;
        }
    });
    r.play(25.0, Controls::default());
    for (survivor, safe) in [(31, 30), (32, 67), (33, 66)] {
        let (i, at) = (r.who(survivor), r.point(safe));
        r.g.people[i].fig.pos = at;
    }
    assert!(r.until(20.0, |r| r.said("I got them all in safely")), "{:?}", r.seen);
    assert!(r.until(10.0, |r| r.g.items.iter().any(|i| i.way == 77)), "no explosives");
    r.fetch(77);
    r.press(125);
    assert!(r.said("Gate open"), "{:?}", r.seen);
    // Onto the first platform, at the edge of her roof; off it onto the second as it comes
    // by at the far end, and off that onto the walkway it ends by.
    // Told apart by their heights: one runs along x at 448, the other along z at 608.
    let platform = |r: &Run, y: f32| r.g.platforms.iter().position(|p| (p.pos.y - y).abs() < 8.0).unwrap_or_else(|| panic!("no platform at {y}"));
    let (a, b) = (platform(&r, 448.0), platform(&r, 608.0));
    assert!(r.until(20.0, |r| (r.g.platforms[a].pos.x - 13824.0).abs() < 8.0), "the first platform never came back");
    let at = r.g.platforms[a].pos;
    r.stand(at + vec3(0.0, 100.0, 0.0));
    let me = r.player();
    let met = |r: &Run| (r.g.platforms[a].pos.x - 16128.0).abs() < 8.0 && (r.g.platforms[b].pos.z - 13056.0).abs() < 60.0;
    for _ in 0..(30.0 / DT) as usize {
        if met(&r) {
            break;
        }
        r.play(DT, Controls::default());
    }
    assert!(met(&r), "the platforms never met: {} {}", r.g.platforms[a].pos, r.g.platforms[b].pos);
    r.g.people[me].fig.angle = uc_world::character::angle_of(vec2(0.0, -1.0));
    for f in 0..45 {
        r.play(DT, Controls { wish: vec2(0.0, -1.0), run: true, jump: f == 3, ..Default::default() });
    }
    assert!(r.until(20.0, |r| (r.g.platforms[b].pos.z - 10496.0).abs() < 8.0), "the second platform never got there");
    r.play(2.0, Controls { wish: vec2(1.0, 0.0), ..Default::default() });
    let p = r.g.people[me].fig.pos;
    assert!(p.x > 16384.0 && p.y > 600.0 && !r.g.people[me].is_dead(), "not over the drop: {p}");
    r.press(48);
    r.press(50);
    assert!(r.said("Fuel Pump 1 ACTIVE") && r.said("Fuel Pump 2 ACTIVE"), "{:?}", r.seen);
    let t = r.point(62);
    r.stand(t + vec3(150.0, 0.0, 0.0));
    r.take_out(Weapon::Explosives);
    r.play(DT, Controls { action: true, ..Default::default() });
    r.stand(t + vec3(0.0, 0.0, 1500.0));
    assert!(r.until(10.0, |r| r.g.point_active(63)), "the pumps never went up");
    // The safe zone is on that roof, and its beacon there.
    let safe = r.point(78);
    r.stand(safe);
    let p = r.g.people[me].fig.pos;
    assert!(p.y > 400.0, "the safe zone's over the drop: {p}");
    let beacon = r.g.beacons().into_iter().find(|b| b.0 == "Safe Zone").expect("no beacon to the safe zone").1;
    assert!(vec2(beacon.x - p.x, beacon.z - p.z).length() < 256.0, "the beacon's at {beacon}, the safe zone {p}");
    assert!(r.until(15.0, |r| r.g.outcome.is_some()));
    r.won();
}

/// Officer Training past the jumps (point 107): a beacon to the loading lift's switch (81),
/// then to the finish (82), which the lift takes her up to (`finish_unfinished`).
#[test]
fn officer_training_beacons_the_way_on() {
    let Some(mut r) = common::start("train11") else { return };
    r.play(2.0, Controls::default());
    r.talk_to(17, vec3(110.0, 0.0, 0.0), 10.0);
    assert!(r.said("Follow me"), "{:?}", r.seen);
    assert!(r.labels().is_empty(), "{:?}", r.labels());
    let jumps = r.point(107);
    r.stand(jumps);
    r.play(1.0, Controls::default());
    assert_eq!(r.labels(), ["Lift"]);
    r.press(81);
    assert!(r.said("Loading lift activated"), "{:?}", r.seen);
    assert_eq!(r.labels(), ["Finish"]);
    // On at the bottom, off at the top on the finish's side.
    let lift = |r: &Run| r.g.platforms.iter().find(|p| p.way == 70).unwrap().pos;
    assert!(r.until(30.0, |r| lift(r).y > 900.0) && r.until(30.0, |r| lift(r).y < 20.0));
    let on = lift(&r);
    r.stand(on + vec3(0.0, 64.0, 0.0));
    assert!(r.until(20.0, |r| lift(r).y > 1000.0));
    r.play(8.0, Controls { wish: vec2(0.0, -1.0), run: true, ..Default::default() });
    let me = r.player();
    let finish = r.point(82);
    for _ in 0..(20.0 / DT) as usize {
        let p = r.g.people[me].fig.pos;
        r.play(DT, Controls { wish: vec2(finish.x - p.x, finish.z - p.z).normalize_or_zero(), run: true, ..Default::default() });
        if r.g.outcome.is_some() {
            break;
        }
    }
    assert!(r.until(40.0, |r| r.g.outcome.is_some()));
    assert!(r.said("completed the course successfully"), "{:?}", r.seen);
    r.won();
}

/// Officer Training (`train11`, in `UNRELEASED`): the end of the assault course wins.
#[test]
fn officer_training() {
    let Some(mut r) = common::start("train11") else { return };
    r.play(2.0, Controls::default());
    let end = r.point(82);
    r.stand(end);
    assert!(r.until(40.0, |r| r.g.outcome.is_some()));
    assert!(r.said("completed the course successfully"), "{:?}", r.seen);
    r.won();
}

impl Run {
    /// Mr Mcintyre (`gang2`) up to the kidnap: the boss at the den (point 10), Bish (110) and
    /// Mclean (113) fetched, each beacon going as they join, both following Mako at once,
    /// Blood (120) met, and Mcintyre (122) talked to.
    fn up_to_the_kidnap(&mut self) {
        self.play(2.0, Controls::default());
        let mut labels = self.labels();
        labels.sort();
        assert_eq!(labels, ["Bish", "Mclean"]);
        self.talk_to(10, vec3(110.0, 0.0, 0.0), 10.0);
        assert!(self.said("Go fetch hoods Bish and Mclean"), "{:?}", self.seen);
        self.talk_to(110, vec3(110.0, 0.0, 0.0), 10.0);
        assert!(self.until(5.0, |r| r.labels() == ["Mclean"]), "{:?}", self.labels());
        self.talk_to(113, vec3(110.0, 0.0, 0.0), 10.0);
        assert!(self.until(10.0, |r| r.said("meet Blood")), "{:?}", self.seen);
        assert_eq!(self.labels(), ["Blood"]);
        // Both after Mako at once: brought to him, they keep up as he runs off.
        let (bish, mclean, me) = (self.who(110), self.who(113), self.player());
        let at = self.g.people[me].fig.pos;
        self.g.people[bish].fig.pos = at + vec3(200.0, 0.0, 0.0);
        self.g.people[mclean].fig.pos = at + vec3(-200.0, 0.0, 0.0);
        self.play(1.0, Controls::default());
        let blood = self.point(119);
        assert!(self.walk_to(blood, 60.0), "stuck at {}", self.g.people[me].fig.pos);
        let now = self.g.people[me].fig.pos;
        assert!((now - at).length() > 2000.0, "Mako hardly moved: {at} to {now}");
        self.play(2.0, Controls::default());
        for (who, i) in [("Bish", bish), ("Mclean", mclean)] {
            let p = &self.g.people[i];
            assert!(!p.is_dead() && (p.fig.pos - now).length() < 800.0, "{who} left behind at {}, Mako at {now}", p.fig.pos);
        }
        assert!(self.until(10.0, |r| r.g.people.iter().any(|p| p.way == 120)), "no Blood");
        self.talk_to(120, vec3(110.0, 0.0, 0.0), 40.0);
        assert!(self.said("kidnap a Mr Frazer Mcintyre"), "{:?}", self.seen);
        assert!(self.until(5.0, |r| r.labels() == ["Kidnap"]), "{:?}", self.labels());
        assert!(self.until(10.0, |r| r.g.people.iter().any(|p| p.way == 122)), "no Mcintyre");
        self.talk_to(122, vec3(110.0, 0.0, 0.0), 20.0);
        assert!(self.said("You'll have to fight me first"), "{:?}", self.seen);
        assert_eq!(self.g.outcome, None);
    }
}

/// Mr Mcintyre (`gang2`): beaten half dead, Mcintyre gives in and follows Mako, and getting
/// him back to the den alive wins. The mission had no ending (`uc_world::level::finish_unfinished`).
#[test]
fn mr_mcintyre() {
    let Some(mut r) = common::start("gang2") else { return };
    r.up_to_the_kidnap();
    let m = r.who(122);
    let at = r.g.people[m].fig.pos;
    let half = r.g.people[m].health / 2 + 1;
    r.g.people[m].take_hit(at + vec3(50.0, 0.0, 0.0), half, 0, false, false, Weapon::None);
    assert!(r.until(10.0, |r| r.said("I SURRENDER")), "{:?}", r.seen);
    assert!(r.until(5.0, |r| r.labels() == ["The den"]), "{:?}", r.labels());
    // Another blow once he's given in doesn't send him running.
    let at = r.g.people[m].fig.pos;
    r.g.people[m].take_hit(at + vec3(50.0, 0.0, 0.0), 1, 0, false, false, Weapon::None);
    r.play(5.0, Controls::default());
    // And he comes along all the way back to the den.
    let den = r.point(127);
    let me = r.player();
    // (Won on the way in, the walk's cut short.)
    assert!(r.walk_to(den, 150.0) || r.g.outcome.is_some(), "stuck at {}", r.g.people[me].fig.pos);
    assert!(r.until(20.0, |r| r.g.outcome.is_some()), "{:?} at {}, Mako at {}", r.seen, r.g.people[m].fig.pos, r.g.people[me].fig.pos);
    assert!(!r.g.people[m].is_dead());
    assert!(r.said("You got him!"), "{:?}", r.seen);
    r.won();
}

/// ...and killing him wins too, back at the den: "Well done...that guy was just too dumb."
#[test]
fn mr_mcintyre_killed() {
    let Some(mut r) = common::start("gang2") else { return };
    r.up_to_the_kidnap();
    r.knock_out(122);
    assert!(r.until(10.0, |r| r.said("you killed the guy")), "{:?}", r.seen);
    assert!(!r.said("I SURRENDER"), "{:?}", r.seen);
    assert!(r.until(5.0, |r| r.labels() == ["The den"]), "{:?}", r.labels());
    assert_eq!(r.g.outcome, None, "won before getting back");
    let den = r.point(127);
    r.stand(den);
    assert!(r.until(10.0, |r| r.g.outcome.is_some()));
    assert!(r.said("too dumb"), "{:?}", r.seen);
    r.won();
}
