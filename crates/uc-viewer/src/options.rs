//! The Options screen's settings (#201; the manual's Options, pp. 22-24): the volumes and the
//! detail switches, kept in `options.txt` beside the progress file.

use crate::*;

/// What's switched: three volumes in tenths, the detail, and the textures.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Options {
    pub(crate) master: u8,
    pub(crate) effects: u8,
    pub(crate) music: u8,
    pub(crate) shadows: bool,
    pub(crate) dirt: bool,
    pub(crate) mist: bool,
    pub(crate) rain: bool,
    pub(crate) skyline: bool,
    pub(crate) moon: bool,
    pub(crate) crinkles: bool,
    pub(crate) fog: bool,
    /// Smooth texture filtering (`--linear`), from the next start.
    pub(crate) linear: bool,
    /// The upscaled textures (`tools/upscale/`) in place of the originals, from
    /// [`Options::textures_dir`], from the next level loaded (the console's `textures`
    /// switches them on the spot).
    pub(crate) upscaled: bool,
    /// Where they are (`textures_dir=`): [`TEXTURES`] beside the progress file if not set.
    pub(crate) textures_dir: Option<PathBuf>,
    /// Walls, floors and roofs, people, cars and props lit by their upscaled pages' normal
    /// maps (#474, #521): off, soft or strong ([`RELIEF`]), from the next level loaded (the
    /// console's `relief` goes round them on the spot).
    pub(crate) relief: u8,
    /// What lights itself glowing (#523): off, soft or strong ([`BLOOM`]), on the spot.
    pub(crate) bloom: u8,
    /// The roads wet on a rainy night (#522), on the spot.
    pub(crate) wet: bool,
    /// `--overrides DIR`: the textures for this run, whatever's set here. Not saved.
    pub(crate) forced: Option<PathBuf>,
}

/// The relief's strengths (#474): off, soft and strong. At 1 the normal maps light the
/// relief by as much as the lights light the faces, which in daylight, lit mostly by the
/// ambient light whose direction moves it by a quarter, hardly shows.
pub(crate) const RELIEF: [f32; 3] = [0.0, 1.5, 3.0];
const RELIEF_NAMES: [&str; 3] = ["off", "soft", "strong"];

/// The bloom's strengths (#523): off, soft and strong. At 1, at night, a lit window's glow
/// is added round it as bright again.
pub(crate) const BLOOM: [f32; 3] = [0.0, 1.2, 2.4];

/// The upscaled textures' folder when none is set, beside the progress file.
pub(crate) const TEXTURES: &str = "textures";

impl Default for Options {
    fn default() -> Options {
        Options {
            master: 10, effects: 10, music: 10, shadows: true, dirt: true, mist: true, rain: true, skyline: true, moon: true,
            crinkles: true, fog: true, linear: false, upscaled: false, textures_dir: None, relief: 0, bloom: 0, wet: true, forced: None,
        }
    }
}

/// The rows of the screen, top to bottom: the settings, then the controls' page and back.
pub(crate) const ROWS: usize = 18;
pub(crate) const TEXTURES_ROW: usize = 12;
pub(crate) const RELIEF_ROW: usize = 13;
pub(crate) const BLOOM_ROW: usize = 14;
pub(crate) const WET_ROW: usize = 15;
pub(crate) const CONTROLS_ROW: usize = 16;

/// The effects and music at full volume before these were here (`Audio`'s defaults).
const EFFECTS: f32 = 0.8;
const MUSIC: f32 = 0.55;

impl Options {
    fn path() -> Option<PathBuf> {
        Some(progress::Progress::folder()?.join("options.txt"))
    }

    /// The saved settings, the defaults for any not saved.
    pub(crate) fn load() -> Options {
        Options::parse(&Options::path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default())
    }

    /// From the file's `key=value` lines.
    fn parse(text: &str) -> Options {
        let mut o = Options::default();
        for (key, value) in text.lines().filter_map(|l| l.split_once('=')) {
            let (key, value) = (key.trim(), value.trim());
            let on = value == "on";
            let tenths = value.parse::<u8>().ok().map(|v| v.min(10));
            match key {
                "master" => o.master = tenths.unwrap_or(o.master),
                "effects" => o.effects = tenths.unwrap_or(o.effects),
                "music" => o.music = tenths.unwrap_or(o.music),
                "shadows" => o.shadows = on,
                "dirt" => o.dirt = on,
                "mist" => o.mist = on,
                "rain" => o.rain = on,
                "skyline" => o.skyline = on,
                "moon" => o.moon = on,
                "crinkles" => o.crinkles = on,
                "fog" => o.fog = on,
                "filtering" => o.linear = value == "smooth",
                "textures" => o.upscaled = value == "upscaled",
                "relief" => o.relief = RELIEF_NAMES.iter().position(|&n| n == value).unwrap_or(0) as u8,
                "bloom" => o.bloom = RELIEF_NAMES.iter().position(|&n| n == value).unwrap_or(0) as u8,
                "wet" => o.wet = on,
                "textures_dir" if !value.is_empty() => o.textures_dir = Some(PathBuf::from(value)),
                _ => {}
            }
        }
        o
    }

    pub(crate) fn save(&self) {
        let Some(path) = Options::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.text());
    }

    /// As the file has them.
    fn text(&self) -> String {
        let on = |b: bool| if b { "on" } else { "off" };
        let dir = self.textures_dir.as_ref().map_or(String::new(), |d| format!("textures_dir={}\n", d.display()));
        format!(
            "master={}\neffects={}\nmusic={}\nshadows={}\ndirt={}\nmist={}\nrain={}\nskyline={}\nmoon={}\ncrinkles={}\nfog={}\nfiltering={}\ntextures={}\nrelief={}\nbloom={}\nwet={}\n{dir}",
            self.master, self.effects, self.music, on(self.shadows), on(self.dirt), on(self.mist), on(self.rain),
            on(self.skyline), on(self.moon), on(self.crinkles), on(self.fog), if self.linear { "smooth" } else { "crisp" },
            if self.upscaled { "upscaled" } else { "original" }, RELIEF_NAMES[self.relief as usize % 3],
            RELIEF_NAMES[self.bloom as usize % 3], on(self.wet),
        )
    }

    /// The upscaled textures' folder: the one set, else [`TEXTURES`] beside the progress
    /// file. A folder with a `clumps` folder in it (the upscaler's own output, which has the
    /// fonts and the like beside the clumps) means that one.
    pub(crate) fn textures_dir(&self) -> Option<PathBuf> {
        let dir = self.textures_dir.clone().or_else(|| Some(progress::Progress::folder()?.join(TEXTURES)))?;
        let clumps = dir.join("clumps");
        Some(if clumps.is_dir() { clumps } else { dir })
    }

    /// The folder the levels take their textures from: `--overrides`'s, else the upscaled
    /// textures' when they're on.
    pub(crate) fn overrides(&self) -> Option<PathBuf> {
        self.forced.clone().or_else(|| self.upscaled.then(|| self.textures_dir()).flatten())
    }

    /// Row `k`: its name and what it's set to.
    pub(crate) fn row(&self, k: usize) -> (&'static str, String) {
        let on = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
        let bar = |v: u8| format!("{}{} {}", "|".repeat(v as usize), ".".repeat(10 - v as usize), v * 10);
        match k {
            0 => ("MASTER VOLUME", bar(self.master)),
            1 => ("EFFECTS VOLUME", bar(self.effects)),
            2 => ("MUSIC VOLUME", bar(self.music)),
            3 => ("SHADOWS", on(self.shadows)),
            4 => ("LEAVES AND LITTER", on(self.dirt)),
            5 => ("MIST", on(self.mist)),
            6 => ("RAIN", on(self.rain)),
            7 => ("SKYLINE", on(self.skyline)),
            8 => ("MOON", on(self.moon)),
            9 => ("CRINKLES", on(self.crinkles)),
            10 => ("FOG", on(self.fog)),
            11 => ("TEXTURE FILTERING", if self.linear { "SMOOTH (NEXT START)".into() } else { "CRISP (NEXT START)".into() }),
            TEXTURES_ROW => ("TEXTURES", self.textures_value()),
            RELIEF_ROW => ("RELIEF", match self.relief {
                0 => "OFF".into(),
                1 => "SOFT (UPSCALED TEXTURES, NEXT LEVEL)".into(),
                _ => "STRONG (UPSCALED TEXTURES, NEXT LEVEL)".into(),
            }),
            BLOOM_ROW => ("BLOOM", ["OFF", "SOFT", "STRONG"][self.bloom as usize % 3].into()),
            WET_ROW => ("WET ROADS", on(self.wet)),
            CONTROLS_ROW => ("CONTROLS", String::new()),
            _ => ("BACK", String::new()),
        }
    }

    /// Row `k` changed by `by` (Left -1, Right +1): a volume a tenth, a switch flipped.
    pub(crate) fn change(&mut self, k: usize, by: i8) {
        let step = |v: &mut u8| *v = (*v as i8 + by).clamp(0, 10) as u8;
        match k {
            0 => step(&mut self.master),
            1 => step(&mut self.effects),
            2 => step(&mut self.music),
            3 => self.shadows = !self.shadows,
            4 => self.dirt = !self.dirt,
            5 => self.mist = !self.mist,
            6 => self.rain = !self.rain,
            7 => self.skyline = !self.skyline,
            8 => self.moon = !self.moon,
            9 => self.crinkles = !self.crinkles,
            10 => self.fog = !self.fog,
            11 => self.linear = !self.linear,
            TEXTURES_ROW => self.upscaled = !self.upscaled,
            RELIEF_ROW => self.relief = (self.relief as i8 + by).rem_euclid(3) as u8,
            BLOOM_ROW => self.bloom = (self.bloom as i8 + by).rem_euclid(3) as u8,
            WET_ROW => self.wet = !self.wet,
            _ => {}
        }
    }

    /// The textures row's value: which, and for the upscaled ones whether their folder is
    /// there, with how many missions' clumps it has.
    fn textures_value(&self) -> String {
        if self.forced.is_some() {
            return "--OVERRIDES (THIS RUN)".into();
        }
        if !self.upscaled {
            return "ORIGINAL".into();
        }
        let dir = self.textures_dir();
        let missions = dir.as_ref().and_then(|d| std::fs::read_dir(d).ok())
            .map_or(0, |r| r.flatten().filter(|e| e.path().is_dir()).count());
        let pages = dir.as_ref().and_then(|d| std::fs::read_dir(d).ok())
            .map_or(0, |r| r.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("page_")).count());
        match (missions, pages) {
            (0, 0) => "UPSCALED (NO FOLDER: SEE README)".into(),
            (0, n) => format!("UPSCALED ({n} PAGES, NEXT LEVEL)"),
            (n, _) => format!("UPSCALED ({n} CLUMPS, NEXT LEVEL)"),
        }
    }

    /// The effects' and music's volumes, from the master's and their own.
    pub(crate) fn volumes(&self) -> (f32, f32) {
        let m = self.master as f32 / 10.0;
        (EFFECTS * m * self.effects as f32 / 10.0, MUSIC * m * self.music as f32 / 10.0)
    }

    /// What the game draws.
    pub(crate) fn show(&self) -> uc_game::Show {
        uc_game::Show { shadows: self.shadows, dirt: self.dirt, mist: self.mist, skyline: self.skyline, moon: self.moon }
    }

    /// Into `args`, for the worlds made from them: crinkles, fog (`fog` when it's on), the
    /// rain and the other extras, the texture filtering and the textures.
    pub(crate) fn apply_to(&self, args: &mut Args, fog: Option<(f32, f32)>) {
        args.overrides = self.overrides();
        args.relief = RELIEF[self.relief as usize % 3];
        args.bloom = BLOOM[self.bloom as usize % 3];
        args.wet = self.wet;
        args.crinkles = self.crinkles;
        args.fog = if self.fog { fog } else { None };
        args.rain = self.rain;
        args.show = self.show();
        args.linear = self.linear;
    }

    /// The command line's switches win for the run they're given on: `--no-crinkles`,
    /// `--no-fog`, `--linear`, `--relief`, `--bloom`, `--no-wet` and `--overrides` (`--mute` silences everything
    /// anyway).
    pub(crate) fn with_args(mut self, args: &Args) -> Options {
        self.crinkles &= args.crinkles;
        self.fog &= args.fog.is_some();
        self.wet &= args.wet;
        self.linear |= args.linear;
        if args.relief > 0.0 {
            self.relief = 2;
        }
        if args.bloom > 0.0 {
            self.bloom = 2;
        }
        self.forced = args.overrides.clone();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_changes_and_volumes_stop_at_the_ends() {
        let mut o = Options::default();
        for k in 0..CONTROLS_ROW {
            let before = o.clone();
            o.change(k, -1);
            assert_ne!(o, before, "row {k} ({}) didn't change", o.row(k).0);
        }
        o.master = 10;
        o.change(0, 1);
        assert_eq!(o.master, 10);
        o.master = 0;
        o.change(0, -1);
        assert_eq!(o.master, 0);
        assert_eq!(o.volumes().0, 0.0);
        assert_eq!(Options::default().volumes(), (EFFECTS, MUSIC));
    }

    #[test]
    fn saved_and_read_back_the_same() {
        let mut o = Options::default();
        for k in [1, 1, 2, 4, 9, 11, WET_ROW] {
            o.change(k, -1);
        }
        assert_eq!(Options::parse(&o.text()), o);
        assert_eq!(Options::parse(""), Options::default());
        let odd = Options::parse("master=99\nfog=off\nnonsense");
        assert_eq!((odd.master, odd.fog), (10, false));
        let up = Options::parse("textures=upscaled\ntextures_dir=/somewhere/textures_4x\nrelief=strong");
        assert!(up.upscaled);
        assert_eq!(up.relief, 2);
        assert_eq!(Options::parse(&up.text()), up);
    }

    /// The upscaled textures are the levels' only when they're on; `--overrides` wins for
    /// its run, on or off. A folder with `clumps` in it means that.
    #[test]
    fn the_textures_folder() {
        let dir = std::env::temp_dir().join(format!("uc-textures-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("clumps/police1")).unwrap();
        let mut o = Options { textures_dir: Some(dir.clone()), ..Options::default() };
        assert_eq!(o.overrides(), None);
        o.upscaled = true;
        assert_eq!(o.overrides(), Some(dir.join("clumps")));
        assert_eq!(o.row(TEXTURES_ROW).1, "UPSCALED (1 CLUMPS, NEXT LEVEL)");
        let cli = Args { overrides: Some(PathBuf::from("/elsewhere")), ..Args::default() };
        let o = o.with_args(&cli);
        assert_eq!(o.overrides(), Some(PathBuf::from("/elsewhere")));
        assert_eq!(o.row(TEXTURES_ROW).1, "--OVERRIDES (THIS RUN)");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
