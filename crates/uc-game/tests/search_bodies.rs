//! Searching someone dead, out cold or arrested (`SUB_STATE_SEARCH_CORPSE`): they keep their
//! weapons and whatever their point says they drop until the player holds action over them,
//! and then it's all put on the ground round them. Skipped without an install.

use uc_game::person::ko;
use uc_game::{Controls, Weapon};

mod common;
use common::{Run, DT};

impl Run {
    /// Stands the player on person `i`'s pelvis and holds action for `secs` (the viewer
    /// passes action held on as `sprint`, the first frame too).
    fn search(&mut self, i: usize, secs: f32) {
        let at = self.g.people[i].fig.hips();
        self.stand(at);
        self.play(DT, Controls { action: true, sprint: true, ..Default::default() });
        self.play(secs, Controls { sprint: true, ..Default::default() });
    }

    /// The items lying within `r` of person `i`'s pelvis.
    fn items_by(&self, i: usize, r: f32) -> Vec<u8> {
        let at = self.g.people[i].fig.hips();
        self.g.items.iter().filter(|it| (it.pos - at).length() < r).map(|it| it.kind).collect()
    }
}

/// Gun Hunt's armed suspect (point 74) has a pistol. Knocked out he keeps it; searched, he
/// gives it up, and there's nothing more on him the second time.
#[test]
fn a_knocked_out_suspect_gives_up_his_gun_when_searched() {
    let Some(mut r) = common::start("police3") else { return };
    r.play(3.0, Controls::default());
    // As the mission goes: Deeks says where the gun is, Darci picks it up, its owner comes.
    let deeks = r.who(153);
    let at = r.g.people[deeks].fig.pos;
    r.stand(at + glam::vec3(120.0, 0.0, 0.0));
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(40.0, Controls::default());
    let gun = r.g.items.iter().find(|it| it.way == 54).expect("the stashed weapon").pos;
    r.stand(gun);
    r.play(22.0, Controls::default());
    let i = r.who(74);
    check(&mut r, i);
}

fn check(r: &mut Run, i: usize) {
    assert_eq!(r.g.people[i].weapon, Weapon::Pistol, "the suspect should be armed");
    let from = r.g.people[i].fig.pos + glam::vec3(50.0, 0.0, 0.0);
    r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    r.play(2.0, Controls::default());
    assert!(r.g.people[i].knocked_out || r.g.people[i].is_dead(), "he should be down for good");
    assert!(!r.items_by(i, 200.0).contains(&2), "the gun shouldn't drop by itself");
    assert_eq!(r.g.people[i].weapon, Weapon::Pistol, "he keeps it until searched");

    // Too short a search finds nothing...
    r.search(i, 0.5);
    assert!(!r.items_by(i, 200.0).contains(&2), "half a second's search shouldn't find it");
    // ...the whole of one does (the bar fills in 3.2 s).
    r.play(0.5, Controls::default());
    let rounds = r.g.player().unwrap().ammo[Weapon::Pistol.slot().unwrap()];
    r.search(i, 3.5);
    assert!(r.g.people[i].searched, "he should be searched");
    // It lands by her feet, and she walks over it: on the ground still, or picked up.
    let took = r.g.player().unwrap().ammo[Weapon::Pistol.slot().unwrap()] > rounds;
    assert!(took || r.items_by(i, 200.0).contains(&2), "the search should turn up his pistol");
    assert_eq!(r.g.people[i].weapon, Weapon::None);
}

/// Darci can't search someone on their feet.
#[test]
fn nobody_on_their_feet_is_searched() {
    let Some(mut r) = common::start("police3") else { return };
    r.play(1.0, Controls::default());
    let i = r.g.people.iter().position(|p| p.way == 153).expect("Deeks");
    r.search(i, 0.5);
    assert!(r.g.search.is_none(), "Deeks is on his feet: nothing to search");
}

/// The drop on an enemy's point reaches them: Cop Killers' Wildcats carry health packs
/// (`SPECIAL_HEALTH`, item 3) for whoever searches them.
#[test]
fn enemies_carry_their_points_drop() {
    let Some(mut r) = common::start("AWOL1") else { return };
    r.play(1.0, Controls::default());
    let carrying = r.g.people.iter().filter(|p| p.drop == Some(3)).count();
    assert!(carrying > 0, "nobody carries a health pack");
}
