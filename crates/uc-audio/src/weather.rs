//! The world's own sounds out of doors (fallen/Source/Sound.cpp): `SND_BeginAmbient`'s world
//! type from the map's texture set, the loops `process_weather` keeps going, and the odd
//! effects `new_outdoors_effects` drops in round the player (sirens, birds, dogs, wind,
//! thunder, a foghorn).
//!
//! [`Weather::turn`] runs once a game turn (20 a second) while the player is out of doors
//! and says what to play; the caller plays it. Ids are [`crate::sounds`] ids.

use glam::Vec3;

/// `S_WIND_START`: seven gusts.
const WIND: usize = 1;
/// `S_THUNDER_START`: four claps.
const THUNDER: usize = 8;
/// `S_RAIN_START`.
pub const RAIN: usize = 12;
/// `S_AMBIENCE_END`: the city.
pub const CITY: usize = 14;
/// `S_SIREN_START` to `S_SIREN_END`.
const SIRENS: [usize; 4] = [15, 16, 17, 18];
/// `city_animals`: three dogs and four cats.
const CITY_ANIMALS: [usize; 7] = [91, 92, 93, 94, 95, 96, 97];
/// `S_CITYMISC_START` to `S_CITYMISC_END`: distant sirens and a train.
const CITY_MISC: [usize; 3] = [340, 341, 342];
/// `S_BIRD_START`.
const BIRDS: usize = 301;
/// `S_START_SNOW_WIND`: five gusts.
const SNOW_WIND: usize = 308;
const WOLF: usize = 313;
const AIRCRAFT: usize = 314;
/// `S_FOGHORN`.
pub const FOGHORN: usize = 334;
/// `S_TROPICAL`: the jungle's loop.
pub const TROPICAL: usize = 335;
/// `jungle_sounds`: a birdcall, two cockatoos, a cricket and a peacock (`S_BIRD_END`).
const JUNGLE: [usize; 5] = [330, 331, 332, 333, 307];
/// `snow_sounds`: crows and wolves.
const SNOW: [usize; 4] = [156, 313, 156, 384];

/// `wtype`: which sort of place the map is, from its texture set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum World {
    Jungle,
    Snow,
    Estate,
    BusyCity,
    QuietCity,
}

impl World {
    /// `SND_BeginAmbient`: set 1 is the jungle, 5 snow, 10 the estate, 16 a quiet city,
    /// anything else (or none) a busy one.
    pub fn of(texture_set: Option<i32>) -> World {
        match texture_set {
            Some(1) => World::Jungle,
            Some(5) => World::Snow,
            Some(10) => World::Estate,
            Some(16) => World::QuietCity,
            _ => World::BusyCity,
        }
    }

    /// The loop that plays under everything else out of doors: the jungle's
    /// (`SND_BeginAmbient`), none elsewhere.
    pub fn bed(self) -> Option<usize> {
        (self == World::Jungle).then_some(TROPICAL)
    }
}

/// Where a sound comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Place {
    /// Everywhere at once (`MFX_play_ambient`).
    Ambient,
    /// From this point (`PlayAmbient3D`): far enough off that it's heard from its direction
    /// rather than faded by its distance.
    At(Vec3),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cue {
    pub id: usize,
    /// 0 to 1 (`MFX_set_gain`'s 0 to 255).
    pub gain: f32,
    pub place: Place,
}

/// `PlayAmbient3D`'s heights.
#[derive(Clone, Copy)]
enum Height {
    OnGround,
    InAir,
}

pub struct Weather {
    pub world: World,
    /// `GF_RAINING`.
    pub raining: bool,
    /// `siren_time`, `thunder_time`: turns to the next.
    siren: i32,
    thunder: i32,
    /// `tick_tock`: the turn of the next gust of wind.
    tick_tock: i32,
    turn: i32,
    rng: u32,
}

impl Weather {
    pub fn new(world: World, raining: bool) -> Weather {
        Weather { world, raining, siren: 0, thunder: 0, tick_tock: 0, turn: 0, rng: 0x57a7 }
    }

    /// The weather's loop out of doors: rain if it's raining, which drowns out the city's
    /// own; the city's for a city; none otherwise (`process_weather`).
    pub fn looped(&self) -> Option<usize> {
        if self.raining {
            Some(RAIN)
        } else {
            matches!(self.world, World::BusyCity | World::QuietCity).then_some(CITY)
        }
    }

    /// The loop's gain (`rain_gain`): full on the ground, nothing 16 squares above it.
    /// `above` is how far the listener is above the ground.
    pub fn loop_gain(above: f32) -> f32 {
        (1.0 - above / 4096.0).clamp(0.0, 1.0)
    }

    fn random(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(69069).wrapping_add(1);
        self.rng >> 16
    }

    /// `PlayAmbient3D`: somewhere 16 squares off round the player, on the ground or up in
    /// the air.
    fn around(&mut self, id: usize, player: Vec3, height: Height) -> Cue {
        let a = (self.random() & 2047) as f32 * std::f32::consts::TAU / 2048.0;
        let y = match height {
            Height::OnGround => 0.0,
            Height::InAir => player.y + (512 + (self.random() & 1023)) as f32,
        };
        let at = Vec3::new(player.x + a.cos() * 4096.0, y, player.z + a.sin() * 4096.0);
        Cue { id, gain: 1.0, place: Place::At(at) }
    }

    /// One game turn out of doors (`new_outdoors_effects` and the wind of
    /// `process_weather`): what to play now. `player` is where the player is (world units)
    /// and `height` how high the listener is.
    pub fn turn(&mut self, player: Vec3, height: f32) -> Vec<Cue> {
        let mut out = Vec::new();
        self.turn += 1;
        // Now and then something far off: a siren, an aircraft, a wolf.
        self.siren -= 1;
        if self.siren < 0 {
            self.siren = 5000;
            let id = match self.world {
                World::Snow => {
                    // "this one goes on forever, so it's made extra rare"
                    self.siren = 1500 + ((self.random() & 0xffff) >> 5) as i32;
                    Some(WOLF)
                }
                World::Estate => {
                    self.siren = 500 + ((self.random() & 0xffff) >> 5) as i32;
                    Some(AIRCRAFT)
                }
                World::BusyCity => {
                    let id = SIRENS[self.random() as usize % SIRENS.len()];
                    self.siren = 300 + ((self.random() & 0xffff) >> 5) as i32;
                    Some(id)
                }
                World::Jungle | World::QuietCity => None,
            };
            if let Some(id) = id {
                out.push(self.around(id, player, Height::InAir));
            }
        }
        if self.raining {
            self.thunder -= 1;
            if self.thunder < 0 {
                self.thunder = 300 + ((self.random() & 0xffff) >> 5) as i32;
                let id = THUNDER + self.random() as usize % 4;
                out.push(self.around(id, player, Height::InAir));
            }
        }
        // Birdsong, jungle calls, crows and wolves, often.
        if matches!(self.world, World::Estate | World::Jungle | World::Snow) && self.random() & 0x1f00 == 0x1f00 {
            let id = match self.world {
                World::Estate => BIRDS + self.random() as usize % 6,
                World::Jungle => JUNGLE[self.random() as usize % 5],
                _ => SNOW[self.random() as usize & 3],
            };
            out.push(self.around(id, player, Height::InAir));
        }
        // Dogs and cats, breaking glass, a train.
        if self.world == World::BusyCity && self.random() & 0x1fc0 == 0x1fc0 {
            let id = if self.random() & 48 != 0 {
                CITY_ANIMALS[self.random() as usize % 7]
            } else {
                CITY_MISC[self.random() as usize % 3]
            };
            out.push(self.around(id, player, Height::OnGround));
        }
        // The jungle's foghorn, out past x = 64 squares, louder the further out.
        if self.world == World::Jungle && player.x > 64.0 * 256.0 && self.random() & 0x1fc0 == 0x1fc0 {
            let gain = ((player.x - 64.0 * 256.0) / 64.0).min(255.0) / 255.0;
            out.push(Cue { id: FOGHORN, gain, place: Place::Ambient });
        }
        // Gusts of wind, louder the higher up (`wind_gain`).
        if self.turn - self.tick_tock >= 25 {
            let wind_gain = (height / 16.0).clamp(0.0, 255.0) / 255.0;
            let wind = match self.world {
                World::BusyCity | World::QuietCity => {
                    self.tick_tock = self.turn + 30 + (self.random() & 63) as i32;
                    Some((WIND + self.random() as usize % 7, wind_gain))
                }
                World::Snow => {
                    self.tick_tock = self.turn + (self.random() & 15) as i32;
                    Some((SNOW_WIND + self.random() as usize % 5, 127.0 / 255.0))
                }
                _ => None,
            };
            if let Some((id, gain)) = wind {
                out.push(Cue { id, gain, place: Place::Ambient });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: &mut Weather, player: Vec3, turns: usize) -> Vec<Cue> {
        (0..turns).flat_map(|_| w.turn(player, player.y)).collect()
    }

    #[test]
    fn worlds_by_texture_set() {
        assert_eq!(World::of(Some(1)), World::Jungle);
        assert_eq!(World::of(Some(5)), World::Snow);
        assert_eq!(World::of(Some(10)), World::Estate);
        assert_eq!(World::of(Some(16)), World::QuietCity);
        assert_eq!(World::of(Some(3)), World::BusyCity);
        assert_eq!(World::of(None), World::BusyCity);
        assert_eq!(World::Jungle.bed(), Some(TROPICAL));
        assert_eq!(World::BusyCity.bed(), None);
    }

    #[test]
    fn rain_drowns_out_the_city() {
        assert_eq!(Weather::new(World::BusyCity, false).looped(), Some(CITY));
        assert_eq!(Weather::new(World::BusyCity, true).looped(), Some(RAIN));
        assert_eq!(Weather::new(World::Jungle, false).looped(), None);
        assert_eq!(Weather::loop_gain(0.0), 1.0);
        assert_eq!(Weather::loop_gain(5000.0), 0.0);
    }

    /// Ten minutes in the jungle: its calls, no sirens, dogs or wind, and the foghorn only
    /// out past 64 squares.
    #[test]
    fn the_jungle() {
        let mut w = Weather::new(World::Jungle, false);
        let inland = run(&mut w, Vec3::new(40.0 * 256.0, 0.0, 40.0 * 256.0), 12000);
        assert!(inland.iter().filter(|c| JUNGLE.contains(&c.id)).count() > 10);
        assert!(inland.iter().all(|c| JUNGLE.contains(&c.id)), "{:?}", inland.iter().find(|c| !JUNGLE.contains(&c.id)));
        let shore = run(&mut w, Vec3::new(100.0 * 256.0, 0.0, 40.0 * 256.0), 12000);
        let horn = shore.iter().find(|c| c.id == FOGHORN).expect("no foghorn");
        assert!((horn.gain - 144.0 / 255.0).abs() < 0.01, "gain {}", horn.gain);
    }

    #[test]
    fn the_city() {
        let mut w = Weather::new(World::BusyCity, false);
        let cues = run(&mut w, Vec3::new(40.0 * 256.0, 1024.0, 40.0 * 256.0), 12000);
        assert!(cues.iter().any(|c| SIRENS.contains(&c.id)));
        assert!(cues.iter().any(|c| CITY_ANIMALS.contains(&c.id) || CITY_MISC.contains(&c.id)));
        let wind: Vec<_> = cues.iter().filter(|c| (WIND..WIND + 7).contains(&c.id)).collect();
        assert!(wind.len() > 50);
        assert!(wind.iter().all(|c| (c.gain - 64.0 / 255.0).abs() < 0.01));
        assert!(!cues.iter().any(|c| (THUNDER..THUNDER + 4).contains(&c.id)), "thunder with no rain");
    }

    /// Thunder in the rain, every 300 to 2347 turns, up in the sky.
    #[test]
    fn thunder() {
        let mut w = Weather::new(World::QuietCity, true);
        let me = Vec3::new(40.0 * 256.0, 0.0, 40.0 * 256.0);
        let claps: Vec<_> = run(&mut w, me, 12000).into_iter().filter(|c| (THUNDER..THUNDER + 4).contains(&c.id)).collect();
        assert!((5..=41).contains(&claps.len()), "{} claps", claps.len());
        for c in claps {
            let Place::At(at) = c.place else { panic!() };
            assert!(at.y >= 512.0 && at.y < 1536.0);
            assert!(((at - me) * Vec3::new(1.0, 0.0, 1.0)).length() > 4000.0);
        }
    }
}
