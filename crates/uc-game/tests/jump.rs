//! The Jump (`police2`): Barnaby stands on the edge of the Brooks Tower roof. Walk straight up
//! to him and he talks, then runs off the edge (his patrol takes him to a point on the
//! ground); come at him from behind and Darci grabs him. Checks the script and the people,
//! with Darci put where she needs to be. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::{Controls, Game, Outcome};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start() -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    assert!(g.unsupported().len() <= 2, "unsupported: {:?}", g.unsupported());
    for _ in 0..(5.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    Some((level, g))
}

fn point(level: &Level, i: usize) -> Vec3 {
    let e = level.mission.event_points.iter().find(|e| e.index == i).unwrap();
    vec3(e.x as f32, e.y as f32, e.z as f32)
}

/// Puts Darci at `p` (on whatever is there) and plays `secs` seconds, printing messages.
fn go(level: &Level, g: &mut Game, p: Vec3, secs: f32, seen: &mut Vec<String>) {
    let pi = g.player.unwrap();
    let y = level.ground.surface(&level.map, p.x, p.z, p.y + 64.0);
    g.people[pi].fig.pos = vec3(p.x, y, p.z);
    for _ in 0..(secs / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), level);
        for m in &g.messages {
            if !seen.contains(&m.text) {
                println!("[{:5.1}s] {}", g.time, m.text);
                seen.push(m.text.clone());
            }
        }
        if g.outcome.is_some() {
            return;
        }
    }
}

#[test]
fn barnaby_is_talked_down_from_behind() {
    let Some((level, mut g)) = start() else { return };
    let barnaby = g.people.iter().position(|p| p.way == 54).expect("the jumper");
    let on_roof = g.people[barnaby].fig.pos;
    println!("Barnaby at {on_roof:?}");
    assert!(on_roof.y > 1500.0, "the jumper isn't up on the tower");
    // Behind him (cuboid 300), without walking into his line of sight (radius 71).
    let mut seen = Vec::new();
    go(&level, &mut g, point(&level, 300), 60.0, &mut seen);
    let b = &g.people[barnaby];
    println!("outcome {:?}, Barnaby {:?} at {:?}", g.outcome, b.state, b.fig.pos);
    assert!(!b.is_dead(), "Barnaby died");
    assert_eq!(g.outcome, Some(Outcome::Won));
}

#[test]
fn walking_straight_up_to_barnaby_makes_him_jump() {
    let Some((level, mut g)) = start() else { return };
    let barnaby = g.people.iter().position(|p| p.way == 54).expect("the jumper");
    let mut seen = Vec::new();
    go(&level, &mut g, point(&level, 71), 60.0, &mut seen);
    let b = &g.people[barnaby];
    println!("outcome {:?}, Barnaby {:?} at {:?} health {}", g.outcome, b.state, b.fig.pos, b.health);
    assert!(b.is_dead(), "Barnaby didn't jump");
    assert!(matches!(g.outcome, Some(Outcome::Lost(_))));
}

#[test]
fn the_zip_line_drops_darci_behind_barnaby() {
    // The subtle way: from the top of the tall building, the zip line runs down onto Barnaby's
    // roof, well behind him; then along the roof to him.
    let Some((level, mut g)) = start() else { return };
    let cable = level.ground.cables.iter().position(|c| (c.at(0.0).x - 7936.0).abs() < 1.0).expect("the zip line");
    let top = level.ground.cables[cable].at(0.0);
    let pi = g.player.unwrap();
    // Standing on the roof under the cable's top end, a little way along it, and jumping.
    let along = level.ground.cables[cable].at(0.008);
    g.people[pi].fig.pos = vec3(along.x, level.ground.surface(&level.map, along.x, along.z, top.y), along.z);
    let mut rode = false;
    let behind = point(&level, 300);
    // The mission opens with a conversation, which holds her still: skip through it (a
    // press every so often cuts each line short) and jump once it's over.
    for f in 0..(80.0 / DT) as usize {
        g.skip_cutscene();
        let p = &g.people[pi];
        // Jump for the cable; once off it, walk along the roof to Barnaby's back.
        let off = rode && !matches!(p.state, uc_game::State::Zip { .. });
        let to = glam::vec2(behind.x - p.fig.pos.x, behind.z - p.fig.pos.z);
        let wish = if off && to.length() > 60.0 { to.normalize() } else { glam::Vec2::ZERO };
        // Another conversation can follow straight on, so keep trying until she's on it.
        let jump = !rode && !g.in_conversation() && f % 15 == 0;
        let punch = g.in_conversation() && f % 20 == 0;
        let c = Controls { jump, punch, wish, ..Default::default() };
        g.update(DT, &c, &level);
        rode |= matches!(g.people[pi].state, uc_game::State::Zip { .. });
        if g.outcome.is_some() {
            break;
        }
    }
    let p = &g.people[pi];
    println!("rode {rode}, Darci at {:?} {:?} health {}, outcome {:?}", p.fig.pos, p.state, p.health, g.outcome);
    assert!(rode, "Darci didn't catch the zip line");
    assert!(p.fig.pos.y > 2000.0, "Darci fell off the zip line");
    assert_eq!(p.health, p.max_health, "Darci was hurt landing");
    assert_eq!(g.outcome, Some(Outcome::Won), "coming at Barnaby from behind should win");
}

#[test]
fn the_jump_side_objectives_say_what_to_do() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police2", &LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let goals = uc_game::goals::goals(&level.mission);
    let labels: Vec<&str> = goals.sides.iter().map(|s| s.label.as_str()).collect();
    println!("{labels:?}");
    assert_eq!((goals.badges.len(), goals.sides.len()), (19, 3));
    assert!(labels.iter().all(|l| l.starts_with("Arrest") || l.starts_with("Talk") || l.starts_with("Take")), "{labels:?}");
}
