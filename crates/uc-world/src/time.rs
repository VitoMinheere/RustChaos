//! Time of day (#515): any map played at day, dusk, night or dawn, not in the original. A
//! mission's lighting file sets its time; this picks another file for the same map where
//! the game shipped one ([`shipped`]), and otherwise derives one from the mission's own
//! ([`derive`]).

use uc_formats::lgt::{Lighting, FLAG_DAYTIME, FLAG_LIGHTS_UNDER_LAMPOSTS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeOfDay {
    Day,
    Dusk,
    Night,
    Dawn,
}

impl TimeOfDay {
    /// In the order the pickers go round them.
    pub const ALL: [TimeOfDay; 4] = [TimeOfDay::Day, TimeOfDay::Dusk, TimeOfDay::Night, TimeOfDay::Dawn];

    pub fn name(self) -> &'static str {
        match self {
            TimeOfDay::Day => "day",
            TimeOfDay::Dusk => "dusk",
            TimeOfDay::Night => "night",
            TimeOfDay::Dawn => "dawn",
        }
    }

    /// The time called `name`, in any case.
    pub fn named(name: &str) -> Option<TimeOfDay> {
        TimeOfDay::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(name))
    }

    /// The next in a picker that starts and ends with the map's own lighting (`None`).
    pub fn cycle(time: Option<TimeOfDay>) -> Option<TimeOfDay> {
        match time {
            None => Some(TimeOfDay::ALL[0]),
            Some(t) => TimeOfDay::ALL.get(TimeOfDay::ALL.iter().position(|&x| x == t)? + 1).copied(),
        }
    }

    /// Its number, for files: 0 the map's own, then 1 up in [`TimeOfDay::ALL`]'s order.
    pub fn to_u8(time: Option<TimeOfDay>) -> u8 {
        time.and_then(|t| TimeOfDay::ALL.iter().position(|&x| x == t)).map_or(0, |k| k as u8 + 1)
    }

    pub fn from_u8(n: u8) -> Option<TimeOfDay> {
        TimeOfDay::ALL.get((n as usize).checked_sub(1)?).copied()
    }

    /// The sun's up: the file is `NIGHT_FLAG_DAYTIME`, so no rain, moon or headlights.
    pub fn daylight(self) -> bool {
        matches!(self, TimeOfDay::Day | TimeOfDay::Dawn)
    }
}

/// The lighting files the game shipped for one map at several times: (day, dusk, night,
/// dawn), each the file's name or none. A file in a family that isn't the time asked for
/// is derived from the family's `base`.
struct Family {
    files: [Option<&'static str>; 4],
    /// The file the missing times are derived from.
    base: &'static str,
    /// Other files that light the map the same way, which missions use.
    also: &'static [&'static str],
}

/// The game's variants of one map's lighting. Gangland's `gang1.lgt` is darker than its
/// dawn and lighter than its night, with the lamps lit: its dusk. `southside DUSK2.lgt` is
/// `miss1.lgt` byte for byte but the name. `e7.lgt` is `e7night.lgt`.
const FAMILIES: [Family; 4] = [
    Family { files: [None, Some("gang1.lgt"), Some("gang1NIGHT.lgt"), Some("gang1DAWN.lgt")], base: "gang1DAWN.lgt", also: &[] },
    Family { files: [Some("e6.lgt"), None, Some("e6night.lgt"), None], base: "e6.lgt", also: &[] },
    Family { files: [Some("e7day.lgt"), None, Some("e7night.lgt"), None], base: "e7day.lgt", also: &["e7.lgt"] },
    Family {
        files: [None, Some("southside DUSK2.lgt"), Some("southside NIGHT3b.lgt"), Some("southside DAWN.lgt")],
        base: "southside DAWN.lgt", also: &["miss1.lgt"],
    },
];

/// The file name at the end of a mission's lighting path (`Data\Lighting\gang1.lgt`).
fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// `path` with its file name swapped for `file`.
fn with_file(path: &str, file: &str) -> String {
    let dir = &path[..path.len() - file_name(path).len()];
    format!("{dir}{file}")
}

/// Where the lighting for `time` comes from, given a mission's lighting file `path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A file as it is: one the game shipped for that time, or the mission's own when it's
    /// already that time.
    File(String),
    /// Derived ([`derive`]) from this file.
    Derived(String),
}

/// Where `time`'s lighting comes from for a mission lit by `path`: a variant the game
/// shipped for the map; else the mission's own file, as it is if it's the right side of
/// day and night already (a day file for day, a night one for night), else derived from it.
/// `daytime` is whether the mission's own file is `NIGHT_FLAG_DAYTIME`.
pub fn source(path: &str, daytime: bool, time: TimeOfDay) -> Source {
    let file = file_name(path);
    let k = TimeOfDay::ALL.iter().position(|&t| t == time).unwrap_or(0);
    let is = |f: &str| f.eq_ignore_ascii_case(file);
    if let Some(fam) = FAMILIES.iter().find(|f| f.files.iter().flatten().any(|&f| is(f)) || f.also.iter().any(|&f| is(f))) {
        return match fam.files[k] {
            Some(f) => Source::File(with_file(path, f)),
            None => Source::Derived(with_file(path, fam.base)),
        };
    }
    match time {
        TimeOfDay::Day if daytime => Source::File(path.to_string()),
        TimeOfDay::Night if !daytime => Source::File(path.to_string()),
        _ => Source::Derived(path.to_string()),
    }
}

/// What a derived file is given for each time: (ambient, sky, static lights' strength,
/// lampposts' strength). The ambients are the shipped files' for each time, about: 79,79,79
/// for `e7day.lgt`, 15,23,33 for `gang1NIGHT.lgt`. The day sky is the 210,200,240 most of
/// the day files have; the night's `gang1NIGHT.lgt`'s. Dusk and dawn are greyed, as the
/// haze rule ([`crate::level`]'s `haze`) keeps the distance from going a strong colour.
struct Preset {
    ambient: [i32; 3],
    sky: [u8; 3],
    lights: f32,
    lamps: f32,
}

fn preset(time: TimeOfDay) -> Preset {
    match time {
        TimeOfDay::Day => Preset { ambient: [76, 74, 68], sky: [210, 200, 240], lights: 0.5, lamps: 0.0 },
        TimeOfDay::Dusk => Preset { ambient: [46, 36, 44], sky: [96, 72, 86], lights: 1.0, lamps: 1.0 },
        TimeOfDay::Night => Preset { ambient: [18, 24, 38], sky: [0, 0, 26], lights: 1.0, lamps: 1.0 },
        TimeOfDay::Dawn => Preset { ambient: [70, 60, 52], sky: [176, 164, 172], lights: 0.75, lamps: 0.5 },
    }
}

/// `base` turned to `time`: the ambient light and the sky (and so the fog) the time's, a
/// quarter of the ambient's tint kept from `base`; daylight or not; and the static lights
/// and lampposts faded by day, at full strength from dusk to night, the lampposts lit even
/// if `base` had them off.
pub fn derive(base: &Lighting, time: TimeOfDay) -> Lighting {
    let p = preset(time);
    let mut l = base.clone();
    let top = |c: [i32; 3]| c.into_iter().max().unwrap_or(0).max(1);
    let tint = base.ambient.map(|c| c * top(p.ambient) / top(base.ambient));
    l.ambient = std::array::from_fn(|k| (p.ambient[k] * 3 + tint[k]) / 4);
    l.sky = p.sky;
    l.flags = if time.daylight() { l.flags | FLAG_DAYTIME } else { l.flags & !FLAG_DAYTIME };
    l.flags = if p.lamps > 0.0 { l.flags | FLAG_LIGHTS_UNDER_LAMPOSTS } else { l.flags & !FLAG_LIGHTS_UNDER_LAMPOSTS };
    let fade = |rgb: [i8; 3], by: f32| rgb.map(|c| (c as f32 * by).round() as i8);
    for light in &mut l.lights {
        light.rgb = fade(light.rgb, p.lights);
    }
    l.lampost_rgb = fade(l.lampost_rgb, p.lamps);
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    const GANG: &str = r"Data\Lighting\gang1.lgt";

    #[test]
    fn names_and_numbers_round_trip() {
        for t in TimeOfDay::ALL {
            assert_eq!(TimeOfDay::named(&t.name().to_uppercase()), Some(t));
        }
        assert_eq!(TimeOfDay::named("noon"), None);
        for t in [None].into_iter().chain(TimeOfDay::ALL.map(Some)) {
            assert_eq!(TimeOfDay::from_u8(TimeOfDay::to_u8(t)), t);
        }
        // The picker goes round all four and back to the map's own.
        let mut t = None;
        let mut seen = Vec::new();
        for _ in 0..5 {
            t = TimeOfDay::cycle(t);
            seen.push(t);
        }
        assert_eq!(seen, [Some(TimeOfDay::Day), Some(TimeOfDay::Dusk), Some(TimeOfDay::Night), Some(TimeOfDay::Dawn), None]);
    }

    /// The maps with variants get them, whichever of the family the mission uses.
    #[test]
    fn shipped_variants_are_used() {
        let file = |s: &str| Source::File(s.to_string());
        assert_eq!(source(GANG, false, TimeOfDay::Night), file(r"Data\Lighting\gang1NIGHT.lgt"));
        assert_eq!(source(r"Data\Lighting\gang1NIGHT.lgt", false, TimeOfDay::Dawn), file(r"Data\Lighting\gang1DAWN.lgt"));
        assert_eq!(source(r"Data\Lighting\gang1DAWN.lgt", true, TimeOfDay::Dusk), file(GANG));
        assert_eq!(source(r"Data\Lighting\e6.lgt", true, TimeOfDay::Night), file(r"Data\Lighting\e6night.lgt"));
        assert_eq!(source(r"Data\Lighting\e7night.lgt", false, TimeOfDay::Day), file(r"Data\Lighting\e7day.lgt"));
        assert_eq!(source(r"Data\Lighting\e7.lgt", false, TimeOfDay::Day), file(r"Data\Lighting\e7day.lgt"));
        assert_eq!(source(r"Data\Lighting\southside DAWN.lgt", true, TimeOfDay::Night), file(r"Data\Lighting\southside NIGHT3b.lgt"));
        assert_eq!(source(r"Data\Lighting\miss1.lgt", false, TimeOfDay::Dusk), file(r"Data\Lighting\southside DUSK2.lgt"));
        assert_eq!(source(r"Data\Lighting\southside NIGHT3b.lgt", false, TimeOfDay::Dawn), file(r"Data\Lighting\southside DAWN.lgt"));
        // In any case.
        assert_eq!(source(r"Data\Lighting\GANG1night.lgt", false, TimeOfDay::Night), file(r"Data\Lighting\gang1NIGHT.lgt"));
        // A time the family hasn't got is derived from its base.
        assert_eq!(source(GANG, false, TimeOfDay::Day), Source::Derived(r"Data\Lighting\gang1DAWN.lgt".into()));
        assert_eq!(source(r"Data\Lighting\e6night.lgt", false, TimeOfDay::Dusk), Source::Derived(r"Data\Lighting\e6.lgt".into()));
        assert_eq!(source(r"Data\Lighting\southside DUSK2.lgt", false, TimeOfDay::Day),
                   Source::Derived(r"Data\Lighting\southside DAWN.lgt".into()));
    }

    /// Elsewhere a day file stays for day and a night one for night; the rest is derived.
    #[test]
    fn other_maps_derive_the_other_times() {
        let p = r"Data\Lighting\disturb1.lgt";
        assert_eq!(source(p, true, TimeOfDay::Day), Source::File(p.into()));
        assert_eq!(source(p, true, TimeOfDay::Night), Source::Derived(p.into()));
        assert_eq!(source(p, true, TimeOfDay::Dawn), Source::Derived(p.into()));
        let p = r"Data\Lighting\disturb2.lgt";
        assert_eq!(source(p, false, TimeOfDay::Night), Source::File(p.into()));
        assert_eq!(source(p, false, TimeOfDay::Day), Source::Derived(p.into()));
        assert_eq!(source(p, false, TimeOfDay::Dusk), Source::Derived(p.into()));
    }

    fn lighting(flags: u32, ambient: [i32; 3], sky: [u8; 3]) -> Lighting {
        Lighting {
            version: 1, flags, ambient, sky, lampost_rgb: [70, 70, 36], lampost_radius: 255,
            lights: vec![uc_formats::lgt::Light { range: 40, rgb: [100, 80, -20], pos: [0, 0, 0] }],
        }
    }

    /// A night derived from a day file: darker, a dark sky, not daylight (so it rains), the
    /// lampposts lit and every light at full strength.
    #[test]
    fn night_from_day() {
        let day = lighting(FLAG_DAYTIME, [79, 79, 61], [210, 200, 240]);
        let night = derive(&day, TimeOfDay::Night);
        assert!(night.ambient.iter().all(|&c| c < 40), "{:?}", night.ambient);
        assert!(night.ambient[2] > night.ambient[0], "bluer: {:?}", night.ambient);
        assert_eq!(night.sky, [0, 0, 26]);
        assert_eq!(night.flags & FLAG_DAYTIME, 0);
        assert_ne!(night.flags & FLAG_LIGHTS_UNDER_LAMPOSTS, 0);
        assert_eq!(night.lights[0].rgb, [100, 80, -20]);
        assert_eq!(night.lampost_rgb, [70, 70, 36]);
    }

    /// A day derived from a night file: brighter, a light sky, daylight, the lampposts off
    /// and the static lights faded.
    #[test]
    fn day_from_night() {
        let night = lighting(FLAG_LIGHTS_UNDER_LAMPOSTS | 2, [15, 23, 33], [0, 0, 26]);
        let day = derive(&night, TimeOfDay::Day);
        assert!(day.ambient.iter().all(|&c| c > 60), "{:?}", day.ambient);
        assert_eq!(day.sky, [210, 200, 240]);
        assert_ne!(day.flags & FLAG_DAYTIME, 0);
        assert_eq!(day.flags & FLAG_LIGHTS_UNDER_LAMPOSTS, 0);
        assert_eq!(day.flags & 2, 2, "other flags kept");
        assert_eq!(day.lights[0].rgb, [50, 40, -10]);
        assert_eq!(day.lampost_rgb, [0, 0, 0]);
        // Dusk and dawn fall between.
        let (dusk, dawn) = (derive(&night, TimeOfDay::Dusk), derive(&night, TimeOfDay::Dawn));
        let sum = |l: &Lighting| l.ambient.iter().sum::<i32>();
        assert!(sum(&derive(&night, TimeOfDay::Night)) < sum(&dusk) && sum(&dusk) < sum(&dawn) && sum(&dawn) < sum(&day));
        assert_eq!(dusk.flags & FLAG_DAYTIME, 0);
        assert_ne!(dawn.flags & FLAG_DAYTIME, 0);
    }
}
