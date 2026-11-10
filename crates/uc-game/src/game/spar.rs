//! The sparring room (#547): the player and one opponent on a map with no script, to try
//! the moves of [`crate::moves`] on. Not in the original, whose only practice is the combat
//! tutorial.
//!
//! A second man can be brought in ([`Game::spar_two`]) for the kicks to the side and behind,
//! which need someone there: with one opponent she always turns to face him.
//!
//! The opponent does as he's told ([`Stance`]): stands still, blocks, or fights back with
//! his fists, a knife or a bat. Knocked out or cuffed, he's back on his feet a few seconds
//! later. The player's health and ammunition don't run out, and neither loses a weapon by
//! being knocked down. Each blow that lands is named
//! ([`Landed`]) with what it took off him, blows in a row are counted, and each move of the
//! list is ticked off the first time it's done ([`Spar::done`]); one can be asked for
//! ([`Game::spar_want`]), and the room says when it's been done.

use crate::moves::Tech;
use crate::person::{Grab, Move};
use crate::*;
use uc_world::character::person as ptype;

/// How far in front of the player the opponent starts (world units).
const APART: f32 = 2.0 * 256.0;
/// How far from her the second man keeps (world units): the side and back kicks reach 160.
const BESIDE: f32 = 125.0;
/// Seconds the opponent lies knocked out, or cuffed, before he's up again: long enough to
/// stamp on him or cuff him.
const OUT: f32 = 6.0;
const CUFFED: f32 = 2.5;
/// Blows this close together (seconds) are a combo.
const COMBO: f32 = 1.6;
/// How many landed blows are kept to show.
const KEPT: usize = 5;

/// What the opponent does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stance {
    Still,
    Blocks,
    Fights,
    Knife,
    Bat,
}

impl Stance {
    pub const ALL: [Stance; 5] = [Stance::Still, Stance::Blocks, Stance::Fights, Stance::Knife, Stance::Bat];

    pub fn name(self) -> &'static str {
        match self {
            Stance::Still => "STANDS STILL",
            Stance::Blocks => "BLOCKS",
            Stance::Fights => "FIGHTS BACK",
            Stance::Knife => "HAS A KNIFE",
            Stance::Bat => "HAS A BAT",
        }
    }

    fn weapon(self) -> Weapon {
        match self {
            Stance::Knife => Weapon::Knife,
            Stance::Bat => Weapon::Bat,
            _ => Weapon::None,
        }
    }

    fn fights(self) -> bool {
        matches!(self, Stance::Fights | Stance::Knife | Stance::Bat)
    }
}

/// A blow of the player's that landed, or a move done.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landed {
    pub name: &'static str,
    /// What it took off the opponent's health.
    pub damage: i32,
    /// When ([`Game::time`]).
    pub at: f32,
}

#[derive(Debug, Clone)]
pub struct Spar {
    pub opponent: usize,
    /// A second man, for the kicks to the side and behind: with one, she's always turned to
    /// face him and there's nobody there for them to land on. Out of the room until asked
    /// for ([`Game::spar_two`]), or a move that needs him is ([`Game::spar_want`]).
    pub second: Option<usize>,
    pub two: bool,
    pub stance: Stance,
    /// The last few blows landed, the newest last.
    pub landed: Vec<Landed>,
    /// Blows landed in a row, and the most ever.
    pub combo: u32,
    pub best_combo: u32,
    /// The moves done so far.
    pub done: BTreeSet<Tech>,
    /// The move asked for, until it's done.
    pub want: Option<Tech>,
    /// The move last ticked off, and when: for the screen to say so.
    pub ticked: Option<(Tech, f32)>,
    /// Where she started, and each of them.
    start: Vec3,
    homes: [Vec3; 2],
    /// Seconds each of them has been out or cuffed.
    out: [f32; 2],
    /// What the player was doing last turn, to see a move start.
    was: Doing,
    /// Whether each of them was cuffed last turn.
    was_cuffed: [bool; 2],
}

impl Spar {
    /// The men in the room now: the opponent, and the second if he's in.
    pub fn men(&self) -> impl Iterator<Item = usize> {
        [Some(self.opponent), self.second.filter(|_| self.two)].into_iter().flatten()
    }

    /// Whether `tech` is a kick for someone at her side or behind her, which takes two men.
    pub fn takes_two(tech: Tech) -> bool {
        matches!(tech, Tech::SideKick | Tech::BackKick)
    }
}

/// What of the player's state the room watches for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Doing {
    Other,
    Flip,
    Grapple(Grab),
}

impl Doing {
    fn of(p: &Person) -> Doing {
        match p.state {
            State::Flip { .. } => Doing::Flip,
            State::Grapple { grab, .. } => Doing::Grapple(grab),
            _ => Doing::Other,
        }
    }
}

/// What a blow of `mv` is called, and the move of the list it completes.
fn blow(mv: Move) -> (&'static str, Option<Tech>) {
    match mv {
        Move::Punch1 => ("PUNCH", None),
        Move::Punch2 => ("SECOND PUNCH", None),
        Move::Punch3 => ("PUNCH COMBO", Some(Tech::PunchCombo)),
        Move::Kick1 => ("KICK", None),
        Move::Kick2 => ("SECOND KICK", None),
        Move::Kick3 => ("KICK COMBO", Some(Tech::KickCombo)),
        Move::KickBack => ("BACK KICK", Some(Tech::BackKick)),
        Move::KickLeft | Move::KickRight => ("SIDE KICK", Some(Tech::SideKick)),
        Move::KickNad => ("GROIN KICK", Some(Tech::GroinKick)),
        Move::KickJump => ("JUMP KICK", Some(Tech::JumpKick)),
        Move::KickNear => ("CLOSE KICK", None),
        Move::LegSweep => ("FOOT SWEEP", Some(Tech::FootSweep)),
        Move::Stomp => ("STOMP", Some(Tech::Stomp)),
        Move::Knife1 => ("SLASH", None),
        Move::Knife2 => ("SECOND SLASH", None),
        Move::Knife3 => ("KNIFE COMBO", Some(Tech::KnifeCombo)),
        Move::Bat1 => ("BAT SWING", None),
        Move::Bat2 => ("BAT COMBO", Some(Tech::BatCombo)),
        Move::Whip1 | Move::Whip2 => ("GUN CLUB", Some(Tech::GunClub)),
    }
}

impl Game {
    /// The sparring room on `level`: its map with no script and nobody on it but the player,
    /// carrying a knife, a bat, a pistol and a shotgun (bare hands to start with), and one
    /// thug a couple of squares in front, standing still.
    pub fn new_spar(level: &Level) -> Game {
        let mut g = Game::build(level, false, false);
        g.items.clear();
        let Some(pi) = g.player else { return g };
        let (start, angle) = (g.people[pi].fig.pos, g.people[pi].fig.angle);
        let p = &mut g.people[pi];
        for w in [Weapon::Knife, Weapon::Bat, Weapon::Pistol, Weapon::Shotgun] {
            p.give(w);
        }
        p.weapon = Weapon::None;
        // In front of her if there's room, else whichever way there is.
        let eye = Vec3::new(0.0, 100.0, 0.0);
        let free: Vec<Vec3> = (0..8)
            .map(|k| {
                let d = uc_world::character::facing(angle + k as f32 * std::f32::consts::FRAC_PI_4);
                start + Vec3::new(d.x, 0.0, d.y) * APART
            })
            .filter(|&at| {
                let y = level.ground.surface(&level.map, at.x, at.z, start.y + 64.0);
                (y - start.y).abs() < 32.0 && level.ground.line_of_sight(start + eye, at + eye)
            })
            .collect();
        let home = free.first().copied().unwrap_or(start + Vec3::new(APART, 0.0, 0.0));
        // The second man: as far round her from the first as there's room.
        let other = free.iter().copied().max_by(|a, b| a.distance(home).total_cmp(&b.distance(home))).filter(|&at| at.distance(home) > APART);
        let make = |g: &mut Game, at: Vec3| {
            let back = uc_world::character::angle_of(flat(start, at));
            let i = g.spawn(level, ptype::THUG_RASTA, false, 0, at, back)?;
            let o = &mut g.people[i];
            o.ai = person::AI_GANG;
            (o.mv, o.follow) = (5, 0);
            o.skill = 6;
            Some(i)
        };
        let Some(i) = make(&mut g, home) else { return g };
        let second = other.and_then(|at| make(&mut g, at));
        if let Some(k) = second {
            g.people[k].removed = true;
        }
        let homes = [g.people[i].fig.pos, second.map_or(home, |k| g.people[k].fig.pos)];
        g.spar = Some(Spar {
            opponent: i, second, two: false, stance: Stance::Still, landed: Vec::new(), combo: 0, best_combo: 0, done: BTreeSet::new(),
            want: None, ticked: None, start, homes, out: [0.0; 2], was: Doing::Other, was_cuffed: [false; 2],
        });
        g
    }

    /// The sparring room, if this is it.
    pub fn spar(&self) -> Option<&Spar> {
        self.spar.as_ref()
    }

    /// What the opponent does from now on.
    pub fn spar_stance(&mut self, stance: Stance) {
        let Some(s) = self.spar.as_mut() else { return };
        s.stance = stance;
        for i in s.men().collect::<Vec<_>>() {
            let o = &mut self.people[i];
            (o.knife, o.bat) = (stance == Stance::Knife, stance == Stance::Bat);
            o.weapon = stance.weapon();
            if !stance.fights() {
                o.calm_down();
            }
        }
    }

    /// The second man in the room, or out of it. He comes in where he started (or, with her
    /// standing there, where she did), doing as the first does; told to stand still or block,
    /// he does it at her back ([`Game::spar_puppet`]).
    pub fn spar_two(&mut self, on: bool) {
        let Some(s) = self.spar.as_mut() else { return };
        let Some(k) = s.second.filter(|_| s.two != on) else { return };
        s.two = on;
        (s.out[1], s.was_cuffed[1]) = (0.0, false);
        let (stance, home, start) = (s.stance, s.homes[1], s.start);
        let her = self.player.map_or(start, |p| self.people[p].fig.pos);
        let o = &mut self.people[k];
        if on {
            let at = if flat(home, her).length() > 160.0 { home } else { start };
            o.respawn(at);
            o.stand_still();
            o.fig.angle = uc_world::character::angle_of(flat(her, at));
            (o.knife, o.bat) = (stance == Stance::Knife, stance == Stance::Bat);
            o.weapon = stance.weapon();
        } else {
            o.calm_down();
            o.stand_still();
            o.removed = true;
        }
    }

    /// The move the room waits for (none: whatever she likes).
    pub fn spar_want(&mut self, want: Option<Tech>) {
        let Some(s) = self.spar.as_mut() else { return };
        s.want = want.filter(|&t| crate::moves::entry(t).sparred);
        // A kick for someone at her side or behind her: the second man comes in for it.
        if s.want.is_some_and(Spar::takes_two) {
            self.spar_two(true);
        }
    }

    /// What the opponent does this turn when it isn't up to him: nothing, or his guard up.
    /// None when he fights as anyone would, or this isn't him.
    pub(crate) fn spar_puppet(&self, i: usize) -> Option<Controls> {
        let s = self.spar.as_ref().filter(|s| s.men().any(|m| m == i))?;
        if s.stance.fights() {
            return None;
        }
        let stand = if s.stance == Stance::Blocks { Controls { block: true, ..Controls::default() } } else { Controls::default() };
        // The second man keeps where the kicks he's there for go: behind her as the first
        // sees her (at her side while a side kick's wanted), a step inside their reach.
        let her = self.player.map(|p| self.people[p].fig.pos).filter(|_| Some(i) == s.second);
        let Some(her) = her else { return Some(stand) };
        let away = flat(her, self.people[s.opponent].fig.pos).normalize_or_zero();
        let way = if s.want == Some(Tech::SideKick) { Vec2::new(-away.y, away.x) } else { away };
        let spot = her + Vec3::new(way.x, 0.0, way.y) * BESIDE;
        let to = flat(spot, self.people[i].fig.pos);
        if to.length() < 40.0 {
            return Some(stand);
        }
        Some(Controls { wish: to.normalize(), run: to.length() > 400.0, ..Controls::default() })
    }

    /// A blow of `attacker`'s landed on `victim`, taking `damage` off them.
    pub(crate) fn spar_hit(&mut self, attacker: usize, victim: usize, damage: i32) {
        let Some(s) = self.spar.as_ref() else { return };
        // One on the player ends her run of blows; one she stopped is a block.
        if Some(victim) == self.player {
            let blocked = damage <= 0 && matches!(self.people[victim].state, State::Block { .. });
            if blocked {
                self.spar_did("BLOCK", Some(Tech::Block), 0, false);
            } else if let Some(s) = self.spar.as_mut() {
                s.combo = 0;
            }
            return;
        }
        if Some(attacker) != self.player || !s.men().any(|m| m == victim) || damage <= 0 {
            return;
        }
        let (name, tech) = match self.people[attacker].state {
            State::Attack { mv, .. } => blow(mv),
            State::Slide { .. } => ("SLIDE TACKLE", Some(Tech::SlideTackle)),
            State::Grapple { grab: Grab::Throw, .. } => ("THROW", Some(Tech::Throw)),
            State::Grapple { .. } => ("KNEE", Some(Tech::Knee)),
            State::Shoot { .. } => ("SHOT", Some(Tech::Shoot)),
            // A shot on the run.
            _ if self.people[attacker].weapon.is_gun() => ("SHOT", Some(Tech::Shoot)),
            _ => return,
        };
        self.spar_did(name, tech, damage, true);
    }

    /// Something done: shown, counted in the run of blows if it's one, and its move ticked.
    fn spar_did(&mut self, name: &'static str, tech: Option<Tech>, damage: i32, blow: bool) {
        let now = self.time;
        let Some(s) = self.spar.as_mut() else { return };
        if blow {
            let running = s.landed.last().is_some_and(|l| now - l.at < COMBO) && s.combo > 0;
            s.combo = if running { s.combo + 1 } else { 1 };
            s.best_combo = s.best_combo.max(s.combo);
        }
        s.landed.push(Landed { name, damage, at: now });
        if s.landed.len() > KEPT {
            s.landed.remove(0);
        }
        if let Some(t) = tech {
            self.spar_tick(t);
        }
    }

    /// Move `t` done: ticked, and no longer waited for.
    fn spar_tick(&mut self, t: Tech) {
        let now = self.time;
        let Some(s) = self.spar.as_mut() else { return };
        let new = s.done.insert(t);
        if new || s.want == Some(t) {
            s.ticked = Some((t, now));
        }
        if s.want == Some(t) {
            s.want = None;
        }
    }

    /// The room's turn, after everyone's: the moves that aren't blows seen as they start,
    /// both of them kept whole, and the opponent set on her or called off.
    pub(crate) fn run_spar(&mut self, dt: f32, level: &Level) {
        let Some(s) = self.spar.as_ref() else { return };
        let (stance, start, homes, was, was_cuffed) = (s.stance, s.start, s.homes, s.was, s.was_cuffed);
        let Some(pi) = self.player else { return };
        // Grapples and flips, as she goes into them.
        let doing = Doing::of(&self.people[pi]);
        if doing != was {
            match doing {
                Doing::Flip => self.spar_did("SIDE FLIP", Some(Tech::SideFlip), 0, false),
                Doing::Grapple(Grab::Grab) => self.spar_did("ARM GRAB", Some(Tech::ArmGrab), 0, false),
                Doing::Grapple(Grab::Knee1) => self.spar_did("KNEE", Some(Tech::Knee), 0, false),
                Doing::Grapple(Grab::PistolWhip) => self.spar_did("TAKEDOWN", Some(Tech::Takedown), 0, false),
                _ => {}
            }
        }
        let men: Vec<usize> = self.spar.as_ref().map_or(Vec::new(), |s| s.men().collect());
        for (k, &o) in men.iter().enumerate() {
            if self.people[o].arrested && !was_cuffed[k] {
                self.spar_did("ARREST", Some(Tech::Arrest), 0, false);
            }
        }
        // She doesn't run out of health or rounds, and gets up from anything.
        let p = &mut self.people[pi];
        if p.knocked_out {
            let y = level.ground.surface(&level.map, start.x, start.z, start.y + 64.0);
            p.respawn(Vec3::new(start.x, y, start.z));
            p.stand_still();
        }
        p.health = p.max_health;
        for (slot, &w) in Weapon::GUNS.iter().enumerate() {
            if p.guns & (1 << slot) != 0 {
                p.ammo[slot] = p.ammo[slot].max(w.clip());
            }
        }
        // What she's knocked down with in her hand she'd let go of: it's hers again (to get
        // out again), and nothing's left lying about.
        for w in [Weapon::Knife, Weapon::Bat, Weapon::Pistol, Weapon::Shotgun] {
            if !p.has(w) {
                p.give(w);
            }
        }
        let player_at = p.fig.pos;
        // Toe to toe with one of them she's in a fight, gun or no gun, as she'd be in a
        // mission once he'd landed one on her: so a two-handed gun clubs him.
        let toe_to_toe = men.iter().any(|&o| self.people[o].on_feet() && flat(self.people[o].fig.pos, player_at).length() < 150.0);
        if toe_to_toe {
            self.people[pi].hit_in_a_fight();
        }
        self.items.clear();
        let (mut outs, mut cuffs) = (self.spar.as_ref().map_or([0.0; 2], |s| s.out), was_cuffed);
        for (k, &o) in men.iter().enumerate() {
            // He's back up a while after going out, or being cuffed: where he lies, or back
            // where he started if she's standing on him.
            // (Counted afresh from the cuffs going on: being cuffed takes a while itself.)
            let op = &self.people[o];
            let cuffed = op.arrested;
            let gone = op.knocked_out || op.out_cold || op.arrested || op.removed;
            let so_far = if cuffed && !was_cuffed[k] { 0.0 } else { outs[k] };
            let mut out = if gone { so_far + dt } else { 0.0 };
            if out > if cuffed { CUFFED } else { OUT } {
                out = 0.0;
                let lies = op.fig.pos;
                let at = if flat(lies, player_at).length() > 160.0 { lies } else if flat(homes[k], player_at).length() > 160.0 { homes[k] } else { start };
                let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
                let op = &mut self.people[o];
                op.respawn(Vec3::new(at.x, y, at.z));
                op.stand_still();
                op.fig.angle = uc_world::character::angle_of(flat(player_at, at));
            }
            // He drops his knife or his bat going down, too.
            let op = &mut self.people[o];
            if op.on_feet() && op.weapon != stance.weapon() {
                (op.knife, op.bat) = (stance == Stance::Knife, stance == Stance::Bat);
                op.weapon = stance.weapon();
            }
            if stance.fights() && op.on_feet() {
                op.go_for();
            } else if !stance.fights() {
                op.calm_down();
            }
            (outs[k], cuffs[k]) = (out, op.arrested);
        }
        if let Some(s) = self.spar.as_mut() {
            (s.out, s.was, s.was_cuffed) = (outs, doing, cuffs);
            // A run of blows is over once nothing's landed for a while.
            if s.landed.last().is_none_or(|l| self.time - l.at > COMBO) {
                s.combo = 0;
            }
        }
    }
}
