//! The Men in Black (`PersonIsMIB`, fallen/Source/Person.cpp): they walk, jump clear of
//! blows, and a while after going down they go up in lightning. In RTA, with one of the crowd
//! made one. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::character::{anim, person as ptype};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start(dist: f32) -> Option<(Level, Game, usize, usize)> {
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
    let i = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && q.kind != ptype::COP).expect("nobody");
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != p && j != i && (q.hostile() || q.kind == ptype::COP) {
            q.removed = true;
        }
    }
    let m = &mut g.people[i];
    m.kind = ptype::MIB1;
    m.fake = false;
    m.mv = 1;
    m.fig.pos = vec3(at.x, y, at.z + dist);
    m.fig.angle = uc_world::character::angle_of(vec2(0.0, -1.0));
    m.state = State::Move;
    // And this is where he stands now, so he doesn't walk back to where the mission made him.
    m.home = Some((m.fig.pos, m.fig.angle));
    Some((level, g, p, i))
}

fn run(g: &mut Game, level: &Level, secs: f32, c: &Controls) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, c, level);
    }
}

#[test]
fn going_up_in_lightning() {
    let Some((level, mut g, p, i)) = start(150.0) else { return };
    let before = g.people[p].health;
    g.people[p].invulnerable = false;
    let from = g.people[p].fig.pos;
    g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::Pistol);
    assert!(g.people[i].knocked_out);
    run(&mut g, &level, 5.0, &Controls::default());
    assert!(!g.people[i].removed, "went up too soon");
    // Her health is topped up each frame by `run`: step by hand to see the blast land.
    let mut hurt = false;
    for _ in 0..(3.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        if g.people[i].removed {
            hurt = g.people[p].health < before || g.people[p].is_down();
            break;
        }
    }
    assert!(g.people[i].removed, "never went up");
    assert!(hurt, "the blast didn't reach her 150 off");
}

#[test]
fn blows_dont_land() {
    let Some((level, mut g, _, i)) = start(90.0) else { return };
    g.people[i].max_health = 300;
    g.people[i].health = 300;
    let mut jumped = false;
    for _ in 0..(2.0 / DT) as usize {
        run(&mut g, &level, DT, &Controls { punch: true, ..Default::default() });
        jumped |= matches!(g.people[i].state, State::Fall { .. });
    }
    assert_eq!(g.people[i].health, 300, "a blow landed");
    assert!(jumped, "never jumped clear");
}

#[test]
fn never_running() {
    let Some((level, mut g, _, i)) = start(1500.0) else { return };
    g.people[i].ai = 17;
    for _ in 0..(3.0 / DT) as usize {
        run(&mut g, &level, DT, &Controls::default());
        assert_ne!(g.people[i].fig.anim(), anim::RUN, "running");
    }
    assert_eq!(g.people[i].weapon, Weapon::Ak47, "no AK");
}
