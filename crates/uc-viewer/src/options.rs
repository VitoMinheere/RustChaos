//! The Options screen's settings (#201; the manual's Options, pp. 22-24): the volumes and the
//! detail switches, kept in `options.txt` beside the progress file.

use crate::*;

/// What's switched: three volumes in tenths, and the detail.
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl Default for Options {
    fn default() -> Options {
        Options {
            master: 10, effects: 10, music: 10, shadows: true, dirt: true, mist: true, rain: true, skyline: true, moon: true,
            crinkles: true, fog: true, linear: false,
        }
    }
}

/// The rows of the screen, top to bottom.
pub(crate) const ROWS: usize = 13;

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
        format!(
            "master={}\neffects={}\nmusic={}\nshadows={}\ndirt={}\nmist={}\nrain={}\nskyline={}\nmoon={}\ncrinkles={}\nfog={}\nfiltering={}\n",
            self.master, self.effects, self.music, on(self.shadows), on(self.dirt), on(self.mist), on(self.rain),
            on(self.skyline), on(self.moon), on(self.crinkles), on(self.fog), if self.linear { "smooth" } else { "crisp" },
        )
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
            _ => {}
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
    /// rain and the other extras, and the texture filtering.
    pub(crate) fn apply_to(&self, args: &mut Args, fog: Option<(f32, f32)>) {
        args.crinkles = self.crinkles;
        args.fog = if self.fog { fog } else { None };
        args.rain = self.rain;
        args.show = self.show();
        args.linear = self.linear;
    }

    /// The command line's switches win for the run they're given on: `--no-crinkles`,
    /// `--no-fog` and `--linear` (`--mute` silences everything anyway).
    pub(crate) fn with_args(mut self, args: &Args) -> Options {
        self.crinkles &= args.crinkles;
        self.fog &= args.fog.is_some();
        self.linear |= args.linear;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_changes_and_volumes_stop_at_the_ends() {
        let mut o = Options::default();
        for k in 0..ROWS - 1 {
            let before = o;
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
        for k in [1, 1, 2, 4, 9, 11] {
            o.change(k, -1);
        }
        assert_eq!(Options::parse(&o.text()), o);
        assert_eq!(Options::parse(""), Options::default());
        let odd = Options::parse("master=99\nfog=off\nnonsense");
        assert_eq!((odd.master, odd.fog), (10, false));
    }
}
