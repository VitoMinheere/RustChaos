//! Several bodyguards (`PCOM_AI_BODYGUARD`) at once, in the missions that have them: Mako's
//! boys in Nitro Car (`Bankbomb1`, points 3 and 4) and Rat Catch (`Gangorder2`, McLean and
//! Bish, 110 and 113), and Mr X's three in his Getaway (`Getaway1`, 148, 149 and 161). They
//! all keep up, side by side rather than in a heap, ride along, share out who's going for
//! their man, and carry on when one of them is lost. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, State, Weapon};

mod common;
use common::{Run, DT};

impl Run {
    /// Walks up to the person event point `way` created, talks to them, and lets them finish.
    fn talk_to(&mut self, way: usize, secs: f32) {
        let at = self.g.people[self.who(way)].fig.pos;
        self.stand(at + vec3(130.0, 0.0, 0.0));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }

    /// How far person `i` is from the player.
    fn off_player(&self, i: usize) -> f32 {
        self.g.people[i].fig.pos.distance(self.g.player().unwrap().fig.pos)
    }

    /// The way from the player with the longest clear run, of sixteen.
    fn open_way(&self) -> glam::Vec2 {
        let at = self.g.player().unwrap().fig.pos + vec3(0.0, 60.0, 0.0);
        let clear = |d: glam::Vec2| self.level.ground.first_hit(at, at + vec3(d.x, 0.0, d.y) * 4000.0).unwrap_or(1.0);
        (0..16)
            .map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                vec2(a.cos(), a.sin())
            })
            .max_by(|a, b| clear(*a).total_cmp(&clear(*b)))
            .unwrap()
    }
}

/// Rat Catch up to McLean and Bish signing on to follow Mako.
fn rat_catch() -> Option<(Run, [usize; 2])> {
    let mut r = common::start("Gangorder2")?;
    r.play(2.0, Controls::default());
    r.talk_to(10, 45.0);
    r.talk_to(110, 25.0);
    r.talk_to(113, 25.0);
    let boys = [r.who(110), r.who(113)];
    for &i in &boys {
        assert_eq!(r.g.people[i].ai, uc_game::person::AI_BODYGUARD);
        assert_eq!(r.g.people[i].mv, 5, "point {} should be following Mako", r.g.people[i].way);
    }
    Some((r, boys))
}

/// Nitro Car with Cliffy and Zit signed on (as `adjust_enemy` points 77 and 78 leave them:
/// following Mako, bent as those points say), all three where the street is open (point 22),
/// the boys a little behind him.
fn nitro_car() -> Option<(Run, [usize; 2])> {
    let mut r = common::start("Bankbomb1")?;
    r.play(2.0, Controls::default());
    r.stand(r.point(22));
    let at = r.g.player().unwrap().fig.pos;
    let boys = [r.who(3), r.who(4)];
    for (k, &i) in boys.iter().enumerate() {
        let bent = r.level.mission.event_points.iter().find(|e| e.index == 77 + k).unwrap().data[4] as u8;
        let p = &mut r.g.people[i];
        (p.mv, p.follow, p.bent) = (5, 1, bent);
        p.fig.pos = at + vec3(-400.0, 0.0, -150.0 + 300.0 * k as f32);
    }
    Some((r, boys))
}

/// Both of Mako's boys keep up with him through the streets, running and sprinting, and
/// stand beside him once he stops rather than on top of each other: everyone makes for the
/// person they follow (`PCOM_set_person_move_mav_to_thing`) and people push each other
/// apart.
#[test]
fn both_boys_keep_up_side_by_side() {
    let Some((mut r, boys)) = rat_catch() else { return };
    // McLean was talked to first and has some catching up to do.
    r.play(12.0, Controls::default());
    for &i in &boys {
        assert!(r.off_player(i) < 400.0, "point {} is {:.0} off", r.g.people[i].way, r.off_player(i));
    }
    let way = r.open_way();
    for k in 0..20 {
        r.play(0.3, Controls { wish: way, run: true, sprint: k >= 10, ..Default::default() });
    }
    r.play(4.0, Controls::default());
    for &i in &boys {
        assert!(r.off_player(i) < 300.0, "point {} fell behind: {:.0} off", r.g.people[i].way, r.off_player(i));
    }
    let apart = r.g.people[boys[0]].fig.pos.distance(r.g.people[boys[1]].fig.pos);
    assert!(apart > 48.0, "the boys are piled up, {apart:.0} apart");
}

/// Mr X's three bodyguards walk with him once the contact has set him off, all of them near
/// and none on top of another.
#[test]
fn mr_xs_three_bodyguards_walk_with_him() {
    let Some(mut r) = common::start("Getaway1") else { return };
    r.play(2.0, Controls::default());
    let at = r.g.people[r.who(162)].fig.pos;
    r.stand(at + vec3(110.0, 0.0, 0.0));
    let me = r.player();
    r.g.people[me].fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(40.0, Controls::default());
    let x = r.who(131);
    let guards = [r.who(148), r.who(149), r.who(161)];
    let start = r.g.people[x].fig.pos;
    r.play(20.0, Controls::default());
    let pos = |r: &Run, i: usize| r.g.people[i].fig.pos;
    assert!(pos(&r, x).distance(start) > 1000.0, "Mr X hasn't gone anywhere");
    for &g in &guards {
        assert!(pos(&r, g).distance(pos(&r, x)) < 400.0, "point {} is {:.0} from him", r.g.people[g].way, pos(&r, g).distance(pos(&r, x)));
    }
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        let apart = pos(&r, guards[a]).distance(pos(&r, guards[b]));
        assert!(apart > 40.0, "two of them are piled up, {apart:.0} apart");
    }
}

/// Anyone else's bodyguards aren't out of harm's way for being far from the player: only
/// the player's own are, twenty blocks from them (pcom.cpp:11225).
#[test]
fn mr_xs_bodyguards_can_be_hurt_far_from_darci() {
    let Some(mut r) = common::start("Getaway1") else { return };
    r.play(2.0, Controls::default());
    let at = r.g.people[r.who(162)].fig.pos;
    r.stand(at + vec3(110.0, 0.0, 0.0));
    let me = r.player();
    r.g.people[me].fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(15.0, Controls::default());
    let g = r.who(148);
    let x = r.g.people[r.who(131)].fig.pos;
    r.stand(x + vec3(30.0 * 256.0, 0.0, 0.0));
    r.play(DT, Controls::default());
    assert!(r.off_player(g) > 20.0 * 256.0 * 1.25, "this wants Darci well away from them");
    assert!(!r.g.people[g].invulnerable, "Mr X's bodyguard can't be hurt so far from Darci");
}

/// Both boys climb into the car after Mako.
#[test]
fn both_boys_ride_along() {
    let Some((mut r, boys)) = nitro_car() else { return };
    let v = r.g.vehicles.iter().position(|v| v.way == 2).expect("the car bomb");
    let door = r.g.vehicles[v].door();
    r.stand(vec3(door.x, r.g.vehicles[v].pos.y, door.y));
    let at = r.g.player().unwrap().fig.pos;
    for (k, &i) in boys.iter().enumerate() {
        r.g.people[i].fig.pos = at + vec3(-300.0, 0.0, -100.0 + 200.0 * k as f32);
    }
    r.play(DT, Controls { action: true, ..Default::default() });
    let riding = |r: &Run| boys.iter().all(|&i| matches!(r.g.people[i].state, State::InCar { vehicle } if vehicle == v));
    assert!(r.until(10.0, riding), "not both in: {:?}", boys.map(|i| r.g.people[i].state.clone()));
}

/// Two cops come for Mako, who has his gun on one: the boys both go for the other
/// (`PCOM_find_bodyguard_victim` counts the one the client is at four times as far), though
/// the one he's at is nearer them; and between them they put both down.
#[test]
fn the_boys_go_for_the_cop_mako_isnt_at() {
    let Some((mut r, boys)) = nitro_car() else { return };
    let me = r.player();
    let at = r.g.people[me].fig.pos;
    // Mako faces west, the boys behind him to the west: one cop between them, the other
    // behind Mako.
    r.g.people[me].fig.angle = uc_world::character::angle_of(vec2(-1.0, 0.0));
    let cops: Vec<usize> = (0..r.g.people.len())
        .filter(|&i| r.g.people[i].kind == uc_world::character::person::COP && !r.g.people[i].removed)
        .take(2)
        .collect();
    let (near, far) = (cops[0], cops[1]);
    for (c, off) in [(near, vec3(-200.0, 0.0, 0.0)), (far, vec3(250.0, 0.0, 0.0))] {
        r.g.people[c].fig.pos = at + off;
        r.g.people[c].go_for();
    }
    // He draws on the one in front of him.
    r.take_out(Weapon::Pistol);
    assert_eq!(r.g.target, Some(near), "Mako should have his gun on the cop in front of him");
    for &b in &boys {
        assert_eq!(r.g.people[b].foe, Some(far), "point {} should go for the cop Mako isn't at", r.g.people[b].way);
    }
    let down = |r: &Run| cops.iter().all(|&c| r.g.people[c].is_down() || r.g.people[c].is_dead());
    assert!(r.until(15.0, down), "the cops are still up");
}

/// One of the boys is killed: the other carries on following Mako, and goes for the next
/// one to come for him.
#[test]
fn the_other_boy_carries_on_when_one_is_killed() {
    let Some((mut r, boys)) = rat_catch() else { return };
    r.stop_at_outcome = false;
    r.play(3.0, Controls::default());
    let from = r.g.people[boys[0]].fig.pos + Vec3::X;
    r.g.people[boys[0]].take_hit(from, 10_000, ko::FINISHERS, false, false, Weapon::None);
    r.play(2.0, Controls::default());
    assert!(r.g.people[boys[0]].is_dead());
    let other = boys[1];
    let way = r.open_way();
    r.play(3.0, Controls { wish: way, run: true, ..Default::default() });
    r.play(4.0, Controls::default());
    assert!(r.off_player(other) < 300.0, "Bish stopped following: {:.0} off", r.off_player(other));

    let at = r.g.player().unwrap().fig.pos;
    let cop = (0..r.g.people.len())
        .find(|&i| r.g.people[i].kind == uc_world::character::person::COP && !r.g.people[i].removed && !r.g.people[i].is_dead()
            && !matches!(r.g.people[i].state, State::InCar { .. }))
        .expect("a cop");
    r.g.people[cop].fig.pos = at + vec3(way.x, 0.0, way.y) * 200.0;
    r.g.people[cop].go_for();
    r.play(DT * 2.0, Controls::default());
    assert_eq!(r.g.people[other].foe, Some(cop), "Bish should go for the cop");
}
