//! Leaves and cans in the streets (`DIRT_set_focus`, `DIRT_gust`): lying round the player
//! from the start, blown about as she runs through them, and snow in the snow. Skipped
//! without an install.

use uc_game::dirt::{Kind, FOCUS_RADIUS};
use uc_game::{Controls, Game};

use crate::common;
use common::DT;

fn kinds(g: &Game, kind: Kind) -> usize {
    g.dirt.pieces.iter().flatten().filter(|p| p.kind == kind).count()
}

#[test]
fn leaves_and_cans_round_darci() {
    let Some((level, mut g)) = common::begin("bankbomb1") else { return };
    let me = g.player().unwrap().fig.pos;
    assert!(kinds(&g, Kind::Leaf) > 300, "{} leaves", kinds(&g, Kind::Leaf));
    assert!(kinds(&g, Kind::Can) > 0, "no cans");
    assert_eq!(kinds(&g, Kind::Snow), 0);
    for p in g.dirt.pieces.iter().flatten() {
        let (dx, dz) = ((p.x as f32 - me.x).abs(), (p.z as f32 - me.z).abs());
        assert!(dx.max(dz) <= FOCUS_RADIUS as f32 * 1.2, "a piece {dx},{dz} away");
        assert!(!level.map.square((p.x >> 8) as usize, (p.z >> 8) as usize).has(uc_formats::iam::flags::HIDDEN) || p.y > 0, "dirt inside a building");
    }
    // Running about, she sets some of it blowing.
    let mut blown = false;
    for _ in 0..90 {
        g.update(DT, &Controls { wish: glam::vec2(0.0, 1.0), run: true, ..Default::default() }, &level);
        blown |= g.dirt.pieces.iter().flatten().any(|p| !p.still);
    }
    assert!(blown, "nothing stirred");
}

#[test]
fn snow_in_the_snow() {
    let Some((_, g)) = common::begin("snow2") else { return };
    assert!(kinds(&g, Kind::Snow) > 300, "{} flakes", kinds(&g, Kind::Snow));
    assert_eq!(kinds(&g, Kind::Leaf) + kinds(&g, Kind::Can), 0);
}
