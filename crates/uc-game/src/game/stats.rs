//! A record of how the player plays (#132): arrests, takedowns, knockouts, shots, how far
//! she's gone and how, ladders, zip wires, falls and roofs. Counted as the game applies what
//! happens, a turn at a time; the viewer adds each mission's to the totals it keeps.

use crate::*;

/// What the player did, in one mission or run (or, added up, ever). Distances in squares.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub arrests: u32,
    /// Takedowns from behind (the pistol whip).
    pub takedowns: u32,
    /// People searched.
    pub searched: u32,
    pub red_marks: u32,
    /// Knockouts by fists and feet, by knife or bat, and by gun.
    pub ko_fists: u32,
    pub ko_weapons: u32,
    pub ko_guns: u32,
    pub shots: u32,
    pub hits: u32,
    pub on_foot: f32,
    pub driven: f32,
    pub biked: f32,
    pub zipped: f32,
    /// The longest zip-wire ride.
    pub longest_zip: f32,
    pub ladders: u32,
    /// Falls of two storeys or more she got up from.
    pub falls: u32,
    /// Roofs reached by pulling herself up.
    pub roofs: u32,
    /// Missions won without firing a shot or a red mark, with an arrest (the viewer counts
    /// these as it records a win).
    pub by_the_book: u32,
    /// Missions won with every badge and side objective in them done (counted by the
    /// viewer too).
    pub perfect: u32,
}

impl Game {
    /// The win screen's criminals (`ScoresDraw`, Attract.cpp:684): the gangs and the Men in
    /// Black the player killed (`stat_killed_thug`, Person.cpp:2330) and arrested
    /// (`stat_arrested_thug`, :9310), and those still standing. None unless she's Darci or
    /// Roper ("This only works for Darci or Roper!").
    pub fn criminals(&self) -> Option<(u32, u32, u32)> {
        use uc_world::character::person as ptype;
        let me = self.player?;
        if !matches!(self.people[me].kind, ptype::DARCI | ptype::ROPER) {
            return None;
        }
        let criminal = |p: &person::Person| p.thug() || p.is_mib();
        let mut counts = (0, 0, 0);
        for p in self.people.iter().filter(|p| !p.player && criminal(p)) {
            if p.arrested || matches!(p.state, State::Cuffed) {
                counts.1 += 1;
            } else if p.knocked_out && p.hurt_by == Some(me) {
                counts.0 += 1;
            } else if !p.removed && !p.is_dead() {
                counts.2 += 1;
            }
        }
        Some(counts)
    }
}

impl Stats {
    /// Each count by name, for saving.
    pub fn fields(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("arrests", self.arrests as f64),
            ("takedowns", self.takedowns as f64),
            ("searched", self.searched as f64),
            ("red_marks", self.red_marks as f64),
            ("ko_fists", self.ko_fists as f64),
            ("ko_weapons", self.ko_weapons as f64),
            ("ko_guns", self.ko_guns as f64),
            ("shots", self.shots as f64),
            ("hits", self.hits as f64),
            ("on_foot", self.on_foot as f64),
            ("driven", self.driven as f64),
            ("biked", self.biked as f64),
            ("zipped", self.zipped as f64),
            ("longest_zip", self.longest_zip as f64),
            ("ladders", self.ladders as f64),
            ("falls", self.falls as f64),
            ("roofs", self.roofs as f64),
            ("by_the_book", self.by_the_book as f64),
            ("perfect", self.perfect as f64),
        ]
    }

    /// Sets the count called `name` (as [`Stats::fields`] names it); unknown names are
    /// ignored.
    pub fn set(&mut self, name: &str, v: f64) {
        let n = v.max(0.0) as u32;
        let f = v.max(0.0) as f32;
        match name {
            "arrests" => self.arrests = n,
            "takedowns" => self.takedowns = n,
            "searched" => self.searched = n,
            "red_marks" => self.red_marks = n,
            "ko_fists" => self.ko_fists = n,
            "ko_weapons" => self.ko_weapons = n,
            "ko_guns" => self.ko_guns = n,
            "shots" => self.shots = n,
            "hits" => self.hits = n,
            "on_foot" => self.on_foot = f,
            "driven" => self.driven = f,
            "biked" => self.biked = f,
            "zipped" => self.zipped = f,
            "longest_zip" => self.longest_zip = f,
            "ladders" => self.ladders = n,
            "falls" => self.falls = n,
            "roofs" => self.roofs = n,
            "by_the_book" => self.by_the_book = n,
            "perfect" => self.perfect = n,
            _ => {}
        }
    }

    /// These and `other` together: counts added, the longest ride the longer.
    pub fn plus(&self, other: &Stats) -> Stats {
        let mut out = Stats::default();
        let theirs = other.fields();
        for ((name, a), (_, b)) in self.fields().into_iter().zip(theirs) {
            out.set(name, if name == "longest_zip" { a.max(b) } else { a + b });
        }
        out
    }

    /// Knockouts of all kinds.
    pub fn knockouts(&self) -> u32 {
        self.ko_fists + self.ko_weapons + self.ko_guns
    }
}

/// What the stats need to remember from one turn to the next about the player.
#[derive(Debug, Clone, Default)]
pub(crate) struct Track {
    pos: Option<Vec3>,
    /// Where a fall started, while she's falling.
    fall_from: Option<f32>,
    /// How far along a zip wire she's ridden so far, while on one.
    ridden: f32,
    climbing: bool,
    pulling_up: bool,
}

impl Game {
    /// A blow or a shot put `victim` down: the player's knockout, if it was hers.
    pub(crate) fn count_knockout(&mut self, attacker: usize, weapon: Weapon) {
        if Some(attacker) != self.player {
            return;
        }
        match weapon {
            Weapon::None => self.record.ko_fists += 1,
            w if w.is_gun() => self.record.ko_guns += 1,
            _ => self.record.ko_weapons += 1,
        }
    }

    /// Every turn: how far the player went and how, and what she climbed, rode and fell.
    pub(crate) fn run_stats(&mut self, level: &Level) {
        self.record.red_marks = self.red_marks as u32;
        let Some(p) = self.player() else { return };
        let (pos, state) = (p.fig.pos, p.state.clone());
        let alive = !p.is_dead() && p.health > 0;
        let t = &mut self.stats_track;
        // Distance, by how she went. A jump of more than a few squares in a turn is a
        // teleport, not travel.
        if let Some(last) = t.pos {
            let d = Vec2::new(pos.x - last.x, pos.z - last.z).length();
            if d < 3.0 * 256.0 {
                let squares = d / 256.0;
                match state {
                    State::InCar { .. } => self.record.driven += squares,
                    State::Bike { .. } => self.record.biked += squares,
                    State::Zip { .. } | State::Climb { .. } | State::Mount { .. } | State::OntoLadder { .. } => {}
                    _ => self.record.on_foot += squares,
                }
            }
        }
        t.pos = Some(pos);
        // Zip wires: the ride counts when she comes off.
        match state {
            State::Zip { ridden, .. } => t.ridden = ridden,
            _ if t.ridden > 0.0 => {
                let squares = t.ridden / 256.0;
                self.record.zipped += squares;
                self.record.longest_zip = self.record.longest_zip.max(squares);
                t.ridden = 0.0;
            }
            _ => {}
        }
        // Ladders: each time she gets on one.
        let climbing = matches!(state, State::Climb { .. });
        if climbing && !t.climbing {
            self.record.ladders += 1;
        }
        t.climbing = climbing;
        // Falls: two storeys or more, landed on her feet or got up from.
        match state {
            State::Fall { .. } => {
                t.fall_from = Some(t.fall_from.map_or(pos.y, |y| y.max(pos.y)));
            }
            State::Land { .. } | State::Down | State::GetUp | State::Move => {
                if let Some(from) = t.fall_from.take() {
                    if from - pos.y >= 2.0 * 256.0 && alive {
                        self.record.falls += 1;
                    }
                }
            }
            _ => {}
        }
        // Roofs: pulled up onto somewhere a storey and a half over the street.
        let pulling_up = matches!(state, State::PullUp { .. });
        if t.pulling_up && !pulling_up {
            let street = level.map.ground_height(pos.x, pos.z);
            if pos.y - street > 1.5 * 256.0 {
                self.record.roofs += 1;
            }
        }
        t.pulling_up = pulling_up;
    }
}
