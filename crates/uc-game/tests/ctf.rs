//! Capture the flag against a bot (#133): `Capflag1`, with Roper as the bot. Reaching the
//! other's disk and getting home wins, for either seat; the bot's win is Darci's loss.
//! Skipped without an install.

use glam::Vec3;
use uc_game::{Controls, Outcome};

mod common;

#[test]
fn capflag_loads_with_roper_as_the_bot() {
    let Some(mut r) = common::start("Capflag1") else { return };
    r.play(1.0, Controls::default());
    let c = r.g.ctf().expect("not a game of capture the flag");
    let bot = &r.g.people[c.bot_person];
    assert!(!bot.player && bot.kind == uc_world::character::person::ROPER);
    assert_eq!(r.g.player().unwrap().kind, uc_world::character::person::DARCI);
    // Each seat's disk is at the other's end of the map.
    assert!(c.player.disk.distance(c.bot.start) < c.player.disk.distance(c.player.start));
    assert!(c.bot.disk.distance(c.player.start) < c.bot.disk.distance(c.bot.start));
    assert!(r.g.beacons().iter().any(|(l, _)| l == "The disk"));
}

/// Darci to Roper's disk and back home: the script says so and she wins.
#[test]
fn darci_takes_the_disk_home_and_wins() {
    let Some(mut r) = common::start("Capflag1") else { return };
    r.play(1.0, Controls::default());
    r.g.cheats.freeze = true;
    let c = r.g.ctf().unwrap().clone();
    assert!(r.g.teleport(c.player.disk, &r.level));
    r.play(2.0, Controls::default());
    assert!(r.g.ctf().unwrap().player.has_disk);
    assert!(r.g.teleport(c.player.home, &r.level));
    r.play(15.0, Controls::default());
    assert_eq!(r.g.outcome, Some(Outcome::Won), "messages: {:?}", r.seen);
}

/// Left alone (Darci out of the way), Roper goes for her disk, takes it home, and she loses.
#[test]
fn the_bot_takes_her_disk_home() {
    let Some(mut r) = common::start("Capflag1") else { return };
    r.play(1.0, Controls::default());
    let c = r.g.ctf().unwrap().clone();
    // Darci out of the way, halfway along the map's edge.
    let aside = Vec3::new(c.player.start.x, c.player.start.y, c.bot.start.z);
    assert!(r.g.teleport(aside, &r.level));
    for _ in 0..240 {
        r.play(1.0, Controls::default());
        if r.g.outcome.is_some() {
            break;
        }
    }
    let bot = r.g.ctf().unwrap();
    assert!(bot.bot.has_disk, "Roper never reached her disk (he's at {:?})", r.g.people[bot.bot_person].fig.pos);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "outcome {:?}", r.g.outcome);
}

/// Knocked out, Roper comes back at his start a few seconds later, whole.
#[test]
fn a_knocked_out_player_comes_back_at_home() {
    let Some(mut r) = common::start("Capflag1") else { return };
    r.play(1.0, Controls::default());
    let c = r.g.ctf().unwrap().clone();
    let b = c.bot_person;
    r.play(5.0, Controls::default());
    let from = r.g.people[b].fig.pos + glam::vec3(50.0, 0.0, 0.0);
    r.g.people[b].take_hit(from, 100_000, uc_game::person::ko::FINISHERS, false, false, uc_game::Weapon::None);
    assert!(r.g.people[b].knocked_out);
    r.play(6.0, Controls::default());
    let p = &r.g.people[b];
    assert!(!p.knocked_out && p.health == p.max_health, "still down");
    assert!(glam::vec2(p.fig.pos.x - c.bot.start.x, p.fig.pos.z - c.bot.start.z).length() < 4.0 * 256.0, "not back at home");
    assert_eq!(r.g.outcome, None);
}
