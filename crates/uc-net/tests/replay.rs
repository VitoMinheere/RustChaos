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
