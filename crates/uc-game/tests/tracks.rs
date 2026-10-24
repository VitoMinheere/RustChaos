//! Footprints and tyre tracks (#107, fallen/Source/tracks.cpp): prints in the snow and off
//! grass and gravel but not off tarmac, the bike's muddy tyre and a sliding car's skids.
//! Skipped without an install.

use glam::{vec2, Vec3};
use uc_game::tracks::{FOOTPRINT_PAGE, TYREMARK_PAGE, TYRETRACK_PAGE};
use uc_game::{Controls, Drive, State};

mod common;
use common::{Run, DT};

impl Run {
    /// Gives every floor square within `r` squares of `at` texture `page`.
    fn paint(&mut self, at: Vec3, r: i32, page: u16) {
        let (cx, cz) = ((at.x / 256.0) as i32, (at.z / 256.0) as i32);
        for x in (cx - r).max(0)..(cx + r).min(127) {
            for z in (cz - r).max(0)..(cz + r).min(127) {
                let sq = &mut self.level.map.squares[x as usize * 128 + z as usize];
                sq.texture = (sq.texture & !0x3ff) | page;
            }
        }
    }

    /// The marks on the ground drawn with `page`.
    fn marks(&self, page: u32) -> usize {
        self.g.tracks.list.iter().filter(|t| t.page == page).count()
    }

    /// Walks the player round in a slow circle for `secs`.
    fn walk(&mut self, secs: f32) {
        for k in 0..(secs / DT) as usize {
            let a = k as f32 * DT * 0.5;
            self.play(DT, Controls { wish: vec2(a.cos(), a.sin()), ..Default::default() });
        }
    }
}

/// Target UC is a snow world: walking leaves dark prints, left and right.
#[test]
fn prints_in_the_snow() {
    let Some(level) = common::load("snow2") else { return };
    let mut r = Run::new(level);
    r.play(1.0, Controls::default());
    r.walk(4.0);
    let prints: Vec<_> = r.g.tracks.list.iter().filter(|t| t.page == FOOTPRINT_PAGE).collect();
    println!("{} prints", prints.len());
    assert!(prints.len() >= 4, "only {} prints", prints.len());
    assert!(prints.iter().all(|t| t.colour == [0, 0, 0] && t.faded == 0));
    assert!(prints.iter().any(|t| t.flip == Some(true)) && prints.iter().any(|t| t.flip == Some(false)));
    // Lying on the ground she walked on.
    let at = prints.last().unwrap().at;
    let floor = r.level.ground.surface(&r.level.map, at.x, at.z, at.y + 64.0);
    assert!((at.y - floor).abs() < 8.0, "a print at {} over ground at {floor}", at.y);
}

/// In the city, tarmac leaves nothing on her feet; grass does, and its prints fade out once
/// she's back on the tarmac.
#[test]
fn muddy_prints_off_grass_not_tarmac() {
    let Some(level) = common::load("police1") else { return };
    let mut r = Run::new(level);
    r.play(1.0, Controls::default());
    let at = r.g.people[r.player()].fig.pos;
    // Page 1 is a road texture, not grass or gravel.
    r.paint(at, 8, 1);
    r.walk(3.0);
    assert_eq!(r.marks(FOOTPRINT_PAGE), 0, "prints off tarmac");
    // `TRACKS_GroundAtXZ`'s grass.
    r.paint(at, 8, 70);
    r.walk(3.0);
    let muddy = r.marks(FOOTPRINT_PAGE);
    assert!(muddy >= 4, "only {muddy} prints off grass");
    assert!(r.g.tracks.list.iter().all(|t| t.colour == [0x48, 0x20, 0x00]));
    // Back on tarmac: a few more, each fainter, then none.
    r.paint(at, 8, 1);
    r.walk(20.0);
    let all = r.marks(FOOTPRINT_PAGE);
    assert!(all > muddy && all <= muddy + 32, "{muddy} then {all}");
    let last = r.g.tracks.list.back().unwrap();
    assert!(last.faded > 200, "the last print is only {} faded", last.faded);
}

/// The bike's back tyre leaves a track off grass (bike.cpp:850), and none on tarmac.
#[test]
fn the_bike_leaves_tyre_tracks() {
    let Some(level) = common::load("train1") else { return };
    let mut r = Run::new(level);
    r.until(60.0, |r| r.g.time > 1.0 && !r.g.in_conversation());
    let k = r.g.bikes.iter().position(|(b, _)| b.way == 66).expect("no bike");
    let p = r.player();
    let at = r.g.bikes[k].0.pos();
    r.paint(at, 12, 1);
    r.stand(at + Vec3::new(100.0, 0.0, 0.0));
    r.g.people[p].state = State::Move;
    r.play(DT, Controls { action: true, ..Default::default() });
    r.play(3.0, Controls::default());
    assert!(matches!(r.g.people[p].state, State::Bike { phase: 1, .. }), "not riding: {:?}", r.g.people[p].state);
    let go = Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() };
    r.play(1.0, go);
    assert_eq!(r.marks(TYRETRACK_PAGE), 0, "tracks on tarmac");
    r.paint(r.g.bikes[k].0.pos(), 12, 70);
    r.play(1.5, go);
    let tracks = r.marks(TYRETRACK_PAGE);
    println!("{tracks} tyre tracks");
    assert!(tracks >= 4, "only {tracks} tyre tracks");
}

impl Run {
    /// Puts the player at the wheel of the first car.
    fn drive_off(&mut self) {
        self.play(2.0, Controls::default());
        let p = self.player();
        self.g.vehicles[0].driver = Some(p);
        self.g.people[p].state = State::InCar { vehicle: 0 };
    }
}

/// A car sliding round on the handbrake leaves skid marks; driving straight it doesn't.
#[test]
fn a_sliding_car_skids() {
    let Some(level) = common::load("testdrive1a") else { return };
    let mut r = Run::new(level);
    r.drive_off();
    let go = Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() };
    r.play(2.0, go);
    let off = r.marks(TYREMARK_PAGE);
    r.play(1.0, go);
    assert_eq!(r.marks(TYREMARK_PAGE), off, "skids driving straight");
    r.play(1.0, Controls { drive: Drive { throttle: 0.5, steer: 1.0, handbrake: true, ..Default::default() }, ..Default::default() });
    let skids = r.marks(TYREMARK_PAGE) - off;
    println!("{skids} skid marks");
    assert!(skids >= 8, "only {skids} skid marks");
    // On the road under the car, not up on its roof.
    let road = r.g.vehicles[0].pos.y;
    for t in r.g.tracks.list.iter().filter(|t| t.page == TYREMARK_PAGE) {
        let floor = r.level.ground.road(&r.level.map, t.at.x, t.at.z, t.at.y);
        assert!((t.at.y - floor - 5.0).abs() < 1.0 && t.at.y < road + 40.0, "a skid at {} over road at {floor}", t.at.y);
    }
}

/// Pulling away hard spins the wheels (`Smokin`, vehicle.cpp:3625): smoke and skid marks
/// till it's going, then none cruising on.
#[test]
fn pulling_away_hard_spins_the_wheels() {
    let Some(level) = common::load("testdrive1a") else { return };
    let mut r = Run::new(level);
    r.drive_off();
    assert!(!r.g.vehicles[0].smokin && r.marks(TYREMARK_PAGE) == 0);
    let go = Controls { drive: Drive { throttle: 1.0, ..Default::default() }, ..Default::default() };
    let smoke = r.g.pyro.puffs.len();
    r.play(DT, go);
    assert!(r.g.vehicles[0].smokin, "no wheelspin pulling away");
    let mut spun = 0.0;
    while r.g.vehicles[0].smokin {
        r.play(DT, go);
        spun += DT;
        assert!(spun < 2.0, "still spinning at {}", r.g.vehicles[0].speed);
    }
    let off = r.marks(TYREMARK_PAGE);
    println!("spun {spun:.2}s: {off} marks, {} puffs", r.g.pyro.puffs.len() - smoke);
    assert!(off >= 8, "only {off} skid marks pulling away");
    assert!(r.g.pyro.puffs.len() >= smoke + 8, "no tyre smoke");
    assert!(r.g.vehicles[0].speed > 400.0);
    r.play(2.0, go);
    assert_eq!(r.marks(TYREMARK_PAGE), off, "skids cruising");
}
