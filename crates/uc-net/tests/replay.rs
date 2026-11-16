//! Replays (#184): a run recorded the way the viewer plays it (frames of uneven length, presses
//! between turns, a cutscene skipped, the viewer losing the mission itself) and saved as
//! bytes plays back from them into the same game, its state hash agreeing all the way; a
//! co-op run does too; and a replay that's been changed says where it drifted. Skipped
//! without an install.

use uc_game::{Controls, Difficulty, Game, Outcome, Weapon};
use uc_net::replay::{Event, Mode, Playback, Recorder, Replay, Start};
use uc_net::{bot_input, play_turn, Input, Setup};
use uc_world::{Level, LoadOptions};

fn load(mission: &str) -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, mission, &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap())
}

/// Plays `replay` back from its bytes into a new game: the game at the end, and the
/// playback.
fn play_back(level: &Level, replay: &Replay) -> (Game, Playback) {
    let replay = Replay::from_bytes(&replay.to_bytes()).unwrap();
    let mut g = replay.start.game(level);
    let mut p = Playback::new(replay);
    p.skip(&mut g, level, usize::MAX);
    assert!(p.done());
    (g, p)
}

#[test]
fn a_mission_plays_back_the_same() {
    let Some(level) = load("police1") else { return };
    let start = Start {
        weapon: Weapon::Pistol, dogs: 2, view_dist: Some(40.0 * 256.0),
        ..Start::new("police1", Mode::Mission, Difficulty::Hard, [2, 0, 1, 3])
    };
    let mut g = start.game(&level);
    let mut rec = Recorder::new(&mut g, start);
    // Frames of 7 to 40 ms, a second's pattern of the bot's input each, presses on the frame
    // the second starts only, as a tap is.
    let mut skipped = false;
    let mut frame = 0usize;
    while g.time < 45.0 {
        let dt = [0.007, 0.016, 0.040, 0.025, 0.011][frame % 5];
        let second = (g.time as usize).min(44);
        let mut c = bot_input(0, second);
        if (g.time - second as f32) > 0.1 {
            (c.punch, c.kick, c.jump) = (false, false, false);
        }
        if g.cutscene.is_some() && g.time > 1.0 {
            g.skip_cutscene();
            skipped = true;
        }
        g.update(dt, &c, &level);
        rec.take(&mut g);
        if frame == 1500 {
            g.outcome = Some(Outcome::Lost("Too many civilian casualties".into()));
            rec.event(&mut g, Event::Lose("Too many civilian casualties".into()));
        }
        frame += 1;
    }
    assert!(skipped, "police1 starts with a fly-by to skip");
    let replay = rec.finish(&mut g);
    assert_eq!(replay.turns.len() as u64, g.turns());
    assert!(replay.turns.iter().any(|t| t[0].skip()));
    assert_eq!(replay.hashes.len(), replay.turns.len() / 30);
    assert!(replay.outcome.starts_with("lost"), "{}", replay.outcome);
    let bytes = replay.to_bytes();
    println!("{} turns in {} bytes", replay.turns.len(), bytes.len());
    assert!(bytes.len() < replay.turns.len() * 18, "the runs of the same input are kept once");

    let (back, p) = play_back(&level, &replay);
    assert_eq!(p.drift, None);
    assert_eq!(back.turns(), g.turns());
    assert_eq!(back.state_hash(), g.state_hash());
    assert_eq!(back.outcome, g.outcome);
}

#[test]
fn a_coop_run_plays_back_the_same() {
    let Some(level) = load("police1") else { return };
    let setup = Setup::new("police1", Difficulty::Normal, [0, 1, 0, 1]);
    let mut g = setup.game(&level);
    let mut rec = Recorder::new(&mut g, Start::coop(&setup));
    for t in 0..30 * 30 {
        // Roper skips the fly-by a second in.
        let inputs = [Input::new(&bot_input(0, t / 30), false), Input::new(&bot_input(1, t / 30), t == 30)];
        play_turn(&mut g, &level, &inputs);
    }
    let replay = rec.finish(&mut g);
    assert_eq!(replay.seats, 2);
    assert_eq!(replay.start.mode, Mode::Coop);
    let (back, p) = play_back(&level, &replay);
    assert_eq!(p.drift, None);
    assert_eq!(back.state_hash(), g.state_hash());
    assert_eq!(back.seats(), 2);
}

/// A changed input shows at the next hash, not before.
#[test]
fn a_changed_replay_says_where_it_drifted() {
    let Some(level) = load("police1") else { return };
    let start = Start::new("police1", Mode::Mission, Difficulty::Normal, [0; 4]);
    let mut g = start.game(&level);
    let mut rec = Recorder::new(&mut g, start);
    for t in 0..30 * 20 {
        g.skip_cutscene();
        let mut inputs = vec![Controls::default()];
        inputs[0] = bot_input(0, t / 30);
        g.step(&inputs, &level);
    }
    let mut replay = rec.finish(&mut g);
    // Standing still for the second from 10 s on.
    for t in &mut replay.turns[300..330] {
        t[0] = Input::default();
    }
    let (_, p) = play_back(&level, &replay);
    assert_eq!(p.drift, Some(330));
}

/// A run left and carried on (#544): the game made again and played up to where the save
/// was taken stands as the one left, the two play on the same turn for turn, and the replay
/// of the run carried on is the replay of one never left.
#[test]
fn a_saved_run_carries_on_the_same() {
    let Some(level) = load("police1") else { return };
    let start = Start { weapon: Weapon::Pistol, dogs: 1, ..Start::new("police1", Mode::Mission, Difficulty::Normal, [1, 0, 2, 0]) };
    let turn = |g: &mut Game, t: usize| {
        play_turn(g, &level, &[Input::new(&bot_input(0, t / 30), t == 40)]);
    };
    let mut g = start.game(&level);
    let mut rec = Recorder::new(&mut g, start.clone());
    for t in 0..30 * 25 + 7 {
        turn(&mut g, t);
    }
    let saved = Replay::from_bytes(&rec.finish(&mut g).to_bytes()).unwrap();

    let mut back = saved.start.game(&level);
    let mut again = Recorder::new(&mut back, saved.start.clone());
    let mut turns = 0;
    again.resume(&mut back, &level, &saved, |_, r| {
        turns += 1;
        assert_eq!(r.turns(), turns);
    }).unwrap();
    assert_eq!(turns as u64, g.turns());
    assert_eq!(back.state_hash(), g.state_hash());
    for t in 30 * 25 + 7..30 * 45 {
        turn(&mut g, t);
        turn(&mut back, t);
        assert_eq!(back.state_hash(), g.state_hash(), "after turn {t}");
    }
    let (whole, carried) = (rec.finish(&mut g), again.finish(&mut back));
    assert_eq!(carried, whole);
    let (_, p) = play_back(&level, &carried);
    assert_eq!(p.drift, None);
}

/// A save that isn't of this game (as another build's wouldn't be) is refused at the first
/// hash that differs.
#[test]
fn a_save_that_has_drifted_is_refused() {
    let Some(level) = load("police1") else { return };
    let start = Start::new("police1", Mode::Mission, Difficulty::Normal, [0; 4]);
    let mut g = start.game(&level);
    let mut rec = Recorder::new(&mut g, start.clone());
    for t in 0..30 * 12 {
        play_turn(&mut g, &level, &[Input::new(&bot_input(0, t / 30), t == 40)]);
    }
    let mut saved = rec.finish(&mut g);
    saved.hashes[5].1 ^= 1;
    let mut back = start.game(&level);
    let mut again = Recorder::new(&mut back, start);
    assert_eq!(again.resume(&mut back, &level, &saved, |_, _| {}), Err(saved.hashes[5].0));
}
