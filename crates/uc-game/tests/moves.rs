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
        // Pushing on until the hands catch, then nothing held while she catches hold
        // (`ANIM_FLY_GRABBING_LEDGE`) and hangs.
        until(&mut p, level, walk, 3.0, 90.0, |p| matches!(p.state, State::Dangle { .. }));
        until(&mut p, level, Controls::default(), 2.0, 40.0, |p| p.fig.anim() == uc_world::character::anim::DANGLE);
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
    // The grab, a pull up on it, and then she hangs (`ANIM_WIRE_SLIDE_HANG`, #342).
    until(&mut p, &level, Controls::default(), 5.0, 120.0, |p| p.fig.anim() == uc_world::character::anim::WIRE_SLIDE_HANG);
    assert!(matches!(p.state, State::Zip { .. }), "off before hanging: {:?}", p.state);
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
    // Settling onto it before climbing (`ANIM_LANDED_ON_FENCE`, #342).
    until(&mut p, &level, Controls::default(), 3.0, 60.0, |p| p.fig.anim() == uc_world::character::anim::LANDED_ON_FENCE);
    until(&mut p, &level, Controls::default(), 8.0, 60.0, |p| !matches!(p.state, State::Fence { .. }));
    until(&mut p, &level, Controls::default(), 4.0, 90.0, |p| matches!(p.state, State::Move));
    assert!(p.fig.pos.z > z + 20.0, "still on the near side at {:?}", p.fig.pos);
    assert!((standing_height(&p) - 96.0).abs() < 30.0);
}


/// A punch that isn't followed up comes back (#342, `ANIM_PUNCH_RETURN1`) before she's
/// ready again; a punch while it does starts the combo afresh once it's done.
#[test]
fn an_unfollowed_punch_comes_back() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let mut p = darci(&level, vec3(46.5 * 256.0, 0.0, 55.0 * 256.0), vec2(1.0, 0.0));
    let punch = Controls { punch: true, ..Default::default() };
    until(&mut p, &level, punch, 0.2, 40.0, |p| matches!(p.state, State::Attack { .. }));
    until(&mut p, &level, Controls::default(), 2.0, 40.0, |p| matches!(p.state, State::Recover { .. }));
    assert_eq!(p.fig.anim(), anim::PUNCH_RETURN1);
    until(&mut p, &level, Controls::default(), 2.0, 40.0, |p| matches!(p.state, State::Move));
    // Again, pressing while it comes back.
    until(&mut p, &level, punch, 0.2, 40.0, |p| matches!(p.state, State::Attack { .. }));
    until(&mut p, &level, Controls::default(), 2.0, 40.0, |p| matches!(p.state, State::Recover { .. }));
    run(&mut p, &level, punch, DT, 40.0, |_| false);
    until(&mut p, &level, Controls::default(), 2.0, 40.0, |p| !matches!(p.state, State::Recover { .. }));
    assert!(matches!(p.state, State::Attack { .. }) && p.fig.anim() == anim::PUNCH_COMBO1, "{:?}", p.state);
}

/// Someone of kind `kind` (#342), as `darci` makes her.
fn person(level: &Level, kind: u8, pos: Vec3, facing: Vec2) -> Person {
    let angle = uc_world::character::angle_of(facing);
    let y = level.ground.surface(&level.map, pos.x, pos.z, pos.y);
    let fig = level.people.as_ref().unwrap().figure(kind, 0, vec3(pos.x, y, pos.z), angle).unwrap();
    Person::new(fig, kind, false, 0)
}

/// Knocked down by a high blow, they go over with `ANIM_KD_*_HI` (`knock_down`,
/// Person.cpp:2409), and get up as the original has it (Person.cpp:17173): off the front
/// with `ANIM_KO_BEHIND_BIG_GU`; off the back Darci springs up, a cop gets up slowly
/// (`ANIM_KO_BACK_GU`). The hips don't jump on the way.
#[test]
fn knocked_down_high_and_getting_up() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    for (kind, behind, fall, get_up) in [
        (ptype::DARCI, false, anim::KD_FRONT_HI, anim::QUICK_GET_UP),
        (ptype::COP, false, anim::KD_FRONT_HI, anim::KO_BACK_GU),
        (ptype::DARCI, true, anim::KD_BACK_HI, anim::KO_BEHIND_BIG_GU),
    ] {
        let mut p = person(&level, kind, at, vec2(1.0, 0.0));
        let from = at + if behind { vec3(-100.0, 0.0, 0.0) } else { vec3(100.0, 0.0, 0.0) };
        p.take_hit(from, 10, ko::KNOCKDOWN, true, false, uc_game::Weapon::None);
        assert_eq!(p.fig.anim(), fall, "kind {kind}, behind {behind}");
        until(&mut p, &level, Controls::default(), 8.0, 60.0, |p| matches!(p.state, State::GetUp));
        assert_eq!(p.fig.anim(), get_up, "kind {kind}, behind {behind}");
        until(&mut p, &level, Controls::default(), 5.0, 60.0, |p| matches!(p.state, State::Move));
    }
}

/// With a gun out the jumps are the gun's own (Person.cpp:10162, 10288): a standing jump
/// with a two-handed one, a running jump with any; and a bat guards with itself.
#[test]
fn armed_jumps_and_guards() {
    use uc_game::Weapon;
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let mut p = darci(&level, at, vec2(-1.0, 0.0));
    p.give(Weapon::Shotgun);
    p.weapon = Weapon::Shotgun;
    run(&mut p, &level, Controls { jump: true, ..Default::default() }, DT, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::STANDING_JUMP_AK);
    until(&mut p, &level, Controls::default(), 3.0, 60.0, |p| matches!(p.state, State::Move));

    let mut p = darci(&level, at, vec2(-1.0, 0.0));
    p.give(Weapon::Pistol);
    p.weapon = Weapon::Pistol;
    let ahead = Controls { wish: vec2(-1.0, 0.0), run: true, ..Default::default() };
    run(&mut p, &level, ahead, 0.5, 60.0, |_| false);
    run(&mut p, &level, Controls { jump: true, ..ahead }, DT, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::RUN_JUMP_LEFT_AK);

    let mut p = darci(&level, at, vec2(-1.0, 0.0));
    p.give(Weapon::Bat);
    p.weapon = Weapon::Bat;
    run(&mut p, &level, Controls { block: true, ..Default::default() }, DT, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::BAT_BLOCK);
}

/// The gangs walk their own walk, and stand holding a shotgun (Person.cpp:8694, 8800).
#[test]
fn a_thug_walks_and_stands_his_own_way() {
    use uc_game::Weapon;
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let mut p = person(&level, ptype::THUG_RED, at, vec2(-1.0, 0.0));
    run(&mut p, &level, Controls { wish: vec2(-1.0, 0.0), ..Default::default() }, 0.5, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::THUG_WALK);
    p.give(Weapon::Shotgun);
    p.weapon = Weapon::Shotgun;
    run(&mut p, &level, Controls { wish: vec2(-1.0, 0.0), ..Default::default() }, 0.5, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::THUG_WALK_SHOTGUN);
    run(&mut p, &level, Controls::default(), 0.5, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::SHOTGUN_IDLE);
}

/// Kick in the air off a running jump (`SUB_STATE_FLYING_KICK`, Person.cpp:13610): out with
/// `ANIM_FLYKICK_START`, held on the way down, down with `ANIM_FLYKICK_LAND`; and whoever's in
/// the way goes down.
#[test]
fn a_flying_kick_off_a_running_jump() {
    use uc_game::person::Event;
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let mut people = [darci(&level, at, vec2(-1.0, 0.0)), person(&level, ptype::THUG_RED, at - vec3(900.0, 0.0, 0.0), vec2(1.0, 0.0))];
    let ahead = Controls { wish: vec2(-1.0, 0.0), run: true, ..Default::default() };
    let (mut kicked, mut landed, mut floored) = (false, false, false);
    for _ in 0..(4.0 / DT) as usize {
        let gap = (people[1].fig.pos - people[0].fig.pos).length();
        let c = match people[0].state {
            State::Move if gap < 450.0 => Controls { jump: true, ..ahead },
            State::Fall { .. } => Controls { kick: true, ..ahead },
            _ => ahead,
        };
        let snaps = [people[0].snap(), people[1].snap()];
        let mut events = Vec::new();
        people[0].step(0, &c, &snaps, &level.ground, &level.map, DT, &mut events);
        people[1].step(1, &Controls::default(), &snaps, &level.ground, &level.map, DT, &mut events);
        for e in events {
            if let Event::Hit { victim: 1, damage, ko, high, low, weapon, .. } = e {
                let from = people[0].fig.pos;
                people[1].take_hit(from, damage, ko, high, low, weapon);
            }
        }
        for p in &mut people {
            p.advance(DT, &level.ground);
        }
        kicked |= people[0].fig.anim() == anim::FLYKICK_START;
        landed |= people[0].fig.anim() == anim::FLYKICK_LAND;
        floored |= matches!(people[1].state, State::Down);
    }
    assert!(kicked, "never kicked");
    assert!(landed, "never landed from it");
    assert!(floored, "he never went down");
}

/// In a fight, jump with no way held is the jump kick (`ANIM_KICK_NS`), which nothing hits;
/// with a way to the side, a flip that goes that way (interfac.cpp:5070, 5096).
#[test]
fn jumping_in_a_fight() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let darci_and = |c: Controls| {
        let mut people = [darci(&level, at, vec2(-1.0, 0.0)), person(&level, ptype::THUG_RED, at - vec3(150.0, 0.0, 0.0), vec2(1.0, 0.0))];
        let snaps = [people[0].snap(), uc_game::person::Snap { fighter: true, ..people[1].snap() }];
        let mut events = Vec::new();
        people[0].step(0, &c, &snaps, &level.ground, &level.map, DT, &mut events);
        people
    };
    let mut p = darci_and(Controls { jump: true, ..Default::default() });
    assert_eq!(p[0].fig.anim(), anim::KICK_NS);
    let had = p[0].health;
    p[0].take_hit(at - vec3(100.0, 0.0, 0.0), 30, 0, true, false, uc_game::Weapon::None);
    assert_eq!(p[0].health, had, "hit mid jump kick");
    // Facing -x, +z is to her side: the flip goes that way.
    let mut p = darci_and(Controls { jump: true, wish: vec2(0.0, 1.0), ..Default::default() });
    assert!(matches!(p[0].state, State::Flip { .. }), "{:?}", p[0].state);
    let z = p[0].fig.pos.z;
    let snaps = [p[0].snap()];
    let mut events = Vec::new();
    for _ in 0..20 {
        p[0].step(0, &Controls::default(), &snaps, &level.ground, &level.map, DT, &mut events);
        p[0].advance(DT, &level.ground);
    }
    assert!(p[0].fig.pos.z > z + 20.0, "flipped the wrong way: {} -> {}", z, p[0].fig.pos.z);
}

/// Someone to fight turning up, the player squares up (`ANIM_FIGHT_READY`); nobody left, she
/// drops the stance (`ANIM_UNFIGHT`) (Person.cpp:11406, 8959).
#[test]
fn squaring_up_and_standing_down() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let mut me = darci(&level, at, vec2(-1.0, 0.0));
    let thug = person(&level, ptype::THUG_RED, at - vec3(150.0, 0.0, 0.0), vec2(1.0, 0.0));
    let mut seen = Vec::new();
    let mut events = Vec::new();
    for there in [true, false] {
        for _ in 0..30 {
            let snaps = [me.snap(), uc_game::person::Snap { fighter: true, present: there, standing: there, ..thug.snap() }];
            me.step(0, &Controls::default(), &snaps, &level.ground, &level.map, DT, &mut events);
            me.advance(DT, &level.ground);
            if seen.last() != Some(&me.fig.anim()) {
                seen.push(me.fig.anim());
            }
        }
    }
    let at = |a| seen.iter().position(|&x| x == a);
    let (ready, fight, unfight) = (at(anim::FIGHT_READY), at(anim::FIGHT), at(anim::UNFIGHT));
    assert!(ready.is_some() && ready < fight && fight < unfight, "{seen:?}");
}

/// Onto a ladder at its foot (`ANIM_MOUNT_LADDER`), up it, back down it playing the climb
/// backwards (`SUB_STATE_CLIMB_DOWN_LADDER`), and off at the foot (`ANIM_OFF_LADDER_BOT`),
/// standing where she got on; the hips never jump. Jump lets go.
#[test]
fn up_and_down_a_ladder() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let l = level.ground.ladders[0];
    let start = l.foot + l.out * 40.0;
    let mut p = darci(&level, vec3(start.x, 128.0, start.y), -l.out);
    let up = Controls { wish: -l.out, ..Default::default() };
    let down = Controls { wish: l.out, ..Default::default() };
    until(&mut p, &level, up, 3.0, 40.0, |p| matches!(p.state, State::Mount { .. }));
    assert_eq!(p.fig.anim(), anim::MOUNT_LADDER);
    until(&mut p, &level, up, 3.0, 40.0, |p| matches!(p.state, State::Climb { .. }));
    until(&mut p, &level, up, 10.0, 40.0, |p| p.fig.pos.y > 128.0 + 200.0);
    until(&mut p, &level, down, 10.0, 40.0, |p| matches!(p.state, State::Press { .. }));
    assert_eq!(p.fig.anim(), anim::OFF_LADDER_BOT);
    until(&mut p, &level, Controls::default(), 3.0, 40.0, |p| matches!(p.state, State::Move));
    assert_eq!(p.fig.pos.y, 128.0, "back at the foot");
    assert!((standing_height(&p) - 96.0).abs() < 20.0);

    // Up a way, and jump: off it.
    until(&mut p, &level, up, 3.0, 40.0, |p| matches!(p.state, State::Climb { .. }));
    until(&mut p, &level, up, 10.0, 40.0, |p| p.fig.pos.y > 128.0 + 200.0);
    run(&mut p, &level, Controls { jump: true, ..Default::default() }, DT, 60.0, |_| false);
    assert!(matches!(p.state, State::Fall { .. }), "{:?}", p.state);
}

/// Lying injured, now and then a struggle (`ANIM_INJURED_LOOP_STRUGGLE`, Person.cpp:17479),
/// and back to lying there.
#[test]
fn the_injured_struggle_now_and_then() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let mut p = person(&level, ptype::CIV, at, vec2(1.0, 0.0));
    p.state = State::Injured;
    p.fig.restart(anim::INJURED_LOOP, uc_world::Play::Loop);
    let mut seen = Vec::new();
    let mut events = Vec::new();
    for _ in 0..(40.0 / DT) as usize {
        let snaps = [p.snap()];
        p.step(0, &Controls::default(), &snaps, &level.ground, &level.map, DT, &mut events);
        p.advance(DT, &level.ground);
        if seen.last() != Some(&p.fig.anim()) {
            seen.push(p.fig.anim());
        }
    }
    assert!(seen.windows(2).any(|w| w == [anim::INJURED_LOOP_STRUGGLE, anim::INJURED_LOOP]), "no struggle and back in 40 s: {seen:?}");
}

/// Running into a wall half a storey high she gets up onto it (`ANIM_GET_UP_HALF_BLOCK`,
/// `set_person_climb_half`); walking into it she doesn't (see `low_ledges_are_jumped_onto`).
#[test]
fn running_into_a_half_block_gets_up_it() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let mut p = darci(&level, vec3(46.0 * 256.0, 0.0, 55.0 * 256.0), vec2(1.0, 0.0));
    let run_at = Controls { wish: vec2(1.0, 0.0), run: true, ..Default::default() };
    until(&mut p, &level, run_at, 2.0, 60.0, |p| matches!(p.state, State::PullUp { .. }));
    assert_eq!(p.fig.anim(), anim::GET_UP_HALF_BLOCK);
    until(&mut p, &level, Controls::default(), 3.0, 60.0, |p| matches!(p.state, State::Move));
    assert_eq!(p.fig.pos.y, 128.0);
    assert!((standing_height(&p) - 96.0).abs() < 30.0);
}

/// A kick at someone within 120 is the near kick (`ANIM_KICK_NEAR`); further off, the
/// combo's first (`turn_to_target_and_kick`, combat.cpp:3493).
#[test]
fn a_kick_up_close_is_the_near_kick() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    for (gap, kick) in [(100.0, anim::KICK_NEAR), (200.0, anim::KICK_COMBO1)] {
        let mut me = darci(&level, at, vec2(-1.0, 0.0));
        let thug = person(&level, ptype::THUG_RED, at - vec3(gap, 0.0, 0.0), vec2(1.0, 0.0));
        let snaps = [me.snap(), uc_game::person::Snap { fighter: true, ..thug.snap() }];
        let mut events = Vec::new();
        me.step(0, &Controls { kick: true, ..Default::default() }, &snaps, &level.ground, &level.map, DT, &mut events);
        assert_eq!(me.fig.anim(), kick, "{gap} away");
    }
}

/// Landing a running jump with nothing held, she lands to stand (`ANIM_LAND_STAND`); coming
/// down hard (`DY` under -15000) but not hard enough to hurt, a big landing (`ANIM_BIG_LAND`).
#[test]
fn landing_to_stand_and_landing_hard() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let mut p = darci(&level, at, vec2(-1.0, 0.0));
    let ahead = Controls { wish: vec2(-1.0, 0.0), run: true, ..Default::default() };
    run(&mut p, &level, ahead, 0.5, 60.0, |_| false);
    until(&mut p, &level, Controls { jump: true, ..ahead }, 0.2, 60.0, |p| matches!(p.state, State::Fall { .. }));
    until(&mut p, &level, Controls::default(), 3.0, 60.0, |p| matches!(p.state, State::Land { .. }));
    assert_eq!(p.fig.anim(), anim::LAND_STAND);

    let mut p = darci(&level, at, vec2(-1.0, 0.0));
    let health = p.health;
    p.fig.pos.y += 4.0;
    p.state = State::Fall { vy: -1250.0, vel: Vec2::ZERO };
    run(&mut p, &level, Controls::default(), DT, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::BIG_LAND);
    assert_eq!(p.health, health, "hurt");
}

/// A standing jump that catches nothing falls on in its own last pose (`ANIM_FALLING_QUEUED`),
/// not the plain fall's (`PERSON_DROP_DOWN_QUEUED`, Person.cpp:13254).
#[test]
fn a_standing_jump_falls_on_in_its_own_pose() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let mut p = darci(&level, vec3(46.5 * 256.0, 0.0, 55.0 * 256.0), vec2(-1.0, 0.0));
    until(&mut p, &level, Controls { jump: true, ..Default::default() }, 0.2, 60.0, |p| matches!(p.state, State::Fall { .. }));
    until(&mut p, &level, Controls::default(), 2.0, 60.0, |p| p.fig.anim() == anim::FALLING_QUEUED || matches!(p.state, State::Land { .. }));
    assert_eq!(p.fig.anim(), anim::FALLING_QUEUED, "{:?}", p.state);
}

/// Action standing by the top of a ladder climbs down onto it, climbing off it backwards
/// (`set_person_climb_down_onto_ladder`, `ANIM_OFF_LADDER_TOP`); down it, and off at the
/// foot. The hips never jump.
#[test]
fn climbing_down_onto_a_ladder_from_the_top() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let l = level.ground.ladders[0];
    let top = level.ground.on_ladder(0, l.top) - vec3(l.out.x, 0.0, l.out.y) * 100.0;
    let mut p = darci(&level, vec3(top.x, l.top, top.z), -l.out);
    assert!((p.fig.pos.y - l.top).abs() < 8.0, "on the roof: {} vs {}", p.fig.pos.y, l.top);
    run(&mut p, &level, Controls { action: true, ..Default::default() }, DT, 60.0, |_| false);
    assert!(matches!(p.state, State::OntoLadder { .. }), "{:?}", p.state);
    assert_eq!(p.fig.anim(), anim::OFF_LADDER_TOP);
    until(&mut p, &level, Controls::default(), 3.0, 40.0, |p| matches!(p.state, State::Climb { .. }));
    // Still, she stays on it; then down to the foot and off.
    run(&mut p, &level, Controls::default(), 0.5, 40.0, |_| false);
    assert!(matches!(p.state, State::Climb { .. }), "{:?}", p.state);
    let down = Controls { wish: l.out, ..Default::default() };
    until(&mut p, &level, down, 15.0, 40.0, |p| matches!(p.state, State::Press { .. }));
    until(&mut p, &level, Controls::default(), 3.0, 40.0, |p| matches!(p.state, State::Move));
    assert!((p.fig.pos.y - l.base).abs() < 16.0, "at the foot: {}", p.fig.pos.y);
}

/// The strafe key held (#309): to the side a step (`ANIM_STEP_LEFT`, `_RIGHT`) that carries
/// her 45 that way, still facing the same way; back walks backwards (`ANIM_BACK_WALK`);
/// jump with back is a back flip (`ANIM_BACK_FLIP`); standing, she turns on the spot to face
/// the camera's way (`ANIM_ROTATE_L`, `_R`).
#[test]
fn strafing() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let f = vec2(1.0, 0.0);
    let right = vec2(-f.y, f.x);
    let strafe = |wish: Vec2| Controls { wish, strafe: Some(f), ..Default::default() };
    let facing = |p: &Person| p.fig.facing().dot(f);

    for (side, a) in [(right, anim::STEP_RIGHT), (-right, anim::STEP_LEFT)] {
        let mut p = darci(&level, at, f);
        let from = p.fig.pos;
        run(&mut p, &level, strafe(side), DT, 30.0, |_| false);
        assert!(matches!(p.state, State::Step), "{:?}", p.state);
        assert_eq!(p.fig.anim(), a);
        until(&mut p, &level, Controls::default(), 2.0, 30.0, |p| matches!(p.state, State::Move));
        run(&mut p, &level, Controls::default(), 0.3, 30.0, |_| false);
        let moved = vec2(p.fig.pos.x - from.x, p.fig.pos.z - from.z);
        assert!((moved.dot(side) - 45.0).abs() < 10.0 && moved.dot(f).abs() < 10.0, "{a}: moved {moved:?}");
        assert!(facing(&p) > 0.99, "{a}: turned");
    }

    let mut p = darci(&level, at, f);
    let from = p.fig.pos;
    run(&mut p, &level, strafe(-f), 1.0, 30.0, |_| false);
    assert_eq!(p.fig.anim(), anim::BACK_WALK);
    assert!((p.fig.pos.x - from.x) < -60.0 && facing(&p) > 0.99, "backwards: {:?}", p.fig.pos - from);

    let mut p = darci(&level, at, f);
    let from = p.fig.pos;
    run(&mut p, &level, Controls { jump: true, ..strafe(-f) }, DT, 60.0, |_| false);
    assert_eq!(p.fig.anim(), anim::BACK_FLIP);
    until(&mut p, &level, Controls::default(), 4.0, 60.0, |p| matches!(p.state, State::Move));
    assert!((p.fig.pos.x - from.x) < -40.0 && facing(&p) > 0.99, "flipped back: {:?}", p.fig.pos - from);

    // Turning: the camera round to her right.
    let mut p = darci(&level, at, f);
    let turn = Controls { strafe: Some(right), ..Default::default() };
    run(&mut p, &level, turn, DT, 30.0, |_| false);
    assert_eq!(p.fig.anim(), anim::ROTATE_R);
    until(&mut p, &level, turn, 3.0, 30.0, |p| p.fig.anim() != anim::ROTATE_R);
    assert!(p.fig.facing().dot(right) > 0.99);
    assert!((p.fig.pos - at).length() < 300.0 && (vec2(p.fig.pos.x, p.fig.pos.z) - vec2(at.x, at.z)).length() < 1.0, "turned on the spot");
}

/// Up from a knockdown with a two-handed gun, they hold it ready (`ANIM_SHOTGUN_STAND`,
/// `set_person_locked_idle_ready`).
#[test]
fn up_again_with_a_shotgun_ready() {
    use uc_world::character::anim;
    let Some(level) = level() else { return };
    let at = vec3(46.5 * 256.0, 0.0, 55.0 * 256.0);
    let mut p = person(&level, ptype::COP, at, vec2(1.0, 0.0));
    p.give(uc_game::Weapon::Shotgun);
    p.weapon = uc_game::Weapon::Shotgun;
    p.take_hit(at + vec3(100.0, 0.0, 0.0), 10, ko::KNOCKDOWN, false, false, uc_game::Weapon::None);
    until(&mut p, &level, Controls::default(), 8.0, 60.0, |p| matches!(p.state, State::GetUp));
    until(&mut p, &level, Controls::default(), 5.0, 60.0, |p| matches!(p.state, State::Move));
    assert_eq!(p.fig.anim(), anim::SHOTGUN_STAND);
}
