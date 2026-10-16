//! Grenades (fallen/Source/grenade.cpp), in The Fallen, which leaves one lying out: picked up
//! three at a time, primed and thrown, and held too long. Skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::{Controls, Game, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start() -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "MIB", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    // Past the opening, and any talk that holds her still.
    for _ in 0..(90.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    Some((level, g))
}

fn run(g: &mut Game, level: &Level, secs: f32, c: Controls) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        // Nobody else hostile about: what's tested is the grenade.
        for p in g.people.iter_mut().filter(|p| p.hostile()) {
            p.removed = true;
        }
        g.update(DT, &c, level);
    }
}

fn press_action(g: &mut Game, level: &Level) {
    run(g, level, DT, Controls { action: true, ..Default::default() });
    run(g, level, 1.0, Controls::default());
}

/// Standing on the one lying out picks up three.
#[test]
fn grenades_are_picked_up_three_at_a_time() {
    let Some((level, mut g)) = start() else { return };
    let it = g.items.iter().find(|i| i.kind == 22).expect("no grenade lying out").pos;
    let p = g.player.unwrap();
    g.people[p].fig.pos = it;
    run(&mut g, &level, 0.5, Controls::default());
    assert_eq!(g.people[p].grenades, 3);
    assert!(g.people[p].has(Weapon::Grenade));
}

/// Primed, then thrown: it flies ahead, lands, and goes off when its six seconds are up,
/// knocking down whoever stands where it came down.
#[test]
fn a_thrown_grenade_flies_lands_and_goes_off() {
    let Some((level, mut g)) = start() else { return };
    let p = g.player.unwrap();
    // Somewhere open and flat: the ground level and nothing solid for eight squares ahead.
    let me = g.people[p].fig.pos;
    let level_ground = |d: Vec2| (0..32).all(|k| {
        let at = me + vec3(d.x, 0.0, d.y) * (k as f32 * 64.0);
        (level.ground.map_height(&level.map, at.x, at.z) - me.y).abs() < 4.0
    });
    let dir = [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)].into_iter()
        .find(|&d| level_ground(d) && level.ground.line_of_sight(me + vec3(0.0, 100.0, 0.0), me + vec3(d.x, 0.0, d.y) * 2048.0 + vec3(0.0, 100.0, 0.0)))
        .expect("nowhere open");
    let pl = &mut g.people[p];
    pl.grenades = 3;
    pl.weapon = Weapon::Grenade;
    pl.fig.angle = uc_world::character::angle_of(dir);
    press_action(&mut g, &level);
    assert!(g.primed.is_some(), "the first press should prime it");
    press_action(&mut g, &level);
    assert_eq!(g.grenades.len(), 1, "the second press should throw it");
    assert_eq!(g.people[p].grenades, 2);
    // Down and still within a few seconds, having bounced its way to a stop.
    run(&mut g, &level, 4.0, Controls::default());
    let gr = g.grenades.first().expect("gone off too soon").clone();
    assert!(gr.vel.length() < 1.0, "still moving: {:?}", gr.vel);
    let thrown = Vec2::new(gr.pos.x - me.x, gr.pos.z - me.z);
    assert!(thrown.dot(dir) > 400.0, "it only went {:.0} ahead", thrown.dot(dir));
    // Someone standing on it when it goes off.
    let victim = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead()).expect("nobody about");
    g.people[victim].fig.pos = gr.pos + vec3(60.0, 0.0, 0.0);
    g.people[victim].removed = false;
    run(&mut g, &level, 2.5, Controls::default());
    assert!(g.grenades.is_empty(), "never went off");
    assert!(g.people[victim].is_down() || g.people[victim].is_dead(), "the blast missed: {:?}", g.people[victim].state);
}

/// Attack primes it and throws it too, as in the original (`player_activate_in_hand`): she
/// plays the throw (`ANIM_CAN_RELEASE`), and it leaves her hand partway through, not at once.
#[test]
fn attack_throws_with_the_throw_animation() {
    let Some((level, mut g)) = start() else { return };
    let p = g.player.unwrap();
    let pl = &mut g.people[p];
    pl.grenades = 3;
    pl.weapon = Weapon::Grenade;
    let punch = Controls { punch: true, ..Default::default() };
    run(&mut g, &level, DT, punch);
    run(&mut g, &level, 0.3, Controls::default());
    assert!(g.primed.is_some(), "attack should prime it");
    assert!(g.grenades.is_empty());
    run(&mut g, &level, DT, punch);
    assert!(matches!(g.people[p].state, uc_game::person::State::Throw { .. }), "no throw: {:?}", g.people[p].state);
    assert_eq!(g.people[p].fig.anim(), uc_game::game::grenades::ANIM_CAN_RELEASE);
    assert!(g.grenades.is_empty(), "out of her hand before the throw");
    let mut thrown = false;
    for _ in 0..60 {
        run(&mut g, &level, DT, Controls::default());
        thrown |= !g.grenades.is_empty();
    }
    assert!(thrown, "the throw never let it go");
    assert_eq!(g.people[p].grenades, 2);
    assert!(g.primed.is_none());
    assert!(matches!(g.people[p].state, uc_game::person::State::Move), "still throwing");
}

/// Held past its fuse, it goes off in her hand.
#[test]
fn a_grenade_held_too_long_goes_off_in_hand() {
    let Some((level, mut g)) = start() else { return };
    let p = g.player.unwrap();
    let pl = &mut g.people[p];
    pl.grenades = 1;
    pl.weapon = Weapon::Grenade;
    let health = pl.health;
    press_action(&mut g, &level);
    assert!(g.primed.is_some());
    run(&mut g, &level, 6.5, Controls::default());
    assert!(g.primed.is_none());
    assert_eq!(g.people[p].grenades, 0);
    assert_eq!(g.people[p].weapon, Weapon::None);
    assert!(g.people[p].health < health || g.people[p].is_down(), "it didn't hurt her");
}
