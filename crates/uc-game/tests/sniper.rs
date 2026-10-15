//! The sniper rifle: picked up like any gun, and its scope the original's sniper mode
//! (snipe.cpp), which the shipped game never let anyone use. In RTA, out in the street.
//! Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::{Controls, Drive, Game, Item, State, Weapon, SNIPER};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// RTA, with Darci out on open ground (as the route tests find it), facing +z.
fn start() -> Option<(Level, Game, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let p = g.player.unwrap();
    let at = vec3(66.0 * 256.0, 0.0, 74.0 * 256.0);
    let y = level.ground.surface(&level.map, at.x, at.z, 1.0e4);
    g.people[p].fig.pos = vec3(at.x, y, at.z);
    g.people[p].fig.angle = uc_world::character::angle_of(vec2(0.0, 1.0));
    g.people[p].state = State::Move;
    Some((level, g, p))
}

fn run(g: &mut Game, level: &Level, secs: f32, c: Controls) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &c, level);
    }
}

#[test]
fn picked_up_like_any_gun() {
    let Some((level, mut g, p)) = start() else { return };
    let at = g.people[p].fig.pos;
    g.items.push(Item { way: 0, kind: SNIPER, pos: at + vec3(30.0, 0.0, 0.0) });
    run(&mut g, &level, 0.2, Controls::default());
    assert!(g.people[p].has(Weapon::Sniper), "not picked up");
    assert_eq!(g.people[p].weapon, Weapon::Sniper);
    assert_eq!(g.people[p].rounds(), Some(10));
}

/// Z looks through the scope: its lens eases in from 20 to 30, the keys turn it, and Z again puts
/// it down. Only with the rifle in hand.
#[test]
fn the_scope() {
    let Some((level, mut g, p)) = start() else { return };
    let scope = Controls { scope: true, ..Default::default() };
    run(&mut g, &level, DT, scope);
    assert!(g.sniping().is_none(), "a scope with no rifle");
    g.people[p].give(Weapon::Sniper);
    g.people[p].weapon = Weapon::Sniper;
    run(&mut g, &level, DT, scope);
    let (dir, zoom) = g.sniping().expect("no scope").view();
    assert!(dir.dot(vec3(0.0, 0.0, 1.0)) > 0.99, "looking {dir:?}, not her way");
    assert!((zoom - 5.0).abs() < 0.5, "zoom {zoom}");
    // The lens creeps in, all the way over sixteen seconds or so.
    run(&mut g, &level, 3.0, Controls::default());
    let (_, zoom) = g.sniping().unwrap().view();
    assert!(zoom > 5.2 && zoom < 7.0, "zoomed to {zoom} after 3 seconds");
    run(&mut g, &level, 20.0, Controls::default());
    let (_, zoom) = g.sniping().unwrap().view();
    assert!((zoom - 7.5).abs() < 0.3, "zoomed to {zoom}");
    // Left turns it left, slowly for fine aim (about 17 degrees a second); up tilts it up,
    // and no further than the spring lets it (300 of 2048, 53 degrees).
    run(&mut g, &level, 2.0, Controls { drive: Drive { steer: -1.0, ..Default::default() }, ..Default::default() });
    let (dir, _) = g.sniping().unwrap().view();
    assert!(dir.x > 0.2 && dir.x < 0.8, "turned left to {dir:?}");
    run(&mut g, &level, 8.0, Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() });
    let (dir, _) = g.sniping().unwrap().view();
    assert!(dir.y > 0.6 && dir.y < 0.9, "tilted to {}", dir.y);
    // She stood still meanwhile.
    assert!(matches!(g.people[p].state, State::Move));
    run(&mut g, &level, DT, scope);
    assert!(g.sniping().is_none());
}

/// Through the scope, a shot hits whoever is in its sights eight squares off, for 260.
#[test]
fn a_shot_through_the_scope() {
    let Some((level, mut g, p)) = start() else { return };
    g.people[p].give(Weapon::Sniper);
    g.people[p].weapon = Weapon::Sniper;
    let me = g.people[p].fig.pos;
    let t = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && q.way != 0).expect("nobody to shoot");
    let at = me + vec3(0.0, 0.0, 8.0 * 256.0);
    let y = level.ground.surface(&level.map, at.x, at.z, 1.0e4);
    g.people[t].fig.pos = vec3(at.x, y, at.z);
    g.people[t].state = State::Move;
    g.people[t].max_health = 1000;
    g.people[t].health = 1000;
    run(&mut g, &level, DT, Controls { scope: true, ..Default::default() });
    assert!(g.sniping().is_some());
    run(&mut g, &level, DT, Controls { punch: true, ..Default::default() });
    assert_eq!(g.people[p].rounds(), Some(9));
    assert!(g.people[t].health <= 1000 - 260, "health {}", g.people[t].health);
    // Off to one side, nobody's in its sights.
    let before = g.people[t].health;
    g.people[t].fig.pos = Vec3::new(at.x + 4.0 * 256.0, y, at.z);
    run(&mut g, &level, DT, Controls { punch: true, ..Default::default() });
    assert_eq!(g.people[t].health, before, "hit off to the side");
}

/// Not in the original: a few missions have the rifle lying just in front of the player at
/// the start, to try it out.
#[test]
fn lying_by_the_start() {
    let Some(game) = uc_formats::default_install() else { return };
    for m in ["train11", "police1", "police2", "Stealtst1", "Gangorder1", "MIB"] {
        let level = Level::load(&game, m, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
        let mut g = Game::new(&level, true);
        g.update(DT, &Controls::default(), &level);
        let me = g.people[g.player.unwrap()].fig.pos;
        let rifle = g.items.iter().find(|i| i.kind == SNIPER).unwrap_or_else(|| panic!("{m}: no rifle")).pos;
        assert!((rifle - me).length() < 200.0, "{m}: the rifle's at {rifle:?}, the player at {me:?}");
        assert!((rifle.y - me.y).abs() < 64.0, "{m}: the rifle's at another height: {rifle:?}, the player at {me:?}");
    }
}
