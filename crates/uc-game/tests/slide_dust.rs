//! A sliding tackle is heard while it slides (`S_SLIDE_START`, looped to `ANIM_SLIDER_END`)
//! and kicks up dust at the left foot (`ANIM_SLIDER_HOLD`, Person.cpp:16056); searching a body
//! has its own loop (`S_SEARCH_END`). Skipped without an install.

use uc_game::{Controls, Game, Search, Searched, State};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

#[test]
fn a_slide_scrapes_and_raises_dust() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "fight1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..30 {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
    }
    let me = g.player.unwrap();
    for (i, p) in g.people.iter_mut().enumerate() {
        if i != me {
            p.removed = true;
        }
    }
    let run = Controls { wish: glam::vec2(1.0, 0.0), run: true, ..Default::default() };
    for _ in 0..20 {
        g.update(DT, &run, &level);
    }
    assert!(g.slide_noise(g.people[me].fig.pos).is_none(), "heard before sliding");
    g.update(DT, &Controls { kick: true, ..run }, &level);
    assert!(matches!(g.people[me].state, State::Slide { .. }), "{:?}", g.people[me].state);
    let (mut heard, mut dust) = (false, 0);
    for _ in 0..40 {
        let before = g.pyro.puffs.len();
        g.update(DT, &Controls::default(), &level);
        heard |= g.slide_noise(g.people[me].fig.pos).is_some();
        if matches!(g.people[me].state, State::Slide { phase: 1, .. }) {
            dust += g.pyro.puffs.len().saturating_sub(before);
        }
    }
    assert!(heard, "never heard sliding");
    assert!(dust > 0, "no dust");
    assert!(g.slide_noise(g.people[me].fig.pos).is_none(), "still heard after");
    // Going through someone's pockets has its own loop; a prop's search doesn't.
    assert!(!g.searching_body());
    g.search = Some(Search { what: Searched::Body(0), time: 0.0 });
    assert!(g.searching_body());
    g.search = Some(Search { what: Searched::Prop(0), time: 0.0 });
    assert!(!g.searching_body());
}
