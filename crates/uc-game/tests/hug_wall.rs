//! Back to a wall (#205, `STATE_HUG_WALL`): action standing in front of a building's wall
//! puts her back to it; she edges along it, looks round its end, and leaps out. Skipped
//! without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::person::Hug;
use uc_game::{Controls, Game, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// A building wall in police1 she can put her back to: (where to stand, facing the wall,
/// the wall's ends).
fn find_wall(level: &Level) -> Option<(glam::Vec3, Vec2, Vec2, Vec2)> {
    use uc_formats::iam::facet_type;
    for f in level.map.facets.iter().skip(1).filter(|f| f.kind == facet_type::NORMAL) {
        let (a, b) = (vec2(f.x[0] as f32, f.z[0] as f32) * 256.0, vec2(f.x[1] as f32, f.z[1] as f32) * 256.0);
        if (b - a).length() < 1000.0 {
            continue;
        }
        let along = (b - a).normalize();
        for n in [vec2(-along.y, along.x), vec2(along.y, -along.x)] {
            let at = (a + b) * 0.5 + n * 48.0;
            let y = level.ground.surface(&level.map, at.x, at.y, f.y[0] as f32 + 32.0);
            if (y - f.y[0] as f32).abs() > 8.0 {
                continue;
            }
            let pos = vec3(at.x, y, at.y);
            if level.ground.wall_to_hug(&level.map, pos, -n).is_some() {
                return Some((pos, -n, a, b));
            }
        }
    }
    None
}

#[test]
fn darci_backs_to_a_wall_edges_along_it_and_leaps_out() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let (pos, facing, a, b) = find_wall(&level).expect("a wall to hug");
    let pi = g.player.unwrap();
    // Nobody else about to get in the way.
    for (i, p) in g.people.iter_mut().enumerate() {
        if i != pi {
            p.removed = true;
        }
    }
    g.people[pi].fig.pos = pos;
    g.people[pi].fig.angle = uc_world::character::angle_of(facing);
    g.people[pi].state = State::Move;
    let run = |g: &mut Game, c: Controls, secs: f32| {
        for _ in 0..(secs / DT) as usize {
            g.update(DT, &c, &level);
        }
    };
    g.update(DT, &Controls { action: true, ..Default::default() }, &level);
    assert!(matches!(g.people[pi].state, State::HugWall { hug: Hug::Turn, .. }), "action turns her to the wall: {:?}", g.people[pi].state);
    run(&mut g, Controls::default(), 2.0);
    let p = &g.people[pi];
    let State::HugWall { hug, out, .. } = p.state else { panic!("not against the wall: {:?}", p.state) };
    assert_eq!(hug, Hug::Stand);
    assert!(p.fig.facing().dot(out) > 0.9, "she faces away from it");
    let along = (b - a).normalize();
    let off = (vec2(p.fig.pos.x, p.fig.pos.z) - a).perp_dot(along).abs();
    assert!((off - 32.0).abs() < 4.0, "she's 32 off the wall: {off}");

    // Along it towards `b`: she edges there and looks round its end.
    let start = vec2(p.fig.pos.x, p.fig.pos.z);
    run(&mut g, Controls { wish: along, ..Default::default() }, 0.5);
    let moved = (vec2(g.people[pi].fig.pos.x, g.people[pi].fig.pos.z) - start).dot(along);
    assert!(moved > 30.0, "she edges along: {moved}");
    let mut looked = false;
    for _ in 0..(30.0 / DT) as usize {
        g.update(DT, &Controls { wish: along, ..Default::default() }, &level);
        if let State::HugWall { hug: Hug::Look { .. }, .. } = g.people[pi].state {
            looked = true;
            break;
        }
        if !matches!(g.people[pi].state, State::HugWall { .. }) {
            panic!("off the wall: {:?}", g.people[pi].state);
        }
    }
    println!("reached the end and looked round: {looked}");
    if looked {
        run(&mut g, Controls::default(), 1.0);
        let peek = g.people[pi].peek().expect("the camera leads round the corner");
        assert!(peek.dot(along) > 200.0, "it leads round: {peek:?}");
        // Action: out round the corner, and back on her feet.
        g.update(DT, &Controls { action: true, ..Default::default() }, &level);
        assert!(matches!(g.people[pi].state, State::HugWall { hug: Hug::LeapOut, .. }));
        run(&mut g, Controls::default(), 3.0);
        assert_eq!(g.people[pi].state, State::Move);
    } else {
        // Stopped short of the end by something in the way: still against the wall.
        assert!(matches!(g.people[pi].state, State::HugWall { .. }));
    }

    // Walking away from a wall takes her off it.
    g.people[pi].fig.pos = pos;
    g.people[pi].fig.angle = uc_world::character::angle_of(facing);
    g.update(DT, &Controls { action: true, ..Default::default() }, &level);
    run(&mut g, Controls::default(), 2.0);
    assert!(g.people[pi].hugging());
    run(&mut g, Controls { wish: -facing, ..Default::default() }, 0.2);
    assert!(!g.people[pi].hugging(), "walking away leaves the wall");
}
