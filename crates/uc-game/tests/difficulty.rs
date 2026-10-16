//! Difficulty levels (#131): Normal is the game as it was; Easy and Hard move enemy skill,
//! how often they shoot, and how much Darci is hurt. Skipped without an install.

use uc_game::{Controls, Difficulty, Game};
use uc_world::Level;

mod common;
use common::DT;

fn start(mission: &str, d: Difficulty) -> Option<(Level, Game)> {
    let level = common::load(mission)?;
    let mut g = Game::new(&level, true);
    g.set_difficulty(d);
    Some((level, g))
}

/// The enemies' skills, by person.
fn skills(g: &Game) -> Vec<u8> {
    g.people.iter().filter(|p| !p.player).map(|p| p.skill).collect()
}

/// Normal changes nothing: the same run, turn for turn.
#[test]
fn normal_is_the_game_as_it_was() {
    let Some(level) = common::load("police1") else { return };
    let mut a = Game::new(&level, true);
    let mut b = Game::new(&level, true);
    b.set_difficulty(Difficulty::Normal);
    for _ in 0..300 {
        let c = Controls { wish: glam::vec2(0.0, 1.0), ..Default::default() };
        a.update(DT, &c, &level);
        b.update(DT, &c, &level);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

/// Hard adds three to every enemy's skill (to 15 at most), Easy takes three off; and they
/// wait longer between shots on Easy, less on Hard.
#[test]
fn skill_and_shooting_follow_the_difficulty() {
    let Some((_, normal)) = start("Gangorder1", Difficulty::Normal) else { return };
    let (_, hard) = start("Gangorder1", Difficulty::Hard).unwrap();
    let (_, easy) = start("Gangorder1", Difficulty::Easy).unwrap();
    for ((n, h), e) in skills(&normal).into_iter().zip(skills(&hard)).zip(skills(&easy)) {
        assert_eq!(h, (n + 3).min(15));
        assert_eq!(e, n.saturating_sub(3));
    }
    let wait = |g: &Game| g.people.iter().find(|p| !p.player).map(|p| p.shot_wait).unwrap();
    assert!(wait(&easy) > wait(&normal) && wait(&normal) > wait(&hard));
    // And it holds for those the script makes later.
    let (level, mut late) = start("Gangorder1", Difficulty::Hard).unwrap();
    let before = late.people.len();
    for _ in 0..600 {
        late.update(DT, &Controls::default(), &level);
    }
    for p in late.people.iter().skip(before).filter(|p| !p.player && p.hostile()) {
        assert!(p.skill >= 3 && p.shot_wait < 1.0, "a later enemy wasn't made for Hard");
    }
}

/// The same wave hurts Darci less on Easy than on Hard (an endless run in RTA's park, the
/// same seed both times).
#[test]
fn darci_is_hurt_less_on_easy() {
    let lost = |d: Difficulty| -> Option<i32> {
        let level = common::load("police1")?;
        let mut g = Game::new_endless(&level, 5, Some(glam::vec2(8827.0, 10274.0)), 9.0);
        g.set_difficulty(d);
        let me = g.player.unwrap();
        let full = g.people[me].health;
        let mut taken = 0;
        for _ in 0..(40.0 / DT) as usize {
            let h = g.people[me].health;
            g.update(DT, &Controls::default(), &level);
            taken += (h - g.people[me].health).max(0);
            // Patched up, so the fight goes on.
            if g.people[me].health < full / 2 {
                g.people[me].health = full;
            }
        }
        Some(taken)
    };
    let Some(easy) = lost(Difficulty::Easy) else { return };
    let hard = lost(Difficulty::Hard).unwrap();
    println!("Darci lost {easy} on Easy, {hard} on Hard");
    assert!(easy > 0, "the wave never hit her");
    assert!(hard > easy, "Hard ({hard}) should hurt more than Easy ({easy})");
}
