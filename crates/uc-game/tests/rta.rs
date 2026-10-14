//! RTA (`police1`), the first street mission: runs its script to a win, with the legwork done
//! by cheating (Darci and the car are moved to where they need to be, the mugger is knocked
//! down for her). Checks the mission logic, not the driving or fighting. Skipped without an
//! install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, State, Weapon, BADGE};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

impl Run {
    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    /// Plays `secs` seconds with `c`, printing new messages.
    fn run(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT) as usize {
            // Darci stands about while the muggers are out: she shrugs off any blows.
            let pi = self.player();
            self.g.people[pi].health = self.g.people[pi].max_health;
            self.g.skip_cutscene();
            self.g.update(DT, &c, &self.level);
            for m in &self.g.messages {
                if !self.seen.contains(&m.text) {
                    println!("[{:6.1}s] {}", self.g.time, m.text);
                    self.seen.push(m.text.clone());
                }
            }
            if self.g.outcome.is_some() {
                return;
            }
        }
    }

    fn player(&mut self) -> usize {
        self.g.player.unwrap()
    }

    /// Puts Darci (on foot) at `p`.
    fn put(&mut self, p: Vec3) {
        let pi = self.player();
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        self.g.people[pi].fig.pos = vec3(p.x, y, p.z);
    }

    fn active(&self, i: usize) -> bool {
        self.g.point_active(i)
    }
}

#[test]
fn rta_script_runs_to_a_win() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    println!("unsupported: {:?}", g.unsupported());
    let mut r = Run { level, g, seen: Vec::new() };
    let wanderer = r.g.people.iter().position(|p| p.way == 24).expect("the civilian at point 24");
    let start = r.g.people[wanderer].fig.pos;
    r.run(5.0, Controls::default());

    // Badges lie about the city (on rooftops, mostly): walking into one raises a stat.
    let badges: Vec<Vec3> = r.g.items.iter().filter(|it| (BADGE..BADGE + 4).contains(&it.kind)).map(|it| it.pos).collect();
    println!("{} badges", badges.len());
    assert!(badges.len() >= 15);
    r.put(badges[0]);
    r.run(0.5, Controls::default());
    assert_eq!(r.g.badges.iter().sum::<u8>(), 1, "the badge wasn't collected");
    // RTA's goals beyond winning: 19 badges, 4 side objectives (the two extra muggers, the
    // man caught peeing, the tramp); the badge picked up counts towards them.
    let goals = uc_game::goals::goals(&r.level.mission);
    for side in &goals.sides {
        println!("side objective {}: {}", side.id, side.label);
    }
    assert_eq!((goals.badges.len(), goals.sides.len()), (19, 4));
    assert!(r.g.found.len() == 1 && goals.badges.contains(r.g.found.iter().next().unwrap()));
    assert!(r.g.sides_done(&goals).is_empty());
    assert!(r.seen.iter().any(|m| m.ends_with("increased")));

    // Civilians who wander (move 4) stroll about.
    r.run(15.0, Controls::default());
    let moved = (r.g.people[wanderer].fig.pos - start).length();
    println!("the wanderer moved {moved:.0}");
    assert!(moved > 256.0, "the civilian stood still");

    // The car crash: the cop there talks, then the beacon moves to the car's drop-off.
    let crash = r.point(150);
    r.put(crash);
    r.run(30.0, Controls::default());
    assert!(r.active(25), "no conversation at the crash");
    assert!(r.active(59), "no 'Vehicle Drop' beacon");

    // Into the abandoned car (point 71), and to the station's drop-off (point 90).
    let car = r.g.vehicles.iter().position(|v| v.way == 71).expect("the RTA car");
    let pi = r.player();
    r.g.vehicles[car].driver = Some(pi);
    r.g.people[pi].state = State::InCar { vehicle: car };
    let drop = r.point(90);
    r.g.vehicles[car].pos = vec3(drop.x, r.g.vehicles[car].pos.y, drop.z);
    r.run(2.0, Controls::default());
    assert!(r.active(90) && r.active(45), "the car wasn't delivered");

    // Out of the car. The homicide call comes in; the witness at the projects (point 78)
    // talks when Darci gets there.
    r.g.vehicles[car].driver = None;
    r.g.people[pi].state = State::Move;
    r.put(drop + vec3(400.0, 0.0, 0.0));
    r.run(10.0, Controls::default());
    assert!(r.active(38), "no homicide call");
    let witness = r.point(78);
    r.put(witness);
    r.run(40.0, Controls::default());
    assert!(r.active(81), "the witness never finished talking");

    // The mugger (point 103) turns up on Kleetus Avenue: knocked down and arrested.
    r.run(2.0, Controls::default());
    let mugger = r.g.people.iter().position(|p| p.way == 103).expect("the mugger");
    let at = r.g.people[mugger].fig.pos;
    r.g.people[mugger].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, Weapon::None);
    // Next to him, on the same roof (he's up high, and may have wandered to the edge).
    let body = r.g.people[mugger].body_pos();
    let beside = (0..8)
        .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
        .find(|p| (r.level.ground.surface(&r.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
        .expect("somewhere to stand");
    r.put(beside);
    r.run(0.2, Controls { action: true, ..Default::default() });
    r.run(6.0, Controls::default());
    let m = &r.g.people[mugger];
    assert!(m.arrested || matches!(m.state, State::Cuffed), "the mugger wasn't arrested: {:?}", m.state);
    assert!(!r.seen.iter().any(|m| m.contains("terminated")), "the arrest counted as a kill");
    r.run(10.0, Controls::default());
    println!("outcome {:?} at {:.0}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

#[test]
fn crash_cops_go_after_the_drunk_not_darci() {
    // At the car crash two cops (points 84, 85) are made assassins of the drunk driver (86):
    // they shoot him, not Darci, and go back to being cops once he's down.
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    let mut r = Run { level, g, seen: Vec::new() };
    // Nothing happens while Darci is still at the station...
    for _ in 0..(10.0 / DT) as usize {
        r.g.skip_cutscene();
        r.g.update(DT, &Controls::default(), &r.level);
    }
    assert!(!r.g.people.iter().find(|p| p.way == 86).unwrap().is_dead(), "the drunk was shot before Darci got there");
    // ...but once she's at the crash, the cops deal with him.
    let crash = r.point(150);
    r.put(crash);
    let pi = r.player();
    let mut lowest = i32::MAX;
    for _ in 0..(15.0 / DT) as usize {
        r.g.skip_cutscene();
        r.g.update(DT, &Controls::default(), &r.level);
        lowest = lowest.min(r.g.people[pi].health);
    }
    let drunk = r.g.people.iter().find(|p| p.way == 86).unwrap();
    let cops: Vec<u8> = r.g.people.iter().filter(|p| p.way == 84 || p.way == 85).map(|p| p.ai).collect();
    println!("darci lowest {lowest}, drunk dead {}, cops ai {cops:?}", drunk.is_dead());
    assert_eq!(lowest, r.g.people[pi].max_health, "Darci was shot");
    assert!(drunk.is_dead(), "the cops didn't stop the drunk");
    assert_eq!(cops, [5, 5]);
}

/// Knocks person `i` down and has Darci arrest them (standing beside them on the same level).
fn arrest(r: &mut Run, i: usize) -> bool {
    let at = r.g.people[i].fig.pos;
    r.g.people[i].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, Weapon::None);
    // Let her get over any blow she took in the fight first: a flinch eats the button.
    for _ in 0..60 {
        let pi = r.player();
        if matches!(r.g.people[pi].state, State::Move) {
            break;
        }
        r.run(DT * 1.01, Controls::default());
    }
    // Then beside wherever he has come to rest.
    let body = r.g.people[i].body_pos();
    let Some(beside) = (0..8)
        .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
        .find(|p| (r.level.ground.surface(&r.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
    else {
        return false;
    };
    r.put(beside);
    r.run(0.2, Controls { action: true, ..Default::default() });
    r.run(4.0, Controls::default());
    r.g.people[i].arrested || matches!(r.g.people[i].state, State::Cuffed)
}

#[test]
fn rta_side_objectives_can_all_be_done() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    let mut r = Run { level, g, seen: Vec::new() };
    let goals = uc_game::goals::goals(&r.level.mission);
    r.run(3.0, Controls::default());
    let find = |r: &Run, way: usize| r.g.people.iter().position(|p| p.way == way);

    // 1. The man peeing by his van (cuboid 206): he talks, then fights; arrest him.
    let van = r.point(206);
    r.put(van);
    r.run(30.0, Controls::default());
    let peeing = find(&r, 238).expect("the man at the van");
    assert!(arrest(&mut r, peeing), "couldn't arrest the man at the van");

    // 2. The tramp: he appears once Darci has been through the south-east (cuboid 109), and
    // talks when she sees him.
    r.put(r.point(109));
    r.run(2.0, Controls::default());
    let tramp = find(&r, 194).expect("the tramp");
    let at = r.g.people[tramp].fig.pos;
    r.put(at + vec3(0.0, 0.0, -200.0));
    let pi = r.player();
    r.g.people[pi].face_towards(at);
    r.run(40.0, Controls::default());

    // 3. The mugger caught in the act: there once Darci has also been to the west end of Zarb
    // Drive (cuboid 267).
    r.put(r.point(267));
    r.run(3.0, Controls::default());
    let mugger = find(&r, 11).expect("the mugger in the act");
    assert!(arrest(&mut r, mugger), "couldn't arrest the mugger in the act");

    // 4. The rooftop Wildcat on Kleetus Avenue: there after the witness has talked.
    r.put(r.point(78));
    r.run(40.0, Controls::default());
    r.run(2.0, Controls::default());
    let wildcat = find(&r, 233).expect("the rooftop Wildcat");
    assert!(arrest(&mut r, wildcat), "couldn't arrest the rooftop Wildcat");

    let done = r.g.sides_done(&goals);
    for s in &goals.sides {
        println!("{} {}", if done.contains(&s.id) { "[x]" } else { "[ ]" }, s.label);
    }
    println!("outcome {:?}", r.g.outcome);
    assert_eq!(done.len(), 4);
}
