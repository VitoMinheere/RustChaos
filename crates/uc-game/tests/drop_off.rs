//! Route-finding with drops (`MAV_ACTION_FALL_OFF`): someone up on a ledge comes down off it
//! to get at Darci below, as the original's people may (`MAV_CAPS_DARCI`). In RTA. Skipped
//! without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::{Controls, Game, State};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// A spot standing at least a storey above the square next to it, with walkable ground
/// below: (up there, down here).
fn ledge(level: &Level) -> Option<(Vec3, Vec3)> {
    let s = |x: f32, z: f32, from: f32| level.ground.surface(&level.map, x, z, from);
    for zx in 20..100 {
        for xx in 20..100 {
            let (x, z) = (xx as f32 * 256.0 + 128.0, zx as f32 * 256.0 + 128.0);
            let top = s(x, z, 1.0e4);
            for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let (bx, bz) = (x + dx * 512.0, z + dz * 512.0);
                let below = s(bx, bz, top + 64.0);
                let (bx2, bz2) = (bx + dx * 512.0, bz + dz * 512.0);
                let further = s(bx2, bz2, below + 64.0);
                if top - below > 300.0 && top - below < 700.0 && (further - below).abs() < 8.0 && below > -100.0 {
                    return Some((vec3(x, top, z), vec3(bx2, below, bz2)));
                }
            }
        }
    }
    None
}

#[test]
fn down_off_a_ledge_after_her() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let (up, down) = ledge(&level).expect("no ledge in RTA");
    println!("up {up:?} down {down:?}");
    let p = g.player.unwrap();
    let i = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && q.kind != ptype::COP).expect("nobody");
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != p && j != i && (q.hostile() || q.kind == ptype::COP) {
            q.removed = true;
        }
    }
    g.people[p].fig.pos = down;
    g.people[p].state = State::Move;
    let t = &mut g.people[i];
    t.ai = 17;
    t.fake = false;
    t.mv = 1;
    t.fig.pos = up;
    t.state = State::Move;
    t.fig.angle = uc_world::character::angle_of(vec2(down.x - up.x, down.z - up.z));
    for _ in 0..(15.0 / DT) as usize {
        g.skip_cutscene();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &Controls::default(), &level);
        if (g.people[i].fig.pos.y - down.y).abs() < 32.0 {
            break;
        }
    }
    let at = g.people[i].fig.pos;
    assert!((at.y - down.y).abs() < 32.0, "still up at {at:?}, she's at {down:?}");
}
