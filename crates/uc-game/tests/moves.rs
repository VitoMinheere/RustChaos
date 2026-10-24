//! Moves that change height must look continuous: the drawn hips may not jump between
//! frames, and must end at standing (or lying) height on the new floor. Uses the combat
//! tutorial map; skipped without an install.

use glam::{vec2, vec3, Vec2, Vec3};
use uc_game::person::{ko, Controls, Person, State};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

fn level() -> Option<Level> {
    let game = uc_formats::default_install()?;
    Some(Level::load(&game, "ftutor1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap())
}

fn darci(level: &Level, pos: Vec3, facing: Vec2) -> Person {
    let angle = uc_world::character::angle_of(facing);
    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y);
    let fig = level.people.as_ref().unwrap().figure(ptype::DARCI, 0, vec3(pos.x, y, pos.z), angle).unwrap();
    Person::new(fig, ptype::DARCI, true, 0)
}

/// Where the hips are drawn.
fn hips(p: &Person) -> Vec3 {
    p.fig.drawn_pos() + p.fig.to_world(p.fig.root())
}

/// Steps `p` with `c` until `done` (or `secs` pass), checking the hips never jump more than
/// `max_step` units in one frame. Returns whether `done` became true.
fn run(p: &mut Person, level: &Level, c: Controls, secs: f32, max_step: f32, done: impl Fn(&Person) -> bool) -> bool {
    let mut last = hips(p);
    let mut events = Vec::new();
    for i in 0..(secs / DT) as usize {
        let snaps = [p.snap()];
        p.step(0, &c, &snaps, &level.ground, &level.map, DT, &mut events);
        p.advance(DT, &level.ground);
        let h = hips(p);
        if std::env::var("MOVES_TRACE").is_ok() {
            println!("{i} {:?} anim {} loops {:.2} root {:.0?} pos {:.0?} hips {:.0?}", p.state, p.fig.anim(), p.fig.loops(), p.fig.root(), p.fig.pos, h);
        }
        assert!((h - last).length() < max_step, "frame {i}: hips jumped {:?} -> {:?} in {:?} (anim {})", last, h, p.state, p.fig.anim());
        last = h;
        if done(p) {
            return true;
        }
    }
    false
}

/// Runs until `done`, failing the test if that doesn't happen within `secs`.
fn until(p: &mut Person, level: &Level, c: Controls, secs: f32, max_step: f32, done: impl Fn(&Person) -> bool) {
    assert!(run(p, level, c, secs, max_step, done), "not done after {secs}s: {:?} at {:?}", p.state, p.fig.pos);
}

fn standing_height(p: &Person) -> f32 {
    hips(p).y - p.fig.pos.y
}

#[test]
fn low_ledges_are_jumped_onto() {
    let Some(level) = level() else { return };
    // West of the central platform, whose rim is 128 up (the wall is at x = 47 squares).
    let mut p = darci(&level, vec3(46.5 * 256.0, 0.0, 55.0 * 256.0), vec2(1.0, 0.0));
    let walk = Controls { wish: vec2(1.0, 0.0), ..Default::default() };
    // Walking into it doesn't climb.
    run(&mut p, &level, walk, 1.0, 40.0, |_| false);
    assert!(matches!(p.state, State::Move) && p.fig.pos.y == 0.0, "{:?} at {:?}", p.state, p.fig.pos);
    // Jumping lands on it.
    until(&mut p, &level, Controls { jump: true, ..walk }, 0.2, 60.0, |p| matches!(p.state, State::Fall { .. }));
    until(&mut p, &level, walk, 3.0, 60.0, |p| matches!(p.state, State::Land { .. }));
    run(&mut p, &level, Controls::default(), 1.0, 60.0, |_| false);
    assert_eq!(p.fig.pos.y, 128.0);
    assert!((standing_height(&p) - 96.0).abs() < 30.0);
}

#[test]
fn high_ledges_are_grabbed_and_climbed() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "assault1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    // A 256-high crate at (54.0, 64.2) squares: too high to jump onto, so grab and pull up.
    let c = vec2(54.0, 64.2) * 256.0;
    let mut p = darci(&level, vec3(c.x - 230.0, 0.0, c.y), vec2(1.0, 0.0));
    let walk = Controls { wish: vec2(1.0, 0.0), ..Default::default() };
    run(&mut p, &level, walk, 0.3, 60.0, |_| false);
    until(&mut p, &level, Controls { jump: true, ..walk }, 0.2, 60.0, |p| matches!(p.state, State::Fall { .. }));
    until(&mut p, &level, walk, 3.0, 90.0, |p| matches!(p.state, State::PullUp { .. }));
    until(&mut p, &level, Controls::default(), 5.0, 90.0, |p| matches!(p.state, State::Move));
    assert_eq!(p.fig.pos.y, 256.0);
    assert!((standing_height(&p) - 96.0).abs() < 30.0);
}

/// Jumping at the same crate and letting go of the keys: she hangs from its edge (`STATE_DANGLING`)
/// until told otherwise. Back lets go and she drops to the ground; forward pulls her up; left
/// or right takes her a hand along the edge.
#[test]
fn hanging_from_a_ledge() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "assault1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let c = vec2(54.0, 64.2) * 256.0;
    let walk = Controls { wish: vec2(1.0, 0.0), ..Default::default() };
    let grab = |level: &Level| {
        let mut p = darci(level, vec3(c.x - 230.0, 0.0, c.y), vec2(1.0, 0.0));
        run(&mut p, level, walk, 0.3, 60.0, |_| false);
        until(&mut p, level, Controls { jump: true, ..walk }, 0.2, 60.0, |p| matches!(p.state, State::Fall { .. }));
        // Pushing on until the hands catch, then nothing held.
        until(&mut p, level, walk, 3.0, 90.0, |p| matches!(p.state, State::Dangle { .. }));
        p
    };
    let mut p = grab(&level);
    // Hanging: still, hands at the top (the hips well below it).
    run(&mut p, &level, Controls::default(), 2.0, 20.0, |_| false);
    assert!(matches!(p.state, State::Dangle { shift: None, .. }), "{:?}", p.state);
    let h = hips(&p);
    assert!(h.y < 256.0 - 50.0 && h.y > 0.0, "hips at {}", h.y);
    // Back: let go, and down onto the ground. From this crate her feet hang at the ground, so
    // she lands at once, from the stretched hang to a crouch (`ANIM_LAND_VERT`, foot-locked).
    until(&mut p, &level, Controls { wish: vec2(-1.0, 0.0), ..Default::default() }, 0.2, 40.0, |p| matches!(p.state, State::Fall { .. }));
    until(&mut p, &level, Controls::default(), 3.0, 90.0, |p| matches!(p.state, State::Move));
    let floor = level.ground.surface(&level.map, p.fig.pos.x, p.fig.pos.z, 0.0);
    assert_eq!(p.fig.pos.y, floor, "not back on the ground");
    assert!(p.fig.pos.y < 64.0 && p.fig.pos.x < c.x - 128.0, "on the crate: {:?}", p.fig.pos);

    // Forward: up onto it.
    let mut p = grab(&level);
    run(&mut p, &level, Controls::default(), 0.5, 20.0, |_| false);
    until(&mut p, &level, walk, 0.2, 90.0, |p| matches!(p.state, State::PullUp { .. }));
    until(&mut p, &level, Controls::default(), 5.0, 90.0, |p| matches!(p.state, State::Move));
    assert_eq!(p.fig.pos.y, 256.0);

    // Sideways: a hand along the edge, still hanging.
    let mut p = grab(&level);
    run(&mut p, &level, Controls::default(), 0.5, 20.0, |_| false);
    let before = p.fig.pos;
    until(&mut p, &level, Controls { wish: vec2(0.0, 1.0), ..Default::default() }, 0.2, 40.0, |p| matches!(p.state, State::Dangle { shift: Some(_), .. }));
    until(&mut p, &level, Controls::default(), 3.0, 40.0, |p| matches!(p.state, State::Dangle { shift: None, .. }));
    let moved = p.fig.pos - before;
    assert!((moved.z.abs() - 74.0).abs() < 20.0 && moved.x.abs() < 10.0, "moved {moved:?}");
}

#[test]
fn stairs_have_steps() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "assault1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    // Stair props (prim 41, steps 32 apart up to 128) at (61, 66) and (62, 66) squares.
    let mut heights = Vec::new();
    for i in 0..=40 {
        for j in 0..=40 {
            let (x, z) = ((59.0 + i as f32 * 0.1) * 256.0, (64.0 + j as f32 * 0.1) * 256.0);
            let h = level.ground.surface(&level.map, x, z, 1000.0);
            if !heights.contains(&h) {
                heights.push(h);
            }
        }
    }
    heights.sort_by(f32::total_cmp);
    println!("heights {heights:?}");
    for h in [32.0, 64.0, 96.0, 128.0] {
        assert!(heights.contains(&h), "no step at {h}: {heights:?}");
    }
}

#[test]
fn ladder_climb_reaches_the_roof() {
    let Some(level) = level() else { return };
    // Ladder 0 rises from the platform rim (128) to the roof (640); its climber faces +z.
    let l = level.ground.ladders[0];
    let start = l.foot + l.out * 40.0;
    let mut p = darci(&level, vec3(start.x, 128.0, start.y), -l.out);
    let up = Controls { wish: -l.out, ..Default::default() };
    until(&mut p, &level, up, 3.0, 40.0, |p| matches!(p.state, State::Climb { .. }));
    until(&mut p, &level, up, 20.0, 40.0, |p| matches!(p.state, State::PullUp { .. }));
    until(&mut p, &level, Controls::default(), 5.0, 90.0, |p| matches!(p.state, State::Move));
    assert_eq!(p.fig.pos.y, 640.0, "should stand on the roof");
    assert!((standing_height(&p) - 96.0).abs() < 20.0);
}

#[test]
fn a_ladder_off_the_ground_is_caught_with_a_jump() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "MIB", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    // The way up to The Fallen's first clue: a ladder from 128 to 640 over ground at 0, out of
    // reach of anyone walking into it.
    let l = *level.ground.ladders.iter().find(|l| l.base == 128.0 && (l.foot - vec2(18048.0, 15072.0)).length() < 1.0).unwrap();
    let start = l.foot + l.out * 40.0;
    let mut p = darci(&level, vec3(start.x, 0.0, start.y), -l.out);
    assert_eq!(p.fig.pos.y, 0.0);
    assert_eq!(level.ground.ladder_near(p.fig.pos, 90.0), None);
    let up = Controls { wish: -l.out, ..Default::default() };
    // A jump catches it on the way down, and from there it climbs as any other.
    until(&mut p, &level, Controls { jump: true, ..up }, 0.2, 60.0, |p| matches!(p.state, State::Fall { .. }));
    until(&mut p, &level, up, 2.0, 60.0, |p| matches!(p.state, State::Climb { .. }));
    until(&mut p, &level, up, 20.0, 60.0, |p| matches!(p.state, State::PullUp { .. }));
    until(&mut p, &level, Controls::default(), 5.0, 90.0, |p| matches!(p.state, State::Move));
    // On the walkway at the top, a prop a little under the ladder's top.
    assert_eq!(p.fig.pos.y, 614.0, "should stand at the top");
}

#[test]
fn running_jump_keeps_momentum() {
    let Some(level) = level() else { return };
    // Open ground in the yard.
    let mut p = darci(&level, vec3(40.0 * 256.0, 0.0, 66.0 * 256.0), vec2(0.0, 1.0));
    let runc = Controls { wish: vec2(0.0, 1.0), run: true, ..Default::default() };
    run(&mut p, &level, runc, 1.0, 60.0, |_| false);
    until(&mut p, &level, Controls { jump: true, ..runc }, 0.2, 60.0, |p| matches!(p.state, State::Fall { .. }));
    let takeoff = p.fig.pos;
    until(&mut p, &level, runc, 3.0, 60.0, |p| matches!(p.state, State::Land { .. }));
    let landed = p.fig.pos;
    assert!((landed - takeoff).length() > 200.0, "jumped only {}", (landed - takeoff).length());
    // Still running a moment after landing.
    until(&mut p, &level, runc, 1.0, 60.0, |p| matches!(p.state, State::Move));
    let z = p.fig.pos.z;
    run(&mut p, &level, runc, 0.3, 60.0, |_| false);
    assert!(p.fig.pos.z - z > 100.0, "stopped after landing");
}

#[test]
fn knocked_down_people_end_up_on_the_floor() {
    let Some(level) = level() else { return };
    for (low, from) in [(true, vec2(0.0, -1.0)), (false, vec2(0.0, -1.0)), (true, vec2(0.0, 1.0)), (false, vec2(0.0, 1.0))] {
        let pos = vec3(40.0 * 256.0, 0.0, 66.0 * 256.0);
        let mut p = darci(&level, pos, vec2(0.0, -1.0));
        p.ai = uc_game::script::PCOM_AI_FIGHT_TEST;
        p.ko_moves = ko::SLIDE | ko::COMBO_ANY;
        let attacker = pos + vec3(from.x, 0.0, from.y) * 100.0;
        p.take_hit(attacker, 0, if low { ko::SLIDE } else { ko::COMBO_ANY }, false, low, uc_game::Weapon::None);
        assert!(p.is_down());
        run(&mut p, &level, Controls::default(), 4.0, 60.0, |_| false);
        assert!(standing_height(&p) < 50.0, "low {low} from {from:?}: hips still {} up (anim {})", standing_height(&p), p.fig.anim());
    }
}

#[test]
fn blocking_stops_blows_from_the_front() {
    let Some(level) = level() else { return };
    let pos = vec3(40.0 * 256.0, 0.0, 66.0 * 256.0);
    let mut p = darci(&level, pos, vec2(0.0, -1.0));
    let block = Controls { block: true, ..Default::default() };
    run(&mut p, &level, block, 0.3, 60.0, |_| false);
    assert!(matches!(p.state, State::Block { .. }));
    let health = p.health;
    // In front (facing is -z), then behind.
    p.take_hit(pos + vec3(0.0, 0.0, -100.0), 20, 0, true, false, uc_game::Weapon::None);
    assert_eq!(p.health, health, "a blow from the front got through the guard");
    p.take_hit(pos + vec3(0.0, 0.0, 100.0), 20, 0, true, false, uc_game::Weapon::None);
    assert!(p.health < health, "a blow from behind was blocked");
}

#[test]
fn knocked_down_people_get_up_again() {
    let Some(level) = level() else { return };
    let pos = vec3(40.0 * 256.0, 0.0, 66.0 * 256.0);
    let mut p = darci(&level, pos, vec2(0.0, -1.0));
    p.take_hit(pos + vec3(0.0, 0.0, -100.0), 30, ko::COMBO_PPP, false, false, uc_game::Weapon::None);
    assert!(p.is_down() && !p.knocked_out && !p.is_dead());
    until(&mut p, &level, Controls::default(), 8.0, 90.0, |p| matches!(p.state, State::Move));
    assert!((standing_height(&p) - 96.0).abs() < 30.0);
    // Out of health: stays down.
    p.take_hit(pos, 10_000, 0, false, false, uc_game::Weapon::None);
    run(&mut p, &level, Controls::default(), 6.0, 90.0, |_| false);
    assert!(p.is_down() && p.knocked_out && p.is_dead());
}

#[test]
fn cables_carry_you_down() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "assault1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let cable = &level.ground.cables[0];
    let (high, low) = if cable.at(0.0).y > cable.at(1.0).y { (0.0, 1.0) } else { (1.0, 0.0) };
    // Jump up to the cable near its high end.
    let at = cable.at(high * 0.9 + low * 0.1);
    let mut p = darci(&level, at - vec3(0.0, 330.0, 0.0), vec2(0.0, 1.0));
    p.fig.pos = at - vec3(0.0, 330.0, 0.0);
    p.state = State::Fall { vy: 400.0, vel: Vec2::ZERO };
    until(&mut p, &level, Controls::default(), 1.0, 90.0, |p| matches!(p.state, State::Zip { .. }));
    let start = p.fig.pos;
    until(&mut p, &level, Controls::default(), 20.0, 120.0, |p| !matches!(p.state, State::Zip { .. }));
    println!("slid from {start:?} to {:?}", p.fig.pos);
    assert!(start.y - p.fig.pos.y > 1000.0, "only slid down {}", start.y - p.fig.pos.y);
    // Off the end: falls and lands, without grabbing the cable again.
    until(&mut p, &level, Controls::default(), 5.0, 120.0, |p| matches!(p.state, State::Move));
}

#[test]
fn chain_link_fences_can_be_climbed() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = Level::load(&game, "assault1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    // A fence along z = 72 squares from x = 58 to 64, 256 high.
    let z = 72.0 * 256.0;
    let mut p = darci(&level, vec3(61.0 * 256.0, 0.0, z - 120.0), vec2(0.0, 1.0));
    let walk = Controls { wish: vec2(0.0, 1.0), ..Default::default() };
    until(&mut p, &level, walk, 3.0, 60.0, |p| matches!(p.state, State::Fence { .. }));
    until(&mut p, &level, Controls::default(), 8.0, 60.0, |p| !matches!(p.state, State::Fence { .. }));
    until(&mut p, &level, Controls::default(), 4.0, 90.0, |p| matches!(p.state, State::Move));
    assert!(p.fig.pos.z > z + 20.0, "still on the near side at {:?}", p.fig.pos);
    assert!((standing_height(&p) - 96.0).abs() < 30.0);
}

