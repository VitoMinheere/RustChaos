//! Rat Catch (`Gangorder2`): Mako again, this time collecting McIntyre's son for the Boss.
//! Checks the script end to end, with the walking done by putting people where they have to
//! be and the fighting by knocking them down, as the other mission tests do. Skipped without
//! an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game, Outcome};
use uc_world::character::person as ptype;
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

fn start() -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "Gangorder2", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {
    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
    }

    fn who(&self, way: usize) -> usize {
        self.g.people.iter().position(|p| p.way == way).unwrap_or_else(|| panic!("nobody from point {way}"))
    }

    fn play(&mut self, secs: f32, c: Controls) {
        for _ in 0..(secs / DT).max(1.0) as usize {
            self.g.skip_cutscene();
            let me = self.g.player.unwrap();
            self.g.people[me].health = self.g.people[me].max_health;
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

    fn stand(&mut self, p: Vec3) {
        let y = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 64.0);
        let i = self.g.player.unwrap();
        self.g.people[i].fig.pos = vec3(p.x, y, p.z);
    }

    fn said(&self, text: &str) -> bool {
        self.seen.iter().any(|m| m.contains(text))
    }

    /// Stands Mako `dist` away from person `i` and has him look at them: `can_a_see_b` wants
    /// the looker facing roughly the right way, not just standing near.
    fn look_at(&mut self, i: usize, dist: f32) {
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(dist, 0.0, 0.0));
        let me = self.g.player.unwrap();
        let from = self.g.people[me].fig.pos;
        self.g.people[me].fig.angle = uc_world::character::angle_of(glam::vec2(at.x - from.x, at.z - from.z));
    }

    /// Walks up to the person event point `way` created, talks to them, and lets them finish.
    fn talk_to(&mut self, way: usize, secs: f32) {
        let i = self.who(way);
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(130.0, 0.0, 0.0));
        self.play(DT, Controls { action: true, ..Default::default() });
        self.play(secs, Controls::default());
    }
}

/// The Boss's errand, start to finish: round up the boys, get the job from Spiv, run the son
/// down, and walk him to the drop.
#[test]
fn rat_catch_script_runs_to_a_win() {
    let Some(mut r) = start() else { return };
    assert_eq!(r.g.player().unwrap().kind, ptype::THUG_RASTA, "Rat Catch is Mako's too");
    assert_eq!(r.g.player_name(), "Mako");
    r.play(2.0, Controls::default());

    // The Wildcat sends him to Spiv, by way of McLean and Bish.
    r.talk_to(10, 45.0);
    assert!(r.said("pick up McLean and Bish"), "no briefing: {:?}", r.seen);
    r.talk_to(110, 25.0);
    r.talk_to(113, 25.0);
    let (mclean, bish) = (r.who(110), r.who(113));
    assert_eq!(r.g.people[mclean].mv, 5, "McLean should fall in behind Mako");
    assert_eq!(r.g.people[bish].mv, 5, "Bish should fall in behind Mako");

    // Spiv names the job.
    r.talk_to(120, 70.0);
    assert!(r.said("McIntyre"), "Spiv never named the job: {:?}", r.seen);
    let son = r.who(122);

    // Walking up to the son starts the argument, and he bolts.
    r.look_at(son, 200.0);
    r.play(50.0, Controls::default());
    assert!(r.said("have to kill me first"), "the son never spoke up: {:?}", r.seen);
    assert_eq!(r.g.people[son].ai, 16, "he should take to his heels (PCOM_AI_FLEE_PLAYER)");

    // Run him down: half his health and he gives it up.
    let son = r.who(122);
    let from = r.g.player().unwrap().fig.pos;
    let half = r.g.people[son].max_health / 2 + 1;
    r.g.people[son].take_hit(from, half, 0, false, false, uc_game::Weapon::None);
    r.play(6.0, Controls::default());
    assert!(r.said("I SURRENDER"), "he never gave up: {:?}", r.seen);

    // Stand over him and he comes quietly. Where he ran to depends on who was in his way,
    // the city's crowd included, and the cops about can shoot Mako there, which breaks the
    // conversation off: keep them from it.
    let me = r.g.player.unwrap();
    r.g.people[me].protected = true;
    r.look_at(son, 160.0);
    r.play(25.0, Controls::default());
    r.g.people[me].protected = false;
    assert!(r.said("I'll come with you"), "he never agreed to come: {:?}", r.seen);
    assert_eq!(r.g.people[son].mv, 5, "he should follow Mako now");

    // Walk him to the drop.
    let drop = r.point(69);
    let y = r.level.ground.surface(&r.level.map, drop.x, drop.z, drop.y + 64.0);
    r.g.people[son].fig.pos = vec3(drop.x, y, drop.z);
    r.stand(drop);
    r.play(5.0, Controls::default());
    println!("outcome {:?} at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("You got him"), "the drop never registered: {:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}

/// Killing the son doesn't end it: the Boss takes it out of Mako instead, and the way back
/// into his good books is five more dead cops. The original keeps that tally in counter 7,
/// bumped wherever a `PERSON_COP` dies.
#[test]
fn killing_the_son_turns_into_the_boss_punishment() {
    let Some(mut r) = start() else { return };
    r.play(2.0, Controls::default());
    r.talk_to(10, 45.0);
    r.talk_to(110, 25.0);
    r.talk_to(113, 25.0);
    r.talk_to(120, 70.0);
    let son = r.who(122);
    let from = r.g.people[son].fig.pos;
    r.g.people[son].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
    r.play(4.0, Controls::default());
    assert!(r.said("you KILLED the guy"), "nobody noticed: {:?}", r.seen);
    assert_eq!(r.g.outcome, None, "the Boss punishes it rather than calling the job off");

    // Back to the Wildcat, then to Spiv for the punishment.
    r.talk_to(10, 30.0);
    assert!(r.said("sack of shit"), "no dressing down: {:?}", r.seen);
    r.talk_to(120, 40.0);
    assert!(r.said("take out at least 5 more"), "no punishment set: {:?}", r.seen);

    // Five dead cops puts it right.
    let mut killed = 0;
    while killed < 5 {
        // (On foot: the ones driving patrol cars are out of reach in them.)
        let cop = r.g.people.iter().position(|p| {
            p.kind == ptype::COP && !p.removed && !p.is_dead() && !matches!(p.state, uc_game::State::InCar { .. })
        });
        let Some(cop) = cop else { break };
        let from = r.g.people[cop].fig.pos;
        r.g.people[cop].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
        killed += 1;
        r.play(1.0, Controls::default());
    }
    assert_eq!(killed, 5, "the mission should have five cops about to oblige");
    r.play(6.0, Controls::default());
    println!("outcome {:?} after {killed} cops at {:.1}s", r.g.outcome, r.g.time);
    assert!(r.said("The boss kinda likes you again"), "the Boss never came round: {:?}", r.seen);
    assert_eq!(r.g.outcome, Some(Outcome::Won));
}
