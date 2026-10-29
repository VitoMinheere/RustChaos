//! The win screen's criminals (`ScoresDraw`, Attract.cpp:684): the gangs and the Men in Black
//! the player killed and arrested, and those still standing. Skipped without an install.

use uc_game::person::ko;
use uc_game::{Controls, Game, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn killed_arrested_and_at_large() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "fight1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let thugs: Vec<usize> = (0..g.people.len()).filter(|&i| !g.people[i].removed && !g.people[i].player && g.people[i].thug()).collect();
    assert!(thugs.len() >= 2, "{} gang members", thugs.len());
    let (killed, arrested, left) = g.criminals().expect("Darci");
    assert_eq!((killed, arrested, left as usize), (0, 0, thugs.len()));
    // One put down for good by Darci, one by someone else: only hers counts as killed.
    let me = g.player.unwrap();
    let (a, b) = (thugs[0], thugs[1]);
    for (i, by) in [(a, Some(me)), (b, None)] {
        let from = g.people[i].fig.pos + glam::vec3(100.0, 0.0, 0.0);
        g.people[i].hurt_by = by;
        g.people[i].take_hit(from, 10_000, ko::KNOCKDOWN, false, false, Weapon::None);
    }
    let (killed, _, left) = g.criminals().unwrap();
    assert_eq!((killed, left as usize), (1, thugs.len() - 2));
}
