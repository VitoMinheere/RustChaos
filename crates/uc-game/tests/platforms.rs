//! Moving platforms (`link_platform`): Grim Gardens' point 15 sets its prop going between
//! it and point 16, ten storeys up, from the start. Skipped without an install.

use glam::vec3;
use uc_game::plat::State;
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn grim_gardens_platform_moves_and_carries() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "botanicc", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    assert!(!level.platforms.is_empty(), "the mission has platforms");
    let mut g = Game::new(&level, true);
    let k = g.platforms.iter().position(|p| p.way == 15).expect("point 15's platform");
    let start = g.platforms[k].pos;
    g.update(DT, &Controls::default(), &level);
    assert_ne!(g.platforms[k].state, State::Waiting, "point 15 has no trigger");

    // Stand on it.
    let top = level.ground.surface(&level.map, start.x, start.z, start.y + 2000.0);
    assert!(top > start.y, "its top should be walkable ({top} over {})", start.y);
    let me = g.player.unwrap();
    g.people[me].fig.pos = vec3(start.x, top, start.z);
    let mut moved = 0.0f32;
    for _ in 0..(20.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        moved = moved.max((g.platforms[k].pos - start).length());
        if moved > 256.0 {
            break;
        }
    }
    println!("state {:?}, moved {moved:.0}", g.platforms[k].state);
    assert!(moved > 256.0, "it should have gone somewhere");
    let p = g.platforms[k].pos;
    let d = g.people[me].fig.pos - vec3(p.x, g.people[me].fig.pos.y, p.z);
    assert!(d.length() < 160.0, "she should have ridden along: {:.0} off", d.length());
    let risen = g.people[me].fig.pos.y - top;
    assert!((risen - (p.y - start.y)).abs() < 40.0, "and up with it: she rose {risen:.0}, it {:.0}", p.y - start.y);
}
