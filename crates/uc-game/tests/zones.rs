//! The mission's zone map (#232): no-go squares are walls nobody walks through, no-wander
//! squares are off the pavements the city's extras stroll on, and someone kept to a zone
//! only goes after a person standing in it. Skipped without an install.

use glam::{vec2, vec3, Vec3};
use uc_formats::iam::flags;
use uc_formats::ucm::zone_flags;
use uc_game::wand::Wander;
use uc_game::Controls;

mod common;
use common::Run;

#[test]
fn nobody_walks_into_a_no_go_square() {
    // Arms Breaker fences parts of the city off with no-go squares.
    let Some(mut r) = common::start("wstores1") else { return };
    r.play(1.0, Controls::default());
    let map = &r.level.map;
    // Squares on open ground with a no-go square on their +x side, both at the same height.
    let open = |x: usize, z: usize| !map.square(x, z).has(flags::HIDDEN) && !map.square(x, z).has(flags::NOGO);
    let candidates: Vec<(usize, usize)> = (2..126)
        .flat_map(|x| (1..127).map(move |z| (x, z)))
        .filter(|&(x, z)| {
            open(x, z) && open(x - 1, z) && map.square(x + 1, z).has(flags::NOGO) && !map.square(x + 1, z).has(flags::HIDDEN)
                && (map.ground_height(x as f32 * 256.0 - 128.0, z as f32 * 256.0 + 128.0)
                    - map.ground_height(x as f32 * 256.0 + 384.0, z as f32 * 256.0 + 128.0)).abs() < 8.0
        })
        .collect();
    assert!(!candidates.is_empty(), "Arms Breaker has no-go squares beside open ground");
    // Run at each from a square back until one where nothing else is in the way.
    let me = r.player();
    let mut tried = 0;
    for (mx, mz) in candidates {
        let edge = (mx + 1) as f32 * 256.0;
        r.stand(vec3(mx as f32 * 256.0 - 128.0, 0.0, mz as f32 * 256.0 + 128.0));
        let mut furthest: f32 = 0.0;
        for _ in 0..45 {
            r.play(common::DT, Controls { wish: vec2(1.0, 0.0), run: true, ..Default::default() });
            furthest = furthest.max(r.g.people[me].fig.pos.x);
        }
        if furthest < edge - 120.0 {
            continue; // a wall, a fence or a prop stopped her first
        }
        tried += 1;
        assert!(furthest <= edge - 87.0, "she ran into the no-go square at ({}, {mz}): got to {furthest}, its edge is at {edge}", mx + 1);
    }
    assert!(tried >= 3, "only {tried} no-go squares could be run at");
}

#[test]
fn nobody_wanders_onto_a_no_wander_square() {
    let Some(game) = uc_formats::default_install() else { return };
    let level = uc_world::Level::load(&game, "police1", &uc_world::LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let mut wand = Wander::new(&level.map, &level.game);
    let marked: Vec<usize> = (0..128 * 128)
        .filter(|&k| level.mission.zones[k] & zone_flags::NO_WANDER != 0 && !level.map.squares[k].has(flags::HIDDEN))
        .collect();
    let before = marked.iter().filter(|&&k| wand.is_wander((k / 128) as i32, (k % 128) as i32)).count();
    assert!(before > 0, "RTA's no-wander squares should include some pavement");
    wand.forbid(&level.map, &level.mission.zones);
    let after = marked.iter().filter(|&&k| wand.is_wander((k / 128) as i32, (k % 128) as i32)).count();
    assert_eq!(after, 0, "{after} no-wander squares are still wandered on");
    assert!(wand.any(), "there's still somewhere to wander");
}

impl Run {
    /// A spot on the ground 300 to 1500 from person `i`, in sight of them, that is (or isn't)
    /// in zone `zone`.
    fn spot(&self, i: usize, zone: u8, inside: bool) -> Vec3 {
        let at = self.g.people[i].fig.pos;
        let eye = vec3(0.0, 140.0, 0.0);
        (-24..=24)
            .flat_map(|dx| (-24..=24).map(move |dz| at + vec3(dx as f32 * 64.0, 0.0, dz as f32 * 64.0)))
            .filter(|p| (300.0..1500.0).contains(&(*p - at).length()))
            .map(|p| vec3(p.x, self.level.ground.surface(&self.level.map, p.x, p.z, at.y + 64.0), p.z))
            .filter(|p| (p.y - at.y).abs() < 16.0)
            .filter(|p| (self.level.mission.zones_at(p.x, p.z) & zone != 0) == inside)
            .filter(|p| !self.level.map.square((p.x / 256.0) as usize, (p.z / 256.0) as usize).has(flags::NOGO))
            .find(|p| self.level.ground.line_of_sight(at + eye, *p + eye))
            .unwrap_or_else(|| panic!("no spot {} zone {zone} near {at}", if inside { "in" } else { "out of" }))
    }

    /// Stands Darci at `p` with person `i` turned to look at her, and plays `secs`.
    fn face_off(&mut self, i: usize, p: Vec3, secs: f32) {
        self.stand(p);
        let at = self.g.people[i].fig.pos;
        self.g.people[i].fig.angle = uc_world::character::angle_of(vec2(p.x - at.x, p.z - at.z));
        self.play(secs, Controls::default());
    }
}

#[test]
fn someone_kept_to_a_zone_only_goes_after_people_in_it() {
    // Media Trouble's point 144 is a guard kept to zone 1, by the TV station.
    let Some(mut r) = common::start("e3") else { return };
    r.play(0.5, Controls::default());
    let i = r.who(144);
    assert_eq!(r.g.people[i].zone, 1);
    assert_eq!(r.g.people[i].ai, uc_game::person::AI_GANG, "a guard kept to a zone is made a gang member");
    let out = r.spot(i, 1, false);
    r.face_off(i, out, 3.0);
    assert!(!r.g.people[i].aggro(), "Darci isn't in their zone, so they should leave her be");
    let inside = r.spot(i, 1, true);
    r.face_off(i, inside, 3.0);
    assert!(r.g.people[i].aggro(), "Darci's in their zone now, so they should go for her");
}
