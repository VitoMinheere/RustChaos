//! Gun sights (`track_gun_sight`, `PANEL_draw_gun_sight`): on whoever Darci's gun is on,
//! closing in as her aim steadies (`Gunaim` down from `MAX_GUN_AIM`), and on her when an
//! armed thug is about to shoot. In RTA. Skipped without an install.

use glam::{vec2, vec3};
use uc_game::person::AI_GANG;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn start(dist: f32) -> Option<(Level, Game, usize, usize)> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let p = g.player.unwrap();
    let at = vec3(66.0 * 256.0, 0.0, 74.0 * 256.0);
    let y = level.ground.surface(&level.map, at.x, at.z, 1.0e4);
    g.people[p].fig.pos = vec3(at.x, y, at.z);
    g.people[p].fig.angle = uc_world::character::angle_of(vec2(0.0, 1.0));
    g.people[p].state = State::Move;
    let i = g.people.iter().position(|q| !q.player && !q.removed && !q.is_dead() && q.kind != ptype::COP).expect("nobody");
    for (j, q) in g.people.iter_mut().enumerate() {
        if j != p && j != i && (q.hostile() || q.kind == ptype::COP) {
            q.removed = true;
        }
    }
    let t = &mut g.people[i];
    t.ai = AI_GANG;
    t.fake = false;
    t.mv = 1;
    t.fig.pos = vec3(at.x, y, at.z + dist);
    t.fig.angle = uc_world::character::angle_of(vec2(0.0, -1.0));
    t.state = State::Move;
    Some((level, g, p, i))
}

fn run(g: &mut Game, level: &Level, secs: f32) {
    for _ in 0..(secs / DT).max(1.0) as usize {
        g.skip_cutscene();
        let p = g.player.unwrap();
        g.people[p].health = g.people[p].max_health;
        g.update(DT, &Controls::default(), level);
    }
}

#[test]
fn her_sight_closes_in() {
    let Some((level, mut g, p, i)) = start(1200.0) else { return };
    g.people[p].give(Weapon::Pistol);
    g.people[p].weapon = Weapon::Pistol;
    g.people[i].max_health = 10_000;
    g.people[i].health = 10_000;
    run(&mut g, &level, DT);
    assert_eq!(g.target, Some(i), "the gun isn't on him");
    let first = g.gun_sights()[0];
    run(&mut g, &level, 2.0 * DT);
    let soon = g.gun_sights()[0].accuracy;
    run(&mut g, &level, 2.0);
    let later = g.gun_sights()[0].accuracy;
    assert!(first.accuracy > 90.0, "starts {}", first.accuracy);
    assert!(soon < first.accuracy && later < soon, "{} {soon} {later}", first.accuracy);
    // Gun away: no sight.
    g.people[p].weapon = Weapon::None;
    run(&mut g, &level, DT);
    assert!(g.gun_sights().iter().all(|s| (s.at - g.people[i].fig.pos).length() > 400.0), "still a sight on him");
}

#[test]
fn a_sight_on_her_before_a_thug_fires() {
    let Some((level, mut g, p, i)) = start(800.0) else { return };
    g.people[i].give(Weapon::Pistol);
    g.people[i].weapon = Weapon::Pistol;
    g.people[i].ai = 17;
    let mut seen = false;
    for _ in 0..(4.0 / DT) as usize {
        run(&mut g, &level, DT);
        let me = g.people[p].fig.pos;
        seen |= g.gun_sights().iter().any(|s| (s.at - me).length() < 260.0);
    }
    assert!(seen, "no sight on her");
}
