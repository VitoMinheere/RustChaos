//! Who is talking, and how it's shown: the speaker's face beside their words, and for a
//! conversation, the camera on whoever has the line (`PANEL_new_face`, `PANEL_new_text_draw`
//! and `PANEL_new_widescreen` in fallen/DDEngine/Source/panel.cpp; `EWAY_cam_converse` in
//! fallen/Source/eway.cpp).
//!
//! The faces are two sheets in every level's texture clump, `face1.tga` and `face2.tga`
//! (`TEXTURE_page_face1`, `_face2`): 64-pixel faces on the first (and a few on the second) for
//! a conversation's widescreen, 32-pixel ones on the second for messages.

use glam::{Vec2, Vec3};
use uc_world::character::person as ptype;

/// The texture pages of the two face sheets: `TEXTURE_NUM_STANDARD + 18` and `+ 19`.
pub const FACE_PAGES: [u32; 2] = [22 * 64 + 18, 22 * 64 + 19];

/// `PANEL_FACE_*`.
pub mod face {
    pub const RADIO: u8 = 0;
    pub const DARCI: u8 = 1;
    pub const ROPER: u8 = 2;
    pub const THUG_RASTA: u8 = 3;
    pub const THUG_GREY: u8 = 4;
    pub const THUG_RED: u8 = 5;
    pub const CIV_1: u8 = 6;
    pub const CIV_3: u8 = 8;
    pub const CIV_4: u8 = 9;
    pub const SLAG: u8 = 10;
    pub const FAT_SLAG: u8 = 11;
    pub const HOSTAGE: u8 = 12;
    pub const WORKMAN: u8 = 13;
    pub const BOSS_GOATIE: u8 = 14;
    pub const COP: u8 = 16;
    pub const MIB: u8 = 18;
    pub const TRAMP: u8 = 21;
}

/// `face_large[]` and `face_small[]`: which sheet, and where on it in 32-pixel steps.
const LARGE: [(u8, u8, u8); 22] = [
    (0, 4, 4), (0, 0, 0), (0, 2, 0), (0, 4, 0), (1, 0, 4), (0, 4, 2), (0, 6, 2), (0, 6, 4), (0, 4, 6), (0, 6, 6),
    (0, 0, 6), (0, 2, 6), (0, 2, 4), (1, 2, 4), (0, 6, 0), (0, 6, 0), (0, 0, 4), (0, 6, 0), (0, 0, 2), (1, 2, 6),
    (1, 4, 6), (1, 0, 6),
];
const SMALL: [(u8, u8, u8); 22] = [
    (1, 2, 2), (1, 0, 0), (1, 1, 0), (1, 2, 0), (1, 4, 0), (1, 2, 1), (1, 3, 1), (1, 3, 2), (1, 2, 3), (1, 3, 3),
    (1, 0, 3), (1, 1, 3), (1, 1, 2), (1, 5, 0), (1, 3, 0), (1, 3, 0), (1, 0, 2), (1, 0, 3), (1, 0, 1), (1, 6, 0),
    (1, 7, 0), (1, 4, 1),
];

/// Where face `f` is: (sheet 0 or 1, [u0, v0, u1, v1] on it).
pub fn face_uv(f: u8, large: bool) -> (usize, [f32; 4]) {
    let (page, u, v) = if large { LARGE } else { SMALL }[(f as usize).min(21)];
    let size = if large { 64.0 } else { 32.0 } / 256.0;
    let (u, v) = (u as f32 * 32.0 / 256.0, v as f32 * 32.0 / 256.0);
    (page as usize, [u, v, u + size, v + size])
}

/// The face of someone of `PERSON_*` type `kind`. A civilian's depends on which of the four
/// civilian bodies they have (`PersonID - 6`); in Estate of Emergency the tramp is the boss
/// (`estate`, "desperate bodge").
pub fn face_of(kind: u8, civ_body: usize, estate: bool) -> u8 {
    match kind {
        ptype::DARCI => face::DARCI,
        ptype::ROPER => face::ROPER,
        ptype::COP => face::COP,
        ptype::CIV => match civ_body {
            0 => face::CIV_3,
            2 => face::CIV_4,
            _ => face::CIV_1,
        },
        ptype::THUG_RASTA => face::THUG_RASTA,
        ptype::THUG_GREY => face::THUG_GREY,
        ptype::THUG_RED => face::THUG_RED,
        ptype::SLAG_TART => face::SLAG,
        ptype::SLAG_FATUGLY => face::FAT_SLAG,
        ptype::HOSTAGE => face::HOSTAGE,
        ptype::MECHANIC => face::WORKMAN,
        ptype::TRAMP if estate => face::BOSS_GOATIE,
        ptype::TRAMP => face::TRAMP,
        _ => face::MIB,
    }
}

/// `calc_angle`: the angle (0..2048) whose `(SIN, COS)` points along `d`.
fn calc_angle(d: Vec2) -> f32 {
    d.x.atan2(d.y).rem_euclid(std::f32::consts::TAU) * 2048.0 / std::f32::consts::TAU
}

fn wrap(a: f32) -> f32 {
    (a + 1024.0).rem_euclid(2048.0) - 1024.0
}

/// `EWAY_cam_converse`: where to put the camera for `speaker` (feet at `s`, facing engine angle
/// `angle`) talking to `listener` (feet at `l`), and what it looks at. `clear` says whether
/// the camera could see from one point to another; `rand` is the game's.
///
/// It tries sixteen places round the speaker, as far out as the two stand apart (one and a
/// half to three squares) and a square up, and scores each: it must see the speaker's head
/// (40), the two shouldn't line up (15), and it's better from in front of the speaker (10).
/// The original means to score seeing the listener too (25), but the check it makes is
/// whether it *can't* see the speaker, which every place left can, so it never counts. Its
/// "too near a wall" pass halves the places either side of the first one, comparing indices
/// where it meant scores; that's kept. Its check for sitting inside a prop isn't here.
pub fn converse_camera(s: Vec3, angle: f32, l: Vec3, clear: impl Fn(Vec3, Vec3) -> bool, rand: &mut impl FnMut() -> u32) -> (Vec3, Vec3) {
    const ANGLES: usize = 16;
    let d = Vec2::new(l.x - s.x, l.z - s.z).abs();
    let dist = (d.max_element() + d.min_element() / 2.0).clamp(0x180 as f32, 0x300 as f32);
    let head = s + Vec3::new(0.0, 0x60 as f32, 0.0);
    let facing = angle * 2048.0 / std::f32::consts::TAU;
    let mut score = [0u8; ANGLES];
    for (i, sc) in score.iter_mut().enumerate() {
        let a = (i * 2048 / ANGLES) as f32 * std::f32::consts::TAU / 2048.0;
        let at = s + Vec3::new(a.sin() * dist, 0x100 as f32, a.cos() * dist);
        if !clear(at, head) {
            continue;
        }
        *sc += 40;
        let a1 = calc_angle(Vec2::new(s.x - at.x, s.z - at.z));
        let a2 = calc_angle(Vec2::new(l.x - at.x, l.z - at.z));
        if wrap(a2 - a1).abs() > 100.0 {
            *sc += 15;
        }
        if wrap(a1 - facing).abs() < 400.0 {
            *sc += 10;
        }
    }
    for (i, sc) in score.iter_mut().enumerate() {
        let (a, b) = ((i + ANGLES - 1) % ANGLES, (i + 1) % ANGLES);
        if a == 0 || b == 0 {
            *sc >>= 1;
        }
    }
    let best = (0..ANGLES).filter(|&i| score[i] > 0).fold(None, |b: Option<usize>, i| match b {
        Some(j) if score[j] >= score[i] => Some(j),
        _ => Some(i),
    });
    let Some(best) = best else {
        // Emergency: over the pair of them.
        let mid = (s + l) / 2.0;
        return (mid + Vec3::new(0.0, 0x300 as f32, 0.0), mid);
    };
    // A little off the chosen angle, at random, and aimed at somewhere up to halfway from the
    // speaker to the listener.
    let division = 2048 / ANGLES as u32;
    let a = ((best as u32 * division + (rand() & (division - 1))) as f32 - (division / 2) as f32) * std::f32::consts::TAU / 2048.0;
    let t = (rand() & 127) as f32 / 256.0;
    let base = s + (l - s) * t;
    let target = base + Vec3::new(0.0, 0xa0 as f32, 0.0);
    let cam = base + Vec3::new(a.sin() * dist, 0x100 as f32, a.cos() * dist);
    (cam, target)
}
