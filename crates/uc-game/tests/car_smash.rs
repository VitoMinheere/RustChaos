//! Cars running into props (`DoDamage`, fallen/Source/vehicle.cpp:2094): fast enough, the
//! prop is damaged as a blast would damage it; slower, it isn't. Skipped without an install.

use glam::{vec3, Vec2, Vec3};
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start() -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    Some((level, g))
}

/// A breakable prop on open ground, with a straight run-up to it: (prop, where the car starts,
/// which way it faces).
fn run_up(level: &Level) -> (usize, Vec3, Vec2) {
    level.map.objects.iter().enumerate()
        .filter(|&(i, _)| level.ground.prop_damage.get(i).is_some_and(|d| d.0 & 1 != 0))
        .find_map(|(i, o)| {
            let at = vec3(o.x as f32, o.y as f32, o.z as f32);
            let ground = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
            (0..8).map(|k| Vec2::from_angle(k as f32 * std::f32::consts::FRAC_PI_4)).find_map(|d| {
                // Flat road for the car to come along, and nothing in the way but the prop.
                let from = at - vec3(d.x, 0.0, d.y) * 600.0;
                let flat = (0..10).all(|s| {
                    let p = from + vec3(d.x, 0.0, d.y) * (s as f32 * 50.0);
                    (level.ground.surface(&level.map, p.x, p.z, ground + 64.0) - ground).abs() < 8.0
                });
                let clear = level.ground.first_hit(from + vec3(0.0, 40.0, 0.0), at + vec3(0.0, 40.0, 0.0))
                    .is_none_or(|t| t > 0.7);
                (flat && clear).then(|| (i, vec3(from.x, ground, from.z), d))
            })
        })
        .expect("no breakable prop with a run-up")
}

fn ram(speed: f32) -> Option<(Game, usize)> {
    let (level, mut g) = start()?;
    let (prop, from, dir) = run_up(&level);
    let v = g.vehicles.iter().position(|v| v.driver.is_none()).expect("no car");
    let car = &mut g.vehicles[v];
    car.pos = from;
    car.yaw = dir.x.atan2(dir.y);
    for _ in 0..(1.5 / DT) as usize {
        // Kept at the speed it was going until it hits something.
        if g.vehicles[v].speed > 0.0 || g.time < 0.1 {
            g.vehicles[v].speed = speed;
        }
        g.update(DT, &Controls::default(), &level);
        if g.vehicles[v].speed <= 0.0 {
            break;
        }
    }
    Some((g, prop))
}

#[test]
fn a_fast_car_smashes_the_prop_it_hits() {
    let Some((g, prop)) = ram(2500.0) else { return };
    assert!(g.damaged.contains(&prop), "prop {prop} came through unharmed");
}

#[test]
fn a_slow_car_only_bumps_it() {
    let Some((g, prop)) = ram(600.0) else { return };
    assert!(!g.damaged.contains(&prop), "a bump broke prop {prop}");
}
