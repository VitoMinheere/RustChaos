//! The wall kick (`set_person_kick_off_wall`, `ANIM_WALL_KICK`): jump again in a jump up
//! against a wall and Darci kicks off it, and jumps on the other way. The original never
//! calls it; here it ends in a jump rather than standing to fight. In RTA. Skipped without
//! an install.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_game::{Controls, Game, State};
use uc_world::character::anim;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start() -> Option<(Level, Game, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..60 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let p = g.player.unwrap();
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != p {
            q.removed = true;
        }
    }
    Some((level, g, p))
}

/// A tall wall on open ground near Darci's start, and the side of it there's room on: (a
/// point on the open side, a little off its middle, and the way out from the wall).
fn wall(level: &Level, near: Vec3) -> (Vec3, Vec2) {
    level.ground.walls.iter()
        .filter_map(|w| {
            let (a, b) = (Vec2::from(w.a), Vec2::from(w.b));
            let mid = (a + b) * 0.5;
            if (b - a).length() < 512.0 || (mid - vec2(near.x, near.z)).length() > 40.0 * 256.0 {
                return None;
            }
            let n = vec2(a.y - b.y, b.x - a.x).normalize_or_zero();
            [n, -n].into_iter().find_map(|out| {
                let at = mid + out * 80.0;
                let y = level.ground.surface(&level.map, at.x, at.y, w.y0 + 64.0);
                // On the floor at its foot, the wall well over head height above it, and room
                // to jump back the other way.
                let back = mid + out * 600.0;
                let room = level.ground.surface(&level.map, back.x, back.y, y + 64.0);
                // And it stops someone going in, here.
                let ahead = (0..4).fold(at, |q, _| level.ground.slide(q, y + 60.0, -out * 24.0, 36.0));
                let stops = (ahead - at).dot(-out) < 48.0;
                ((y - w.y0).abs() < 8.0 && w.y1 - y > 400.0 && (room - y).abs() < 8.0 && stops
                    && level.ground.line_of_sight(vec3(at.x, y + 100.0, at.y), vec3(back.x, y + 100.0, back.y)))
                    .then(|| (vec3(at.x, y + 60.0, at.y), out))
            })
        })
        .next()
        .expect("a wall to kick off")
}

#[test]
fn jump_again_against_a_wall_to_kick_off_it() {
    let Some((level, mut g, p)) = start() else { return };
    let (at, out) = wall(&level, g.people[p].fig.pos);
    // In the air just off it, going in.
    let me = &mut g.people[p];
    me.fig.pos = at;
    me.fig.angle = uc_world::character::angle_of(-out);
    me.state = State::Fall { vy: 50.0, vel: -out * 400.0 };
    g.update(DT, &Controls { jump: true, ..Default::default() }, &level);
    assert!(matches!(g.people[p].state, State::WallKick { .. }), "no wall kick: {:?}", g.people[p].state);
    assert_eq!(g.people[p].fig.anim(), anim::WALL_KICK);
    // Off it, turned round and jumping away.
    let mut off = None;
    for _ in 0..90 {
        g.update(DT, &Controls::default(), &level);
        if let State::Fall { vel, .. } = g.people[p].state {
            off = Some(vel);
            break;
        }
    }
    let vel = off.expect("never kicked off");
    assert!(vel.normalize().dot(out) > 0.9, "off the wrong way: {vel}");
    assert!(g.people[p].facing_dir().dot(out) > 0.9, "not turned round");
}

/// Jumping in the open, a second press does nothing.
#[test]
fn not_in_the_open() {
    let Some((level, mut g, p)) = start() else { return };
    let (at, out) = wall(&level, g.people[p].fig.pos);
    let me = &mut g.people[p];
    me.fig.pos = at + vec3(out.x, 0.0, out.y) * 300.0;
    me.state = State::Fall { vy: 50.0, vel: out * 400.0 };
    g.update(DT, &Controls { jump: true, ..Default::default() }, &level);
    assert!(matches!(g.people[p].state, State::Fall { .. } | State::Land { .. }), "{:?}", g.people[p].state);
}
