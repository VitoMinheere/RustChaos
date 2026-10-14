//! The city's extra people and traffic (`fake_civs`, `fake_cars`): RTA asks for fifteen
//! civilians and seven cars. Checks they're made where they belong, walk and drive, are
//! brought back near the player once out of sight, run from gunfire and have something to
//! say. Skipped without an install.

use glam::{vec2, Vec3};
use uc_game::wand::Wander;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    Some((level, g))
}

fn civs(g: &Game) -> Vec<usize> {
    (0..g.people.len()).filter(|&i| g.people[i].fake && !matches!(g.people[i].state, State::InCar { .. })).collect()
}

fn drivers(g: &Game) -> Vec<usize> {
    (0..g.people.len()).filter(|&i| g.people[i].fake && matches!(g.people[i].state, State::InCar { .. })).collect()
}

fn run(level: &Level, g: &mut Game, secs: f32, c: Controls) {
    for _ in 0..(secs / DT) as usize {
        let pi = g.player.unwrap();
        g.people[pi].health = g.people[pi].max_health;
        g.skip_cutscene();
        g.update(DT, &c, level);
    }
}

#[test]
fn rta_has_its_crowd_and_traffic() {
    let Some((level, mut g)) = load("police1") else { return };
    let wand = Wander::new(&level.map, &level.game);
    let c = civs(&g);
    let d = drivers(&g);
    println!("{} civilians, {} cars", c.len(), d.len());
    assert_eq!(c.len(), level.mission.fake_civs as usize);
    assert_eq!(d.len(), level.mission.fake_cars as usize);
    for &i in &c {
        let p = g.people[i].fig.pos;
        assert!(wand.is_wander(p.x as i32 >> 8, p.z as i32 >> 8), "civilian {i} off the pavement at {p}");
    }
    // Everyone gets moving: the civilians stroll, the cars drive off down their roads.
    let civ_start: Vec<Vec3> = c.iter().map(|&i| g.people[i].fig.pos).collect();
    let car_start: Vec<Vec3> = g.vehicles.iter().map(|v| v.pos).collect();
    // Out of the way of anything the script wants, but not so far that everyone is moved.
    run(&level, &mut g, 8.0, Controls::default());
    let walked = c.iter().zip(&civ_start).filter(|(&i, s)| (g.people[i].fig.pos - **s).length() > 128.0).count();
    let drove = g.vehicles.iter().zip(&car_start).filter(|(v, s)| (v.pos - **s).length() > 512.0).count();
    println!("{walked} civilians walked, {drove} cars drove");
    assert!(walked * 2 >= c.len(), "only {walked} of {} civilians moved", c.len());
    assert!(drove * 2 >= d.len(), "only {drove} of {} cars moved", d.len());
    // No car has hit a wall hard enough to wreck itself.
    assert!(g.vehicles.iter().all(|v| !v.wrecked()));
}

#[test]
fn the_crowd_follows_the_player() {
    let Some((level, mut g)) = load("police1") else { return };
    // Moved somewhere new, the player soon has the crowd about them again: people out of
    // sight are brought to just out of sight ahead, a few at a time, as there's room.
    let pi = g.player.unwrap();
    let near = |g: &Game| {
        let me = g.people[pi].fig.pos;
        let civs = civs(g).iter().filter(|&&i| (g.people[i].fig.pos - me).length() < g.view_dist + 4.0 * 256.0).count();
        let cars = g.vehicles.iter().filter(|v| (v.pos - me).length() < g.view_dist + 8.0 * 256.0).count();
        (civs, cars)
    };
    let all = civs(&g).len();
    for at in [vec2(30.0, 30.0), vec2(95.0, 95.0)] {
        let y = level.ground.surface(&level.map, at.x * 256.0, at.y * 256.0, 1.0e4);
        g.people[pi].fig.pos = Vec3::new(at.x * 256.0, y, at.y * 256.0);
        run(&level, &mut g, 6.0, Controls::default());
        let (civs, cars) = near(&g);
        println!("at {at}: {civs} civilians and {cars} cars near after 6s");
        assert!(civs * 2 >= all, "only {civs} of {all} civilians came along to {at}");
        assert!(cars > 0, "no traffic at {at}");
        // Ahead of a player who stands still there are only so many pavements with room on
        // them: turning round finds more. Nobody is left behind, then: anyone further than
        // a stroll out of sight is someone placed far off at the start.
        for _ in 0..4 {
            g.people[pi].fig.angle += std::f32::consts::FRAC_PI_2;
            run(&level, &mut g, 6.0, Controls::default());
        }
        let (civs, _) = near(&g);
        let me = g.people[pi].fig.pos;
        let behind = self::civs(&g).iter().filter(|&&i| (g.people[i].fig.pos - me).length() > g.view_dist + 8.0 * 256.0).count();
        println!("at {at}: {civs} civilians near after looking round, {behind} left behind");
        assert_eq!(behind, 0, "civilians left behind at {at}");
    }
}

#[test]
fn civilians_run_from_gunfire() {
    let Some((level, mut g)) = load("police1") else { return };
    let pi = g.player.unwrap();
    // Out in the street.
    let me = Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0);
    let me = Vec3::new(me.x, level.ground.surface(&level.map, me.x, me.z, 1.0e4), me.z);
    g.people[pi].fig.pos = me;
    // One of the crowd eight squares off: too far to notice her drawing the gun, near
    // enough to hear it go off.
    let ci = civs(&g)[0];
    g.people[ci].fig.pos = me + Vec3::new(8.0 * 256.0, 0.0, 0.0);
    assert!(g.people[ci].scared.is_none());
    g.people[pi].give(Weapon::Pistol);
    g.people[pi].weapon = Weapon::Pistol;
    run(&level, &mut g, 0.2, Controls::default());
    assert!(g.people[ci].scared.is_none(), "scared by a gun drawn out of sight");
    run(&level, &mut g, 0.2, Controls { punch: true, ..Default::default() });
    assert!(g.people[ci].scared.is_some(), "a shot eight squares away didn't scare them");
    assert!(g.people[ci].annoyed);
    // They run, and further than they'd stroll.
    run(&level, &mut g, 3.0, Controls::default());
    let d = g.people[ci].fig.pos - me;
    println!("now {:.1} squares away", vec2(d.x, d.z).length() / 256.0);
    assert!(vec2(d.x, d.z).length() > 11.0 * 256.0);
}

#[test]
fn civilians_have_something_to_say() {
    let Some((level, mut g)) = load("police1") else { return };
    let lines = uc_game::city::message_lines(b"NORMAL (Innocent Citizens)\r\n\r\n1. Gotta run.\r\n2.   Some heat, huh?  \r\nnot a line\r\n");
    assert_eq!(lines, ["Gotta run.", "Some heat, huh?"]);
    let pi = g.player.unwrap();
    // Somewhere quiet, with one of the crowd beside her.
    let at = Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0);
    let at = Vec3::new(at.x, level.ground.surface(&level.map, at.x, at.z, 1.0e4), at.z);
    g.people[pi].fig.pos = at;
    let ci = civs(&g)[0];
    g.people[ci].fig.pos = at + Vec3::new(100.0, 0.0, 0.0);
    run(&level, &mut g, 1.0, Controls::default());
    g.people[pi].fig.pos = at;
    g.people[ci].fig.pos = at + Vec3::new(100.0, 0.0, 0.0);
    g.messages.clear();
    run(&level, &mut g, DT, Controls { action: true, ..Default::default() });
    // A line of RTA's own chatter.
    let file = uc_formats::resolve(&level.game, &level.mission.citsez).unwrap();
    let chatter = uc_game::city::message_lines(&std::fs::read(file).unwrap());
    let said: Vec<&str> = g.messages.iter().map(|m| m.text.as_str()).collect();
    println!("{said:?}");
    assert!(g.messages.iter().any(|m| m.face.is_some() && chatter.contains(&m.text)), "nothing said");
}
