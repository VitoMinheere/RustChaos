//! Darci's grappling hook (#109): thrown with attack, it flies, and either catches a roof's
//! edge, for her to climb up its line onto the roof, or comes back to her hand. In RTA's
//! streets with no mission running. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::game::hook::Hold;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::Level;

mod common;
use common::DT;

fn start() -> Option<(Level, Game)> {
    let level = common::load("police1")?;
    let g = fresh(&level);
    Some((level, g))
}

/// A game with nothing going on, the hook in her hand.
fn fresh(level: &Level) -> Game {
    let mut g = Game::new(level, false);
    for _ in 0..30 {
        g.update(DT, &Controls::default(), level);
    }
    assert!(g.give("hook"));
    g
}

fn run(g: &mut Game, level: &Level, secs: f32, c: Controls) {
    for _ in 0..(secs / DT) as usize {
        g.update(DT, &c, level);
    }
}

/// Stands her `back` from the foot of ladder `l`'s wall, facing it.
fn face_wall(g: &mut Game, level: &Level, l: usize, back: f32) {
    let ladder = level.ground.ladders[l];
    let at = ladder.foot + ladder.out * back;
    let y = level.ground.surface(&level.map, at.x, at.y, ladder.base + 64.0);
    assert!(g.teleport(vec3(at.x, y, at.y), level));
    let p = g.player.unwrap();
    g.people[p].fig.angle = uc_world::character::angle_of(-ladder.out);
}

fn throw(g: &mut Game, level: &Level) {
    g.update(DT, &Controls { punch: true, ..Default::default() }, level);
}

#[test]
fn the_hook_is_thrown_and_comes_back() {
    let Some((level, mut g)) = start() else { return };
    let p = g.player.unwrap();
    assert_eq!(g.people[p].weapon, Weapon::Hook);
    // Out of the police station, where she starts.
    let outside = uc_game::game::endless::out_of_doors(&level, g.people[p].fig.pos);
    assert!(g.teleport(outside, &level));
    run(&mut g, &level, 0.5, Controls::default());
    throw(&mut g, &level);
    // (A grenade's throw: the hook's own wind-up isn't in the shipped animations.)
    assert!(matches!(g.people[p].state, State::Hook { released: false, .. }), "no throw: {:?}", g.people[p].state);
    assert!(g.hook.is_none(), "gone before the throw let it go");
    let mut flew = false;
    for _ in 0..(3.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        if g.hook.as_ref().is_some_and(|h| h.hold == Hold::Flying) {
            flew = true;
            break;
        }
    }
    assert!(flew, "it never left her hand");
    let from = g.people[p].fig.pos;
    let mut far = 0.0f32;
    for _ in 0..(10.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        let Some(h) = &g.hook else { break };
        far = far.max((h.pos - from).length());
        if matches!(h.hold, Hold::Caught { .. }) {
            // Caught on something: walk off and it's pulled free.
            let _ = g.teleport(from + vec3(20.0 * 256.0, 0.0, 0.0), &level);
        }
    }
    assert!(far > 2.0 * 256.0, "it only went {far:.0}");
    assert!(g.hook.is_none(), "it never came back: {:?}", g.hook.as_ref().map(|h| h.hold));
    assert!(g.people[p].has(Weapon::Hook));
}

/// Thrown at a wall a ladder goes up, the hook catches the roof's edge somewhere, and she
/// climbs its line up onto the roof.
#[test]
fn the_hook_catches_a_roof_and_she_climbs_up() {
    let Some((level, _)) = start() else { return };
    let walls: Vec<usize> = (0..level.ground.ladders.len())
        .filter(|&l| (400.0..900.0).contains(&(level.ground.ladders[l].top - level.ground.ladders[l].base)))
        .collect();
    assert!(!walls.is_empty(), "no ladders to try");
    for &l in &walls {
        let mut g = fresh(&level);
        let p = g.player.unwrap();
        face_wall(&mut g, &level, l, 3.0 * 256.0);
        throw(&mut g, &level);
        let (mut caught, mut out) = (None, false);
        for _ in 0..(5.0 / DT) as usize {
            g.update(DT, &Controls::default(), &level);
            match g.hook.as_ref().map(|h| h.hold) {
                Some(Hold::Caught { edge, dir, to }) => {
                    caught = Some((edge, dir, to));
                    break;
                }
                // Back in her hand.
                None if out => break,
                None => {}
                _ => out = true,
            }
        }
        let Some((edge, dir, to)) = caught else { continue };
        // To the foot of the wall under the hook, and up.
        let foot = vec2(edge.x, edge.z) - dir * 120.0;
        let y = level.ground.surface(&level.map, foot.x, foot.y, g.people[p].fig.pos.y + 64.0);
        if edge.y - y < 300.0 {
            continue;
        }
        assert!(g.teleport(vec3(foot.x, y, foot.y), &level));
        throw(&mut g, &level);
        assert!(matches!(g.people[p].state, State::Rope { .. }), "not on the line: {:?}", g.people[p].state);
        let up = Controls { wish: dir, ..Default::default() };
        let mut top = false;
        for _ in 0..(12.0 / DT) as usize {
            g.update(DT, &up, &level);
            let q = &g.people[p];
            if matches!(q.state, State::Move) && (q.fig.pos.y - to.y).abs() < 48.0 {
                top = true;
                break;
            }
        }
        assert!(top, "never got up: at {:?}, roof at {to:?}, {:?}", g.people[p].fig.pos, g.people[p].state);
        run(&mut g, &level, 2.0, Controls::default());
        assert!(g.hook.is_none(), "the hook was never wound back in");
        return;
    }
    panic!("the hook never caught a roof by any of {} ladders", walls.len());
}
