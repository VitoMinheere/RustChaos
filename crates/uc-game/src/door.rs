//! Doors swinging open (fallen/Source/door.cpp). A door the mission opens
//! ([`uc_world::ground::Ground::open_door_near`]) lets people through at once, and is drawn
//! swinging round on its first end until it's all but fully open (`DOOR_process`, then
//! `FACET_draw`'s `p_facet->Open`, DDEngine/Source/facet.cpp:4009). The level leaves the
//! doors out of its static geometry for this ([`uc_world::Level::doors`]).

use glam::{vec2, Vec2};
use std::collections::BTreeMap;
use uc_formats::iam::facet_flags;
use uc_world::ground::Ground;
use uc_world::level::DoorQuad;
use uc_world::{Batch, Level};

/// `DOOR_AJAR`: how far short of the whole way a door stops (door.cpp:409).
const AJAR: f32 = 15.0;
/// How fast a door swings, in `DFacet::Open`'s units (256 to half a turn) a second.
/// `DOOR_process` moves it 4 a tick, 20 ticks a second, for each of `DOOR_door`'s four
/// slots it's in, and `DOOR_open` puts it in every free one rather than the first
/// (door.cpp:148): so all four, with no other door on the move.
const SPEED: f32 = 4.0 * 4.0 * 20.0;

/// One of the map's doors, as it's drawn.
pub struct Door {
    /// Its quads, shut.
    quads: Vec<DoorQuad>,
    /// What it swings round, in map squares: the facet's first end (`FACET_draw`'s `sx`,
    /// `sz`).
    hinge: Vec2,
    /// How far open it is (`DFacet::Open`): 256 is half a turn.
    pub open: f32,
    /// As far as it goes: `DOOR_AJAR` short of half a turn, or of a quarter for a door
    /// that opens only by 90 degrees (`FACET_FLAG_90DEGREE`, door.cpp:411).
    pub max: f32,
}

/// The map's doors, one for each of [`Ground::doors`].
#[derive(Default)]
pub struct Doors {
    pub doors: Vec<Door>,
}

impl Doors {
    pub fn new(level: &Level) -> Doors {
        let doors = level.ground.doors.iter().zip(level.doors()).map(|(d, quads)| {
            let ninety = level.map.facets[d.facet].flags & facet_flags::NINETY_DEGREE != 0;
            let max = if ninety { 128.0 } else { 255.0 } - AJAR;
            Door { quads, hinge: Vec2::from(d.wall.a) / uc_world::UNIT, open: 0.0, max }
        }).collect();
        Doors { doors }
    }

    /// `DOOR_process`: each door the mission has opened swings on until it's as far open as
    /// it goes. Nothing shuts one again (`DOOR_shut` has no caller).
    pub fn process(&mut self, dt: f32, ground: &Ground) {
        for (d, g) in self.doors.iter_mut().zip(&ground.doors) {
            if g.is_open() {
                d.open = (d.open + SPEED * dt).min(d.max);
            }
        }
    }

    /// Each door turned round its hinge by how far open it is, as `FACET_draw` turns the step
    /// along the facet (facet.cpp:4013).
    pub fn batches(&self, out: &mut BTreeMap<u32, Batch>) {
        for d in &self.doors {
            let (s, c) = (d.open * std::f32::consts::PI / 256.0).sin_cos();
            let turn = |v: Vec2| vec2(c * v.x + s * v.y, -s * v.x + c * v.y);
            for q in &d.quads {
                let p = q.corners.map(|p| {
                    let at = d.hinge + turn(vec2(p.x, p.z) - d.hinge);
                    glam::vec3(at.x, p.y, at.y)
                });
                out.entry(q.page).or_default().quad(p, q.uv, q.colour);
            }
        }
    }
}
