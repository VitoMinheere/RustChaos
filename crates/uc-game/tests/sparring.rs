//! The sparring room (#547): the opponent does as he's told, the blows that land are named
//! and counted, moves are ticked off as they're done, and nobody stays down. On the combat
//! tutorial's map; skipped without an install.

use glam::{vec2, vec3, Vec2};
use uc_game::game::spar::Stance;
use uc_game::moves::Tech;
use uc_game::{Controls, Game, State, Weapon};
use uc_world::Level;

use crate::common;
use common::DT;

fn room() -> Option<(Level, Game)> {
    let level = common::load("ftutor1")?;
    let g = Game::new_spar(&level);
    Some((level, g))
}

/// Stands the player `dist` from the opponent, on the side she started, facing him. The way
/// from her to him.
fn square_up(g: &mut Game, level: &Level, dist: f32) -> Vec2 {
    let (me, him) = (g.player.unwrap(), g.spar().unwrap().opponent);
    let (a, b) = (g.people[me].fig.pos, g.people[him].fig.pos);
    let d = vec2(b.x - a.x, b.z - a.z).normalize();
    let at = b - vec3(d.x, 0.0, d.y) * dist;
    let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
    g.people[me].fig.pos = vec3(at.x, y, at.z);
    g.people[me].fig.angle = uc_world::character::angle_of(d);
    d
}

/// Plays `secs` with `c` held, and the buttons in `tap` pressed ten times a second.
fn play(g: &mut Game, level: &Level, secs: f32, c: Controls, tap: Controls) {
    for i in 0..(secs / DT) as usize {
        let press = i.is_multiple_of(3);
        let c = Controls { punch: tap.punch && press, kick: tap.kick && press, action: tap.action && press, ..c };
        g.update(DT, &c, level);
    }
}

fn punch() -> Controls {
    Controls { punch: true, ..Default::default() }
}

fn kick() -> Controls {
    Controls { kick: true, ..Default::default() }
}

#[test]
fn the_room_has_the_two_of_them_and_no_script() {
    let Some((level, mut g)) = room() else { return };
    let s = g.spar().unwrap().clone();
    let (me, him) = (g.player.unwrap(), s.opponent);
    assert_eq!(g.people.iter().filter(|p| !p.removed).count(), 2, "the tutorial's own people aren't here");
    assert!(!s.two && s.second.is_some_and(|k| g.people[k].removed), "the second man waits outside");
    assert_eq!(s.stance, Stance::Still);
    let apart = (g.people[me].fig.pos - g.people[him].fig.pos).length();
    assert!((300.0..800.0).contains(&apart), "{apart} apart");
    assert_eq!(g.people[me].weapon, Weapon::None, "bare hands to start with");
    assert!(g.people[me].has(Weapon::Knife) && g.people[me].has(Weapon::Bat) && g.people[me].has(Weapon::Pistol));
    // Left alone he stands where he is, and nothing ends it.
    let at = g.people[him].fig.pos;
    play(&mut g, &level, 10.0, Controls::default(), Controls::default());
    assert!((g.people[him].fig.pos - at).length() < 8.0, "he wandered off");
    assert!(g.outcome.is_none());
    assert!(g.messages.is_empty(), "{:?}", g.messages);
}

#[test]
fn a_punch_combo_is_named_counted_and_ticked() {
    let Some((level, mut g)) = room() else { return };
    square_up(&mut g, &level, 100.0);
    play(&mut g, &level, 2.5, Controls::default(), punch());
    let s = g.spar().unwrap();
    let names: Vec<&str> = s.landed.iter().map(|l| l.name).collect();
    assert!(names.starts_with(&["PUNCH", "SECOND PUNCH", "PUNCH COMBO"]), "{names:?}");
    assert!(s.landed.iter().all(|l| l.damage > 0));
    assert!(s.best_combo >= 3, "a run of {}", s.best_combo);
    assert!(s.done.contains(&Tech::PunchCombo));
    assert_eq!(s.ticked.map(|t| t.0), Some(Tech::PunchCombo));
    // The run's over once nothing more lands.
    play(&mut g, &level, 3.0, Controls::default(), Controls::default());
    assert_eq!(g.spar().unwrap().combo, 0);
}

#[test]
fn the_move_asked_for_is_waited_for() {
    let Some((level, mut g)) = room() else { return };
    // (Only the moves the room can see done.)
    g.spar_want(Some(Tech::Run));
    assert_eq!(g.spar().unwrap().want, None);
    g.spar_want(Some(Tech::KickCombo));
    square_up(&mut g, &level, 100.0);
    play(&mut g, &level, 2.5, Controls::default(), punch());
    assert_eq!(g.spar().unwrap().want, Some(Tech::KickCombo), "punches aren't it");
    // (He's down after the combo: once he's up again.)
    let him = g.spar().unwrap().opponent;
    for _ in 0..40 {
        if g.people[him].on_feet() {
            break;
        }
        play(&mut g, &level, 0.5, Controls::default(), Controls::default());
    }
    square_up(&mut g, &level, 130.0);
    play(&mut g, &level, 3.0, Controls::default(), kick());
    let s = g.spar().unwrap();
    assert!(s.done.contains(&Tech::KickCombo), "{:?}", s.landed);
    assert_eq!(s.want, None);
    assert_eq!(s.ticked.map(|t| t.0), Some(Tech::KickCombo));
}

#[test]
fn he_blocks_when_told_to_and_the_sweep_gets_under_it() {
    let Some((level, mut g)) = room() else { return };
    g.spar_stance(Stance::Blocks);
    square_up(&mut g, &level, 100.0);
    play(&mut g, &level, 0.5, Controls::default(), Controls::default());
    play(&mut g, &level, 3.0, Controls::default(), punch());
    let him = g.spar().unwrap().opponent;
    assert_eq!(g.people[him].health, g.people[him].max_health, "punches got through his guard");
    assert!(g.spar().unwrap().landed.is_empty());
    // Kick from a block: the foot sweep.
    square_up(&mut g, &level, 100.0);
    let guard = Controls { block: true, ..Default::default() };
    let me = g.player.unwrap();
    for _ in 0..(5.0 / DT) as usize {
        if matches!(g.people[me].state, State::Block { .. }) {
            break;
        }
        g.update(DT, &guard, &level);
    }
    g.update(DT, &Controls { kick: true, ..guard }, &level);
    play(&mut g, &level, 1.5, guard, Controls::default());
    assert!(g.spar().unwrap().done.contains(&Tech::FootSweep), "{:?}", g.spar().unwrap().landed);
}

#[test]
fn he_fights_back_and_she_never_goes_down_for_good() {
    let Some((level, mut g)) = room() else { return };
    let (me, him) = (g.player.unwrap(), g.spar().unwrap().opponent);
    g.spar_stance(Stance::Bat);
    assert_eq!(g.people[him].weapon, Weapon::Bat);
    let mut hurt = false;
    for _ in 0..(40.0 / DT) as usize {
        g.update(DT, &Controls::default(), &level);
        hurt |= !g.people[me].on_feet() || g.people[me].health < g.people[me].max_health;
    }
    assert!(g.outcome.is_none(), "{:?}", g.outcome);
    assert_eq!(g.people[me].health, g.people[me].max_health);
    assert!(!g.people[me].knocked_out);
    // Her guard up, his blows are blocks of hers.
    square_up(&mut g, &level, 100.0);
    g.spar_stance(Stance::Fights);
    assert_eq!(g.people[him].weapon, Weapon::None);
    let guard = Controls { block: true, ..Default::default() };
    for _ in 0..(30.0 / DT) as usize {
        g.update(DT, &guard, &level);
        if g.spar().unwrap().done.contains(&Tech::Block) {
            break;
        }
    }
    assert!(hurt || g.spar().unwrap().done.contains(&Tech::Block), "he never laid a hand on her");
    // Knocked down, he let go of the bat; up again, he has it back, and none lie about.
    assert!(g.items.is_empty(), "{:?}", g.items);
    assert!(g.spar().unwrap().done.contains(&Tech::Block));
    // Called off, he stands still again.
    g.spar_stance(Stance::Still);
    play(&mut g, &level, 3.0, Controls::default(), Controls::default());
    let at = g.people[him].fig.pos;
    play(&mut g, &level, 4.0, Controls::default(), Controls::default());
    assert!((g.people[him].fig.pos - at).length() < 8.0, "still coming at her");
}

#[test]
fn knocked_out_or_cuffed_he_is_back_on_his_feet() {
    let Some((level, mut g)) = room() else { return };
    let him = g.spar().unwrap().opponent;
    // Shot until he's out: the pistol's rounds never run down.
    let me = g.player.unwrap();
    g.people[me].weapon = Weapon::Pistol;
    square_up(&mut g, &level, 300.0);
    let aim = Controls { target: Some(him), ..Default::default() };
    for _ in 0..(30.0 / DT) as usize {
        if g.people[him].knocked_out {
            break;
        }
        let i = g.turns();
        g.update(DT, &Controls { punch: i.is_multiple_of(12), ..aim }, &level);
    }
    assert!(g.people[him].knocked_out, "he took {:?}", g.spar().unwrap().landed);
    assert!(g.spar().unwrap().done.contains(&Tech::Shoot));
    assert!(g.people[me].ammo[0] >= Weapon::Pistol.clip());
    play(&mut g, &level, 8.0, Controls::default(), Controls::default());
    assert!(g.people[him].on_feet() && g.people[him].health == g.people[him].max_health, "{:?}", g.people[him].state);
    // Knocked down and cuffed.
    g.people[me].weapon = Weapon::None;
    square_up(&mut g, &level, 100.0);
    play(&mut g, &level, 2.5, Controls::default(), punch());
    // (Beside where he lies, which isn't where his figure's origin is.)
    for _ in 0..(8.0 / DT) as usize {
        if g.people[him].arrested {
            break;
        }
        let standing = matches!(g.people[me].state, State::Move);
        if standing && matches!(g.people[him].state, State::Down) {
            let body = g.people[him].body_pos();
            g.people[me].fig.pos = body + vec3(70.0, 0.0, 0.0);
            g.people[me].fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
        }
        let i = g.turns();
        g.update(DT, &Controls { action: standing && i.is_multiple_of(10), ..Default::default() }, &level);
    }
    if g.people[him].arrested {
        assert!(g.spar().unwrap().done.contains(&Tech::Arrest));
        play(&mut g, &level, 4.0, Controls::default(), Controls::default());
        assert!(!g.people[him].arrested && g.people[him].on_feet(), "{:?}", g.people[him].state);
    } else {
        panic!("never cuffed: {:?}", g.people[him].state);
    }
}

/// The same room played the same way comes out the same.
#[test]
fn the_room_plays_the_same_twice() {
    let Some((level, _)) = room() else { return };
    let run = || {
        let mut g = Game::new_spar(&level);
        g.spar_stance(Stance::Fights);
        play(&mut g, &level, 12.0, Controls::default(), punch());
        g.state_hash()
    };
    assert_eq!(run(), run());
}

/// Waits (up to ten seconds) for him to be standing ready again, and her done with whatever
/// she was doing.
fn on_his_feet(g: &mut Game, level: &Level) {
    let him = g.spar().unwrap().opponent;
    for _ in 0..(10.0 / DT) as usize {
        let me = g.player.unwrap();
        if g.people[him].on_feet() && matches!(g.people[him].state, State::Move) && matches!(g.people[me].state, State::Move) {
            return;
        }
        g.update(DT, &Controls::default(), level);
    }
    panic!("he never got up: {:?}", g.people[him].state);
}

/// Plays until `tech` is ticked (up to `secs`), with what `c` gives each turn.
fn until_done(g: &mut Game, level: &Level, secs: f32, tech: Tech, c: impl Fn(&Game, usize) -> Controls) {
    for i in 0..(secs / DT) as usize {
        if g.spar().unwrap().done.contains(&tech) {
            return;
        }
        let c = c(g, i);
        g.update(DT, &c, level);
    }
    let s = g.spar().unwrap();
    let me = g.player.unwrap();
    panic!("{tech:?} never ticked: she's {:?}, he's {:?}, landed {:?}", g.people[me].state, g.people[s.opponent].state, s.landed);
}

/// The moves that aren't a plain blow are seen too: the grab and what follows it, the
/// takedown, the flip, the slide, the stamp and the rest. Every move the room ticks can be.
#[test]
fn every_kind_of_move_is_seen() {
    let Some((level, mut g)) = room() else { return };
    let (me, him) = (g.player.unwrap(), g.spar().unwrap().opponent);
    let tap = |i: usize| i.is_multiple_of(3);
    // Walking into him with a punch: the arm grab, then a knee.
    let d = square_up(&mut g, &level, 80.0);
    until_done(&mut g, &level, 3.0, Tech::ArmGrab, |_, i| Controls { wish: d, punch: tap(i), ..Default::default() });
    until_done(&mut g, &level, 4.0, Tech::Knee, |_, i| Controls { punch: tap(i), ..Default::default() });
    // Grabbed again and thrown.
    on_his_feet(&mut g, &level);
    let d = square_up(&mut g, &level, 80.0);
    until_done(&mut g, &level, 6.0, Tech::Throw, |g, i| {
        let holding = matches!(g.people[g.player.unwrap()].state, State::Grapple { .. });
        Controls { wish: if holding { Vec2::ZERO } else { d }, punch: !holding && tap(i), kick: holding && tap(i), ..Default::default() }
    });
    // Lying there, stamped on.
    for _ in 0..(3.0 / DT) as usize {
        if matches!(g.people[him].state, State::Down) && matches!(g.people[me].state, State::Move) {
            break;
        }
        g.update(DT, &Controls::default(), &level);
    }
    let body = g.people[him].body_pos();
    g.people[me].fig.pos = body + vec3(70.0, 0.0, 0.0);
    g.people[me].fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
    until_done(&mut g, &level, 3.0, Tech::Stomp, |_, i| Controls { kick: tap(i), ..Default::default() });
    // Stepping in on him: between the legs.
    on_his_feet(&mut g, &level);
    let d = square_up(&mut g, &level, 90.0);
    until_done(&mut g, &level, 6.0, Tech::GroinKick, |g, i| {
        let standing = matches!(g.people[g.player.unwrap()].state, State::Move);
        Controls { wish: if standing { d } else { Vec2::ZERO }, kick: i % 20 == 4, ..Default::default() }
    });
    // The kicks to the side and behind land on nobody with one man in the room: she's turned
    // to face him. Asked for, they bring the second in, and land on whoever of the two is
    // at her side or behind her.
    for tech in [Tech::SideKick, Tech::BackKick] {
        on_his_feet(&mut g, &level);
        g.spar_two(false);
        let d = square_up(&mut g, &level, 110.0);
        let turn = if tech == Tech::SideKick { vec2(-d.y, d.x) } else { -d };
        let kicking = |g: &Game, i: usize| {
            let standing = matches!(g.people[g.player.unwrap()].state, State::Move);
            Controls { wish: if standing { turn } else { Vec2::ZERO }, kick: standing && i % 20 == 4, ..Default::default() }
        };
        for i in 0..(3.0 / DT) as usize {
            let c = kicking(&g, i);
            g.update(DT, &c, &level);
        }
        assert!(!g.spar().unwrap().done.contains(&tech), "{tech:?} landed with nobody there");
        on_his_feet(&mut g, &level);
        g.spar_want(Some(tech));
        let second = g.spar().unwrap().second.unwrap();
        assert!(g.spar().unwrap().two && !g.people[second].removed, "no second man for {tech:?}");
        // The second man where the kick goes, a little further off than the first, whom
        // she faces.
        // He comes over by himself to where the kick goes: behind her, or at her side.
        let d = square_up(&mut g, &level, 90.0);
        let turn = if tech == Tech::SideKick { vec2(d.y, -d.x) } else { -d };
        let spot = g.people[me].fig.pos + vec3(turn.x, 0.0, turn.y) * 125.0;
        for _ in 0..(15.0 / DT) as usize {
            if (g.people[second].fig.pos - spot).length() < 60.0 {
                break;
            }
            g.update(DT, &Controls::default(), &level);
        }
        let off = (g.people[second].fig.pos - spot).length();
        assert!(off < 60.0, "the second man is {off} from where the {tech:?} goes");
        // (The way and the kick together: a step that way first and it's him she'd face.)
        until_done(&mut g, &level, 6.0, tech, |g, i| {
            let now = matches!(g.people[g.player.unwrap()].state, State::Move) && i % 30 == 0;
            Controls { wish: if now { turn } else { Vec2::ZERO }, kick: now, ..Default::default() }
        });
        assert_eq!(g.spar().unwrap().want, None);
    }
    g.spar_two(false);
    assert!(g.people[g.spar().unwrap().second.unwrap()].removed);
    on_his_feet(&mut g, &level);
    square_up(&mut g, &level, 100.0);
    until_done(&mut g, &level, 5.0, Tech::JumpKick, |_, i| Controls { jump: i % 30 == 5, ..Default::default() });
    // A flip to the side, in a fight.
    on_his_feet(&mut g, &level);
    let d = square_up(&mut g, &level, 150.0);
    until_done(&mut g, &level, 5.0, Tech::SideFlip, |_, i| Controls { wish: vec2(-d.y, d.x), jump: i % 10 == 5, ..Default::default() });
    // Run at him and slide.
    on_his_feet(&mut g, &level);
    let d = square_up(&mut g, &level, 500.0);
    until_done(&mut g, &level, 6.0, Tech::SlideTackle, |_, i| Controls { wish: d, run: true, kick: i == 12, ..Default::default() });
    // From behind, with him none the wiser: the takedown.
    on_his_feet(&mut g, &level);
    let f = g.people[him].fig.facing();
    let at = g.people[him].fig.pos - vec3(f.x, 0.0, f.y) * 80.0;
    g.people[me].fig.pos = at;
    g.people[me].fig.angle = g.people[him].fig.angle;
    until_done(&mut g, &level, 3.0, Tech::Takedown, |_, i| Controls { punch: tap(i), ..Default::default() });
    // Knife and bat.
    for (w, tech) in [(Weapon::Knife, Tech::KnifeCombo), (Weapon::Bat, Tech::BatCombo)] {
        on_his_feet(&mut g, &level);
        g.people[me].weapon = w;
        square_up(&mut g, &level, 100.0);
        until_done(&mut g, &level, 6.0, tech, |_, i| Controls { punch: tap(i), ..Default::default() });
    }
    // The shotgun, toe to toe: he's clubbed with it.
    on_his_feet(&mut g, &level);
    g.people[me].weapon = Weapon::Shotgun;
    square_up(&mut g, &level, 90.0);
    until_done(&mut g, &level, 5.0, Tech::GunClub, |_, i| Controls { punch: tap(i), ..Default::default() });
    assert_eq!(g.spar().unwrap().done.len(), uc_game::moves::sparred() - 6, "all but the six the other tests do");
}
