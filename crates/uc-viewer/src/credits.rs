//! The credits (#209): the original's, as its outro scrolls them after the last mission
//! (fallen/outro/credits.cpp), then this project's.

use uc_render::Overlay;

/// The music under them (`S_CREDITS_LOOP`, `OS_hack`, fallen/outro/os.cpp:3975).
pub const MUSIC: &str = "GeneralMusic/CreditDue.wav";

/// The credits by section (`CREDITS_section`, credits.cpp:385): a title and its lines. As in
/// the original, a line starting `~B` is a heading and `~I` a subheading, each leading tab
/// indents it a step, and an empty line is a gap.
pub const SECTIONS: [(&str, &[&str]); 8] = [
    ("MUCKYFOOT", MUCKYFOOT),
    ("EIDOS UK", EIDOS_UK),
    ("EIDOS USA", EIDOS_USA),
    ("EIDOS FRANCE", EIDOS_FRANCE),
    ("EIDOS GERMANY", EIDOS_GERMANY),
    ("VOICE PRODUCTION", VOICE_PRODUCTION),
    ("ORIGINAL CD MUSIC", BANDS),
    ("RUSTCHAOS", RUSTCHAOS),
];

/// `CREDITS_muckyfoot`.
const MUCKYFOOT: &[&str] = &[
    "Mucky Foot are Ashley Hampton, Barry Meade, Chris Knott, Eddie",
    "Edwards, Fin McGechie, Gary Carr, Guy Simmons, James 'Dudley'",
    "Watson, Jan Svarovsky, John Steels, Junior Walker, Justin Amore,",
    "Mark Smart, Mark Adami, Martin Oliver, Matthew Rosenfeld, Mike",
    "Burnham, Mike Diskett, Ollie Shaw, Richard Franke, Simon 'Grimmy'",
    "Keating, Stuart Black, Tom Forsyth, Tom Ireland and Wayne Imlach.",
    "",
    "Here's an effort to divide up what we did to make Urban Chaos!",
    "",
    "~BProgramming",
    "\tMike Diskett",
    "\tMark Adami",
    "\tMatthew Rosenfeld",
    "\tJames 'Dudley' Watson",
    "\tEddie Edwards",
    "\tGuy Simmons",
    "\tTom Forsyth",
    "~I\tAdditional programming",
    "\t\tJeremy Longley",
    "",
    "~BArt Direction",
    "\tFin McGechie",
    "",
    "~BArt",
    "\tStuart Black",
    "\tRichard Franke",
    "\tOllie Shaw",
    "\tFin McGechie",
    "\tJunior Walker",
    "\tChris Knott",
    "\tGary Carr",
    "~I\tAdditional art",
    "\t\tTerry Catrell",
    "\t\tJoe Rider",
    "\t\tSteve Brown",
    "",
    "~BAnimation",
    "\tOllie Shaw",
    "\tJunior Walker",
    "\tChris Knott",
    "",
    "~BLevel Design",
    "\tSimon 'Grimmy' Keating",
    "\tBarry Meade",
    "",
    "~BSound and Music",
    "\tMartin Oliver",
    "",
    "~BScripting",
    "\tBarry Meade",
    "\tSimon 'Grimmy' Keating",
    "\tFin McGechie",
    "",
    "~BTesting",
    "\tJustin (mucky)hands Amore",
    "~I\tAdditional testing",
    "\t\tSean Lamacraft",
    "\t\tMarie Colwell",
    "\t\tMark Rose",
    "\t\tDavid Harlow",
    "~I\tFurther testing",
    "\t\tChristopher Absolom",
    "\t\tMark Baker",
    "\t\tAlex Blackwood",
    "\t\tDahman Coombes",
    "\t\tTom Everard",
    "\t\tEamon Meadows",
    "\t\tAnthony Nicholson",
    "\t\tTom Patterson",
    "\t\tLawrence Phillips",
    "\t\tDaniel Purvis",
    "\t\tGary Reed",
    "\t\tAmy Ross",
    "\t\tPeter Ruscoe",
    "\t\tKraig Stone",
    "\t\tLorne Tietjen",
    "\t\tDavid Walker",
    "\t\tDavid Wright",
    "",
    "\tMichael Burnham",
    "\tTom Forsyth",
    "\tGary Carr",
    "\tJohn Steels",
    "\tTom Ireland",
    "\tJan Svarovsky",
    "\tand everyone at MuckyFoot",
    "",
    "~BIT and Mucky Website",
    "\tMichael Burnham",
    "",
    "~BPR",
    "\tCathy Campos at PanachePR",
    "",
    "~BFurry Friends",
    "\tSam",
    "\tLisa",
    "\tBarney",
    "",
    "Special thanks to Glenn Corpes and a big mucky \"Thank You\" from",
    "Mucky Foot to everyone at Eidos. Hello to the Wednesday night",
    "footballers!",
];

/// `CREDITS_eidos_uk`.
const EIDOS_UK: &[&str] = &[
    "~BSenior Producer",
    "\tDarren Hedges",
    "",
    "~BLocalisation Team",
    "\tHolly Andrews",
    "\tFlavia Timiani",
    "",
    "~BMarketing",
    "\tKaren Ridley",
    "\tLorna Evans",
    "",
    "~BPR",
    "\tJonathan Rosenblatt",
    "\tSteve Starvis",
    "",
    "~BCreative Services",
    "\tCaroline Simon",
    "",
    "~BLocalisation QA",
    "\tMichael Weissmuller",
    "\tAlex Lepoureau",
    "\tAlessandro Mantelli",
    "",
    "~BQA Manager",
    "\tTony Bourne",
    "",
    "~BLead QA",
    "\tJean Duret",
    "",
    "~BQA",
    "\tLinus Dominique (Assistant Lead)",
    "\tJason Walker (Hardware Specialist)",
    "\tMichael Hanley",
    "\tAndrew Christopher",
    "\tAnthony Fretwell",
    "\tBJ Samuel Kil",
    "\tChris Ince",
    "\tClint Nembhard",
    "\tEsmond Ferns",
    "\tGuy Cooper",
    "\tLouis Farnham",
    "\tMarlon Grant",
    "\tNoel Cowan",
    "\tPatrick Cowan",
    "\tTyrone O'Neil",
    "",
    "~BWith thanks for the last leg",
    "\tIain McNeil",
    "\tGrant Dean",
];

/// `CREDITS_eidos_usa`.
const EIDOS_USA: &[&str] = &[
    "~BAssociate Producer",
    "\tEric Adams",
    "",
    "~BQA Manager",
    "\tMike McHale",
    "",
    "~BProduct Manager",
    "\tJennifer Fitzsimmons",
    "",
    "~BMarketing Manager",
    "\tSusan Boshkoff",
    "",
    "~BPR",
    "\tBrian Kemp",
    "\tGreg Rizzer",
    "",
    "~BTesting",
    "\tCorey Fong",
    "\tKjell Vistad",
    "\tRalph Ortiz",
    "\tJohn Arvay",
];

/// `CREDITS_eidos_france`, in French as the original has it.
const EIDOS_FRANCE: &[&str] = &[
    "~BChef de produit",
    "\tOlivier Salomon",
    "",
    "~BResponsable localisation",
    "\tStéphan Gonizzi",
    "",
    "~BResponsable RP",
    "\tPriscille Demoly",
    "",
    "~BTesteur de la VF",
    "\tGuillaume Mahouin",
    "",
    "~BTraduction",
    "\tAround the Word, Paris",
    "",
    "~BEnregistrement des voix françaises",
    "\tLe Lotus Rose, Paris",
];

/// `CREDITS_eidos_germany`, in German.
const EIDOS_GERMANY: &[&str] = &[
    "~BLeiter Produktentwicklung",
    "\tBeco Mulderij",
    "",
    "~BProdukt-Manager",
    "\tLars Wittkuhn",
    "",
    "~BLeiter Marketing",
    "\tKnut-Jochen Bergel",
    "",
    "~BPR-Manager",
    "\tSascha Denise Green-Kaiser",
    "",
    "~BLokalisierungs-Manager",
    "\tHeidi Maria Kohne",
    "",
    "~BQA-Manager",
    // Not indented in the original, the only name that isn't.
    "\tSören Winterfeldt",
    "",
    "~BÜbersetzung",
    "\tViolet Media, Isabel Sterner",
    "",
    "~BAudio",
    "\tHastings International Audio Network",
    "",
    "~BTonmeister",
    "\tCedric Hopf",
];

/// `CREDITS_voice_production`.
const VOICE_PRODUCTION: &[&str] = &[
    "~BCasting",
    "\tPhil Morris at AllintheGame",
    "",
    "~BVoice Production",
    "\tBarry Meade",
    "\tMartin Oliver",
    "\tChris O'Saughanessy",
    "\tPhil Morris",
    "",
    "~BVoice Actors",
    "\tJohnnie Fiori",
    "\tDan Russell",
    "\tSharon Holm",
    "\tKerry Shale",
    "\tJulienne Davis",
    "\tColin McFarlane",
    "\tTed Maynard",
    "\tBrad Lavelle",
    "\tTogo Igawa",
    "",
    "~BTranslation",
    "\tAround the Word, Paris",
    "",
    "~BFrench Voices",
    "\tLe Lotus Rose, Paris",
];

/// `CREDITS_bands`.
const BANDS: &[&str] = &[
    "Way Out West - Urban Chaos",
    "The 3 Jays - Feeling it too",
    "Tour De Force",
    "Asian Dub Foundation and Audioactive - Psycho Buds",
    "Infidels - Everything",
    "Infidels - Ooma Gabba",
    // One line in the original, too long for a narrow screen here.
    "Special Forces 'Something Else... The bleep tune'",
    "	courtesy of Photek Productions",
    "",
    "Many Thanks to Miles Jacobson at Anglo.",
];

/// This project's.
const RUSTCHAOS: &[&str] = &[
    "A rewrite of Urban Chaos in Rust,",
    "played with your own copy of the game.",
    "",
    "~BWritten by",
    "\tVito Minheere",
    "",
    "~BWith thanks to",
    "\tdizzy2003, for releasing the original's source code",
    "",
    "Not affiliated with or endorsed by the game's rights holders.",
];

/// Where the credits are: the section on show and its first line's height, as a fraction of
/// the screen from the top (`CREDITS_current_section`, `CREDITS_current_y`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Credits {
    pub section: usize,
    pub y: f32,
}

impl Default for Credits {
    /// The first section, starting at the bottom of the screen (`CREDITS_init`).
    fn default() -> Credits {
        Credits { section: 0, y: 1.0 }
    }
}

/// Screen heights a line takes: 0.05 times its size (credits.cpp:625), headings 1.0 and the
/// rest 0.6.
fn step(line: &str) -> f32 {
    0.05 * if line.starts_with("~B") { 1.0 } else { 0.6 }
}

impl Credits {
    /// Scrolls up for `dt` seconds (`CREDITS_draw`: 0.00005 of the screen a millisecond),
    /// much faster with `fast` (the original's S key), and on to the next section once this
    /// one is off the top, round to the first after the last.
    pub fn advance(&mut self, dt: f32, fast: bool) {
        // Never more than half a second behind.
        let dt = dt.min(0.5);
        self.y -= dt * 0.05 * if fast { 20.0 } else { 1.0 };
        if self.end() < 0.10 {
            self.next(1);
        }
    }

    /// Section `by` on from this one (back with -1), from the bottom of the screen.
    pub fn next(&mut self, by: isize) {
        self.section = (self.section as isize + by).rem_euclid(SECTIONS.len() as isize) as usize;
        self.y = 1.0;
    }

    /// The height just below this section's last line.
    fn end(&self) -> f32 {
        self.y + SECTIONS[self.section].1.iter().map(|l| step(l)).sum::<f32>()
    }

    /// Draws the section: its title top left, faded in as it starts and out as it ends, and
    /// its lines faded in from the bottom and out at the top.
    pub fn draw(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        let (white, gold, dim) = ([235, 235, 225], [255, 210, 70], [190, 190, 180]);
        let alpha = |c: [u8; 3], a: f32| [c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8];
        let (title, lines) = SECTIONS[self.section];
        let end = self.end();
        let title_in = if self.y > 0.6 {
            (1.0 - self.y) / 0.4
        } else if end < 0.5 {
            (end - 0.1) / 0.4
        } else {
            1.0
        };
        o.text(0.03 * w, 0.05 * h, s * 3.0, alpha(gold, title_in), title);
        o.text(0.03 * w, h - 16.0 * s, s, [200, 200, 190, 200], "Down faster   Left/Right section   Esc back");
        let mut y = self.y;
        for &line in lines {
            let (mut text, mut size, mut col) = (line, s, white);
            if let Some(t) = text.strip_prefix("~B") {
                (text, size, col) = (t, s * 2.0, gold);
            } else if let Some(t) = text.strip_prefix("~I") {
                (text, col) = (t, dim);
            }
            let tabs = text.len() - text.trim_start_matches('\t').len();
            let x = (0.06 + 0.03 * tabs as f32) * w;
            let shown = match y {
                y if y < 0.10 => 0.0,
                y if y < 0.30 => (y - 0.10) / 0.2,
                y if y < 0.75 => 1.0,
                y if y < 0.95 => 1.0 - (y - 0.75) / 0.2,
                _ => 0.0,
            };
            if !text.is_empty() && shown > 0.0 {
                o.text(x, y * h, size, alpha(col, shown), text.trim_start_matches('\t'));
            }
            y += step(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_credits_scroll_through_every_section_and_round_again() {
        let mut c = Credits::default();
        let mut seen = vec![0];
        for _ in 0..20 * 60 * 30 {
            c.advance(0.05, true);
            if seen.last() != Some(&c.section) {
                seen.push(c.section);
            }
        }
        assert!(seen.len() > SECTIONS.len());
        assert_eq!(seen[..=SECTIONS.len()], [0, 1, 2, 3, 4, 5, 6, 7, 0]);
    }

    #[test]
    fn a_section_moves_on_once_its_last_line_is_off_the_top() {
        let mut c = Credits::default();
        while c.section == 0 {
            c.advance(0.05, false);
        }
        assert_eq!(c, Credits { section: 1, y: 1.0 });
        c.next(-1);
        c.next(-1);
        assert_eq!(c.section, SECTIONS.len() - 1);
    }
}
