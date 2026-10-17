//! Co-op (#182): playing a mission with someone else through a server, Darci and Roper. The
//! host runs the server in this process ([`uc_net::server::spawn`]) and plays on it; the
//! other player joins by address. Every frame goes to [`Lockstep`] instead of
//! `Game::update`, and the game stays focused on this player's seat, so the camera, the HUD
//! and the listener follow their own person.

use crate::*;
use std::net::SocketAddr;
use uc_net::client::Lockstep;
use uc_net::{Client, Input, Setup};

/// Seconds a notice from the server stays up.
const NOTICE: f32 = 8.0;

pub(crate) struct Coop {
    pub(crate) client: Client,
    pub(crate) lock: Lockstep,
    /// The game being played, as the server started it.
    pub(crate) setup: Option<Setup>,
    /// Notices, with the seconds each has left on screen.
    pub(crate) notices: Vec<(String, f32)>,
    /// Testing (`--coop-bot`): made-up inputs instead of this player's.
    pub(crate) bot: bool,
}

/// The addresses this machine can be reached on: on the local network, and on this machine.
pub(crate) fn addresses(port: u16) -> Vec<String> {
    // Connecting a UDP socket sends nothing, but picks the address the way out would use.
    let lan = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|s| s.connect("8.8.8.8:80").and(s.local_addr()));
    let mut out: Vec<String> = lan.ok().map(|a| format!("{}:{port}", a.ip())).into_iter().collect();
    out.push(format!("127.0.0.1:{port}"));
    out
}

impl Coop {
    /// Hosts: a server on `port` (unless one's running already, at `hosting`), and this
    /// player in it.
    pub(crate) fn host(hosting: &mut Option<SocketAddr>, port: u16, name: &str) -> Result<Coop, String> {
        let addr = match *hosting {
            Some(a) => a,
            None => {
                let a = uc_net::server::spawn(port).map_err(|e| format!("can't host on port {port}: {e}"))?;
                *hosting = Some(a);
                a
            }
        };
        Coop::join(&format!("127.0.0.1:{}", addr.port()), name)
    }

    pub(crate) fn join(addr: &str, name: &str) -> Result<Coop, String> {
        let client = Client::connect(addr, name)?;
        Ok(Coop { client, lock: Lockstep::new(), setup: None, notices: Vec::new(), bot: false })
    }

    pub(crate) fn seat(&self) -> usize {
        self.client.seat
    }

    pub(crate) fn is_host(&self) -> bool {
        self.client.seat == 0
    }

    /// The other player's name.
    pub(crate) fn other(&self) -> String {
        self.client.names.iter().enumerate().find(|(s, n)| *s != self.client.seat && !n.is_empty())
            .map_or("the other player".into(), |(_, n)| n.clone())
    }

    /// The server's notices in, and the old ones going.
    pub(crate) fn notices(&mut self, dt: f32) {
        for n in &mut self.notices {
            n.1 -= dt;
        }
        self.notices.retain(|n| n.1 > 0.0);
        for t in self.client.take_notices() {
            println!("co-op: {t}");
            self.notices.push((t, NOTICE));
        }
        if let Some(why) = &self.client.closed {
            if !self.notices.iter().any(|n| n.0 == *why) {
                self.notices.push((why.clone(), f32::INFINITY));
            }
        }
    }

    /// A frame of the game: this player's `controls` (and a cutscene skipped) go off, and the
    /// turns that have come are played. Every ten seconds the turn and the state hash go to
    /// stdout, to compare the two machines' logs by.
    pub(crate) fn frame(&mut self, w: &mut World, dt: f32, controls: &Controls, skip: bool) {
        let input = match self.bot {
            true => Input::new(&uc_net::bot_input(self.client.seat, (self.lock.turn / 30) as usize), true),
            false => Input::new(controls, skip),
        };
        let (game, level) = (&mut w.game, &w.level);
        self.lock.frame(&mut self.client, dt, input, |turn, inputs, hash| {
            uc_net::play_turn(game, level, inputs);
            let h = hash.then(|| game.state_hash());
            if let (Some(h), true) = (h, (turn + 1).is_multiple_of(300)) {
                println!("co-op: turn {} ({:.0} s) hash {h:016x}", turn + 1, (turn + 1) as f32 / 30.0);
            }
            h
        });
        w.game.set_between(self.lock.between());
    }

    /// The host starts `mission` (again) for everyone.
    pub(crate) fn choose(&mut self, mission: &str) {
        let p = progress::Progress::load();
        self.client.choose(Setup::new(mission, p.difficulty(), p.stats()));
    }

    /// Over the game: who's waiting for whom, and the notices.
    pub(crate) fn overlay(&self, o: &mut Overlay, size: (u32, u32)) {
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).round().max(1.0);
        let mut y = h * 0.22;
        let mut line = |o: &mut Overlay, text: &str, colour: [u8; 4]| {
            let x = w / 2.0 - Overlay::text_width(text, s) / 2.0;
            o.rect(x - 4.0 * s, y - 2.0 * s, Overlay::text_width(text, s) + 8.0 * s, 12.0 * s, [0, 0, 0, 150]);
            o.text(x, y, s, colour, text);
            y += 14.0 * s;
        };
        if self.lock.waiting > 0.3 && self.client.closed.is_none() {
            line(o, &format!("WAITING FOR {}", self.other().to_uppercase()), [255, 210, 70, 255]);
        }
        for (text, _) in &self.notices {
            line(o, text, [235, 235, 225, 255]);
        }
    }
}
