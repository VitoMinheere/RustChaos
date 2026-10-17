//! Round a building's corner: someone after Darci on one side of a corner, she on the other,
//! goes round it to her rather than pushing into the wall (#88). In RTA, at every outside
//! corner of a building out in the street. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::{Controls, Game, State};
use uc_world::mav::Mav;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn people_after_darci_go_round_corners() {
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
    let sq = |x: i32, z: i32| (x as u8, z as u8);
    let f = |v: Vec3| ((v.x / 256.0) as u8, (v.z / 256.0) as u8);
    // Outside corners: a building filling the three by three squares on the diagonal (sx, sz)
    // from square (x, z), and a straight walk along each of its two faces.
    let mut corners = Vec::new();
    for x in 4..124 {
        for z in 4..124 {
            for (sx, sz) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let block = (1..=3).all(|j| (1..=3).all(|k| inside(x + sx * j, z + sz * k)));
                // Out in the street: the squares along both faces, and a square out from them.
                let street = (0..=3).all(|j| (-1..=0).all(|k| !inside(x + sx * j, z + sz * k) && !inside(x + sx * k, z + sz * j)));
                let (a, c, b) = (sq(x + sx * 3, z), sq(x, z), sq(x, z + sz * 3));
                if block && street && m.can_walk(a, c) && m.can_walk(c, b) && !m.can_walk(a, b) {
                    corners.push((x, z, sx, sz));
                }
            }
        }
    }
    println!("{} corners", corners.len());
    assert!(corners.len() > 10, "{} corners", corners.len());
    let mut stuck = Vec::new();
    // Either following her (`PCOM_MOVE_FOLLOW`) or out to fight her.
    let tries = corners.iter().flat_map(|&c| [(1, 3, false), (2, 2, false), (3, 1, false), (1, 3, true), (3, 1, true)].map(|(j, k, f)| (c, j, k, f)));
    for ((x, z, sx, sz), j, k, fighter) in tries {
        let mut g = start();
        let p = g.player.unwrap();
        let put = |g: &mut Game, i: usize, at: Vec3| {
            let y = level.ground.surface(&level.map, at.x, at.z, level.map.ground_height(at.x, at.z));
            g.people[i].fig.pos = vec3(at.x, y, at.z);
            g.people[i].state = State::Move;
        };
        let mid = |x: i32, z: i32| vec3(x as f32 * 256.0 + 128.0, 0.0, z as f32 * 256.0 + 128.0);
        // Darci k squares down one face, the other j along the other, hugging the wall.
        put(&mut g, p, mid(x, z + sz * k));
        let Some(i) = g.people.iter().position(|q| q.fake && !q.is_dead()) else { return };
        put(&mut g, i, mid(x + sx * j, z) + vec3(0.0, 0.0, sz as f32 * 80.0));
        let q = &mut g.people[i];
        q.fake = false;
        if fighter {
            q.go_for();
        } else {
            (q.mv, q.follow) = (5, 0);
        }
        let mut closest = f32::MAX;
        let (mut still, mut longest, mut last) = (0.0f32, 0.0f32, g.people[i].fig.pos);
        for _ in 0..(15.0 / DT) as usize {
            g.update(DT, &Controls::default(), &level);
            let at = g.people[i].fig.pos;
            let her = g.people[p].fig.pos;
            closest = closest.min((at - her).length());
            // Round the corner: she's in plain sight, and no route's needed to get to her.
            let waist = vec3(0.0, 40.0, 0.0);
            if closest < 2.5 * 256.0 || m.can_walk(f(at), f(her)) && level.ground.line_of_sight(at + waist, her + waist) {
                closest = 0.0;
                break;
            }
            if (at - last).length() < 2.0 {
                still += DT;
                longest = longest.max(still);
            } else {
                still = 0.0;
            }
            last = at;
        }
        if closest > 0.0 {
            stuck.push(format!("corner ({x},{z}) towards ({sx},{sz}), {j} and {k} squares off, fighting {fighter}: no nearer than {:.0}, still for {longest:.1}s at {}", closest, g.people[i].fig.pos));
        }
    }
    for s in &stuck {
        println!("{s}");
    }
    assert!(stuck.is_empty(), "{} stuck", stuck.len());
}
