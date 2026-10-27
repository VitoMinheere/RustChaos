//! Warehouses: buildings you can walk into, and the ambience a mission gives each one
//! (`WARE_init`, `WARE_which_contains`, fallen/Source/ware.cpp; `EWAY_DO_WAREFX`).
//!
//! A warehouse is a `BUILDING_TYPE_WAREHOUSE` building. Its floorplan is the bounding box of
//! its outer walls (`STOREY_TYPE_NORMAL` facets, `WARE_bounding_box`), counting only the
//! squares a building stands on (`PAP_FLAG_HIDDEN`, `WARE_in_floorplan`). They are numbered in
//! building order, and a square in two of them belongs to the first.
//!
//! The original knows someone is inside because they walked in through one of its doorways
//! (`slide_into_warehouse` in `slide_along`). Here it's that they're within its floorplan and
//! below its roof, which comes to the same thing for anyone who got there on foot.
//!
//! What the ambience does is in `process_weather` (fallen/Source/Sound.cpp): inside, the
//! city's ambience and its wind stop, and if the warehouse has an ambience its loop plays
//! instead, chosen as you come in. `MUSIC_play_the_mode` (fallen/Source/music.cpp) puts the
//! same loop in place of the music while you're in there.

use glam::Vec3;
use uc_formats::iam::{facet_type, flags, Map, BUILDING_TYPE_WAREHOUSE};
use uc_world::ground::Ground;

/// `indoors_waves[]`, by ambience 1 to 4: the police station, a Posheeta's, an office and a
/// nightclub. The retail release ships the first three without the "MONO" of their names in
/// the sound table, and the club tune (`S_TUNE_CLUB_START`, `FeelinItMONO`) under
/// `AlbumTracks`.
pub const AMBIENCE: [&str; 4] = [
    "Ambience/Police1.wav",
    "Ambience/PosheataAmb1.wav",
    "Ambience/Office1.wav",
    "GeneralMusic/AlbumTracks/FeelinIt.wav",
];

/// One warehouse's floorplan, in map squares (inclusive).
#[derive(Debug, Clone, Copy)]
pub struct Ware {
    pub building: usize,
    pub min: (i32, i32),
    pub max: (i32, i32),
    /// `ambience`: 0 for none, 1-4 for [`AMBIENCE`].
    pub ambience: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Wares {
    pub wares: Vec<Ware>,
}

impl Wares {
    pub fn new(map: &Map) -> Wares {
        let wares = map.buildings.iter().enumerate().skip(1)
            .filter(|(_, b)| b.kind == BUILDING_TYPE_WAREHOUSE)
            .filter_map(|(i, b)| {
                let facets = map.facets.get(b.start_facet as usize..b.end_facet as usize)?;
                let walls = facets.iter().filter(|f| f.kind == facet_type::NORMAL);
                let (mut lo, mut hi) = ((i32::MAX, i32::MAX), (i32::MIN, i32::MIN));
                for f in walls {
                    for k in 0..2 {
                        let (x, z) = (f.x[k] as i32, f.z[k] as i32);
                        lo = (lo.0.min(x), lo.1.min(z));
                        hi = (hi.0.max(x), hi.1.max(z));
                    }
                }
                // `WARE_bounding_box` is exclusive at the far edge. Lazily: a warehouse with
                // no outside walls (gpost3.iam has one) leaves `hi` at its minimum.
                (lo.0 <= hi.0).then(|| Ware { building: i, min: lo, max: (hi.0 - 1, hi.1 - 1), ambience: 0 })
            })
            .collect();
        Wares { wares }
    }

    /// `WARE_which_contains`: the warehouse whose floorplan has square (x, z).
    pub fn which_contains(&self, map: &Map, x: i32, z: i32) -> Option<usize> {
        if !(0..128).contains(&x) || !(0..128).contains(&z) || !map.square(x as usize, z as usize).has(flags::HIDDEN) {
            return None;
        }
        self.wares.iter().position(|w| (w.min.0..=w.max.0).contains(&x) && (w.min.1..=w.max.1).contains(&z))
    }

    /// The warehouse someone at `at` is inside: in its floorplan and under its roof, not on
    /// it.
    pub fn under_roof(&self, map: &Map, ground: &Ground, at: Vec3) -> Option<usize> {
        self.which_contains(map, (at.x as i32) >> 8, (at.z as i32) >> 8)
            .filter(|_| ground.surface(map, at.x, at.z, 1.0e5) - at.y > 128.0)
    }
}
