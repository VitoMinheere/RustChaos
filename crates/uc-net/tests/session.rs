//! A session over real sockets: a server, two players, and one of them behind a link that
//! holds everything up 60 ms each way. Each player's "game" folds every turn's inputs into a
//! hash, so if they're ever given different turns, or the same turns in a different order,
//! the hashes part. Both have to keep to the game's pace and agree throughout.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use uc_net::client::Lockstep;
use uc_net::{Client, Input, Setup};

/// Forwards connections to `to`, holding each chunk up `delay` in both directions.
fn laggy_proxy(to: SocketAddr, delay: Duration) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for inbound in listener.incoming() {
            let inbound = inbound.unwrap();
            let outbound = TcpStream::connect(to).unwrap();
            for (mut from, mut into) in [(inbound.try_clone().unwrap(), outbound.try_clone().unwrap()), (outbound, inbound)] {
                let _ = into.set_nodelay(true);
                let (tx, rx) = mpsc::channel::<(Instant, Vec<u8>)>();
                thread::spawn(move || {
                    let mut buf = [0; 4096];
                    while let Ok(n) = from.read(&mut buf) {
                        if n == 0 || tx.send((Instant::now() + delay, buf[..n].to_vec())).is_err() {
                            return;
                        }
                    }
                });
                thread::spawn(move || {
                    for (when, bytes) in rx {
                        thread::sleep(when.saturating_duration_since(Instant::now()));
                        if into.write_all(&bytes).is_err() {
                            return;
                        }
                    }
                });
            }
        }
    });
    addr
}

/// Plays `secs` of a game as `name` through `addr`, choosing it if it's the host: the turns
/// played, and the hash after every one of them.
fn play(addr: SocketAddr, name: &str, secs: f32) -> (u32, Vec<u64>) {
    let mut client = Client::connect(&addr.to_string(), name).unwrap();
    if client.seat == 0 {
        client.choose(Setup { mission: "test".into(), difficulty: 1, stats: [0; 4] });
    }
    while client.take_start().is_none() {
        client.poll();
        assert!(client.closed.is_none(), "{:?}", client.closed);
        thread::sleep(Duration::from_millis(2));
    }
    let seat = client.seat;
    let mut lock = Lockstep::new();
    let mut state = 0u64;
    let mut hashes = Vec::new();
    let (start, mut last) = (Instant::now(), Instant::now());
    while start.elapsed().as_secs_f32() < secs {
        thread::sleep(Duration::from_millis(16));
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        // Something different every turn, from each player.
        let input = Input { wish: [lock.turn as f32, seat as f32], buttons: (lock.turn % 7) as u16, ..Default::default() };
        lock.frame(&mut client, dt, input, |turn, inputs, _| {
            for i in inputs {
                state = state.rotate_left(5) ^ (turn as u64) ^ (i.wish[0].to_bits() as u64) << 1 ^ (i.wish[1].to_bits() as u64) << 7 ^ i.buttons as u64;
                state = state.wrapping_mul(0x100_0000_01b3);
            }
            hashes.push(state);
            Some(state)
        });
        assert!(client.closed.is_none(), "{:?}", client.closed);
    }
    (lock.turn, hashes)
}

#[test]
fn two_players_one_laggy_agree_and_keep_pace() {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();
    thread::spawn(move || uc_net::server::serve(server, false));
    let laggy = laggy_proxy(addr, Duration::from_millis(60));
    let secs = 4.0;
    let host = thread::spawn(move || play(addr, "Darci", secs));
    thread::sleep(Duration::from_millis(50));
    let guest = thread::spawn(move || play(laggy, "Roper", secs));
    let (a, b) = (host.join().unwrap(), guest.join().unwrap());
    let n = a.1.len().min(b.1.len());
    assert_eq!(a.1[..n], b.1[..n], "the players were given different turns");
    // The game's pace, give or take the start (the guest joins a little late, and each turn
    // waits on the slower link): 30 turns a second.
    for (who, turns) in [("host", a.0), ("guest", b.0)] {
        let rate = turns as f32 / secs;
        println!("{who}: {turns} turns, {rate:.1} a second");
        assert!((27.0..=31.0).contains(&rate), "the {who} played {turns} turns in {secs} s");
    }
}
