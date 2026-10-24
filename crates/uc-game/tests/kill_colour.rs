//! Kill-colour AI (#244, `PCOM_AI_KILL_COLOUR`): it goes for whoever is nearest of the colour
//! of the person its `ai_other` point made. Psycho Park's points 131 and 132 hunt the colour
//! of point 137's man from the start. Skipped without an install.

use uc_game::person::AI_KILL_COLOUR;
use uc_game::Controls;

mod common;

#[test]
fn it_goes_for_the_nearest_of_its_colour() {
    let Some(mut r) = common::start("park2") else { return };
    r.play(1.0, Controls::default());
    let hated = r.g.people[r.who(137)].colour;
    for way in [131, 132] {
        let i = r.who(way);
        let p = &r.g.people[i];
        assert_eq!(p.ai, AI_KILL_COLOUR);
        let foe = p.foe.unwrap_or_else(|| panic!("point {way} has nobody to go for"));
        assert_eq!(r.g.people[foe].colour, hated, "point {way} is after someone of colour {}", r.g.people[foe].colour);
        let nearest = r.g.people.iter().enumerate()
            .filter(|&(j, o)| j != i && o.colour == hated && !o.removed && !o.is_dead() && !o.fake)
            .map(|(_, o)| (o.fig.pos - p.fig.pos).length())
            .fold(f32::MAX, f32::min);
        assert_eq!((r.g.people[foe].fig.pos - p.fig.pos).length(), nearest, "point {way} should go for the nearest");
    }
}
