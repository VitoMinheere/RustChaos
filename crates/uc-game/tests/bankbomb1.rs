//! Nitro Car (`Bankbomb1`), the first mission played as somebody other than Darci: Mako, a
//! gang thug, drives two car bombs across town for the Boss. Checks the script end to end,
//! with the driving done by putting the car where it has to be and the fighting by knocking
//! the cops out, as the other mission tests do. Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome, State};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
    /// Keep Mako on his feet: how well the player fights isn't what these check, and the
    /// city is full of cops once the first bomb goes off.
    patch_him_up: bool,
}

fn start() -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "Bankbomb1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new(), patch_him_up: false })
}

impl Run {
    /// Where event point `i` is.
    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    /// The vehicle the event point `way` created, if it is still about.
    fn car(&self, way: usize) -> Option<usize> {
        self.g.vehicles.iter().position(|v| v.way == way && !v.wrecked())
    }

    /// Plays `secs` seconds with `c` held, printing each new message.
    fn play(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT) as usize {
            self.g.skip_cutscene();
            if self.patch_him_up {
                let i = self.g.player.unwrap();
                self.g.people[i].health = self.g.people[i].max_health;
            }
            self.g.update(DT, &c, &self.level);
            for m in &self.g.messages {
                if !self.seen.contains(&m.text) {
                    println!("[{:6.1}s] {}", self.g.time, m.text);
                    self.seen.push(m.text.clone());
                }
            }
            if self.g.outcome.is_some() {
                return;
            }
        }
    }

    /// Plays `secs` seconds and returns the fastest person `i` ever moved, in world units a
    /// second: what they manage when nothing is in the way.
    fn top_speed(&mut self, i: usize, secs: f32, c: Controls) -> f32 {
        let mut best: f32 = 0.0;
        for _ in 0..(secs / DT) as usize {
            let before = self.g.people[i].fig.pos;
            self.play(DT, c);
            best = best.max((self.g.people[i].fig.pos - before).length() / DT);
        }
        best
    }

    /// Presses action for one frame (holding it would get into a car and straight back out).
    fn tap_action(&mut self) {
        self.play(DT, Controls { action: true, ..Default::default() });
    }

    /// Puts Mako at `p`, on whatever is there.
    fn stand(&mut self, p: Vec3) {
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }

    /// Gets into the car that event point `way` created, by standing at its door and pressing
    /// action. Returns false if it wouldn't open (a locked car).
    fn get_in(&mut self, way: usize) -> bool {
        let v = self.car(way).expect("the car is there");
        let door = self.g.vehicles[v].door();
        self.stand(vec3(door.x, self.g.vehicles[v].pos.y, door.y));
        self.tap_action();
        self.g.player_vehicle().is_some()
    }

    /// Puts the car Mako is driving at `p` (the driving itself is `driving.rs`'s job).
    fn park(&mut self, p: Vec3) {
        let v = self.g.vehicles.iter().position(|v| v.driver == self.g.player).expect("driving");
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        self.g.vehicles[v].pos = vec3(p.x, y, p.z);
        self.g.vehicles[v].speed = 0.0;
    }

    /// Knocks out every cop still standing.
    fn flatten_the_cops(&mut self) -> usize {
        let from = self.g.player().unwrap().fig.pos;
        let mut n = 0;
        for i in 0..self.g.people.len() {
            let p = &mut self.g.people[i];
            if p.kind == ptype::COP && !p.removed && !p.is_dead() {
                p.take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
                n += 1;
            }
        }
        n
    }
}

/// The whole mission: both bombs delivered, the bomb squad and the last cop seen off.
#[test]
fn nitro_car_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    r.patch_him_up = true;

    // Mako is a thug, not Darci: the only mission so far where `create_player` says so.
    assert_eq!(r.g.player().unwrap().kind, ptype::THUG_RASTA);
    assert_eq!(r.g.player_name(), "Mako", "the mission's own dialogue calls him that throughout");
    assert!(!r.g.player().unwrap().can_arrest(), "a gang thug doesn't arrest people");

    // The mission hands him a loaded pistol: its create_player point leaves Data[1] clear,
    // which is what `EWAY_DO_CREATE_PLAYER` reads as `PCOM_HAS_GUN`. It is carried, not
    // drawn, so it shows in the weapons he has on him rather than in his hands.
    let me = r.g.player().unwrap();
    assert_eq!(me.weapon, uc_game::Weapon::None, "the pistol starts stowed, as the original leaves it");
    assert!(me.has(uc_game::Weapon::Pistol), "Mako should start carrying a pistol");
    assert_eq!(me.stowed(), vec![(uc_game::Weapon::Pistol, Some(15))], "a pistol holds fifteen rounds");

    // Cliffy and Zit wait at the hangout, guarding Mako (ai 8, standing still).
    for way in [3, 4] {
        let i = r.g.people.iter().position(|p| p.way == way).expect("one of the boys");
        assert_eq!(r.g.people[i].ai, uc_game::person::AI_BODYGUARD, "point {way} should be a bodyguard");
        assert_eq!(r.g.people[i].other, 1, "point {way} should be guarding Mako");
    }

    // The first car bomb (point 2) is waiting unlocked at the pick-up point.
    r.play(2.0, Controls::default());
    assert!(r.get_in(2), "the first car bomb should be unlocked");

    // Over to the bank courtyard: the charge is primed and the car locks behind you.
    r.park(r.point(5));
    r.play(6.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("BANK")), "no bomb briefing: {:?}", r.seen);
    assert!(r.seen.iter().any(|m| m.contains("explosives primed")), "the charge never went in: {:?}", r.seen);
    let car = r.car(2).expect("the car is still there");
    assert!(r.g.vehicles[car].locked(), "the primed car bomb should lock");

    // Out and well clear before the fifteen seconds are up.
    r.tap_action();
    assert!(r.g.player_vehicle().is_none(), "Mako should get out");
    r.stand(r.point(22));
    r.play(20.0, Controls::default());
    assert_eq!(r.g.outcome, None, "the first bomb shouldn't end the mission");
    assert!(r.car(2).is_none(), "the first car bomb should have gone up");
    assert!(r.g.pyro.fires().count() > 0, "the blast should leave something burning");
    assert!(r.seen.iter().any(|m| m.contains("dent")), "no bang: {:?}", r.seen);

    // The second car arrives, locked, with a cop bomb squad guarding the drop.
    r.play(8.0, Controls::default());
    let second = r.car(37).expect("the second car bomb");
    assert!(r.g.vehicles[second].locked(), "the second car should start locked");
    r.stand(r.point(33));
    assert!(!r.get_in(37), "the second car shouldn't open until the squad is down");

    // See off the bomb squad: that unlocks it.
    r.stand(r.point(50));
    for _ in 0..4 {
        r.flatten_the_cops();
        r.play(2.0, Controls::default());
    }
    assert!(r.seen.iter().any(|m| m.contains("TOAST")), "the squad never went down: {:?}", r.seen);
    let second = r.car(37).expect("the second car bomb");
    assert!(!r.g.vehicles[second].locked(), "the second car should unlock");

    // Drive it to the corner of the department store and get clear again.
    assert!(r.get_in(37), "the second car bomb should open now");
    r.park(r.point(56));
    r.play(6.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("charges set")), "the second charge never went in: {:?}", r.seen);
    r.tap_action();
    r.stand(r.point(101));
    r.play(20.0, Controls::default());
    assert!(r.car(37).is_none(), "the second car bomb should have gone up");
    assert_eq!(r.g.outcome, None, "the second bomb shouldn't end the mission either");

    // A cop jumps Cliffy and Zit back at the hangout. Brutalize that cop.
    r.play(6.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("brutalize")), "no last cop: {:?}", r.seen);
    r.stand(r.point(124));
    for _ in 0..6 {
        r.flatten_the_cops();
        r.play(3.0, Controls::default());
        if r.g.outcome.is_some() {
            break;
        }
    }
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// Wrecking a car bomb on the way fails the mission (points 149 to 152).
#[test]
fn wrecking_the_car_bomb_fails_the_mission() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let v = r.car(2).expect("the first car bomb");
    r.g.vehicles[v].health = 0;
    r.play(1.0, Controls::default());
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "wrecking the bomb should lose: {:?}", r.g.outcome);
}

/// Killing one of the boys fails it too (points 79 to 82), but only while Mako is near
/// enough to be blamed: a bodyguard twenty blocks away can't be hurt at all.
#[test]
fn the_boys_are_safe_until_mako_is_near_them() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let zit = r.g.people.iter().position(|p| p.way == 3).expect("Zit");
    let far = r.point(56);
    r.stand(far);
    r.play(0.5, Controls::default());
    assert!(r.g.people[zit].invulnerable, "Zit should be out of reach with Mako across town");
    let from = r.g.people[zit].fig.pos;
    r.g.people[zit].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(1.0, Controls::default());
    assert_eq!(r.g.outcome, None, "a scuffle nobody saw shouldn't fail the mission");

    // Standing next to him, he can be hurt, and losing him loses the mission.
    r.stand(from + vec3(100.0, 0.0, 0.0));
    r.play(0.5, Controls::default());
    assert!(!r.g.people[zit].invulnerable, "Zit should be vulnerable with Mako beside him");
    r.g.people[zit].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(2.0, Controls::default());
    assert!(matches!(r.g.outcome, Some(Outcome::Lost(_))), "losing one of the boys should lose: {:?}", r.g.outcome);
}

/// Talking to Cliffy and Zit at the hangout: before the first bomb they just banter, and
/// afterwards they sign up and follow Mako (`PCOM_MOVE_FOLLOW`).
#[test]
fn the_boys_can_be_recruited_after_the_first_bomb() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let zit = r.g.people.iter().position(|p| p.way == 3).expect("Zit");
    assert_eq!(r.g.people[zit].mv, 1, "Zit stands still to begin with");
    // Walk up to him and press action.
    let at = r.g.people[zit].fig.pos;
    r.stand(at + vec3(140.0, 0.0, 0.0));
    r.tap_action();
    r.play(30.0, Controls::default());
    assert!(r.seen.iter().any(|m| m.contains("Cliffy's my man")), "no first conversation: {:?}", r.seen);
    // The state of someone caught mid-fall or mid-anything doesn't matter here: he hasn't
    // been asked along yet, so he's still standing where he was.
    assert_eq!(r.g.people[zit].mv, 1, "Zit shouldn't tag along before there's a job");
    assert!(!matches!(r.g.people[zit].state, State::InCar { .. }));
}

/// Once they've been asked along the boys run after Mako rather than ambling after him
/// (`PCOM_set_person_ai_follow` sends a follower off at `PERSON_SPEED_RUN`), and break into a
/// sprint when he does. Both are timed over the same clear stretch of the hangout, since
/// the yard has walls to fetch up against either way.
#[test]
fn the_boys_keep_up_with_mako() {
    use uc_game::person::{Pace, WALK_SPEED};
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let zit = r.g.people.iter().position(|p| p.way == 3).expect("Zit");
    // What `adjust_enemy` point 77 does to him once he agrees to come along.
    r.g.people[zit].mv = 5;
    r.g.people[zit].follow = 1;
    let home = r.g.people[zit].fig.pos;

    // Mako well across the yard, so Zit has somewhere to be going, and Zit back where he
    // started each time: how fast does he set off after him?
    let chase = |r: &mut Run, c: Controls, want: Pace| {
        r.g.people[zit].fig.pos = home;
        r.stand(home + vec3(2400.0, 0.0, 0.0));
        // Two frames: the player's sprint is worked out at the end of a frame, so a follower
        // picks it up on the next one.
        r.play(DT * 2.0, c);
        assert_eq!(r.g.people[zit].pace, want, "Zit should be at a {want:?}");
        r.top_speed(zit, 1.5, c)
    };
    let walking = Controls::default();
    let sprinting = Controls { wish: glam::vec2(1.0, 0.0), run: true, sprint: true, ..Default::default() };
    let run = chase(&mut r, walking, Pace::Run);
    let dash = chase(&mut r, sprinting, Pace::Sprint);

    assert!(run > WALK_SPEED * 1.5, "Zit's best was {run:.0} a second, which is no better than a walk");
    assert!(dash > run * 1.2, "sprinting after Mako ({dash:.0}) should beat running after him ({run:.0})");
}

/// Having seen off a cop, the boys go back to following Mako. Someone in a fight is given
/// nowhere to walk to, so a bodyguard who stayed keyed up after their target went down would
/// stand rooted to the spot for the rest of the mission.
#[test]
fn the_boys_get_going_again_once_the_cop_is_down() {
    use uc_game::person::Pace;
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let zit = r.g.people.iter().position(|p| p.way == 3).expect("Zit");
    r.g.people[zit].mv = 5;
    r.g.people[zit].follow = 1;

    // A cop walks up to Mako, who is standing with his escort.
    let home = r.g.people[zit].fig.pos;
    r.stand(home + vec3(260.0, 0.0, 0.0));
    let cop = r.g.people.iter().position(|p| p.kind == ptype::COP && !p.removed).expect("a cop");
    r.g.people[cop].fig.pos = r.g.player().unwrap().fig.pos + vec3(200.0, 0.0, 0.0);
    r.play(1.5, Controls::default());
    assert_eq!(r.g.people[zit].foe, Some(cop), "Zit should go for the cop");

    // Zit puts him down. Mako then walks off, and Zit must come after him.
    let from = r.g.people[zit].fig.pos;
    r.g.people[cop].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(1.0, Controls::default());
    assert_eq!(r.g.people[zit].foe, None, "Zit should let a downed cop be");
    assert!(!r.g.people[zit].aggro(), "Zit should calm down once the cop is out of it");

    r.stand(home + vec3(2400.0, 0.0, 0.0));
    r.play(DT * 2.0, Controls::default());
    assert_eq!(r.g.people[zit].pace, Pace::Run, "Zit should be running after Mako again");
    let moved = r.top_speed(zit, 1.5, Controls::default());
    assert!(moved > uc_game::person::WALK_SPEED, "Zit is stuck: his best was {moved:.0} a second");
}

/// A bodyguard won't set off after someone they can't see. Nothing here finds a route round
/// a corner, so one that did would walk into the wall between them and stand against it for
/// the rest of the mission, still aiming at a cop it could never reach. The original gives up
/// on a target it can't work out how to get to as well (`PCOM_AI_SUBSTATE_CANTFIND`).
#[test]
fn the_boys_dont_set_off_after_a_cop_behind_a_wall() {
    use uc_game::person::Pace;
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let zit = r.g.people.iter().position(|p| p.way == 3).expect("Zit");
    // As `adjust_enemy` point 77 leaves him once he's been asked along.
    r.g.people[zit].mv = 5;
    r.g.people[zit].follow = 1;
    r.g.people[zit].bent = 0;
    let home = r.g.people[zit].fig.pos;

    // The yard's east wall is about 470 on from Zit: put Mako and a cop beyond it.
    let cop = r.g.people.iter().position(|p| p.kind == ptype::COP && !p.removed).expect("a cop");
    r.g.people[cop].fig.pos = home + vec3(1400.0, 0.0, 0.0);
    r.stand(home + vec3(900.0, 0.0, 0.0));
    r.play(3.0, Controls::default());
    let eye = vec3(0.0, 140.0, 0.0);
    let clear = r.level.ground.line_of_sight(home + eye, r.g.people[cop].fig.pos + eye);
    assert!(!clear, "this test wants a wall between Zit and the cop");
    assert_eq!(r.g.people[zit].foe, None, "Zit shouldn't lock on to a cop he can't see");

    // He's still Mako's man, and still going after him rather than rooted to the spot.
    assert_eq!(r.g.people[zit].pace, Pace::Run);
    r.g.people[zit].fig.pos = home;
    r.stand(home + vec3(2400.0, 0.0, 0.0));
    r.play(DT * 2.0, Controls::default());
    let moved = r.top_speed(zit, 1.5, Controls::default());
    assert!(moved > uc_game::person::WALK_SPEED, "Zit is stuck: his best was {moved:.0} a second");
}

/// The boys guard Mako: a blow landed on him must not turn them on him. Being hit makes
/// anyone go aggro and the player counts as their own foe, which once had them shooting the
/// man they were escorting.
#[test]
fn the_boys_never_turn_on_mako() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    let (zit, cliffy) = (
        r.g.people.iter().position(|p| p.way == 3).expect("Zit"),
        r.g.people.iter().position(|p| p.way == 4).expect("Cliffy"),
    );
    // Stand between them and take a beating.
    let between = (r.g.people[zit].fig.pos + r.g.people[cliffy].fig.pos) / 2.0;
    r.stand(between);
    for _ in 0..5 {
        let me = r.g.player.unwrap();
        let from = r.g.people[me].fig.pos + vec3(100.0, 0.0, 0.0);
        r.g.people[me].take_hit(from, 10, 0, true, false, uc_game::Weapon::None);
        r.play(1.0, Controls::default());
        for (who, i) in [("Zit", zit), ("Cliffy", cliffy)] {
            assert_ne!(r.g.people[i].foe, r.g.player, "{who} turned on the man he's guarding");
        }
    }
    assert_eq!(r.g.outcome, None, "Mako shouldn't be shot by his own escort");

    // A cop going for Mako is fair game, though.
    let me = r.g.player.unwrap();
    let at = r.g.people[me].fig.pos + vec3(300.0, 0.0, 0.0);
    let cop = r.g.people.iter().position(|p| p.kind == ptype::COP && !p.removed);
    let cop = cop.expect("Nitro Car has cops about");
    r.g.people[cop].fig.pos = at;
    r.play(2.0, Controls::default());
    assert_eq!(r.g.people[zit].foe, Some(cop), "Zit should see off the cop on Mako");
}

