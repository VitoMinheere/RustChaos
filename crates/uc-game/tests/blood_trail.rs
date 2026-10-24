//! Blood trails (#308, `TRACKS_Bleed`): someone below a quarter of their health leaves
//! splashes of blood by their feet now and then, the more often the less they have left;
//! the healthy don't. Skipped without an install.

mod common;

use uc_game::Controls;
use uc_world::geometry::BLOODSPLAT_PAGE;

fn splashes(r: &common::Run) -> Vec<glam::Vec3> {
    r.g.tracks.list.iter().filter(|t| t.page == BLOODSPLAT_PAGE).map(|t| t.at).collect()
}

#[test]
fn the_badly_hurt_bleed() {
    let Some(mut r) = common::start("police1") else { return };
    assert!(r.level.texture(BLOODSPLAT_PAGE).is_some(), "no bludsplt.tga");
    r.heal = false;
    r.play(1.0, Controls::default());
    let me = r.player();
    let at = r.g.people[me].fig.pos;
    assert!(splashes(&r).is_empty(), "bleeding at full health");
    for _ in 0..(10.0 / common::DT) as usize {
        let p = &mut r.g.people[me];
        p.health = p.max_health / 10;
        r.play(common::DT, Controls::default());
    }
    let s = splashes(&r);
    assert!(s.len() >= 3, "only {} splashes in 10 s", s.len());
    for p in &s {
        assert!((*p - at).length() < 64.0, "a splash at {p}, her at {at}");
    }
}
