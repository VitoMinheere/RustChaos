//! The co-op server (#182): headless, it lets two players in, gathers their inputs turn by
//! turn and sends them out, and compares their games' state hashes. It needs no game data:
//! the players' machines run the game.
//!
//! ```text
//! uc-server [--port 7777]
//! uc-server --bot <host[:port]> [--mission police1] [--secs 60] [--name Bot]
//! ```
//!
//! `--bot` is a player with no window: it joins, chooses the mission if it's the host,
//! plays it with made-up inputs at the game's pace from the install (`UC_GAME_DIR`, or the
//! GOG default), and prints its state hashes. Two bots on one server show two separate
//! processes staying in step.

use std::time::{Duration, Instant};
use uc_net::client::Lockstep;
use uc_net::{Client, Input, Setup};
use uc_world::{Level, LoadOptions};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut port, mut bot, mut mission, mut secs, mut name) = (uc_net::PORT, None, "police1".to_string(), 60.0, None);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} wants a value"));
        match a.as_str() {
            "--port" => port = value()?.parse().map_err(|e| format!("--port: {e}"))?,
            "--bot" => bot = Some(value()?),
            "--mission" => mission = value()?,
            "--secs" => secs = value()?.parse().map_err(|e| format!("--secs: {e}"))?,
            "--name" => name = Some(value()?),
            "--help" | "-h" => {
                println!("uc-server [--port {}]\nuc-server --bot <host[:port]> [--mission police1] [--secs 60] [--name Bot]", uc_net::PORT);
                return Ok(());
            }
            _ => return Err(format!("unknown argument {a} (try --help)")),
        }
    }
    match bot {
        Some(addr) => play_bot(&addr, &mission, secs, &name.unwrap_or_else(|| "Bot".into())),
        None => {
            let listener = std::net::TcpListener::bind(("0.0.0.0", port)).map_err(|e| format!("port {port}: {e}"))?;
            uc_net::server::serve(listener, true).map_err(|e| e.to_string())
        }
    }
}

fn play_bot(addr: &str, mission: &str, secs: f32, name: &str) -> Result<(), String> {
    let game_dir = uc_formats::default_install().ok_or("no Urban Chaos install (set UC_GAME_DIR)")?;
    let mut client = Client::connect(addr, name)?;
    println!("{name}: in at seat {}", client.seat + 1);
    if client.seat == 0 {
        client.choose(Setup::new(mission, uc_game::Difficulty::Normal, [0; 4]));
    }
    // Waiting for the game to start.
    let setup = loop {
        client.poll();
        if let Some(s) = client.take_start() {
            break s;
        }
        for n in client.take_notices() {
            println!("{name}: {n}");
        }
        if let Some(why) = &client.closed {
            return Err(why.clone());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let opt = LoadOptions { fullbright: true, character: true, overrides: None };
    let level = Level::load(&game_dir, &setup.mission, &opt)?;
    let mut game = setup.game(&level);
    println!("{name}: playing {} as {}", setup.mission, game.seat_person(client.seat).map_or("nobody", |i| uc_world::character::person::name(game.people[i].kind)));
    let mut lock = Lockstep::new();
    let (start, mut last) = (Instant::now(), Instant::now());
    let seat = client.seat;
    while start.elapsed().as_secs_f32() < secs {
        std::thread::sleep(Duration::from_millis(16));
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let t = (lock.turn / 30) as usize;
        let input = Input::new(&uc_net::bot_input(seat, t), false);
        lock.frame(&mut client, dt, input, |turn, inputs, hash| {
            uc_net::play_turn(&mut game, &level, inputs);
            let h = hash.then(|| game.state_hash());
            if let Some(h) = h {
                println!("{name}: turn {turn} hash {h:016x}");
            }
            h
        });
        for n in client.take_notices() {
            println!("{name}: {n}");
        }
        if let Some(why) = &client.closed {
            return Err(why.clone());
        }
        if lock.waiting > 5.0 {
            return Err(format!("{name}: no turn for 5 s"));
        }
    }
    let at = |s: usize| game.seat_person(s).map(|i| game.people[i].fig.pos / 256.0);
    println!("{name}: done after {} turns; seat 1 at {:?}, seat 2 at {:?}", lock.turn, at(0), at(1));
    Ok(())
}
