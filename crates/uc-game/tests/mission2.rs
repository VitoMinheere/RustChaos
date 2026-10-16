//! Southside Offensive (`mission2`): won once the crime rate is down to 10%, lost if it
//! reaches 99%. It starts at the mission's 80%, killing a guilty person takes 2 off and
//! arresting one 4, and killing a passer-by puts 5 on. The fighting is done by cheating.
//! Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Controls, Outcome, State, Weapon};

mod common;
use common::Run;

fn start() -> Option<Run> {
    common::start("mission2")
}

impl Run {
    /// Someone the script made (not the crowd) who is guilty and still standing.
    fn guilty(&self) -> Option<usize> {
        self.g.people.iter().position(|p| !p.player && !p.fake && p.way != 0 && !p.is_dead() && !p.is_down() && p.is_guilty())
    }

    fn kill(&mut self, i: usize) {
        let from = self.g.people[i].fig.pos + vec3(50.0, 0.0, 0.0);
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }
}

#[test]
fn southside_offensive_starts_at_the_missions_crime_rate() {
    let Some(mut r) = start() else { return };
    assert_eq!(r.g.crime_rate, 80);
    assert!(r.g.show_crime_rate, "the mission asks for the rate on the HUD");
    r.play(5.0, Controls::default());
    assert_eq!(r.g.crime_rate, 80, "nothing has happened yet");
    assert_eq!(r.g.outcome, None);
}

/// Every guilty person killed takes 2 off, and the mission is won the moment the rate
/// reaches 10%, not before.
#[test]
fn southside_offensive_is_won_by_bringing_the_crime_rate_down() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    let mut killed = 0;
    while r.g.outcome.is_none() {
        let i = r.guilty().unwrap_or_else(|| panic!("ran out of guilty people at {}%", r.g.crime_rate));
        let before = r.g.crime_rate;
        r.kill(i);
        killed += 1;
        r.play(0.5, Controls::default());
        assert_eq!(r.g.crime_rate, before - 2, "kill {killed} didn't take 2 off");
        if r.g.crime_rate > 10 {
            assert_eq!(r.g.outcome, None, "won early at {}%", r.g.crime_rate);
        } else {
            r.play(1.0, Controls::default());
        }
    }
    println!("won at {}% after {killed} kills", r.g.crime_rate);
    assert_eq!(killed, 35);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// An arrest of someone guilty takes 4 off.
#[test]
fn arresting_the_guilty_brings_it_down_faster() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    let i = r.guilty().unwrap();
    let at = r.g.people[i].fig.pos;
    r.g.people[i].take_hit(at + vec3(50.0, 0.0, 0.0), 0, ko::KNOCKDOWN, false, false, Weapon::None);
    r.play(0.5, Controls::default());
    let body = r.g.people[i].body_pos();
    let beside = (0..8)
        .map(|k| body + vec3((k as f32 * 0.785).sin(), 0.0, (k as f32 * 0.785).cos()) * 60.0)
        .find(|p| (r.level.ground.surface(&r.level.map, p.x, p.z, body.y + 64.0) - body.y).abs() < 16.0)
        .expect("nowhere to stand beside him");
    let p = r.g.player.unwrap();
    r.g.people[p].fig.pos = beside;
    r.play(0.2, Controls { action: true, ..Default::default() });
    r.play(4.0, Controls::default());
    assert!(r.g.people[i].arrested || matches!(r.g.people[i].state, State::Cuffed), "no arrest: {:?}", r.g.people[i].state);
    assert_eq!(r.g.crime_rate, 76);
}

/// Killing passers-by puts 5 on each: four of them take it from 80% to the 99% that loses.
#[test]
fn killing_passers_by_loses_it() {
    let Some(mut r) = start() else { return };
    r.play(1.0, Controls::default());
    for k in 1..=4 {
        let i = r.g.people.iter().position(|p| p.fake && p.ai == uc_game::person::AI_CIV && !p.is_dead() && !p.is_down())
            .expect("no passer-by left");
        r.kill(i);
        r.play(0.5, Controls::default());
        assert_eq!(r.g.crime_rate, (80 + 5 * k).min(100) as u8);
    }
    r.play(1.0, Controls::default());
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}
