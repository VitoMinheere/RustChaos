//! The original's units, and their conversions.
//!
//! - Angles are 2048ths of a turn (`& 2047` wraps one), as in its `SIN`/`COS` tables.
//! - The game runs 20 turns a second.
//! - A map square is 256 world units (`0x100`), and positions are often in 256ths of a world
//!   unit (8.8 fixed point).

use std::f32::consts::TAU;

/// The game's turns (`TICKS_PER_SECOND`) a second.
pub const TURNS_PER_SECOND: f32 = 20.0;

/// World units a map square.
pub const SQUARE: f32 = 256.0;

/// An angle in 2048ths of a turn, in radians.
///
/// Written so it gives exactly what `a * TAU / 2048.0` does inline.
#[inline]
pub fn radians(a: f32) -> f32 {
    a * TAU / 2048.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quarter_turn_is_512() {
        assert_eq!(radians(512.0), TAU / 4.0);
        assert_eq!(radians(2048.0), TAU);
    }
}
