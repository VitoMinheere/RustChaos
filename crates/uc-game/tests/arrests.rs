//! Who Darci can arrest, as the original's `find_arrestee` has it (fallen/Source/Person.cpp):
//! someone knocked down and waiting to get up, never someone lying dead; no Men In Black, no
//! cops unless they're guilty, nobody invulnerable and none of her own bodyguards; and with
//! a choice, Roper and hostages last. Uses the people RTA starts with, brought out into the
//! street and dressed up for each rule. Skipped without an install.

use glam::Vec3;
use uc_game::person::{ko, AI_BODYGUARD};
use uc_game::{Controls, Game, Person, State, Weapon};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn run(level: &Level, g: &mut Game, secs: f32, c: Controls) {
    for _ in 0..((secs / DT) as usize).max(1) {
        let pi = g.player.unwrap();
        g.people[pi].health = g.people[pi].max_health;
        g.skip_cutscene();
        g.update(DT, &c, level);
    }
}

fn load(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    run(&level, &mut g, 1.0, Controls::default());
    Some((level, g))
}

/// Somewhere quiet in RTA's streets, on the ground.
fn street(level: &Level) -> Vec3 {
    let at = Vec3::new(30.0 * 256.0, 0.0, 30.0 * 256.0);
    Vec3::new(at.x, level.ground.surface(&level.map, at.x, at.z, 1.0e4), at.z)
}

/// People RTA has made by now besides Darci, on their feet.
fn others(g: &Game) -> Vec<usize> {
    let pi = g.player.unwrap();
    (0..g.people.len()).filter(|&i| i != pi && !g.people[i].removed && !g.people[i].is_dead() && matches!(g.people[i].state, State::Move)).collect()
}

/// Puts Darci in the street with somebody lying beside her, knocked down with `damage` after
/// `dress` has made them whoever the test wants, and has her press action. Returns whether
/// they ended up arrested.
fn try_arrest(damage: i32, dress: impl Fn(&mut Person)) -> Option<bool> {
    let (level, mut g) = load("police1")?;
    let pi = g.player.unwrap();
    let at = street(&level);
    g.people[pi].fig.pos = at;
    let other = others(&g)[0];
    g.people[other].fig.pos = at + Vec3::new(80.0, 0.0, 0.0);
    g.people[other].take_hit(at, damage, ko::KNOCKDOWN, false, false, Weapon::None);
    // Down on the floor first.
    run(&level, &mut g, 1.5, Controls::default());
    assert!(g.people[other].is_down(), "should be lying down");
    // A mission that ends over it would stop her doing anything at all, arrests included.
    assert_eq!(g.outcome, None, "the mission ended");
    dress(&mut g.people[other]);
    // Beside wherever they came to rest.
    let body = g.people[other].body_pos();
    g.people[pi].fig.pos = body + Vec3::new(60.0, 0.0, 0.0);
    run(&level, &mut g, 0.2, Controls { action: true, ..Default::default() });
    // The panel says so (`PANEL_new_info_message`, Person.cpp:11585).
    let said = g.info.as_ref().is_some_and(|(t, _)| t == "Arrested");
    run(&level, &mut g, 4.0, Controls::default());
    let arrested = g.people[other].arrested || matches!(g.people[other].state, State::Cuffed);
    assert_eq!(said, arrested, "the info line");
    Some(arrested)
}

#[test]
fn the_knocked_down_can_be_arrested() {
    let Some(arrested) = try_arrest(0, |_| {}) else { return };
    assert!(arrested, "someone knocked down should be arrested");
}

#[test]
fn the_dead_cant_be_arrested() {
    let Some(arrested) = try_arrest(10_000, |_| {}) else { return };
    assert!(!arrested, "arrested a dead man");
}

#[test]
fn cops_cant_be_arrested_unless_guilty() {
    let Some(arrested) = try_arrest(0, |p| (p.kind, p.guilty) = (ptype::COP, false)) else { return };
    assert!(!arrested, "arrested a cop");
    let Some(arrested) = try_arrest(0, |p| (p.kind, p.guilty) = (ptype::COP, true)) else { return };
    assert!(arrested, "a guilty cop should be arrested");
}

#[test]
fn men_in_black_cant_be_arrested() {
    let Some(arrested) = try_arrest(0, |p| p.kind = ptype::MIB2) else { return };
    assert!(!arrested, "arrested a Man In Black");
}

#[test]
fn the_invulnerable_cant_be_arrested() {
    let Some(arrested) = try_arrest(0, |p| p.protected = true) else { return };
    assert!(!arrested, "arrested someone invulnerable");
}

#[test]
fn her_own_bodyguards_cant_be_arrested() {
    // Guarding the player: `other` 0.
    let Some(arrested) = try_arrest(0, |p| (p.ai, p.other) = (AI_BODYGUARD, 0)) else { return };
    assert!(!arrested, "arrested her own bodyguard");
}

#[test]
fn roper_and_hostages_come_last() {
    let Some((level, mut g)) = load("police1") else { return };
    let pi = g.player.unwrap();
    let at = street(&level);
    let o = others(&g);
    let (hostage, other) = (o[0], o[1]);
    // Both down, the hostage the nearer of the two.
    for (i, dx) in [(hostage, 70.0), (other, -100.0)] {
        g.people[i].fig.pos = at + Vec3::new(dx, 0.0, 0.0);
        g.people[i].take_hit(at, 0, ko::KNOCKDOWN, false, false, Weapon::None);
    }
    run(&level, &mut g, 1.5, Controls::default());
    g.people[hostage].kind = ptype::HOSTAGE;
    // Wherever they landed, lay the bodies either side of Darci: the hostage 70 off, the
    // other 100, both within her reach.
    for (i, dx) in [(hostage, 70.0), (other, -100.0)] {
        let shift = at + Vec3::new(dx, 0.0, 0.0) - g.people[i].body_pos();
        g.people[i].fig.pos += Vec3::new(shift.x, 0.0, shift.z);
    }
    g.people[pi].fig.pos = at;
    run(&level, &mut g, 0.2, Controls { action: true, ..Default::default() });
    run(&level, &mut g, 4.0, Controls::default());
    let cuffed = |i: usize| g.people[i].arrested || matches!(g.people[i].state, State::Cuffed);
    assert!(cuffed(other), "should have gone for the other one first");
    assert!(!cuffed(hostage), "arrested the hostage first");
}

/// The flags come from the mission: bit 5 of a `create_enemies` point's zone byte makes RTA's
/// point 251 guilty, and bit 4 makes The Fallen's point 13, a cop, invulnerable.
#[test]
fn guilt_and_invulnerability_come_from_the_mission() {
    for (mission, way, guilty, invulnerable) in [("police1", 251, true, false), ("MIB", 13, false, true)] {
        let Some((_, g)) = load(mission) else { return };
        // Both points are made as the mission starts.
        let i = g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("{mission}: point {way} never made anyone"));
        assert_eq!(g.people[i].guilty, guilty, "{mission} point {way} guilty");
        assert_eq!(g.people[i].protected, invulnerable, "{mission} point {way} invulnerable");
    }
}
