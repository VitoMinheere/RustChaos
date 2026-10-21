//! Co-op (#182): a second seat, Roper next to Darci, played with his own input; enemies who
//! go for whichever of them is nearer; the script's radius triggers for either; and two
//! co-op runs agreeing on the state hash every second, as lockstep needs. Skipped without
//! an install.

use glam::{vec2, vec3};
use uc_formats::ucm::trigger;
use uc_game::{Controls, Game};
use uc_world::character::person as ptype;

mod common;
use common::Run;

impl Run {
    /// Starts `mission` with a partner. His index.
    fn coop(mission: &str) -> Option<(Run, usize)> {
        let mut r = Run::new(common::load(mission)?);
        let roper = r.g.add_partner(&r.level).expect("a partner");
        Some((r, roper))
    }

    /// Plays `turns` turns with each seat's controls, skipping cutscenes.
    fn step(&mut self, turns: usize, inputs: &[Controls]) {
        for _ in 0..turns {
            self.g.skip_cutscene();
            self.g.step(inputs, &self.level);
        }
    }
}

#[test]
fn roper_stands_next_to_darci() {
    let Some((r, roper)) = Run::coop("police1") else { return };
    let darci = r.player();
    let (d, p) = (&r.g.people[darci], &r.g.people[roper]);
    assert_eq!(p.kind, ptype::ROPER);
    assert!(p.player, "Roper is a player, not the AI's");
    assert!((d.fig.pos - p.fig.pos).length() < 200.0, "{:?} and {:?}", d.fig.pos, p.fig.pos);
    assert_eq!(r.g.seats(), 2);
    assert_eq!(r.g.seat_person(1), Some(roper));
}

#[test]
fn each_seat_moves_on_its_own_input() {
    let Some((mut r, roper)) = Run::coop("police1") else { return };
    let darci = r.player();
    let (d0, p0) = (r.g.people[darci].fig.pos, r.g.people[roper].fig.pos);
    // Away from her, not into her.
    let away = vec2(p0.x - d0.x, p0.z - d0.z).normalize();
    let walk = Controls { wish: away, ..Default::default() };
    r.step(60, &[Controls::default(), walk]);
    let (d1, p1) = (r.g.people[darci].fig.pos, r.g.people[roper].fig.pos);
    assert!((p1 - p0).length() > 150.0, "Roper walked {}", (p1 - p0).length());
    assert!((d1 - d0).length() < 50.0, "Darci stood still, but moved {}", (d1 - d0).length());
}

/// What the viewer looks through doesn't change how the game plays: two runs, one with
/// Roper's seat in focus between turns, agree all the way.
#[test]
fn the_seat_in_focus_makes_no_difference() {
    let run = |focus: usize| -> Option<Vec<u64>> {
        let (mut r, _) = Run::coop("police1")?;
        let mut out = Vec::new();
        for t in 0..20 * 30 {
            r.g.focus(focus);
            r.step(1, &[input(t / 30), input(t / 30 + 3)]);
            assert_eq!(r.g.focused(), focus);
            if t % 30 == 0 {
                out.push(r.g.state_hash());
            }
        }
        Some(out)
    };
    let Some(a) = run(0) else { return };
    let b = run(1).unwrap();
    assert_eq!(a, b);
}

#[test]
fn the_focused_seat_is_who_the_player_is() {
    let Some((mut r, roper)) = Run::coop("police1") else { return };
    let darci = r.player();
    r.g.focus(1);
    assert_eq!(r.g.player, Some(roper));
    assert_eq!(r.g.player_name(), "Roper");
    assert_eq!(r.g.seat_person(0), Some(darci));
    r.g.focus(0);
    assert_eq!(r.g.player, Some(darci));
}

/// The music follows each seat's own player: Darci sprinting is sprint music for her, while
/// Roper standing about stays calm (he doesn't sprint himself).
#[test]
fn each_seat_has_its_own_music() {
    let Some((mut r, roper)) = Run::coop("police1") else { return };
    let darci = r.player();
    let (d, p) = (r.g.people[darci].fig.pos, r.g.people[roper].fig.pos);
    let sprint = Controls { wish: vec2(d.x - p.x, d.z - p.z).normalize(), run: true, sprint: true, ..Default::default() };
    r.step(30, &[sprint, Controls::default()]);
    assert_eq!(r.g.mood(), uc_game::Mood::Sprint);
    r.g.focus(1);
    assert_eq!(r.g.mood(), uc_game::Mood::Calm);
}

/// The inputs at second `t`: walking about, running, fighting, crouching, a shot or two.
fn input(t: usize) -> Controls {
    let mut c = Controls::default();
    match t % 8 {
        0 | 1 => c.wish = vec2(0.0, 1.0),
        2 => { c.wish = vec2(1.0, 0.5); c.sprint = true }
        3 => c.punch = true,
        4 => c.kick = true,
        5 => c.crouch = true,
        6 => { c.wish = vec2(-0.7, -0.7); c.jump = true }
        _ => { c.fire = true; c.switch = t % 16 == 7 }
    }
    c
}

fn coop_hashes(mission: &str, secs: usize) -> Option<Vec<u64>> {
    let (mut r, _) = Run::coop(mission)?;
    let mut out = Vec::new();
    for t in 0..secs {
        r.step(30, &[input(t), input(t + 3)]);
        out.push(r.g.state_hash());
    }
    Some(out)
}

#[test]
fn two_coop_runs_agree_every_second() {
    for mission in ["police1", "Gangorder1", "Finale1"] {
        let Some(a) = coop_hashes(mission, 40) else { return };
        let b = coop_hashes(mission, 40).unwrap();
        if let Some(t) = a.iter().zip(&b).position(|(x, y)| x != y) {
            panic!("{mission}: the co-op runs part after {} s", t + 1);
        }
    }
}

/// Without a partner, stepping turn by turn is single player as before: `update` and `step`
/// with seat 0's input agree.
#[test]
fn step_with_one_seat_is_update() {
    let Some(level) = common::load("police1") else { return };
    let (mut a, mut b) = (Game::new(&level, true), Game::new(&level, true));
    for t in 0..10 * 30 {
        let c = input(t / 30);
        a.update(1.0 / 30.0, &c, &level);
        b.step(&[c], &level);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

/// A radius trigger the mission's waiting on fires for Roper standing in it, with Darci far
/// away.
#[test]
fn roper_sets_off_radius_triggers() {
    let Some((mut r, roper)) = Run::coop("police1") else { return };
    r.step(30, &[Controls::default(), Controls::default()]);
    let points: Vec<_> = r.level.mission.event_points.iter()
        .filter(|e| e.used != 0 && e.trigger == trigger::RADIUS && e.radius > 0 && !r.g.point_active(e.index))
        .map(|e| (e.index, vec3(e.x as f32, e.y as f32, e.z as f32)))
        .collect();
    assert!(!points.is_empty());
    let darci = r.player();
    let far = r.g.people[darci].fig.pos;
    let mut fired = 0;
    for (i, at) in points {
        if (at - far).length() < 8.0 * 256.0 {
            continue;
        }
        let y = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
        r.g.people[roper].fig.pos = vec3(at.x, y, at.z);
        r.step(5, &[Controls::default(), Controls::default()]);
        if r.g.point_active(i) {
            fired += 1;
        }
        r.g.people[darci].fig.pos = far;
    }
    assert!(fired > 0, "no radius point fired for Roper");
}

/// Knocked out, Roper is back on his feet next to Darci a few seconds later, and the
/// mission goes on.
#[test]
fn roper_gets_up_again() {
    let Some((mut r, roper)) = Run::coop("police1") else { return };
    r.step(30, &[Controls::default(), Controls::default()]);
    let p = &mut r.g.people[roper];
    p.health = 0;
    p.knocked_out = true;
    r.step(8 * 30, &[Controls::default(), Controls::default()]);
    let darci = r.player();
    let p = &r.g.people[roper];
    assert!(!p.knocked_out && p.health == p.max_health, "still down");
    assert!((p.fig.pos - r.g.people[darci].fig.pos).length() < 200.0);
    assert!(r.g.outcome.is_none(), "{:?}", r.g.outcome);
}

/// The enemies go for whichever player is nearer: with Roper in among them and Darci far
/// off, whoever's after anyone is after him.
#[test]
fn enemies_go_for_the_nearer_player() {
    let Some((mut r, roper)) = Run::coop("Gangorder1") else { return };
    r.step(30, &[Controls::default(), Controls::default()]);
    let darci = r.player();
    let foes: Vec<_> = r.g.people.iter().enumerate()
        .filter(|(i, p)| *i != darci && *i != roper && p.hostile() && !p.is_dead())
        .map(|(i, _)| i)
        .collect();
    let Some(&e) = foes.first() else { return };
    let at = r.g.people[e].fig.pos;
    r.g.people[roper].fig.pos = at + vec3(150.0, 0.0, 0.0);
    r.g.people[darci].fig.pos = at + vec3(30.0 * 256.0, 0.0, 0.0);
    r.step(2, &[Controls::default(), Controls::default()]);
    assert_eq!(r.g.people[e].foe, Some(roper));
}


/// Roper gets into a car with action, as Darci does, and drives it on his own input.
#[test]
fn roper_drives_a_car() {
    let Some((mut r, roper)) = Run::coop("police1") else { return };
    r.step(30, &[Controls::default(), Controls::default()]);
    // One anybody can open: some of the mission's cars never do, for her either.
    let Some(car) = r.g.vehicles.iter().position(|v| v.driver.is_none() && !v.wrecked() && !v.locked()) else { return };
    let door = r.g.vehicles[car].door();
    let y = r.level.ground.surface(&r.level.map, door.x, door.y, r.g.vehicles[car].pos.y + 64.0);
    r.g.people[roper].fig.pos = vec3(door.x, y, door.y);
    // Arriving there can start one of the mission's conversations, which holds him still.
    for _ in 0..30 * 30 {
        r.step(1, &[Controls::default(), Controls::default()]);
        if !r.g.in_conversation() {
            break;
        }
    }
    r.g.people[roper].fig.pos = vec3(door.x, y, door.y);
    r.step(1, &[Controls::default(), Controls { action: true, ..Default::default() }]);
    // Getting in takes the door's animation, a little over two seconds.
    r.step(90, &[Controls::default(), Controls::default()]);
    assert!(matches!(r.g.people[roper].state, uc_game::State::InCar { vehicle } if vehicle == car), "{:?}", r.g.people[roper].state);
    let at = r.g.vehicles[car].pos;
    let gas = Controls { drive: uc_game::Drive { throttle: 1.0, ..Default::default() }, ..Default::default() };
    r.step(60, &[Controls::default(), gas]);
    assert!((r.g.vehicles[car].pos - at).length() > 100.0, "the car went {}", (r.g.vehicles[car].pos - at).length());
}
