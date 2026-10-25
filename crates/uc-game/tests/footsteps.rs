//! What footsteps sound like (#299, `footstep_wave`): a page the textures' `soundfx.ini` puts
//! with grass sounds of grass underfoot; road elsewhere. Skipped without an install.

use glam::vec2;
use uc_game::steps::{Steps, ROAD};
use uc_game::{Controls, Sfx};

mod common;

/// The footstep sounds while she walks round in a slow circle for `secs`.
fn steps_heard(r: &mut common::Run, secs: f32) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for k in 0..(secs / common::DT) as usize {
        let a = k as f32 * common::DT * 0.5;
        r.play(common::DT, Controls { wish: vec2(a.cos(), a.sin()), ..Default::default() });
        out.extend(r.g.take_sounds().into_iter().filter_map(|(s, _)| match s {
            Sfx::Footstep { player: true, sound } => sound,
            _ => None,
        }));
    }
    out
}

#[test]
fn grass_sounds_of_grass() {
    let Some(level) = common::load("Assault1") else { return };
    let steps = Steps::load(&level.game, level.map.texture_set);
    let grass = (0..1024u32).find(|&p| steps.on_page(p) == (46, 49)).expect("a grass page");
    let road = (0..1024u32).find(|&p| steps.on_page(p) == ROAD).expect("a road page");
    let mut r = common::Run::new(level);
    // After the opening talk with the instructor.
    r.play(40.0, Controls::default());
    assert!(r.g.indoors().is_none(), "she starts indoors");
    let at = r.g.people[r.player()].fig.pos;
    let paint = |r: &mut common::Run, page: u32| {
        let (cx, cz) = ((at.x / 256.0) as i32, (at.z / 256.0) as i32);
        for x in (cx - 8).max(0)..(cx + 8).min(127) {
            for z in (cz - 8).max(0)..(cz + 8).min(127) {
                let sq = &mut r.level.map.squares[x as usize * 128 + z as usize];
                sq.texture = (sq.texture & !0x3ff) | page as u16;
            }
        }
    };
    paint(&mut r, grass);
    let heard = steps_heard(&mut r, 4.0);
    assert!(!heard.is_empty(), "no footsteps");
    // Puddles are wet whatever's under them; the training ground is dry by day.
    assert!(heard.iter().all(|&s| s == (46, 49)), "{heard:?}");
    paint(&mut r, road);
    let heard = steps_heard(&mut r, 4.0);
    assert!(!heard.is_empty() && heard.iter().all(|&s| s == ROAD), "{heard:?}");
}
