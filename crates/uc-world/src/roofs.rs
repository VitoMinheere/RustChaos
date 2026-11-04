//! Rooftop tiles by the shape of the roof round a square, for roofs whose own textures never
//! shipped: a warehouse's, which the editor kept in its `.map` files (`WARE_roof_tex`,
//! ware.cpp:242). The released maps' rooftops are laid out from a handful of pages each
//! world has: a plain middle, shaded asphalt along a raised part, and a kerb along the roof's
//! edge, with their corners, each turned to the side it's on. [`crate::rooftiles`] has, for
//! each world, the page and turn its rooftops have most for each shape, from the released
//! maps (`uc textures roofs`), and [`tile`] looks one up.

use std::collections::HashMap;
use uc_formats::iam::{flags, Map, MAP_SIZE};

/// A shape's tile in [`crate::rooftiles::ROOF_TILES`]: the shape upright ([`Shape::upright`]),
/// the page, and its turn.
pub type RoofTile = (u16, u16, u8);

/// What's next to a square of roof, on one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The same roof, at the same height.
    Same = 0,
    /// Lower, or no roof: the roof's edge.
    Lower = 1,
    /// Higher: a raised part of the roof, or a wall going up.
    Higher = 2,
}

/// The sides in turn: +z, +x, -z, -x. A tile turned one more (`Square::rotation`, the floor's
/// `FLOOR_UVS`) has what was on one side on the next.
pub const SIDES: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

/// The shape of the roof round a square: its four sides ([`SIDES`]), and whether each corner
/// between two sides the same roof is a notch, the diagonal square not the same (an inside
/// corner). As a number: two bits a side, then a bit a corner, the corner between side `i` and
/// side `i + 1` at bit `8 + i`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Shape(pub u16);

impl Shape {
    pub fn new(sides: [Side; 4], notches: [bool; 4]) -> Shape {
        let mut v = 0u16;
        for i in 0..4 {
            v |= (sides[i] as u16) << (2 * i);
            if notches[i] && sides[i] == Side::Same && sides[(i + 1) % 4] == Side::Same {
                v |= 1 << (8 + i);
            }
        }
        Shape(v)
    }

    fn side(self, i: usize) -> u16 {
        (self.0 >> (2 * i)) & 3
    }

    fn notch(self, i: usize) -> u16 {
        (self.0 >> (8 + i)) & 1
    }

    /// Turned once: what was on side `i` on side `i + 1`.
    fn turned(self) -> Shape {
        let mut v = 0;
        for i in 0..4 {
            v |= self.side(i) << (2 * ((i + 1) % 4));
            v |= self.notch(i) << (8 + (i + 1) % 4);
        }
        Shape(v)
    }

    /// The shape turned to the way it's kept in the table, the least of its four turns, and
    /// how many turns that took.
    pub fn upright(self) -> (Shape, u8) {
        let mut best = (self, 0);
        let mut s = self;
        for k in 1..4 {
            s = s.turned();
            if s.0 < best.0 .0 {
                best = (s, k);
            }
        }
        best
    }

    /// Simpler shapes to try for one the table hasn't got, in turn: without its notches, then
    /// a raised part taken as an edge too, then the middle of a roof.
    fn simpler(self) -> [Shape; 3] {
        let plain = Shape(self.0 & 0xff);
        let mut edges = 0;
        for i in 0..4 {
            edges |= plain.side(i).min(1) << (2 * i);
        }
        [plain, Shape(edges), Shape(0)]
    }
}

/// How high the top of what's on each square is (world units): its rooftop
/// (`ROOF_EXISTS`) or its highest roof face. Squares with neither are the ground.
pub struct Tops(HashMap<(i32, i32), i32>);

impl Tops {
    pub fn new(map: &Map) -> Tops {
        let mut tops = HashMap::new();
        for x in 0..MAP_SIZE {
            for z in 0..MAP_SIZE {
                let sq = map.square(x, z);
                if sq.has(flags::ROOF_EXISTS) {
                    tops.insert((x as i32, z as i32), sq.height as i32 * 64);
                }
            }
        }
        for rf in map.roof_faces.iter().skip(1) {
            let top = tops.entry(((rf.rx & 127) as i32, (rf.rz & 127) as i32)).or_insert(i32::MIN);
            *top = (*top).max(rf.y as i32);
        }
        Tops(tops)
    }

    fn at(&self, x: i32, z: i32) -> Option<i32> {
        self.0.get(&(x, z)).copied()
    }

    /// The shape round (x, z), whose roof is at height `y`.
    pub fn shape(&self, x: i32, z: i32, y: i32) -> Shape {
        let side = |dx: i32, dz: i32| match self.at(x + dx, z + dz) {
            Some(h) if (h - y).abs() < 16 => Side::Same,
            Some(h) if h > y => Side::Higher,
            _ => Side::Lower,
        };
        let sides = SIDES.map(|(dx, dz)| side(dx, dz));
        let notches = std::array::from_fn(|i| {
            let ((ax, az), (bx, bz)) = (SIDES[i], SIDES[(i + 1) % 4]);
            side(ax + bx, az + bz) != Side::Same
        });
        Shape::new(sides, notches)
    }
}

/// The page and turn ([`uc_formats::iam::Square::rotation`]) world `world`'s rooftops have
/// for a square of shape `shape`, from [`crate::rooftiles::ROOF_TILES`], if the table has the
/// world. `has_page` says which pages can be drawn.
pub fn tile(world: i32, shape: Shape, has_page: impl Fn(u32) -> bool) -> Option<(u32, u8)> {
    let (_, tiles) = crate::rooftiles::ROOF_TILES.iter().find(|(w, _)| *w == world)?;
    let (upright, k) = shape.upright();
    let find = |s: Shape| tiles.iter().find(|t| t.0 == s.0 && has_page(t.1 as u32)).map(|t| (t.1 as u32, t.2));
    // The simpler shapes are of the upright one, so the same turn takes them back.
    let found = find(upright).or_else(|| upright.simpler().into_iter().find_map(find))?;
    Some((found.0, (found.1 + 4 - k) % 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_moves_each_side_on() {
        let s = Shape::new([Side::Lower, Side::Same, Side::Same, Side::Higher], [false; 4]);
        let t = s.turned();
        assert_eq!(t, Shape::new([Side::Higher, Side::Lower, Side::Same, Side::Same], [false; 4]));
        assert_eq!(t.turned().turned().turned(), s);
    }

    #[test]
    fn every_turn_of_a_shape_is_kept_the_same_way() {
        let s = Shape::new([Side::Lower, Side::Same, Side::Same, Side::Same], [false, true, false, false]);
        let (up, k) = s.upright();
        let mut t = s;
        for j in 0..4 {
            let (u, m) = t.upright();
            assert_eq!(u, up);
            // Turned j more, it takes j fewer turns back.
            assert_eq!((m + j) % 4, k);
            t = t.turned();
        }
    }
}
