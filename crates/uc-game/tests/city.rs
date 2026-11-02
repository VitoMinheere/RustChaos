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

/// The recorded chatter lines are found by their text, and only they: in `citsez.txt` they're
/// lines 1, 3, 6, 10, 13 and 20 (#486). Skipped without an install.
#[test]
fn the_recorded_chatter_lines_are_known() {
    let Some(game) = uc_formats::default_install() else { return };
    let bytes = std::fs::read(uc_formats::resolve(&game, r"text\citsez.txt").unwrap()).unwrap();
    let lines = uc_game::city::message_lines(&bytes);
    let voiced: Vec<(usize, u8)> = lines.iter().enumerate().filter_map(|(i, l)| Some((i + 1, uc_game::city::voiced_line(l)?))).collect();
    assert_eq!(voiced, [(1, 1), (3, 3), (6, 6), (10, 10), (13, 13), (20, 20)]);
    assert_eq!(uc_game::city::voiced_line("sorry, officer, i'm in a HURRY"), Some(1));
}

/// Talking to the crowd until one says a recorded line: it comes with its cue, a woman's
/// take or a man's as they're drawn, stressed with Darci's gun out. Semtex's crowd has
/// `citsez.txt`'s chatter. Skipped without an install.
#[test]
fn a_recorded_line_is_heard() {
    let Some((level, mut g)) = load("semtex") else { return };
    let pi = g.player.unwrap();
    let at = Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0);
    let at = Vec3::new(at.x, level.ground.surface(&level.map, at.x, at.z, 1.0e4), at.z);
    let Some(&ci) = civs(&g).first() else { panic!("semtex has no crowd") };
    run(&level, &mut g, 1.0, Controls::default());
    for _ in 0..200 {
        g.people[pi].fig.pos = at;
        g.people[ci].fig.pos = at + Vec3::new(100.0, 0.0, 0.0);
        g.people[ci].scared = None;
        g.people[ci].annoyed = false;
        g.messages.clear();
        g.take_sounds();
        run(&level, &mut g, DT, Controls { action: true, ..Default::default() });
        let said: Vec<String> = g.messages.iter().map(|m| m.text.clone()).collect();
        let cues: Vec<uc_game::Sfx> = g.take_sounds().into_iter().map(|(s, _)| s).collect();
        run(&level, &mut g, DT, Controls::default());
        let Some(line) = said.iter().find_map(|t| uc_game::city::voiced_line(t)) else { continue };
        let o = &g.people[ci];
        let female = matches!(o.kind, uc_world::character::person::SLAG_TART | uc_world::character::person::SLAG_FATUGLY)
            || (o.kind == uc_world::character::person::CIV && o.fig.model.body == 9);
        let stressed = g.people[pi].weapon.is_gun();
        println!("line {line}, female {female}, stressed {stressed}: {cues:?}");
        assert!(cues.contains(&uc_game::Sfx::CitizenLine { line, female, stressed }), "said line {line} without its cue: {cues:?}");
        return;
    }
    panic!("no recorded line in 200 tries");
}

/// `citsez.txt`'s annoyed section is kept for the annoyed: 21 innocent lines (the last is
/// ":)") and 20 guilty ones for anyone, 19 annoyed ones, all of them `annoyed.txt`'s too.
/// Skipped without an install.
#[test]
fn the_annoyed_lines_are_kept_apart() {
    let (normal, annoyed) = uc_game::city::chatter_sections(b"NORMAL (Innocent)\r\n1. Hi.\r\n\r\nNORMAL (annoyed)\r\n1. Go away.\r\nGUILTY\r\n1. Back off.\r\n");
    assert_eq!((normal, annoyed), (vec!["Hi.".to_string(), "Back off.".to_string()], vec!["Go away.".to_string()]));
    let Some((level, _)) = load("semtex") else { return };
    let bytes = std::fs::read(uc_formats::resolve(&level.game, r"text\citsez.txt").unwrap()).unwrap();
    let (normal, annoyed) = uc_game::city::chatter_sections(&bytes);
    assert_eq!((normal.len(), annoyed.len()), (41, 19));
    assert!(!normal.iter().any(|l| l.contains("SERPICO")), "an annoyed line is ordinary chatter");
    let chatter = uc_game::city::Chatter::load(&level);
    assert_eq!((chatter.annoyed.len(), chatter.annoyed_file), (20, 20), "the section's lines are all annoyed.txt's");
}

/// A frightened citizen answers with an annoyed line and its cue, numbered as in
/// `annoyed.txt`. Skipped without an install.
#[test]
fn a_frightened_citizen_says_an_annoyed_line() {
    let Some((level, mut g)) = load("semtex") else { return };
    let pi = g.player.unwrap();
    let at = Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0);
    let at = Vec3::new(at.x, level.ground.surface(&level.map, at.x, at.z, 1.0e4), at.z);
    let ci = civs(&g)[0];
    run(&level, &mut g, 1.0, Controls::default());
    g.people[pi].fig.pos = at;
    g.people[ci].fig.pos = at + Vec3::new(100.0, 0.0, 0.0);
    g.people[ci].scared = None;
    g.people[ci].annoyed = true;
    g.messages.clear();
    g.take_sounds();
    run(&level, &mut g, DT, Controls { action: true, ..Default::default() });
    let chatter = uc_game::city::Chatter::load(&level);
    let said = g.messages.iter().find_map(|m| chatter.annoyed.iter().position(|l| *l == m.text)).expect("no annoyed line said");
    let cues: Vec<uc_game::Sfx> = g.take_sounds().into_iter().map(|(s, _)| s).collect();
    assert!(cues.iter().any(|c| matches!(c, uc_game::Sfx::AnnoyedLine { line, .. } if *line as usize == said + 1)), "{cues:?}");
}

/// A mission naming a chatter file that didn't ship gets the older version that did: the
/// last digit is the version. Factory1 asks for `factory13.txt`; `factory12.txt` is there.
#[test]
fn an_older_chatter_file_stands_in() {
    let have = ["text\\factory12.txt", "text\\police11.txt"];
    let exists = |n: &str| have.iter().any(|h| h.eq_ignore_ascii_case(n));
    assert_eq!(uc_game::city::chatter_file("text\\factory13.txt", exists).as_deref(), Some("text\\factory12.txt"));
    assert_eq!(uc_game::city::chatter_file("Text\\police11.txt", exists).as_deref(), Some("Text\\police11.txt"));
    assert_eq!(uc_game::city::chatter_file("text\\Gordout11.txt", exists), None);
    assert_eq!(uc_game::city::chatter_file("", exists), None);
    let Some(game) = uc_formats::default_install() else { return };
    let on_disk = |n: &str| uc_formats::resolve(&game, n).is_some();
    for (asked, got) in [("text\\factory13.txt", "text\\factory12.txt"), ("text\\Gangorder12.txt", "text\\Gangorder11.txt"), ("text\\E32.txt", "text\\E31.txt")] {
        assert_eq!(uc_game::city::chatter_file(asked, on_disk).as_deref(), Some(got));
    }
    // Factory1's crowd says factory12.txt's lines, not citsez.txt's.
    let Some((level, _)) = load("factory1") else { return };
    let chatter = uc_game::city::Chatter::load(&level);
    let own = uc_game::city::chatter_sections(&std::fs::read(uc_formats::resolve(&game, "text\\factory12.txt").unwrap()).unwrap()).0;
    assert_eq!(chatter.normal, own);
}
