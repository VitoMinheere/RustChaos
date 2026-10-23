//! Up a 64-high ledge: the navigation map lets people walk from the street onto a square one
//! `MAV_height` unit (64) higher (`GOTO`), so the walker steps it, as the original's does:
//! `slide_along` lets the feet through a wall whose top is less than 0x50 above them, and
//! they stand on the face they're over (#174). In RTA, Darci walks from the street onto the
//! strips inside its buildings. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::{Controls, Game, State};
use uc_world::mav::{self, Mav};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn darci_walks_up_a_64_high_ledge() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let m = Mav::new(&level);
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let p = g.player.unwrap();
    // Just Darci: nobody to walk into her or start a fight on the way.
    for (i, q) in g.people.iter_mut().enumerate() {
        q.removed |= i != p;
    }
    let inside = |x: i32, z: i32| level.map.square(x as usize, z as usize).has(uc_formats::iam::flags::HIDDEN);
    let mid = |x: i32, z: i32, y: f32| {
        let (cx, cz) = (x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
        vec3(cx, level.ground.surface(&level.map, cx, cz, y), cz)
    };
    // A street square, and a square inside a building next to it one unit up, a plain walk.
    let mut ledges = Vec::new();
    for x in 2..126 {
        for z in 2..126 {
            for (dir, &(dx, dz)) in mav::DIRS.iter().enumerate() {
                let (tx, tz) = (x + dx, z + dz);
                if !inside(x, z) && inside(tx, tz) && m.height(tx, tz) - m.height(x, z) == 1
                    && m.opt(x, z, dir) & mav::caps(mav::GOTO) != 0
                {
                    ledges.push((x, z, dx, dz));
                }
            }
        }
    }
    println!("{} ledges", ledges.len());
    assert!(ledges.contains(&(16, 95, 1, 0)), "{ledges:?}");
    let mut stuck = Vec::new();
    for &(x, z, dx, dz) in &ledges {
        let from = mid(x, z, m.height(x, z) as f32 * 64.0 + 16.0);
        let top = mid(x + dx, z + dz, from.y + 64.0 + 16.0);
        // Only where the street is level, the square's top really is a step up from it, and
        // nothing taller (a prop's box, a fence) stands in the way.
        let over = vec3(0.0, 100.0, 0.0);
        let flat = (from.y - m.height(x, z) as f32 * 64.0).abs() < 16.0;
        if !flat || !(32.0..=80.0).contains(&(top.y - from.y)) || !level.ground.line_of_sight(from + over, top + over) {
            continue;
        }
        g.people[p].fig.pos = from;
        g.people[p].state = State::Move;
        let mut up = false;
        for _ in 0..(3.0 / DT) as usize {
            g.update(DT, &Controls { wish: vec2(dx as f32, dz as f32), ..Default::default() }, &level);
            let at = g.people[p].fig.pos;
            if ((at.x / 256.0).floor() as i32, (at.z / 256.0).floor() as i32) == (x + dx, z + dz) && at.y > top.y - 8.0 {
                up = true;
                break;
            }
        }
        if !up {
            stuck.push(format!("({x},{z}) towards ({dx},{dz}), up to {}: at {}", top.y, g.people[p].fig.pos));
        }
    }
    for s in &stuck {
        println!("{s}");
    }
    assert!(stuck.is_empty(), "{} stuck", stuck.len());
}
