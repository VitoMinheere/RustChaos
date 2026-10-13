//! The Fallen (`MIB`): five clues, each dropped by somebody who has to be found first and then
//! taken down. Every event point of it runs, and the clues can be picked up now that evidence
//! is carryable, but it isn't playable end to end yet: the bot here gets three of the five.
//!
//! The two it misses are further down the chain. One is gated on a fly-by reaching a camera
//! waypoint (point 309) that nothing in reach sets off, and the other on a conversation
//! (point 216) further along still. Finishing it wants a bot that walks the whole trail, which
//! is a bigger job than this; what this pins down is that the clue machinery itself works.
//!
//! Skipped without an install.

use glam::{vec3, Vec3};
use uc_game::person::ko;
use uc_game::{Controls, Game};
use uc_world::{Level, LoadOptions};

const DT: f32 = 1.0 / 30.0;

/// The points that create the five people carrying a clue.
const CLUE_HOLDERS: [usize; 5] = [39, 67, 316, 231, 40];

struct Run {
    level: Level,
    g: Game,
    seen: Vec<String>,
}

fn start() -> Option<Run> {
    let game = uc_formats::default_install()?;
    let level = Level::load(&game, "MIB", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    Some(Run { level, g, seen: Vec::new() })
}

impl Run {
    fn point(&self, i: usize) -> Vec3 {
        let e = self.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        vec3(e.x as f32, e.y as f32, e.z as f32)
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

    /// Stands next to someone and looks at them, which is what both talking to them and
    /// `can_a_see_b` want.
    fn walk_up_to(&mut self, i: usize) {
        let at = self.g.people[i].fig.pos;
        self.stand(at + vec3(130.0, 0.0, 0.0));
        let me = self.g.player.unwrap();
        let from = self.g.people[me].fig.pos;
        self.g.people[me].fig.angle =
            uc_world::character::angle_of(glam::vec2(at.x - from.x, at.z - from.z));
    }

    /// How many of the five clues are in hand.
    fn clues(&self) -> usize {
        // Each clue point sets `specific_item_held` once its item has been picked up; the
        // points watching for that are 367, 227, 366, 365 and 293.
        [367usize, 227, 366, 365, 293].iter().filter(|&&i| self.g.point_active(i)).count()
    }
}

/// The clues that can be reached are found, taken down and picked up.
#[test]
fn the_clues_within_reach_are_collected() {
    let Some(mut r) = start() else { return };
    r.play(3.0, Controls::default());

    // Harney starts the trail, and the woman looking for Pete is found by walking into her
    // corner of the map.
    let harney = r.g.people.iter().position(|p| p.way == 13);
    if let Some(h) = harney {
        r.walk_up_to(h);
        r.play(DT, Controls { action: true, ..Default::default() });
        r.play(40.0, Controls::default());
    }

    // Then just keep at it: put down whoever is carrying a clue, pick up what they drop, and
    // talk to anyone new who turns up.
    let mut rounds = 0;
    while r.clues() < 5 && r.g.outcome.is_none() && rounds < 40 {
        rounds += 1;
        // Anyone holding a clue goes down.
        for way in CLUE_HOLDERS {
            if let Some(i) = r.g.people.iter().position(|p| p.way == way && !p.is_dead() && !p.removed) {
                r.walk_up_to(i);
                let from = r.g.people[i].fig.pos;
                r.g.people[i].take_hit(from, 10_000, ko::FINISHERS, false, false, uc_game::Weapon::None);
            }
        }
        r.play(2.0, Controls::default());
        // Walk over everything lying about, not just the first thing: most of it is scenery
        // she can't pick up, and the clue is somewhere in among it.
        let lying: Vec<Vec3> = r.g.items.iter().map(|it| it.pos).collect();
        for at in lying {
            r.stand(at);
            r.play(0.4, Controls::default());
        }
        // Walk to the places the trail is gated on: the conversations that start when she
        // steps into a corner of the map, and the points that follow them.
        for gate in [65usize, 313, 309, 325, 41, 227, 315] {
            let at = r.point(gate);
            r.stand(at);
            r.play(1.0, Controls::default());
        }
        // And talk to whoever will talk.
        let talkable: Vec<usize> = (0..r.g.people.len())
            .filter(|&i| {
                let p = &r.g.people[i];
                !p.player && p.way != 0 && !p.removed && !p.is_dead() && !p.hostile()
            })
            .collect();
        for i in talkable.into_iter().take(6) {
            r.walk_up_to(i);
            r.play(DT, Controls { action: true, ..Default::default() });
            r.play(3.0, Controls::default());
        }
    }

    println!("clues {} of 5 after {rounds} rounds, outcome {:?}", r.clues(), r.g.outcome);
    assert!(r.said("5 CLUES"), "the chief never set the job: {:?}", r.seen);
    // Three is what the trail gives up without walking all of it. If this ever reads five the
    // mission is finishable and the note at the top of this file is out of date.
    assert!(r.clues() >= 3, "only {} clues, the trail has got worse", r.clues());
    assert_eq!(r.g.outcome, None, "it can't be won yet, so nothing should be claiming it is");
}
