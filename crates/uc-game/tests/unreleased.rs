//! The two finished missions the game never listed: Construction Site Siege (`FreeCD1`, the
//! demo's mission) and El Cossa Island (`jung3`). Neither has a texture clump of its own, so
//! these also load through the borrowed one. The fighting is done by cheating: everyone who
//! isn't a civilian is knocked out a moment after they appear. Skipped without an install.

use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, Weapon};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// `PCOM_AI_CIVILIAN`.
const AI_CIVILIAN: u8 = 1;

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

fn start(mission: &str) -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "{mission}: unsupported {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {
    /// Plays for `secs`, knocking out every fighter once a second if `fight`.
    fn play(&mut self, secs: f32, fight: bool) {
        for frame in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            if fight && frame % 30 == 0 {
                let p = self.g.player.unwrap();
                let from = self.g.people[p].fig.pos;
                self.g.people[p].health = self.g.people[p].max_health;
                for e in self.g.people.iter_mut() {
                    if !e.player && !e.removed && e.ai != AI_CIVILIAN && !e.is_dead() && !e.is_down() {
                        e.take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
                    }
                }
            }
            self.g.update(DT, &Controls::default(), &self.level);
            for m in &self.g.messages {
                if !self.seen.contains(&m.text) {
                    println!("[{:6.1}s] {}", self.g.time, m.text);
                    self.seen.push(m.text.clone());
                }
            }
            if self.g.outcome.is_some() {
                return;
            }
        }
    }

    fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
    }

    /// Kills the first civilian the script made at point `way`.
    fn kill(&mut self, way: usize) {
        let i = self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}"));
        let from = self.g.people[i].fig.pos;
        self.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    }
}

/// Every Wildcat down inside the four minutes, the hostages unhurt.
#[test]
fn construction_site_siege_runs_to_a_win() {
    let Some(mut r) = start("FreeCD1") else { return };
    r.play(3.0, false);
    assert!(r.said("FOUR minutes"), "no briefing: {:?}", r.seen);
    r.play(230.0, true);
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// A hostage worker down loses it.
#[test]
fn construction_site_siege_needs_the_hostages_alive() {
    let Some(mut r) = start("FreeCD1") else { return };
    r.play(3.0, false);
    r.kill(103);
    r.play(3.0, false);
    assert!(r.said("Hostage down!"), "nobody noticed: {:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}

/// Waiting out the clock loses it.
#[test]
fn construction_site_siege_runs_out_of_time() {
    let Some(mut r) = start("FreeCD1") else { return };
    r.play(245.0, false);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}

/// Every Bane workman down, the islanders unhurt.
#[test]
fn el_cossa_island_runs_to_a_win() {
    let Some(mut r) = start("jung3") else { return };
    r.play(3.0, false);
    assert!(r.said("El Cosso Island"), "no briefing: {:?}", r.seen);
    r.play(600.0, true);
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("All Bane workers down!"), "{:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// Killing an islander loses it.
#[test]
fn el_cossa_island_needs_the_islanders_alive() {
    let Some(mut r) = start("jung3") else { return };
    r.play(3.0, false);
    r.kill(81);
    r.play(3.0, false);
    assert!(r.said("You killed a hostage!"), "nobody noticed: {:?}", r.seen);
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "{:?}", r.g.outcome);
}

/// Officer Training (`train11`) and the Missile Silos (`snow1`), both with motorbikes, load
/// with every point supported; the training course has the sniper rifle at its shooting
/// range, beside the pistol.
#[test]
fn officer_training_and_the_missile_silos() {
    for (mission, bikes) in [("train11", 1), ("snow1", 3)] {
        let Some(mut r) = start(mission) else { return };
        r.play(1.0, false);
        assert_eq!(r.g.bikes.len(), bikes, "{mission}'s bikes");
        if mission == "train11" {
            let pistol = r.g.items.iter().find(|i| i.way == 29).expect("no pistol").pos;
            let rifle = r.g.items.iter().find(|i| i.kind == uc_game::SNIPER).expect("no sniper rifle").pos;
            assert!((rifle - pistol).length() < 200.0, "the rifle's at {rifle:?}, the pistol at {pistol:?}");
        }
    }
}
