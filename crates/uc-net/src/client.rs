//! A player's end: connecting, the lobby, the turns as they come in, and [`Lockstep`], which
//! plays them at the game's pace and sends this player's input for the turn `D` ahead.

use crate::*;
use std::collections::BTreeMap;
use std::io;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

/// Connected to a server.
pub struct Client {
    /// This player's seat: 0 is the host's (Darci), 1 Roper's.
    pub seat: usize,
    /// Who's in, by seat.
    pub names: Vec<String>,
    stream: TcpStream,
    rx: Receiver<io::Result<Msg>>,
    game: u32,
    start: Option<Setup>,
    turns: BTreeMap<u32, Vec<Input>>,
    notices: Vec<String>,
    /// Why the connection's gone, once it has.
    pub closed: Option<String>,
}

/// `host`, or `host:port`, as addresses to try.
fn addresses(addr: &str) -> Result<Vec<SocketAddr>, String> {
    let with_port = if addr.rsplit_once(':').is_some_and(|(_, p)| p.parse::<u16>().is_ok()) { addr.to_string() } else { format!("{addr}:{PORT}") };
    let all: Vec<SocketAddr> = with_port.to_socket_addrs().map_err(|e| format!("{addr}: {e}"))?.collect();
    if all.is_empty() {
        return Err(format!("{addr}: no such address"));
    }
    Ok(all)
}

impl Client {
    /// Connects to the server at `addr` (`host` or `host:port`) as `name`, and waits to be
    /// let in.
    pub fn connect(addr: &str, name: &str) -> Result<Client, String> {
        let mut last = String::new();
        let stream = addresses(addr)?.into_iter().find_map(|a| {
            TcpStream::connect_timeout(&a, Duration::from_secs(5)).map_err(|e| last = format!("{a}: {e}")).ok()
        });
        let mut stream = stream.ok_or(last)?;
        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        write_msg(&mut stream, &Msg::Hello { protocol: PROTOCOL, build: BUILD.into(), name: name.into() }).map_err(|e| e.to_string())?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
        let seat = match read_msg(&mut stream) {
            Ok(Msg::Welcome { seat }) => seat as usize,
            Ok(Msg::Refused { why }) => return Err(format!("the server said no: {why}")),
            Ok(m) => return Err(format!("the server said {m:?}")),
            Err(e) => return Err(format!("no answer from the server: {e}")),
        };
        stream.set_read_timeout(None).map_err(|e| e.to_string())?;
        let mut reader = stream.try_clone().map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new().name("co-op client".into()).spawn(move || loop {
            let m = read_msg(&mut reader);
            let end = m.is_err();
            if tx.send(m).is_err() || end {
                return;
            }
        }).map_err(|e| e.to_string())?;
        Ok(Client { seat, names: Vec::new(), stream, rx, game: 0, start: None, turns: BTreeMap::new(), notices: Vec::new(), closed: None })
    }

    /// Takes in everything the server has sent.
    pub fn poll(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(Ok(m)) => match m {
                    Msg::Lobby { names } => self.names = names,
                    Msg::Start { game, setup } => {
                        self.game = game;
                        self.turns.clear();
                        self.start = Some(setup);
                    }
                    Msg::Turn { turn, inputs } if self.game != 0 => {
                        self.turns.insert(turn, inputs);
                    }
                    Msg::Notice { text } => self.notices.push(text),
                    _ => {}
                },
                Ok(Err(e)) => {
                    self.closed.get_or_insert_with(|| format!("the connection broke: {e}"));
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.closed.get_or_insert_with(|| "the connection closed".into());
                    return;
                }
            }
        }
    }

    fn send(&mut self, m: &Msg) {
        if self.closed.is_none() {
            if let Err(e) = write_msg(&mut self.stream, m) {
                self.closed = Some(format!("the connection broke: {e}"));
            }
        }
    }

    /// The host's choice: start `setup` (again).
    pub fn choose(&mut self, setup: Setup) {
        self.send(&Msg::Choose(setup));
    }

    /// A game to start, once: the server has started one.
    pub fn take_start(&mut self) -> Option<Setup> {
        self.start.take()
    }

    /// A game is starting, and hasn't been taken yet.
    pub fn starting(&self) -> bool {
        self.start.is_some()
    }

    /// Notices from the server since the last call.
    pub fn take_notices(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notices)
    }

    /// Every seat's input for `turn`, if it's in.
    pub fn take_turn(&mut self, turn: u32) -> Option<Vec<Input>> {
        self.turns.remove(&turn)
    }

    /// Turns in and not yet played.
    pub fn queued(&self) -> usize {
        self.turns.len()
    }

    pub fn send_input(&mut self, turn: u32, input: Input) {
        let game = self.game;
        self.send(&Msg::Input { game, turn, input });
    }

    pub fn send_hash(&mut self, turn: u32, hash: u64) {
        let game = self.game;
        self.send(&Msg::Hash { game, turn, hash });
    }
}

/// A game played turn by turn as the server sends them.
#[derive(Debug, Default)]
pub struct Lockstep {
    /// The next turn to play.
    pub turn: u32,
    /// Real time not yet played.
    clock: f32,
    /// What this player's pressed since their last input went, and what they hold now.
    pending: Input,
    /// Seconds spent waiting for a turn that's due and hasn't come.
    pub waiting: f32,
}

use uc_game::TURN;
/// Turns a frame may play to catch up.
const MOST: usize = 8;

impl Lockstep {
    pub fn new() -> Lockstep {
        Lockstep::default()
    }

    /// One frame of `dt` seconds with this player's `input`: plays every turn that's due
    /// and in, with `play(turn, inputs, hash)`, which gives back the game's state hash when
    /// `hash` asks for it. After each, this player's input goes for the turn `DELAY` on.
    /// When turns pile up (this machine fell behind), it plays faster to catch up. The turns
    /// played.
    pub fn frame(&mut self, client: &mut Client, dt: f32, input: Input, mut play: impl FnMut(u32, &[Input], bool) -> Option<u64>) -> usize {
        client.poll();
        // A new game: its turns aren't this one's.
        if client.starting() {
            return 0;
        }
        self.pending = self.pending.then(&input);
        self.clock += dt;
        let (mut n, mut stalled) = (0, false);
        // Up to `DELAY` turns are always on their way: more than that in, and this machine
        // has fallen behind.
        while n < MOST && (self.clock >= TURN || client.queued() > DELAY as usize + 2) {
            let Some(inputs) = client.take_turn(self.turn) else {
                stalled = true;
                break;
            };
            let due = (self.turn + 1).is_multiple_of(HASH_EVERY);
            let hash = play(self.turn, &inputs, due);
            if let (true, Some(h)) = (due, hash) {
                client.send_hash(self.turn, h);
            }
            let sent = self.pending;
            client.send_input(self.turn + DELAY, sent);
            // Pressed once is played once; what's held stays held.
            self.pending = Input { buttons: sent.buttons & !button::PRESSES, ..sent };
            self.clock = (self.clock - TURN).max(0.0);
            self.turn += 1;
            n += 1;
        }
        // Due and not in: hold this turn until it is.
        self.clock = self.clock.min(TURN);
        self.waiting = if stalled && self.clock >= TURN { self.waiting + dt } else { 0.0 };
        n
    }

    /// How far into the next turn real time is, 0 to 1, for drawing between turns.
    pub fn between(&self) -> f32 {
        (self.clock / TURN).clamp(0.0, 1.0)
    }
}
