//! The server: the lobby, each turn's inputs gathered from every seat and sent out together,
//! and the state hashes compared. [`Server`] is the logic, fed connections and messages and
//! giving back what to send; [`serve`] runs it over TCP, a thread per connection.

use crate::*;
use std::collections::BTreeMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// A connection, as the server numbers them.
pub type Conn = u32;

/// What the server wants done.
#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    To(Conn, Msg),
    All(Msg),
    /// Hang up on this connection (after anything sent it before).
    Close(Conn),
    /// Something for the server's log.
    Log(String),
}

#[derive(Debug, Clone)]
struct Seat {
    conn: Conn,
    name: String,
    /// Left during the game: their player stands still from then on.
    gone: bool,
    /// Their last few round trips (ms), the latest last.
    rtts: Vec<u64>,
}

/// Round trips kept for each player.
const RTTS: usize = 5;
/// Milliseconds between pings.
const PING_EVERY: u64 = 500;

/// A session's logic, with no sockets of its own.
#[derive(Debug, Default)]
pub struct Server {
    seats: Vec<Option<Seat>>,
    setup: Option<Setup>,
    /// The game being played (counting from 1), or 0 in the lobby.
    game: u32,
    /// The next turn to send out, and the inputs in for it and those after it.
    next: u32,
    inputs: BTreeMap<u32, Vec<Option<Input>>>,
    /// Hashes in for each turn, and whether the games have parted already.
    hashes: BTreeMap<u32, Vec<Option<u64>>>,
    parted: bool,
    /// The time now (ms), the pings out (id: to whom, when), and when they last went.
    now: u64,
    pings: BTreeMap<u32, (Conn, u64)>,
    ping_id: u32,
    last_ping: u64,
}

/// How far ahead of the turn being gathered a client may send: a few seconds.
const AHEAD: u32 = 300;

impl Server {
    pub fn new() -> Server {
        Server { seats: vec![None; SEATS], ..Default::default() }
    }

    fn seat_of(&self, conn: Conn) -> Option<usize> {
        self.seats.iter().position(|s| s.as_ref().is_some_and(|s| s.conn == conn))
    }

    fn lobby(&self) -> Msg {
        Msg::Lobby { names: self.seats.iter().map(|s| s.as_ref().map_or(String::new(), |s| s.name.clone())).collect() }
    }

    fn full(&self) -> bool {
        self.seats.iter().all(|s| s.is_some())
    }

    /// The time, in milliseconds from anywhere: what round trips are timed by.
    pub fn set_time(&mut self, ms: u64) {
        self.now = ms;
    }

    fn ping(&mut self, conn: Conn, out: &mut Vec<Out>) {
        self.ping_id += 1;
        self.pings.insert(self.ping_id, (conn, self.now));
        out.push(Out::To(conn, Msg::Ping { id: self.ping_id }));
    }

    /// Now and then: a ping for everyone in, to keep their round trips up to date.
    pub fn tick(&mut self) -> Vec<Out> {
        let mut out = Vec::new();
        if self.now >= self.last_ping + PING_EVERY {
            self.last_ping = self.now;
            let conns: Vec<Conn> = self.seats.iter().flatten().filter(|s| !s.gone).map(|s| s.conn).collect();
            for c in conns {
                self.ping(c, &mut out);
            }
            // Pings nobody answered.
            let now = self.now;
            self.pings.retain(|_, &mut (_, at)| now < at + 10_000);
        }
        out
    }

    /// The slowest round trip of the players in (ms), each by the slowest of their last few;
    /// `None` until everyone's has been timed at least once.
    fn slowest(&self) -> Option<u64> {
        self.seats.iter().flatten().filter(|s| !s.gone).map(|s| s.rtts.iter().max().copied()).try_fold(0, |a, r| Some(a.max(r?)))
    }

    /// Whether a game is being played.
    pub fn playing(&self) -> bool {
        self.game != 0 && self.seats.iter().flatten().any(|s| !s.gone)
    }

    /// A message from connection `conn`.
    pub fn handle(&mut self, conn: Conn, m: Msg) -> Vec<Out> {
        let mut out = Vec::new();
        let seat = self.seat_of(conn);
        match (m, seat) {
            (Msg::Hello { protocol, build, name }, None) => {
                let why = if protocol != PROTOCOL {
                    Some(format!("protocol {protocol}, the server's is {PROTOCOL}: update the game"))
                } else if build != BUILD {
                    Some(format!("this is build {build}, the server is build {BUILD}: both need the same build"))
                } else if self.playing() {
                    Some("a game is on: the host can restart the mission (R) to let you in".to_string())
                } else if self.full() {
                    Some("the game is full".to_string())
                } else {
                    None
                };
                if let Some(why) = why {
                    out.push(Out::Log(format!("refused {name}: {why}")));
                    out.push(Out::To(conn, Msg::Refused { why }));
                    out.push(Out::Close(conn));
                    return out;
                }
                // A game over and everyone gone but its leavers: back to the lobby.
                if self.game != 0 {
                    self.game_over();
                }
                let s = self.seats.iter().position(|s| s.is_none()).unwrap();
                let name = if name.trim().is_empty() { format!("Player {}", s + 1) } else { name };
                out.push(Out::Log(format!("{name} is in, seat {}", s + 1)));
                self.seats[s] = Some(Seat { conn, name, gone: false, rtts: Vec::new() });
                out.push(Out::To(conn, Msg::Welcome { seat: s as u8 }));
                // Timed at once: the game can't start until it has been.
                self.ping(conn, &mut out);
                out.push(Out::All(self.lobby()));
                if let Some(setup) = self.setup.clone() {
                    out.push(Out::To(conn, Msg::Notice { text: format!("The host has chosen {}", setup.mission) }));
                }
                self.start_if_ready(&mut out);
            }
            // Only the host (seat 0) chooses.
            (Msg::Choose(setup), Some(0)) => {
                out.push(Out::Log(format!("seat 1 chose {} (difficulty {})", setup.mission, setup.difficulty)));
                // Starting over without whoever left: their seats are free again.
                if self.seats.iter().flatten().any(|s| s.gone) {
                    self.game_over();
                    out.push(Out::All(self.lobby()));
                }
                self.setup = Some(setup);
                self.start_if_ready(&mut out);
                if self.game == 0 {
                    out.push(Out::All(Msg::Notice { text: "Waiting for a second player".into() }));
                }
            }
            (Msg::Input { game, turn, input }, Some(s)) if game == self.game && game != 0 => {
                if turn >= self.next && turn < self.next + AHEAD {
                    self.inputs.entry(turn).or_insert_with(|| vec![None; SEATS])[s] = Some(input);
                    self.send_turns(&mut out);
                }
            }
            (Msg::Hash { game, turn, hash }, Some(s)) if game == self.game && game != 0 => {
                let live: Vec<bool> = self.seats.iter().map(|x| x.as_ref().is_some_and(|x| !x.gone)).collect();
                let h = self.hashes.entry(turn).or_insert_with(|| vec![None; SEATS]);
                h[s] = Some(hash);
                if (0..SEATS).all(|k| !live[k] || h[k].is_some()) {
                    let h = self.hashes.remove(&turn).unwrap();
                    let mut seen = h.iter().flatten();
                    let first = seen.next().copied();
                    if seen.any(|&x| Some(x) != first) && !self.parted {
                        self.parted = true;
                        out.push(Out::Log(format!("the games parted at turn {turn}: {h:x?}")));
                        out.push(Out::All(Msg::Notice {
                            text: format!("Out of step at {:.0} s: the games differ. Restart the mission (R on the host).", turn as f32 / 30.0),
                        }));
                    }
                }
                // Old turns nobody will finish (a seat that left meanwhile).
                self.hashes.retain(|&t, _| t + 10 * HASH_EVERY > turn);
            }
            (Msg::Pong { id }, Some(s)) => {
                if let Some((_, at)) = self.pings.remove(&id).filter(|&(c, _)| c == conn) {
                    let rtts = &mut self.seats[s].as_mut().unwrap().rtts;
                    rtts.push(self.now.saturating_sub(at));
                    if rtts.len() > RTTS {
                        rtts.remove(0);
                    }
                    if self.game == 0 {
                        self.start_if_ready(&mut out);
                    }
                }
            }
            // Anything else (input for a game that's over, a guest choosing) is dropped.
            _ => {}
        }
        out
    }

    /// Connection `conn` has gone.
    pub fn left(&mut self, conn: Conn) -> Vec<Out> {
        let mut out = Vec::new();
        let Some(s) = self.seat_of(conn) else { return out };
        let name = self.seats[s].as_ref().unwrap().name.clone();
        out.push(Out::Log(format!("{name} left")));
        if self.playing() {
            self.seats[s].as_mut().unwrap().gone = true;
            if self.playing() {
                out.push(Out::All(Msg::Notice { text: format!("{name} left: their player stands still") }));
                self.send_turns(&mut out);
                return out;
            }
            // The last one out: the game's over.
            self.game_over();
        }
        self.seats[s] = None;
        if s == 0 {
            self.setup = None;
        }
        out.push(Out::All(self.lobby()));
        out
    }

    /// Back to the lobby, with the leavers' seats free.
    fn game_over(&mut self) {
        self.game = 0;
        for s in &mut self.seats {
            if s.as_ref().is_some_and(|s| s.gone) {
                *s = None;
            }
        }
        if self.seats[0].is_none() {
            self.setup = None;
        }
    }

    /// Starts the chosen mission once every seat is taken: the first few turns, which nobody
    /// has had the time to send input for, go out at once with nobody doing anything.
    fn start_if_ready(&mut self, out: &mut Vec<Out>) {
        let Some(setup) = self.setup.clone() else { return };
        if !self.full() || self.seats.iter().flatten().any(|s| s.gone) {
            return;
        }
        let Some(rtt) = self.slowest() else { return };
        let delay = delay_for(rtt);
        self.game += 1;
        (self.next, self.parted) = (0, false);
        self.inputs.clear();
        self.hashes.clear();
        out.push(Out::Log(format!("game {} starts: {}, the slowest round trip {rtt} ms, so a delay of {delay} turns", self.game, setup.mission)));
        out.push(Out::All(Msg::Start { game: self.game, delay: delay as u8, setup }));
        for _ in 0..delay {
            out.push(Out::All(Msg::Turn { turn: self.next, inputs: vec![Input::default(); SEATS] }));
            self.next += 1;
        }
    }

    /// Every turn whose inputs are all in goes out, in order. A seat that's left stands still.
    fn send_turns(&mut self, out: &mut Vec<Out>) {
        loop {
            let gone: Vec<bool> = self.seats.iter().map(|s| s.as_ref().is_none_or(|s| s.gone)).collect();
            let ready = self.inputs.get(&self.next).map_or(gone.iter().all(|&g| g), |i| (0..SEATS).all(|k| gone[k] || i[k].is_some()));
            if !ready {
                return;
            }
            let i = self.inputs.remove(&self.next).unwrap_or_else(|| vec![None; SEATS]);
            if gone.iter().all(|&g| g) {
                return;
            }
            out.push(Out::All(Msg::Turn { turn: self.next, inputs: i.into_iter().map(Option::unwrap_or_default).collect() }));
            self.next += 1;
        }
    }
}

enum Event {
    Joined(Conn, TcpStream),
    Msg(Conn, Msg),
    Left(Conn),
}

/// Listens on `addr` and runs sessions there for good (or until the listener fails). Logs
/// to stdout with `log`.
pub fn serve(listener: TcpListener, log: bool) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let accept = listener.try_clone()?;
    std::thread::spawn(move || {
        let mut n: Conn = 0;
        for stream in accept.incoming() {
            let Ok(stream) = stream else { continue };
            n += 1;
            let conn = n;
            let _ = stream.set_nodelay(true);
            let Ok(mut reader) = stream.try_clone() else { continue };
            if tx.send(Event::Joined(conn, stream)).is_err() {
                return;
            }
            let tx = tx.clone();
            std::thread::spawn(move || {
                while let Ok(m) = read_msg(&mut reader) {
                    if tx.send(Event::Msg(conn, m)).is_err() {
                        return;
                    }
                }
                let _ = tx.send(Event::Left(conn));
            });
        }
    });
    let mut server = Server::new();
    let mut conns: BTreeMap<Conn, TcpStream> = BTreeMap::new();
    if log {
        println!("co-op server on {} (build {BUILD})", listener.local_addr()?);
    }
    let start = Instant::now();
    loop {
        let e = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(e) => Some(e),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        server.set_time(start.elapsed().as_millis() as u64);
        let mut out = server.tick();
        out.extend(match e {
            None => Vec::new(),
            Some(Event::Joined(conn, stream)) => {
                if log {
                    println!("connection {conn} from {}", stream.peer_addr().map_or("?".into(), |a| a.to_string()));
                }
                conns.insert(conn, stream);
                Vec::new()
            }
            Some(Event::Msg(conn, m)) => server.handle(conn, m),
            Some(Event::Left(conn)) => {
                conns.remove(&conn);
                server.left(conn)
            }
        });
        let mut dead = Vec::new();
        for o in out {
            match o {
                Out::To(c, m) => {
                    if let Some(s) = conns.get_mut(&c) {
                        if write_msg(s, &m).is_err() {
                            dead.push(c);
                        }
                    }
                }
                Out::All(m) => {
                    let bytes = wire::encode(&m);
                    for (&c, s) in conns.iter_mut() {
                        if server.seat_of(c).is_some() && io::Write::write_all(s, &bytes).is_err() {
                            dead.push(c);
                        }
                    }
                }
                Out::Close(c) => {
                    if let Some(s) = conns.remove(&c) {
                        let _ = s.shutdown(std::net::Shutdown::Both);
                    }
                }
                Out::Log(text) => {
                    if log {
                        println!("{text}");
                    }
                }
            }
        }
        // A connection that can't be written to is shut, and its reader says it's left.
        for c in dead {
            if let Some(s) = conns.get(&c) {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        }
    }
    Ok(())
}

/// A server on a thread of its own, for a player hosting from the game: on `port` of every
/// address (0 for any free port). Where it's listening.
pub fn spawn(port: u16) -> io::Result<SocketAddr> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    let addr = listener.local_addr()?;
    std::thread::Builder::new().name("co-op server".into()).spawn(move || serve(listener, true))?;
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(name: &str) -> Msg {
        Msg::Hello { protocol: PROTOCOL, build: BUILD.into(), name: name.into() }
    }

    fn setup() -> Setup {
        Setup { mission: "police1".into(), difficulty: 1, stats: [0; 4] }
    }

    fn sent(out: &[Out]) -> Vec<&Msg> {
        out.iter().filter_map(|o| match o { Out::All(m) | Out::To(_, m) => Some(m), _ => None }).collect()
    }

    fn turns(out: &[Out]) -> Vec<u32> {
        sent(out).into_iter().filter_map(|m| match m { Msg::Turn { turn, .. } => Some(*turn), _ => None }).collect()
    }

    /// The answers to the pings in `out` from `conn`, `ms` later.
    fn pong(s: &mut Server, conn: Conn, out: &[Out], ms: u64) -> Vec<Out> {
        s.set_time(s.now + ms);
        let ids: Vec<u32> = out.iter().filter_map(|o| match o { Out::To(c, Msg::Ping { id }) if *c == conn => Some(*id), _ => None }).collect();
        ids.into_iter().flat_map(|id| s.handle(conn, Msg::Pong { id })).collect()
    }

    /// Two players in, the host's choice made: the game starts with its first turns out.
    fn started() -> Server {
        let mut s = Server::new();
        let out = s.handle(1, hello("Darci"));
        pong(&mut s, 1, &out, 0);
        s.handle(1, Msg::Choose(setup()));
        let out = s.handle(2, hello("Roper"));
        // Not before Roper's round trip has been timed.
        assert!(!sent(&out).iter().any(|m| matches!(m, Msg::Start { .. })));
        let out = pong(&mut s, 2, &out, 0);
        assert!(sent(&out).iter().any(|m| matches!(m, Msg::Start { game: 1, delay: 3, .. })));
        assert_eq!(turns(&out), (0..DELAY).collect::<Vec<_>>());
        s
    }

    #[test]
    fn a_slow_round_trip_makes_a_longer_delay() {
        let mut s = Server::new();
        let out = s.handle(1, hello("Darci"));
        pong(&mut s, 1, &out, 1);
        s.handle(1, Msg::Choose(setup()));
        let out = s.handle(2, hello("Roper"));
        let out = pong(&mut s, 2, &out, 150);
        // 150 ms and the margin: 190 ms, six turns.
        assert!(sent(&out).iter().any(|m| matches!(m, Msg::Start { delay: 6, .. })), "{out:?}");
        assert_eq!(turns(&out), (0..6).collect::<Vec<_>>());
    }

    fn input(n: u16) -> Input {
        Input { buttons: n, ..Default::default() }
    }

    #[test]
    fn a_turn_goes_out_once_every_seat_has_sent_for_it() {
        let mut s = started();
        assert!(turns(&s.handle(1, Msg::Input { game: 1, turn: DELAY, input: input(1) })).is_empty());
        let out = s.handle(2, Msg::Input { game: 1, turn: DELAY, input: input(2) });
        let Some(Msg::Turn { turn, inputs }) = sent(&out).first().copied() else { panic!("{out:?}") };
        assert_eq!((*turn, inputs.clone()), (DELAY, vec![input(1), input(2)]));
    }

    #[test]
    fn turns_go_out_in_order() {
        let mut s = started();
        s.handle(1, Msg::Input { game: 1, turn: DELAY + 1, input: input(1) });
        s.handle(2, Msg::Input { game: 1, turn: DELAY + 1, input: input(1) });
        s.handle(1, Msg::Input { game: 1, turn: DELAY, input: input(1) });
        let out = s.handle(2, Msg::Input { game: 1, turn: DELAY, input: input(1) });
        assert_eq!(turns(&out), vec![DELAY, DELAY + 1]);
    }

    #[test]
    fn input_for_another_game_is_dropped() {
        let mut s = started();
        s.handle(1, Msg::Input { game: 1, turn: DELAY, input: input(1) });
        assert!(s.handle(2, Msg::Input { game: 0, turn: DELAY, input: input(1) }).is_empty());
    }

    #[test]
    fn someone_leaving_mid_game_stands_still() {
        let mut s = started();
        s.handle(1, Msg::Input { game: 1, turn: DELAY, input: input(1) });
        let out = s.left(2);
        assert!(sent(&out).iter().any(|m| matches!(m, Msg::Notice { .. })));
        assert_eq!(turns(&out), vec![DELAY]);
        let out = s.handle(1, Msg::Input { game: 1, turn: DELAY + 1, input: input(3) });
        assert_eq!(turns(&out), vec![DELAY + 1]);
    }

    #[test]
    fn different_hashes_are_told_once() {
        let mut s = started();
        s.handle(1, Msg::Hash { game: 1, turn: 29, hash: 5 });
        let ok = s.handle(2, Msg::Hash { game: 1, turn: 29, hash: 5 });
        assert!(sent(&ok).is_empty());
        s.handle(1, Msg::Hash { game: 1, turn: 59, hash: 5 });
        let bad = s.handle(2, Msg::Hash { game: 1, turn: 59, hash: 6 });
        assert!(matches!(sent(&bad)[..], [Msg::Notice { .. }]));
        s.handle(1, Msg::Hash { game: 1, turn: 89, hash: 5 });
        assert!(sent(&s.handle(2, Msg::Hash { game: 1, turn: 89, hash: 6 })).is_empty());
    }

    #[test]
    fn a_third_player_and_another_build_are_turned_away() {
        let mut s = started();
        let out = s.handle(3, hello("Mako"));
        assert!(matches!(sent(&out)[..], [Msg::Refused { .. }]));
        let mut s = Server::new();
        let out = s.handle(1, Msg::Hello { protocol: PROTOCOL, build: "other".into(), name: "A".into() });
        assert!(matches!(sent(&out)[..], [Msg::Refused { .. }]));
    }

    #[test]
    fn choosing_again_starts_a_new_game() {
        let mut s = started();
        let out = s.handle(1, Msg::Choose(setup()));
        assert!(sent(&out).iter().any(|m| matches!(m, Msg::Start { game: 2, .. })));
        assert_eq!(turns(&out), (0..DELAY).collect::<Vec<_>>());
        // Only the host chooses.
        assert!(s.handle(2, Msg::Choose(setup())).is_empty());
    }

    #[test]
    fn everyone_gone_is_back_to_the_lobby() {
        let mut s = started();
        s.left(1);
        s.left(2);
        assert!(!s.playing());
        let out = s.handle(3, hello("Darci"));
        assert!(sent(&out).iter().any(|m| matches!(m, Msg::Welcome { seat: 0 })));
        assert!(sent(&out).iter().any(|m| matches!(m, Msg::Ping { .. })));
    }
}
