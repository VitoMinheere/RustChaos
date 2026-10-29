//! Messages as bytes: each is its length (4 bytes) then a tag byte and its fields, little
//! endian, strings as a 2-byte length then UTF-8.

use crate::{Input, Msg, Setup};
use std::io::{self, Read, Write};

/// No message is anywhere near this long; a length past it means the stream is garbage.
const LONGEST: usize = 64 * 1024;

mod tag {
    pub const HELLO: u8 = 1;
    pub const CHOOSE: u8 = 2;
    pub const INPUT: u8 = 3;
    pub const HASH: u8 = 4;
    pub const PONG: u8 = 5;
    pub const WELCOME: u8 = 10;
    pub const REFUSED: u8 = 11;
    pub const LOBBY: u8 = 12;
    pub const START: u8 = 13;
    pub const TURN: u8 = 14;
    pub const NOTICE: u8 = 15;
    pub const PING: u8 = 16;
}

pub(crate) struct Out(pub(crate) Vec<u8>);

impl Out {
    pub(crate) fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub(crate) fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    pub(crate) fn str(&mut self, s: &str) {
        let b = &s.as_bytes()[..s.len().min(u16::MAX as usize)];
        self.u16(b.len() as u16);
        self.0.extend_from_slice(b);
    }
    pub(crate) fn input(&mut self, i: &Input) {
        self.f32(i.wish[0]);
        self.f32(i.wish[1]);
        self.f32(i.drive[0]);
        self.f32(i.drive[1]);
        self.u16(i.buttons);
        for v in [i.strafe, i.look] {
            match v {
                Some([x, z]) => {
                    self.u8(1);
                    self.f32(x);
                    self.f32(z);
                }
                None => self.u8(0),
            }
        }
    }
    pub(crate) fn setup(&mut self, s: &Setup) {
        self.str(&s.mission);
        self.u8(s.difficulty);
        self.0.extend_from_slice(&s.stats);
    }
}

pub(crate) struct In<'a>(pub(crate) &'a [u8]);

pub(crate) fn short() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "message too short")
}

impl In<'_> {
    pub(crate) fn bytes(&mut self, n: usize) -> io::Result<&[u8]> {
        if self.0.len() < n {
            return Err(short());
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    pub(crate) fn u8(&mut self) -> io::Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    pub(crate) fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    pub(crate) fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    pub(crate) fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    pub(crate) fn f32(&mut self) -> io::Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
    pub(crate) fn str(&mut self) -> io::Result<String> {
        let n = self.u16()? as usize;
        String::from_utf8(self.bytes(n)?.to_vec()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    pub(crate) fn input(&mut self) -> io::Result<Input> {
        self.input_of(4)
    }
    /// An input as replay version `version` had it: from 3 with the strafe key (#309), from 4
    /// with looking round.
    pub(crate) fn input_of(&mut self, version: u8) -> io::Result<Input> {
        let mut i = Input { wish: [self.f32()?, self.f32()?], drive: [self.f32()?, self.f32()?], buttons: self.u16()?, strafe: None, look: None };
        if version >= 3 && self.u8()? != 0 {
            i.strafe = Some([self.f32()?, self.f32()?]);
        }
        if version >= 4 && self.u8()? != 0 {
            i.look = Some([self.f32()?, self.f32()?]);
        }
        Ok(i)
    }
    pub(crate) fn setup(&mut self) -> io::Result<Setup> {
        Ok(Setup { mission: self.str()?, difficulty: self.u8()?, stats: self.bytes(4)?.try_into().unwrap() })
    }
}

pub(crate) fn encode(m: &Msg) -> Vec<u8> {
    let mut o = Out(vec![0; 4]);
    match m {
        Msg::Hello { protocol, build, name } => {
            o.u8(tag::HELLO);
            o.u16(*protocol);
            o.str(build);
            o.str(name);
        }
        Msg::Choose(s) => {
            o.u8(tag::CHOOSE);
            o.setup(s);
        }
        Msg::Input { game, turn, input } => {
            o.u8(tag::INPUT);
            o.u32(*game);
            o.u32(*turn);
            o.input(input);
        }
        Msg::Hash { game, turn, hash } => {
            o.u8(tag::HASH);
            o.u32(*game);
            o.u32(*turn);
            o.u64(*hash);
        }
        Msg::Pong { id } => {
            o.u8(tag::PONG);
            o.u32(*id);
        }
        Msg::Ping { id } => {
            o.u8(tag::PING);
            o.u32(*id);
        }
        Msg::Welcome { seat } => {
            o.u8(tag::WELCOME);
            o.u8(*seat);
        }
        Msg::Refused { why } => {
            o.u8(tag::REFUSED);
            o.str(why);
        }
        Msg::Lobby { names } => {
            o.u8(tag::LOBBY);
            o.u8(names.len() as u8);
            for n in names {
                o.str(n);
            }
        }
        Msg::Start { game, delay, setup } => {
            o.u8(tag::START);
            o.u32(*game);
            o.u8(*delay);
            o.setup(setup);
        }
        Msg::Turn { turn, inputs } => {
            o.u8(tag::TURN);
            o.u32(*turn);
            o.u8(inputs.len() as u8);
            for i in inputs {
                o.input(i);
            }
        }
        Msg::Notice { text } => {
            o.u8(tag::NOTICE);
            o.str(text);
        }
    }
    let n = (o.0.len() - 4) as u32;
    o.0[..4].copy_from_slice(&n.to_le_bytes());
    o.0
}

pub(crate) fn decode(b: &[u8]) -> io::Result<Msg> {
    let mut i = In(b);
    Ok(match i.u8()? {
        tag::HELLO => Msg::Hello { protocol: i.u16()?, build: i.str()?, name: i.str()? },
        tag::CHOOSE => Msg::Choose(i.setup()?),
        tag::INPUT => Msg::Input { game: i.u32()?, turn: i.u32()?, input: i.input()? },
        tag::HASH => Msg::Hash { game: i.u32()?, turn: i.u32()?, hash: i.u64()? },
        tag::WELCOME => Msg::Welcome { seat: i.u8()? },
        tag::REFUSED => Msg::Refused { why: i.str()? },
        tag::LOBBY => {
            let n = i.u8()?;
            Msg::Lobby { names: (0..n).map(|_| i.str()).collect::<io::Result<_>>()? }
        }
        tag::START => Msg::Start { game: i.u32()?, delay: i.u8()?, setup: i.setup()? },
        tag::PONG => Msg::Pong { id: i.u32()? },
        tag::PING => Msg::Ping { id: i.u32()? },
        tag::TURN => {
            let turn = i.u32()?;
            let n = i.u8()?;
            Msg::Turn { turn, inputs: (0..n).map(|_| i.input()).collect::<io::Result<_>>()? }
        }
        tag::NOTICE => Msg::Notice { text: i.str()? },
        t => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown message {t}"))),
    })
}

/// Writes one message, in one write.
pub fn write_msg(w: &mut impl Write, m: &Msg) -> io::Result<()> {
    w.write_all(&encode(m))?;
    w.flush()
}

/// Reads one message, waiting for all of it.
pub fn read_msg(r: &mut impl Read) -> io::Result<Msg> {
    let mut len = [0; 4];
    r.read_exact(&mut len)?;
    let n = u32::from_le_bytes(len) as usize;
    if n > LONGEST {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("message of {n} bytes")));
    }
    let mut b = vec![0; n];
    r.read_exact(&mut b)?;
    decode(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Replays from before strafing (#309) have inputs without it.
    #[test]
    fn inputs_from_before_strafing_still_read() {
        let mut o = Out(Vec::new());
        for v in [0.5, -1.0, 1.0, 0.0] {
            o.f32(v);
        }
        o.u16(7);
        let i = In(&o.0).input_of(2).unwrap();
        assert_eq!(i, Input { wish: [0.5, -1.0], drive: [1.0, 0.0], buttons: 7, strafe: None, look: None });
    }

    /// Replays from before looking round went with the input have the strafe key and no more.
    #[test]
    fn inputs_from_before_looking_round_still_read() {
        let mut o = Out(Vec::new());
        for v in [0.5, -1.0, 1.0, 0.0] {
            o.f32(v);
        }
        o.u16(7);
        o.u8(1);
        o.f32(0.6);
        o.f32(-0.8);
        let i = In(&o.0).input_of(3).unwrap();
        assert_eq!(i, Input { wish: [0.5, -1.0], drive: [1.0, 0.0], buttons: 7, strafe: Some([0.6, -0.8]), look: None });
    }

    #[test]
    fn every_message_round_trips() {
        let input = Input { wish: [0.5, -1.0], drive: [1.0, 0.0], buttons: 0x1234, strafe: Some([0.6, -0.8]), look: Some([1.5, -0.2]) };
        let setup = Setup { mission: "police1".into(), difficulty: 2, stats: [1, 2, 3, 4] };
        let all = [
            Msg::Hello { protocol: 7, build: "abc-dirty".into(), name: "Vito".into() },
            Msg::Choose(setup.clone()),
            Msg::Input { game: 2, turn: 99, input },
            Msg::Hash { game: 2, turn: 30, hash: 0xDEAD_BEEF_0123_4567 },
            Msg::Welcome { seat: 1 },
            Msg::Refused { why: "full".into() },
            Msg::Lobby { names: vec!["A".into(), String::new()] },
            Msg::Start { game: 3, delay: 4, setup },
            Msg::Ping { id: 9 },
            Msg::Pong { id: 9 },
            Msg::Turn { turn: 5, inputs: vec![input, Input::default()] },
            Msg::Notice { text: "Roper left".into() },
        ];
        let mut stream = Vec::new();
        for m in &all {
            write_msg(&mut stream, m).unwrap();
        }
        let mut r = &stream[..];
        for m in &all {
            assert_eq!(&read_msg(&mut r).unwrap(), m);
        }
        assert!(read_msg(&mut r).is_err());
    }
}
