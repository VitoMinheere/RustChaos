//! Mines (`SPECIAL_MINE`), on Cop Killers' mined streets: underfoot, shot, chained by a blast,
//! and under a car's wheel. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::{Controls, Game, Mine, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start() -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "AWOL1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    // Past the opening, and the talk with Roper that holds her still.
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
        g.update(DT, &c, level);
    }
}

/// A mine on open ground with nobody else near it.
fn lone_mine(g: &Game) -> usize {
    g.mines.iter().enumerate()
        .find(|(k, m)| {
            g.mines.iter().enumerate().all(|(j, o)| j == *k || (o.pos - m.pos).length() > 600.0)
                && g.people.iter().all(|p| p.player || (p.fig.pos - m.pos).length() > 600.0)
        })
        .map(|(k, _)| k)
        .expect("no mine on its own")
}

#[test]
fn cop_killers_is_mined() {
    let Some((_, g)) = start() else { return };
    assert_eq!(g.mines.len(), 84);
}

/// Walking onto one: it goes off, and the blast knocks her down.
#[test]
fn a_mine_goes_off_underfoot() {
    let Some((level, mut g)) = start() else { return };
    let k = lone_mine(&g);
    let at = g.mines[k].pos;
    let p = g.player.unwrap();
    g.people[p].fig.pos = at + vec3(150.0, 0.0, 0.0);
    run(&mut g, &level, 0.5, Controls::default());
    let health = g.people[p].health;
    for _ in 0..90 {
        if g.mines[k].gone {
            break;
        }
        run(&mut g, &level, DT, Controls { wish: vec2(-1.0, 0.0), ..Default::default() });
    }
    assert!(g.mines[k].gone, "walked over it");
    run(&mut g, &level, 0.2, Controls::default());
    assert!(g.people[p].health < health || g.people[p].is_down(), "the blast missed her");
}

/// Shot from a distance: the gun picks it as a target, and a hit sets it off.
#[test]
fn a_mine_can_be_shot() {
    let Some((level, mut g)) = start() else { return };
    let k = lone_mine(&g);
    let at = g.mines[k].pos;
    let p = g.player.unwrap();
    let spot = (0..16)
        .map(|i| at + vec3((i as f32 * 0.39).sin(), 0.0, (i as f32 * 0.39).cos()) * 500.0)
        .find(|s| (level.ground.surface(&level.map, s.x, s.z, at.y + 64.0) - at.y).abs() < 16.0
            && level.ground.line_of_sight(*s + vec3(0.0, 140.0, 0.0), at + vec3(0.0, 140.0, 0.0)))
        .expect("nowhere to shoot from");
    let me = &mut g.people[p];
    me.fig.pos = spot;
    me.fig.angle = uc_world::character::angle_of(vec2(at.x - spot.x, at.z - spot.z));
    me.give(Weapon::Pistol);
    me.weapon = Weapon::Pistol;
    run(&mut g, &level, 0.5, Controls::default());
    assert_eq!(g.aiming.mine, Some(k), "the gun isn't on it");
    for _ in 0..10 {
        if g.mines[k].gone {
            break;
        }
        run(&mut g, &level, DT, Controls { punch: true, ..Default::default() });
        run(&mut g, &level, 0.5, Controls::default());
    }
    assert!(g.mines[k].gone, "shots didn't set it off");
}

/// One mine going off lights the fuse of another close by, which follows a moment later.
#[test]
fn mines_set_each_other_off() {
    let Some((level, mut g)) = start() else { return };
    let k = lone_mine(&g);
    let at = g.mines[k].pos;
    g.mines.push(Mine { way: 0, pos: at + vec3(200.0, 0.0, 0.0), fuse: None, gone: false });
    let other = g.mines.len() - 1;
    g.set_off_mine(k, &level);
    assert!(g.mines[other].fuse.is_some(), "the blast didn't light it");
    assert!(!g.mines[other].gone, "it went off at once");
    run(&mut g, &level, 1.0, Controls::default());
    assert!(g.mines[other].gone, "it never went off");
}

/// A car rolled onto one: a wheel sets it off.
#[test]
fn a_wheel_sets_a_mine_off() {
    let Some((level, mut g)) = start() else { return };
    let k = lone_mine(&g);
    let at: Vec3 = g.mines[k].pos;
    let v = g.vehicles.iter().position(|v| v.driver.is_none() && !v.wrecked()).expect("no car");
    // The car's front left wheel right over the mine.
    let (_, _, wx, _, rz) = uc_game::vehicle::TYPES[g.vehicles[v].kind as usize];
    let car = &mut g.vehicles[v];
    car.pos = at;
    let wheel = car.place(vec3(wx, 0.0, -rz));
    car.pos = at - (wheel - at);
    run(&mut g, &level, 0.2, Controls::default());
    assert!(g.mines[k].gone, "the wheel went over it");
}

/// Each mine's red light (`DRAWXTRA_Special`): a glow just above it that comes and goes as it
/// turns towards the camera and away.
#[test]
fn a_mine_flashes() {
    let Some((level, mut g)) = start() else { return };
    let k = lone_mine(&g);
    let at = g.mines[k].pos;
    let eye = at + vec3(600.0, 300.0, 0.0);
    let (mut lit, mut dark) = (0, 0);
    for _ in 0..60 {
        run(&mut g, &level, DT, Controls::default());
        let glow = g.batches(&level, eye).get(&uc_world::GLOW).is_some_and(|b| {
            b.vertices.iter().any(|v| (Vec3::from(v.pos) * uc_world::UNIT - (at + vec3(0.0, 15.0, 0.0))).length() < 60.0)
        });
        if glow { lit += 1 } else { dark += 1 }
    }
    assert!(lit > 0 && dark > 0, "lit {lit} frames, dark {dark}: no flashing");
}
