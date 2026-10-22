//! People up a 64-high ledge: someone out to fight Darci from the street, with her a square
//! along a strip one `MAV_height` unit (64) up, comes up its ledge to her rather than pushing into
//! the wall at its foot (#174, #88). In RTA. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Game, State};
use uc_world::mav::{self, Mav};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn a_fighter_walks_up_a_64_high_ledge() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let m = Mav::new(&level);
    let start = || {
        let mut g = Game::new(&level, true);
        for _ in 0..(60.0 / DT) as usize {
            g.skip_cutscene();
            g.update(DT, &Controls::default(), &level);
            if g.time > 1.0 && !g.in_conversation() {
                break;
            }
        }
        g
    };
    let inside = |x: i32, z: i32| level.map.square(x as usize, z as usize).has(uc_formats::iam::flags::HIDDEN);
    let mid = |x: i32, z: i32, y: f32| {
        let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
        vec3(cx, level.ground.surface(&level.map, cx, cz, y), cz)
    };
    let goto = |x: i32, z: i32, dir: usize| m.opt(x, z, dir) & mav::caps(mav::GOTO) != 0;
    let flat = |x: i32, z: i32| (mid(x, z, m.height(x, z) as f32 * 64.0 + 16.0).y - m.height(x, z) as f32 * 64.0).abs() < 16.0;
    // A ledge: three level street squares in a line, then a square inside a building one unit
    // up, a plain walk, and on along it a square: Darci there, out of reach from the street,
    // and the fighter three squares back.
    let mut tries = Vec::new();
    for x in 4..124 {
        for z in 4..124 {
            for (dir, &(dx, dz)) in mav::DIRS.iter().enumerate() {
                let (tx, tz) = (x + dx, z + dz);
                let street = (0..3).all(|k| {
                    let (sx, sz) = (x - dx * k, z - dz * k);
                    !inside(sx, sz) && m.height(sx, sz) == m.height(x, z) && goto(sx, sz, dir) && flat(sx, sz)
                });
                let (fx, fz) = (tx + dx, tz + dz);
                if !street || !inside(tx, tz) || m.height(tx, tz) - m.height(x, z) != 1
                    || !inside(fx, fz) || m.height(fx, fz) != m.height(tx, tz) || !goto(tx, tz, dir)
                {
                    continue;
                }
                // Up a step, with nothing taller (a prop, a fence) in the way.
                let from = mid(x, z, m.height(x, z) as f32 * 64.0 + 16.0);
                let step = mid(tx, tz, from.y + 80.0);
                let far = mid(fx, fz, step.y + 16.0);
                let over = vec3(0.0, 100.0, 0.0);
                if (32.0..=80.0).contains(&(step.y - from.y)) && (far.y - step.y).abs() < 16.0
                    && level.ground.line_of_sight(from + over, step + over) && level.ground.line_of_sight(step + over, far + over)
                {
                    tries.push(((x - dx * 2, z - dz * 2), (fx, fz), step.y));
                }
            }
        }
    }
    println!("{} ledges", tries.len());
    assert!(tries.len() > 10, "{} ledges", tries.len());
    let mut stuck = Vec::new();
    for &((bx, bz), (hx, hz), y) in &tries {
        let mut g = start();
        let p = g.player.unwrap();
        let Some(i) = g.people.iter().position(|q| q.fake && !q.is_dead()) else { return };
        // Just the two of them.
        for (k, q) in g.people.iter_mut().enumerate() {
            q.removed |= k != p && k != i;
        }
        g.people[p].fig.pos = mid(hx, hz, y + 16.0);
        g.people[i].fig.pos = mid(bx, bz, m.height(bx, bz) as f32 * 64.0 + 16.0);
        g.people[p].state = State::Move;
        g.people[i].state = State::Move;
        g.people[i].fake = false;
        g.people[i].go_for();
        let mut up = false;
        for _ in 0..(15.0 / DT) as usize {
            g.update(DT, &Controls::default(), &level);
            let at = g.people[i].fig.pos;
            if at.y > y - 8.0 {
                up = true;
                break;
            }
        }
        if !up {
            stuck.push(format!("from ({bx},{bz}) to ({hx},{hz}), up to {y}: at {}", g.people[i].fig.pos));
        }
    }
    for s in &stuck {
        println!("{s}");
    }
    assert!(stuck.is_empty(), "{} of {} stuck", stuck.len(), tries.len());
}
