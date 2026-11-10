//! The sparring room (#547): the player and one opponent on a map with no script, to try
//! the moves of [`crate::moves`] on. Not in the original, whose only practice is the combat
//! tutorial.
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
    /// Where the two of them started.
    start: Vec3,
    home: Vec3,
    /// Seconds the opponent's been out or cuffed.
    out: f32,
    /// What the player was doing last turn, to see a move start.
    was: Doing,
    /// Whether the opponent was cuffed last turn.
    was_cuffed: bool,
}

/// What of the player's state the room watches for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Doing {
    Other,
    Flip,
    Grapple(Grab),
    Attack(Move),
}

impl Doing {
    fn of(p: &Person) -> Doing {
        match p.state {
            State::Flip { .. } => Doing::Flip,
            State::Grapple { grab, .. } => Doing::Grapple(grab),
            State::Attack { mv, .. } => Doing::Attack(mv),
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
        let home = (0..8)
            .map(|k| {
                let d = uc_world::character::facing(angle + k as f32 * std::f32::consts::FRAC_PI_4);
                start + Vec3::new(d.x, 0.0, d.y) * APART
            })
            .find(|&at| {
                let y = level.ground.surface(&level.map, at.x, at.z, start.y + 64.0);
                (y - start.y).abs() < 32.0 && level.ground.line_of_sight(start + eye, at + eye)
            })
            .unwrap_or(start + Vec3::new(APART, 0.0, 0.0));
        let back = uc_world::character::angle_of(flat(start, home));
        let Some(i) = g.spawn(level, ptype::THUG_RASTA, false, 0, home, back) else { return g };
        let o = &mut g.people[i];
        o.ai = person::AI_GANG;
        (o.mv, o.follow) = (5, 0);
        o.skill = 6;
        let home = o.fig.pos;
        g.spar = Some(Spar {
            opponent: i, stance: Stance::Still, landed: Vec::new(), combo: 0, best_combo: 0, done: BTreeSet::new(), want: None,
            ticked: None, start, home, out: 0.0, was: Doing::Other, was_cuffed: false,
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
        let o = &mut self.people[s.opponent];
        (o.knife, o.bat) = (stance == Stance::Knife, stance == Stance::Bat);
        o.weapon = stance.weapon();
        if !stance.fights() {
            o.calm_down();
        }
    }

    /// The move the room waits for (none: whatever she likes).
    pub fn spar_want(&mut self, want: Option<Tech>) {
        if let Some(s) = self.spar.as_mut() {
            s.want = want.filter(|&t| crate::moves::entry(t).sparred);
        }
    }

    /// What the opponent does this turn when it isn't up to him: nothing, or his guard up.
    /// None when he fights as anyone would, or this isn't him.
    pub(crate) fn spar_puppet(&self, i: usize) -> Option<Controls> {
        let s = self.spar.as_ref().filter(|s| s.opponent == i)?;
        match s.stance {
            Stance::Still => Some(Controls::default()),
            Stance::Blocks => Some(Controls { block: true, ..Controls::default() }),
            _ => None,
        }
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
        if Some(attacker) != self.player || victim != s.opponent || damage <= 0 {
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
        let (o, stance, start, home, was, was_cuffed) = (s.opponent, s.stance, s.start, s.home, s.was, s.was_cuffed);
        let Some(pi) = self.player else { return };
        // Grapples and flips, as she goes into them.
        let doing = Doing::of(&self.people[pi]);
        if doing != was {
            match doing {
                Doing::Flip => self.spar_did("SIDE FLIP", Some(Tech::SideFlip), 0, false),
                Doing::Grapple(Grab::Grab) => self.spar_did("ARM GRAB", Some(Tech::ArmGrab), 0, false),
                Doing::Grapple(Grab::Knee1) => self.spar_did("KNEE", Some(Tech::Knee), 0, false),
                Doing::Grapple(Grab::PistolWhip) => self.spar_did("TAKEDOWN", Some(Tech::Takedown), 0, false),
                // The kicks to the side and behind are for someone there, and with one
                // opponent she's always turned to face him: doing one is enough.
                Doing::Attack(Move::KickLeft | Move::KickRight) => self.spar_tick(Tech::SideKick),
                Doing::Attack(Move::KickBack) => self.spar_tick(Tech::BackKick),
                _ => {}
            }
        }
        let cuffed = self.people[o].arrested;
        if cuffed && !was_cuffed {
            self.spar_did("ARREST", Some(Tech::Arrest), 0, false);
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
        // Toe to toe with him she's in a fight, gun or no gun, as she'd be in a mission once
        // he'd landed one on her: so a two-handed gun clubs him.
        let him = &self.people[o];
        let toe_to_toe = him.on_feet() && flat(him.fig.pos, player_at).length() < 150.0;
        if toe_to_toe {
            self.people[pi].hit_in_a_fight();
        }
        self.items.clear();
        // He's back up a while after going out, or being cuffed: where he lies, or back
        // where he started if she's standing on him.
        let op = &self.people[o];
        // (Counted afresh from the cuffs going on: being cuffed takes a while itself.)
        let gone = op.knocked_out || op.out_cold || op.arrested || op.removed;
        let so_far = if cuffed && !was_cuffed { 0.0 } else { self.spar.as_ref().map_or(0.0, |s| s.out) };
        let mut out = if gone { so_far + dt } else { 0.0 };
        if out > if cuffed { CUFFED } else { OUT } {
            out = 0.0;
            let lies = op.fig.pos;
            let at = if flat(lies, player_at).length() > 160.0 { lies } else if flat(home, player_at).length() > 160.0 { home } else { start };
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
        if let Some(s) = self.spar.as_mut() {
            (s.out, s.was, s.was_cuffed) = (out, doing, self.people[o].arrested);
            // A run of blows is over once nothing's landed for a while.
            if s.landed.last().is_none_or(|l| self.time - l.at > COMBO) {
                s.combo = 0;
            }
        }
    }
}
