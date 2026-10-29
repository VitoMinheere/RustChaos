//! What things sound like knocked: a cone its own clunk, any other barrel (a bin) a barrel's
//! (`BARREL_hit_noise`, barrel.cpp:72); someone run over a squish (`S_THUMP_SQUISH`,
//! vehicle.cpp:3339). Skipped without an install.

use uc_game::person::Sfx;
use uc_game::{Controls, Game, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn police1() -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    g.take_sounds();
    Some((level, g))
}

/// Darci walks into the first standing barrel of `kind`: the sounds.
fn knock(kind: u8) -> Option<Vec<Sfx>> {
    let (level, mut g) = police1()?;
    let b = (0..g.barrels.len()).find(|&b| g.barrels[b].kind == kind && g.barrels[b].spheres.is_none()).expect("one standing");
    let at = g.barrels[b].pos;
    let me = g.player.unwrap();
    g.people[me].fig.pos = at + glam::vec3(-120.0, 0.0, 0.0);
    let mut heard = Vec::new();
    for _ in 0..45 {
        g.update(DT, &Controls { wish: glam::vec2(1.0, 0.0), ..Default::default() }, &level);
        heard.extend(g.take_sounds().into_iter().filter(|(_, p)| (*p - at).length() < 300.0).map(|(s, _)| s));
    }
    Some(heard)
}

#[test]
fn a_cone_clunks_and_a_bin_doesnt() {
    let Some(cone) = knock(1) else { return };
    let bin = knock(2).unwrap();
    assert!(cone.contains(&Sfx::Cone) && !cone.contains(&Sfx::Barrel), "cone: {cone:?}");
    assert!(bin.contains(&Sfx::Barrel) && !bin.contains(&Sfx::Cone), "bin: {bin:?}");
}

#[test]
fn someone_run_over_squishes() {
    let Some((level, mut g)) = police1() else { return };
    let vi = (0..g.vehicles.len()).find(|&v| g.vehicles[v].driver.is_none()).expect("a parked car");
    let v = &g.vehicles[vi];
    let ahead = v.pos + glam::vec3(v.forward().x, 0.0, v.forward().y) * 300.0;
    let me = g.player.unwrap();
    let victim = (0..g.people.len()).find(|&i| i != me && !g.people[i].removed && matches!(g.people[i].state, State::Move)).expect("someone");
    g.people[victim].fig.pos = ahead;
    let mut squished = false;
    for _ in 0..30 {
        g.vehicles[vi].speed = 800.0;
        g.update(DT, &Controls::default(), &level);
        squished |= g.take_sounds().iter().any(|(s, _)| *s == Sfx::Squish);
    }
    assert!(squished, "no squish; victim {:?}", g.people[victim].state);
}

/// A fly-by that holds the player opens with a sting (`S_CUTSCENE_STING`, eway.cpp:5389):
/// police1's first shot does.
#[test]
fn a_held_fly_by_opens_with_a_sting() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    let mut stung = 0;
    for _ in 0..10 {
        g.update(DT, &Controls::default(), &level);
        stung += g.take_sounds().iter().filter(|(s, _)| *s == Sfx::Sting).count();
    }
    assert_eq!(stung, 1);
}
