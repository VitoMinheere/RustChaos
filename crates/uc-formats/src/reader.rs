//! Little-endian byte cursor. All Urban Chaos files are packed (`/Zp1`) and little-endian.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at offset {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn error<T>(&self, message: impl Into<String>) -> Result<T> {
        Err(ParseError { offset: self.pos, message: message.into() })
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.remaining() {
            return self.error(format!("need {n} bytes, {} left", self.remaining()));
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    pub fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    /// A fixed-size, NUL-terminated Latin-1 string field.
    pub fn cstr(&mut self, n: usize) -> Result<String> {
        let b = self.take(n)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(n);
        Ok(latin1(&b[..end]))
    }

    /// Fails unless the whole input has been consumed.
    pub fn expect_end(&self) -> Result<()> {
        if self.remaining() != 0 {
            return self.error(format!("{} bytes left over", self.remaining()));
        }
        Ok(())
    }
}

pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&c| c as char).collect()
}

/// Reads a non-negative count stored as a signed 32-bit value.
pub fn count(r: &mut Reader, v: i32) -> Result<usize> {
    if v < 0 {
        return r.error(format!("negative count {v}"));
    }
    Ok(v as usize)
}

/// Little-endian byte writer, the counterpart of [`Reader`].
#[derive(Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
}

impl Writer {
    pub fn u8(&mut self, v: u8) { self.bytes.push(v); }
    pub fn i8(&mut self, v: i8) { self.bytes.push(v as u8); }
    pub fn u16(&mut self, v: u16) { self.bytes.extend_from_slice(&v.to_le_bytes()); }
    pub fn i16(&mut self, v: i16) { self.bytes.extend_from_slice(&v.to_le_bytes()); }
    pub fn u32(&mut self, v: u32) { self.bytes.extend_from_slice(&v.to_le_bytes()); }
    pub fn i32(&mut self, v: i32) { self.bytes.extend_from_slice(&v.to_le_bytes()); }
    pub fn raw(&mut self, b: &[u8]) { self.bytes.extend_from_slice(b); }

    /// A fixed-size string field: Latin-1 text, a NUL, then `tail` (original bytes after the
    /// terminator, if preserved) and zero padding. Fails if the text doesn't fit.
    pub fn cstr(&mut self, text: &str, tail: &[u8], size: usize) -> std::result::Result<(), String> {
        let b = to_latin1(text)?;
        if b.len() >= size {
            return Err(format!("string of {} bytes doesn't fit a {size}-byte field: {text:?}", b.len()));
        }
        let start = self.bytes.len();
        self.raw(&b);
        self.u8(0);
        let room = size - b.len() - 1;
        self.raw(&tail[..tail.len().min(room)]);
        self.bytes.resize(start + size, 0);
        Ok(())
    }
}

/// Encodes text as Latin-1 (the game's character set).
pub fn to_latin1(s: &str) -> std::result::Result<Vec<u8>, String> {
    s.chars()
        .map(|c| u8::try_from(c as u32).map_err(|_| format!("character {c:?} is not Latin-1")))
        .collect()
}
