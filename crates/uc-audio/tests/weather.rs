//! The world's own sounds against the install: the jungle map comes out as the jungle, and
//! every sound the weather can ask for ships. Skipped without an install.

use uc_audio::sounds;
use uc_audio::weather::{Place, Weather, World};
use uc_formats::iam::Map;

fn world(game: &std::path::Path, map: &str) -> World {
    let map = Map::parse(&std::fs::read(game.join("data").join(map)).unwrap()).unwrap();
    World::of(map.texture_set)
}

#[test]
fn el_cossa_island_is_the_jungle() {
    let Some(game) = uc_formats::default_install() else { return };
    assert_eq!(world(&game, "jung3.iam"), World::Jungle);
    assert_ne!(world(&game, "bball2.iam"), World::Jungle);
}

/// Every id any world's loops and cues use names a file that ships (allowing for the two
/// birdsongs and three gusts that shipped under other names).
#[test]
fn every_weather_sound_ships() {
    let Some(game) = uc_formats::default_install() else { return };
    let ships = |id: usize| {
        let name = sounds::name(id).unwrap();
        let dir = game.join("data").join("sfx").join("1622");
        std::iter::once(name.to_string()).chain(sounds::shipped_as(name)).any(|n| dir.join(n.replace('\\', "/")).exists())
    };
    for w in [World::Jungle, World::Snow, World::Estate, World::BusyCity, World::QuietCity] {
        for raining in [false, true] {
            let mut weather = Weather::new(w, raining);
            let mut ids: Vec<usize> = weather.looped().into_iter().chain(w.bed()).collect();
            // Ten minutes of game turns, inland and out past the jungle's foghorn line.
            for x in [40.0, 100.0] {
                for _ in 0..12000 {
                    ids.extend(weather.turn(glam::Vec3::new(x * 256.0, 512.0, 40.0 * 256.0), 512.0).iter().map(|c| {
                        if let Place::At(at) = c.place {
                            assert!(at.x.is_finite());
                        }
                        c.id
                    }));
                }
            }
            ids.sort_unstable();
            ids.dedup();
            for id in ids {
                assert!(ships(id), "{w:?}: sound {id} ({:?}) isn't in the install", sounds::name(id));
            }
        }
    }
}
