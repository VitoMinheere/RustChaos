//! The panel's info line (`PANEL_new_info_message`): what the player just picked up, for two
//! seconds; a gun she has already is "Ammo". Skipped without an install.

use uc_game::{Controls, Game, Item, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// Puts item `kind` at the player's feet and lets her pick it up: what the info line says.
fn pick_up(g: &mut Game, level: &Level, kind: u8) -> Option<String> {
    let pos = g.player().unwrap().fig.pos;
    g.items.push(Item { way: 0, kind, pos, rounds: None });
    for _ in 0..5 {
        g.update(DT, &Controls::default(), level);
    }
    g.info.as_ref().map(|(t, _)| t.clone())
}

#[test]
fn it_says_what_she_picked_up() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "fight2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let me = g.player.unwrap();
    g.people[me].health = 50;
    assert_eq!(pick_up(&mut g, &level, 3).as_deref(), Some("Health +50"));
    assert!(!g.people[me].has(Weapon::Shotgun));
    let shotgun = Weapon::Shotgun.item().unwrap();
    assert_eq!(pick_up(&mut g, &level, shotgun).as_deref(), Some("Shotgun"));
    assert_eq!(pick_up(&mut g, &level, shotgun).as_deref(), Some("Ammo"), "a second shotgun is ammo");
    assert_eq!(pick_up(&mut g, &level, 11).as_deref(), Some("Ammo"));
    // Gone after two seconds.
    for _ in 0..(2.0 / DT) as usize + 1 {
        g.update(DT, &Controls::default(), &level);
    }
    assert!(g.info.is_none());
}
