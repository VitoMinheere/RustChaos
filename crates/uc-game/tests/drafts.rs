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
        let p = self.point(way);
        self.stand(p);
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

/// Mr Bates (`gang1`): Bates (point 113) down, and Mako back at the den.
#[test]
fn mr_bates() {
    let Some(mut r) = common::start("gang1") else { return };
    r.play(2.0, Controls::default());
    r.knock_out(113);
    assert!(r.until(5.0, |r| r.said("return to the den")), "{:?}", r.seen);
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
/// (`uc_world::level::stand_in_map`): the three survivors brought to safety, both fuel pumps
/// on, the explosives on the prop point 62 watches, and into the safe zone.
#[test]
fn the_final_reckoning() {
    let Some(mut r) = common::start("Finale1b") else { return };
    assert!(r.level.mission.map.ends_with("Rooftest2.iam"), "{}", r.level.mission.map);
    r.play(25.0, Controls::default());
    for (survivor, safe) in [(31, 30), (32, 67), (33, 66)] {
        let (i, at) = (r.who(survivor), r.point(safe));
        r.g.people[i].fig.pos = at;
    }
    assert!(r.until(20.0, |r| r.said("I got them all in safely")), "{:?}", r.seen);
    assert!(r.until(10.0, |r| r.g.items.iter().any(|i| i.way == 77)), "no explosives");
    r.fetch(77);
    r.press(48);
    r.press(50);
    assert!(r.said("Fuel Pump 1 ACTIVE") && r.said("Fuel Pump 2 ACTIVE"), "{:?}", r.seen);
    let t = r.point(62);
    r.stand(t + vec3(150.0, 0.0, 0.0));
    r.take_out(Weapon::Explosives);
    r.play(DT, Controls { action: true, ..Default::default() });
    r.stand(t + vec3(0.0, 0.0, 1500.0));
    assert!(r.until(10.0, |r| r.g.point_active(63)), "the pumps never went up");
    let safe = r.point(78);
    r.stand(safe);
    assert!(r.until(15.0, |r| r.g.outcome.is_some()));
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
