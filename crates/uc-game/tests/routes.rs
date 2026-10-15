//! Finding the way (`MAV_do`): someone following Darci from the far side of a building goes
//! round it to her, rather than pressing against its wall. In RTA. Skipped without an install.

use glam::vec3;
use uc_game::{Controls, Game, State};
use uc_world::mav::{self, Mav};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;
/// Walking and climbing fences, as the AI gets about.
const WALKER: u8 = mav::caps(mav::GOTO) | mav::caps(mav::CLIMB_OVER);

#[test]
fn a_follower_goes_round_a_building() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let m = Mav::new(&level);
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let p = g.player.unwrap();
    // Two squares a few blocks apart that a straight walk can't join, and the way round on
    // foot twice as far: Darci at one, the follower at the other.
    let open = |x: i32, z: i32| (0..4).all(|d| m.opt(x, z, d) & WALKER != 0);
    let mut rng = 11u32;
    let mut next = || { rng = rng.wrapping_mul(69069).wrapping_add(1); (rng >> 16) as i32 };
    let (spot, here) = loop {
        let a = (20 + next() % 88, 20 + next() % 88);
        let b = (a.0 + next() % 25 - 12, a.1 + next() % 25 - 12);
        if !open(a.0, a.1) || !open(b.0, b.1) || (a.0 - b.0).abs() + (a.1 - b.1).abs() < 16 {
            continue;
        }
        let (a, b) = ((a.0 as u8, a.1 as u8), (b.0 as u8, b.1 as u8));
        if m.can_walk(a, b) || !m.route(a, b, WALKER, &mut || 0).1 {
            continue;
        }
        // A real detour: the way round at least twice as long as the way through.
        let (mut at, mut length) = (a, 0.0);
        for _ in 0..40 {
            if at == b {
                break;
            }
            let (act, _) = m.route(at, b, WALKER, &mut || 0);
            if act.dest == at {
                break;
            }
            length += ((act.dest.0 as f32 - at.0 as f32).powi(2) + (act.dest.1 as f32 - at.1 as f32).powi(2)).sqrt();
            at = act.dest;
        }
        let straight = ((a.0 as f32 - b.0 as f32).powi(2) + (a.1 as f32 - b.1 as f32).powi(2)).sqrt();
        if at == b && length > straight * 2.0 {
            break (a, b);
        }
    };
    let put = |g: &mut Game, i: usize, s: (u8, u8)| {
        let at = vec3(s.0 as f32 * 256.0 + 128.0, 0.0, s.1 as f32 * 256.0 + 128.0);
        let y = level.ground.surface(&level.map, at.x, at.z, 1.0e4);
        g.people[i].fig.pos = vec3(at.x, y, at.z);
        g.people[i].state = State::Move;
    };
    put(&mut g, p, here);
    let me = g.people[p].fig.pos;
    // One of the crowd, made to follow her (`PCOM_MOVE_FOLLOW`) from there.
    let i = g.people.iter().position(|q| q.fake && !q.is_dead()).expect("no crowd");
    put(&mut g, i, spot);
    let q = &mut g.people[i];
    (q.fake, q.mv, q.follow) = (false, 5, 0);
    let start = (q.fig.pos - me).length();
    let mut closest = start;
    for _ in 0..(30.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        closest = closest.min((g.people[i].fig.pos - g.people[p].fig.pos).length());
        if closest < 3.0 * 256.0 {
            break;
        }
    }
    assert!(closest < 3.0 * 256.0, "got from {start} no nearer than {closest}");
}

/// Running from gunfire (`PCOM_set_person_move_runaway`): one of a mission's own civilians,
/// not just the crowd, runs, and by the way the navigation map finds gets well away from the
/// danger rather than into the nearest wall.
#[test]
fn a_mission_civilian_runs_from_gunfire() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "police1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    for _ in 0..(60.0 / DT) as usize {
        g.skip_cutscene();
        g.update(DT, &Controls::default(), &level);
        if g.time > 1.0 && !g.in_conversation() {
            break;
        }
    }
    let Some(i) = g.people.iter().position(|q| !q.fake && q.way != 0 && q.ai == uc_game::person::AI_CIV && !q.is_dead() && matches!(q.state, State::Move))
        else { return };
    let start = g.people[i].fig.pos;
    // A shot fired right beside them, by nobody they can see (a bang where it lands).
    let danger = start + vec3(200.0, 0.0, 0.0);
    g.people[i].scare(None, danger);
    assert!(g.people[i].scared.is_some(), "a mission civilian didn't take fright");
    let mut furthest: f32 = 0.0;
    for _ in 0..(10.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        let d = g.people[i].fig.pos - danger;
        furthest = furthest.max((d.x * d.x + d.z * d.z).sqrt());
    }
    assert!(furthest > 8.0 * 256.0, "got no further than {} from it", furthest / 256.0);
}
