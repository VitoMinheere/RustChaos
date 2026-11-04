//! The characters' lines that were recorded and never played (#486), each at its moment:
//! the cue comes out of the game, and the app plays a take of it. Skipped without an install.

use glam::vec3;
use uc_game::person::ko;
use uc_game::{Bark, Controls, Sfx, State, Weapon};
use uc_world::character::person as ptype;

mod common;
use common::{Run, DT};

impl Run {
    /// The barks said since the last call, by whom (their type).
    fn barks(&mut self) -> Vec<(u8, Bark)> {
        self.g.take_sounds().into_iter().filter_map(|(s, _)| match s {
            Sfx::Bark { kind, bark } => Some((kind, bark)),
            _ => None,
        }).collect()
    }
}

#[test]
fn an_empty_clip_is_moaned_about() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    r.barks();
    let me = r.player();
    r.g.people[me].give(Weapon::Pistol);
    r.g.people[me].weapon = Weapon::Pistol;
    r.g.people[me].ammo = [0; 4];
    for _ in 0..20 {
        r.play(DT, Controls { punch: true, ..Default::default() });
        r.play(DT, Controls::default());
    }
    let all: Vec<Sfx> = r.g.take_sounds().into_iter().map(|(s, _)| s).filter(|s| matches!(s, Sfx::Bark { .. } | Sfx::Click | Sfx::Shot(_))).collect();
    assert!(all.contains(&Sfx::Bark { kind: ptype::DARCI, bark: Bark::Complain }));
}

#[test]
fn a_cop_greets_her_once() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    // Ones she hasn't been near yet (RTA starts her by one), and not busy: some go after the
    // wanted as she comes, so each is tried in turn.
    let here = r.g.people[me].fig.pos;
    let cops: Vec<usize> = (0..r.g.people.len()).filter(|&i| i != me && r.g.people[i].kind == ptype::COP && matches!(r.g.people[i].state, State::Move)
        && !r.g.people[i].aggro() && (r.g.people[i].fig.pos - here).length() > 1024.0).collect();
    let mut greeter = None;
    for &cop in &cops {
        r.barks();
        let at = r.g.people[cop].fig.pos;
        r.stand(at + vec3(120.0, 0.0, 0.0));
        r.play(0.2, Controls::default());
        if r.barks().contains(&(ptype::COP, Bark::Greet)) {
            greeter = Some(cop);
            break;
        }
    }
    let cop = greeter.unwrap_or_else(|| panic!("none of the {} idle cops greeted her", cops.len()));
    // Not again, however long she stays.
    let at = r.g.people[cop].fig.pos;
    r.stand(at + vec3(120.0, 0.0, 0.0));
    r.play(10.0, Controls::default());
    assert!(!r.barks().contains(&(ptype::COP, Bark::Greet)), "greeted twice");
}

#[test]
fn a_frightened_man_cries_out() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    r.barks();
    let man = (0..r.g.people.len()).find(|&i| r.g.people[i].fake && r.g.people[i].kind == ptype::CIV && r.g.people[i].fig.model.body != 9 && r.g.people[i].scared.is_none())
        .expect("a man in the crowd");
    let at = r.g.people[man].fig.pos;
    r.g.people[man].scare(None, at + vec3(200.0, 0.0, 0.0));
    assert!(r.barks().contains(&(ptype::CIV, Bark::Panic)));
}

#[test]
fn standing_about_she_relaxes() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    r.barks();
    r.play(21.0, Controls::default());
    assert!(r.barks().contains(&(ptype::DARCI, Bark::Relax)), "{:?}", r.seen);
}

#[test]
fn a_dead_mans_pockets_are_gross() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    let j = (0..r.g.people.len()).find(|&i| r.g.people[i].fake && !matches!(r.g.people[i].state, State::InCar { .. })).expect("someone in the crowd");
    let at = r.g.people[j].fig.pos;
    r.g.people[j].take_hit(at + vec3(50.0, 0.0, 0.0), 10_000, ko::KNOCKDOWN, false, false, Weapon::None);
    r.play(3.0, Controls::default());
    assert!(r.g.people[j].is_dead(), "not dead: {:?}", r.g.people[j].state);
    let hips = r.g.people[j].fig.hips();
    r.stand(hips + vec3(30.0, 0.0, 0.0));
    r.barks();
    r.play(DT, Controls { action: true, ..Default::default() });
    let _ = me;
    assert!(r.barks().contains(&(ptype::DARCI, Bark::Gross)));
}

/// Takeover1's workers wait for Darci (`PCOM_MOVE_FOLLOW_ON_SEE`): seeing her, one starts
/// following, and she tells him to come along. (They're civilians, not `PERSON_HOSTAGE`, so
/// they don't ask what now: the hostage's voice is someone else's.)
#[test]
fn a_hostage_comes_along() {
    let Some(mut r) = common::start("Takeover1") else { return };
    r.play(1.0, Controls::default());
    r.barks();
    let h = r.who(62);
    r.look_at(h, 200.0);
    // Him facing her too: he follows once he sees her.
    let (from, to) = (r.g.people[h].fig.pos, r.g.people[r.player()].fig.pos);
    r.g.people[h].fig.angle = uc_world::character::angle_of(glam::vec2(to.x - from.x, to.z - from.z));
    r.play(0.5, Controls::default());
    assert_eq!(r.g.people[h].mv, uc_game::person::MOVE_FOLLOW, "he didn't start following");
    let said = r.barks();
    assert!(said.contains(&(ptype::DARCI, Bark::Command)), "{said:?}");
}

/// The player wrecking a police car earns a red mark (`VEH_reduce_health`), and dispatch
/// says so over the radio: "What the hell?" (`police-SURPRISE`).
#[test]
fn a_red_mark_gets_a_what_the_hell() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    r.g.take_sounds();
    let me = r.player();
    let at = r.g.people[me].fig.pos + vec3(2000.0, 0.0, 0.0);
    r.g.vehicles.push(uc_game::Vehicle::new(0, 3, at, 0.0, uc_game::vehicle::Key::from_vk(0)));
    let v = r.g.vehicles.len() - 1;
    r.g.vehicles[v].hurt_by = Some(me);
    r.g.vehicles[v].health = 0;
    r.play(0.5, Controls::default());
    let radio: Vec<Sfx> = r.g.take_sounds().into_iter().map(|(s, _)| s).filter(|s| matches!(s, Sfx::RadioSays(_))).collect();
    assert_eq!(radio, [Sfx::RadioSays(uc_game::RadioLine::Surprise)]);
}

/// Running into a civilian out for a stroll: "Wha-" (`MCIV-SURPRISE`). Walking into one,
/// nothing.
#[test]
fn a_civilian_run_into_says_wha() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    let civ = (0..r.g.people.len()).find(|&i| {
        let p = &r.g.people[i];
        p.kind == ptype::CIV && p.ai == uc_game::person::AI_CIV && p.mv == uc_game::person::MOVE_WANDER && matches!(p.state, State::Move)
    }).expect("someone out for a stroll");
    r.barks();
    let mut said = false;
    for _ in 0..(3.0 / DT) as usize {
        let (from, to) = (r.g.people[me].fig.pos, r.g.people[civ].fig.pos);
        if (to - from).length() > 600.0 {
            r.stand(to + vec3(400.0, 0.0, 0.0));
        }
        let d = glam::vec2(to.x - r.g.people[me].fig.pos.x, to.z - r.g.people[me].fig.pos.z).normalize_or_zero();
        r.play(DT, Controls { wish: d, run: true, ..Default::default() });
        if r.barks().contains(&(ptype::CIV, Bark::Surprise)) {
            said = true;
            break;
        }
    }
    assert!(said, "nothing said running into them");
}

/// A hostage beside someone being cuffed gloats: "I got ya, chump!" (`HOSTAGE-TAUNT`).
#[test]
fn a_hostage_gloats_at_an_arrest() {
    let Some(mut r) = common::start("police1") else { return };
    r.play(1.0, Controls::default());
    let me = r.player();
    let at = r.g.people[me].fig.pos;
    let others: Vec<usize> = (0..r.g.people.len())
        .filter(|&i| i != me && !r.g.people[i].removed && !r.g.people[i].is_dead() && matches!(r.g.people[i].state, State::Move) && r.g.people[i].kind != ptype::COP)
        .collect();
    let (thug, hostage) = (others[0], others[1]);
    r.g.people[hostage].kind = ptype::HOSTAGE;
    r.g.people[thug].fig.pos = at + vec3(80.0, 0.0, 0.0);
    r.g.people[thug].take_hit(at, 0, ko::KNOCKDOWN, false, false, Weapon::None);
    r.play(1.5, Controls::default());
    r.g.people[hostage].fig.pos = r.g.people[thug].body_pos() + vec3(0.0, 0.0, 300.0);
    r.barks();
    let body = r.g.people[thug].body_pos();
    r.stand(body + vec3(60.0, 0.0, 0.0));
    let mut said = Vec::new();
    r.play(DT, Controls { action: true, ..Default::default() });
    for _ in 0..(4.0 / DT) as usize {
        r.g.people[hostage].fig.pos = body + vec3(0.0, 0.0, 300.0);
        r.play(DT, Controls::default());
        said.extend(r.barks());
    }
    assert!(r.g.people[thug].arrested || matches!(r.g.people[thug].state, State::Cuffed), "not arrested");
    assert!(said.contains(&(ptype::HOSTAGE, Bark::Gloat)), "{said:?}");
}
