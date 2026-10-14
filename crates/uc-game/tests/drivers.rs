//! Drivers: a cop driver (`PCOM_AI_COP_DRIVER`) finds a car and drives as any driver does, and
//! a driver frightened by gunfire pulls up, gets out and runs. In Southside Offensive, and in
//! RTA's traffic.
//! Skipped without an install.

use glam::{vec2, vec3};
use uc_game::{Controls, Game, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
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
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &c, level);
    }
}

/// Point 89 is a cop driver: into a car, and off along the road.
#[test]
fn a_cop_driver_takes_a_car_and_drives() {
    let Some((level, mut g)) = start("mission2") else { return };
    run(&mut g, &level, 10.0, Controls::default());
    let cop = g.people.iter().position(|p| p.way == 89).expect("no cop driver");
    assert_eq!(g.people[cop].ai, uc_game::person::AI_COP_DRIVER);
    let State::InCar { vehicle } = g.people[cop].state else { panic!("still on foot: {:?}", g.people[cop].state) };
    let from = g.vehicles[vehicle].pos;
    run(&mut g, &level, 5.0, Controls::default());
    assert!((g.vehicles[vehicle].pos - from).length() > 256.0, "the car hasn't gone anywhere");
}

/// One of RTA's traffic drivers, parked beside Darci when she opens fire: out and away.
#[test]
fn a_frightened_driver_gets_out_and_runs() {
    let Some((level, mut g)) = start("police1") else { return };
    let p = g.player.unwrap();
    let me = g.people[p].fig.pos;
    let v = g.vehicles.iter().position(|v| v.driver.is_some_and(|d| g.people[d].fake && g.people[d].ai == uc_game::person::AI_DRIVER))
        .expect("no traffic");
    let driver = g.vehicles[v].driver.unwrap();
    let y = level.ground.surface(&level.map, me.x + 400.0, me.z, me.y + 64.0);
    g.vehicles[v].reset(vec3(me.x + 400.0, y, me.z), 0.0);
    let pl = &mut g.people[p];
    pl.give(Weapon::Pistol);
    pl.weapon = Weapon::Pistol;
    pl.fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
    run(&mut g, &level, DT, Controls { punch: true, ..Default::default() });
    let mut out = false;
    for _ in 0..90 {
        run(&mut g, &level, DT, Controls::default());
        if !matches!(g.people[driver].state, State::InCar { .. }) {
            out = true;
            break;
        }
    }
    assert!(out, "stayed at the wheel");
    assert!(g.vehicles[v].driver.is_none());
    assert!(g.people[driver].scared.is_some(), "got out but isn't running");
}
