//! Bored passers-by (`PCOM_do_regen`, fallen/Source/pcom.cpp:7066): once nothing has happened
//! for long enough, the next one moved back near Darci comes back as a thug out to get her.
//! Urban Shakedown (`wstores1`) has a boredom rate of 4 (20 seconds); 255 (RTA's) is never. Skipped
//! without an install.

use glam::vec3;
use uc_game::person::{AI_CIV, AI_GANG};
use uc_game::{Controls, Game};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start(mission: &str) -> Option<(Level, Game)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    Some((level, g))
}

fn run(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &Controls::default(), level);
    }
}

/// Runs until person `i` has been brought back near Darci (seen again), for at most thirty
/// seconds: the crowd takes its time finding them a spot.
fn brought_back(g: &mut Game, level: &Level, i: usize) {
    for _ in 0..120 {
        run(g, level, 0.25);
        if g.people[i].unseen < 1.0 {
            return;
        }
    }
    panic!("never brought back");
}

/// Sends one of the crowd far off out of sight, so the crowd brings them back near Darci.
fn send_away(g: &mut Game) -> usize {
    let p = g.player.unwrap();
    let me = g.people[p].fig.pos;
    let i = g.people.iter().position(|q| q.fake && q.ai == AI_CIV && !q.is_dead()).expect("no crowd");
    g.people[i].fig.pos = me + vec3(40.0 * 256.0, 0.0, 0.0);
    g.people[i].unseen = 10.0;
    i
}

#[test]
fn a_bored_city_sends_a_thug() {
    let Some((level, mut g)) = start("wstores1") else { return };
    g.bored = 30.0;
    let i = send_away(&mut g);
    brought_back(&mut g, &level, i);
    let q = &g.people[i];
    assert_eq!(q.kind, ptype::THUG_RASTA, "came back as a {}", q.kind);
    assert_eq!(q.ai, AI_GANG);
    assert_eq!(q.max_health, 200);
    assert!(g.bored < 1.0, "the clock didn't start again");
}

#[test]
fn not_before_the_mission_s_boredom_rate() {
    let Some((level, mut g)) = start("wstores1") else { return };
    g.bored = 10.0;
    let i = send_away(&mut g);
    brought_back(&mut g, &level, i);
    assert_eq!(g.people[i].kind, ptype::CIV, "turned after only ten seconds");
}

/// A rate of 255 is never (as RTA's is): the same mission with its rate set so.
#[test]
fn never_where_the_rate_is_255() {
    let Some(game) = uc_formats::default_install() else { return };
    let mut level = Level::load(&game, "wstores1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    level.mission.boredom_rate = 255;
    let mut g = Game::new(&level, true);
    // Past the opening talk, which holds the crowd still.
    for _ in 0..240 {
        run(&mut g, &level, 0.25);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    g.bored = 10_000.0;
    let i = send_away(&mut g);
    // The rest of the crowd out of the way, and not due back for a while, so the places near
    // her aren't all taken (there are few here).
    let p = g.player.unwrap();
    let me = g.people[p].fig.pos;
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != i && q.fake && q.ai == AI_CIV {
            q.fig.pos = me + vec3(-40.0 * 256.0, 0.0, 0.0);
            q.unseen = 0.0;
        }
    }
    brought_back(&mut g, &level, i);
    assert_eq!(g.people[i].kind, ptype::CIV, "turned with a rate of 255");
}

/// While a real thug is near her the fake one keeps clear; once there's none, it comes in.
#[test]
fn a_fake_thug_waits_for_the_real_ones() {
    let Some((level, mut g)) = start("wstores1") else { return };
    g.bored = 30.0;
    let i = send_away(&mut g);
    brought_back(&mut g, &level, i);
    assert_eq!(g.people[i].kind, ptype::THUG_RASTA);
    let p = g.player.unwrap();
    let me = g.people[p].fig.pos;
    let real = g.people.iter().position(|q| !q.fake && !q.player && !q.removed && !q.is_dead() && q.hostile())
        .expect("no real gang member in the mission");
    g.people[real].fig.pos = me + vec3(300.0, 0.0, 0.0);
    g.people[real].kind = ptype::THUG_RED;
    run(&mut g, &level, 0.3);
    assert_eq!(g.people[i].ai, AI_CIV, "piled in alongside a real one");
    g.people[real].removed = true;
    run(&mut g, &level, 0.3);
    assert_eq!(g.people[i].ai, AI_GANG, "didn't come in once alone");
}
