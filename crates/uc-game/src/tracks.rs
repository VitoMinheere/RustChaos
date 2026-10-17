//! Footprints and tyre tracks (fallen/Source/tracks.cpp): marks laid flat on the ground, kept
//! in a ring of the last [`BUFFER`] and fading as newer ones are laid.
//!
//! A person's prints and a bike's tracks carry what they last stepped in (`muckyfootprint`,
//! `tyrelast`): the kind of surface and how fresh it is. Off it, each step or turn of the
//! wheel lays a fainter mark, until there's nothing left on the feet or the tyre. Puddles
//! would wet the feet too (`TRACK_SURFACE_WATER`), but there aren't any yet.

use std::collections::VecDeque;
use glam::{Vec2, Vec3};
use uc_formats::iam::{Map, MAP_SIZE};

/// `TRACK_BUFFER_LENGTH`: the marks kept, the oldest giving way to the newest.
pub const BUFFER: usize = 300;

pub use uc_world::geometry::{FOOTPRINT_PAGE, TYREMARK_PAGE, TYRETRACK_PAGE};

/// `TRACK_MUDDY_COLOUR`.
const MUDDY: [u8; 3] = [0x48, 0x20, 0x00];
/// `TRACK_ONSNOW_COLOUR`.
const ON_SNOW: [u8; 3] = [0, 0, 0];

/// `TRACK_SURFACE_*`: what's on the feet or the tyre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    None = 0,
    Muddy = 1,
    OnSnow = 3,
}

impl Surface {
    fn of(code: u8) -> Surface {
        match code {
            1 => Surface::Muddy,
            3 => Surface::OnSnow,
            _ => Surface::None,
        }
    }

    fn colour(self) -> [u8; 3] {
        if self == Surface::OnSnow { ON_SNOW } else { MUDDY }
    }
}

/// `TRACK_TYPE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `TRACK_TYPE_TYRE`: a muddy tyre.
    Tyre,
    /// `TRACK_TYPE_TYRE_SKID`: a car's skid.
    Skid,
    LeftPrint,
    RightPrint,
}

/// A foot coming down (`person_splash`'s limb): each step one foot, landing both (`-1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foot {
    Left,
    Right,
    Both,
}

/// `Track`: one mark, a quad on the ground.
#[derive(Debug, Clone, Copy)]
pub struct Track {
    /// One end of it, world units.
    pub at: Vec3,
    /// From there to the other end.
    pub d: Vec3,
    /// Half its width, across `d` (`sx`, `sz`).
    pub side: Vec2,
    pub page: u32,
    pub colour: [u8; 3],
    /// Taken off its alpha from the start: how faded what laid it was (`colour >> 24`).
    pub faded: u8,
    /// A print: which half of the page, flipped for the right foot (`TRACK_FLAGS_FLIPABLE`).
    pub flip: Option<bool>,
    /// How many marks were laid before it.
    laid: u64,
}

/// Every mark on the ground.
#[derive(Debug, Clone, Default)]
pub struct Tracks {
    pub list: VecDeque<Track>,
    /// Marks laid so far (`track_head`).
    laid: u64,
}

impl Tracks {
    /// `TRACKS_AddQuad`: a mark from `at` along `d`, `width` either side.
    #[allow(clippy::too_many_arguments)]
    pub fn add_quad(&mut self, at: Vec3, d: Vec3, page: u32, colour: [u8; 3], faded: u8, width: f32, flip: Option<bool>) {
        // `TRACKS_CalcDiffs`: square to `d`, `width` long.
        let across = Vec2::new(d.z, -d.x);
        let side = if across.length() < 1.0 { Vec2::new(width, 0.0) } else { across.normalize() * width };
        if self.list.len() == BUFFER {
            self.list.pop_front();
        }
        self.list.push_back(Track { at, d, side, page, colour, faded, flip, laid: self.laid });
        self.laid += 1;
    }

    /// `TRACKS_Add`: a mark of `kind` from `at` along `d`, where the ground is `ground`, for
    /// something that last stepped in what `last` says (`(freshness << 8) + surface`).
    /// Returns what it carries on with. On muck or snow it lays a mark only from the second
    /// step on, the first having only dirtied the feet.
    pub fn add(&mut self, at: Vec3, d: Vec3, kind: Kind, last: u16, ground: Surface) -> u16 {
        let (mut age, mut lastkind) = ((last >> 8) as u8, Surface::of(last as u8));
        let (page, flip) = match kind {
            Kind::Skid => {
                self.add_quad(at, d, TYREMARK_PAGE, [0, 0, 0], 0, 10.0, None);
                return last;
            }
            // Not moving more than 5 a turn, nothing changes (the original goes on with a
            // surface it never set).
            Kind::Tyre if d.x * d.x + d.z * d.z <= 25.0 => return last,
            Kind::Tyre => (TYRETRACK_PAGE, None),
            Kind::LeftPrint => (FOOTPRINT_PAGE, Some(false)),
            Kind::RightPrint => (FOOTPRINT_PAGE, Some(true)),
        };
        if lastkind != Surface::None {
            self.add_quad(at, d, page, lastkind.colour(), 255 - age, 10.0, flip);
        }
        // Fade or renew.
        if ground == Surface::None {
            if age > 8 {
                age -= 8;
            } else {
                (lastkind, age) = (Surface::None, 0);
            }
        } else {
            (age, lastkind) = (255, ground);
        }
        ((age as u16) << 8) + lastkind as u16
    }

    /// How opaque `t` is drawn, 0..255 (`TRACKS_DrawTrack`): less the more marks have been
    /// laid since, and less for one laid faded.
    pub fn alpha(&self, t: &Track) -> u8 {
        let since = (self.laid - t.laid).min(255) as i32;
        (255 - since - t.faded as i32).clamp(0, 255) as u8
    }
}

/// What a footprint or a tyre picks up from the floor at (x, z) (`TRACKS_GroundAtXZ`): the
/// grass pages (69 to 74) and gravel (68, 106 to 111) are muddy. A print in a snow world is
/// always on snow (`WORLD_TYPE_SNOW`), which tyres don't notice.
pub fn surface_at(map: &Map, x: f32, z: f32, snow: bool, print: bool) -> Surface {
    if snow && print {
        return Surface::OnSnow;
    }
    let (mx, mz) = ((x / 256.0).floor(), (z / 256.0).floor());
    if mx < 0.0 || mz < 0.0 || mx >= MAP_SIZE as f32 || mz >= MAP_SIZE as f32 {
        return Surface::None;
    }
    match map.square(mx as usize, mz as usize).page() {
        69..=74 | 68 | 106..=111 => Surface::Muddy,
        _ => Surface::None,
    }
}

/// Whether (x, z) is on water (`MAV_SPARE_FLAG_WATER`), where no print is laid.
pub fn on_water(map: &Map, x: f32, z: f32) -> bool {
    let (mx, mz) = ((x / 256.0).floor(), (z / 256.0).floor());
    mx >= 0.0 && mz >= 0.0 && mx < MAP_SIZE as f32 && mz < MAP_SIZE as f32
        && map.square(mx as usize, mz as usize).page() == uc_world::level::WATER_PAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(t: &mut Tracks, last: u16, ground: Surface) -> u16 {
        t.add(Vec3::ZERO, Vec3::new(0.0, 0.0, 40.0), Kind::LeftPrint, last, ground)
    }

    /// The first step in the snow only dirties the feet; from the next on there are prints.
    #[test]
    fn prints_start_from_the_second_step() {
        let mut t = Tracks::default();
        let last = step(&mut t, 0, Surface::OnSnow);
        assert!(t.list.is_empty());
        step(&mut t, last, Surface::OnSnow);
        assert_eq!(t.list.len(), 1);
        assert_eq!(t.list[0].colour, ON_SNOW);
        assert_eq!(t.list[0].faded, 0);
    }

    /// Off the mud, each print is fainter, and after 32 steps there are no more.
    #[test]
    fn muddy_feet_fade_out() {
        let mut t = Tracks::default();
        let mut last = step(&mut t, 0, Surface::Muddy);
        for _ in 0..40 {
            last = step(&mut t, last, Surface::None);
        }
        assert_eq!(t.list.len(), 32);
        assert!(t.list.iter().zip(t.list.iter().skip(1)).all(|(a, b)| b.faded > a.faded));
        assert_eq!(last, 0);
    }

    /// Only the last 300 are kept, and the older a mark the fainter it's drawn.
    #[test]
    fn the_oldest_give_way() {
        let mut t = Tracks::default();
        for _ in 0..400 {
            t.add(Vec3::ZERO, Vec3::X, Kind::Skid, 0, Surface::None);
        }
        assert_eq!(t.list.len(), BUFFER);
        assert_eq!(t.alpha(t.list.back().unwrap()), 254);
        assert_eq!(t.alpha(&t.list[0]), 0);
    }
}
