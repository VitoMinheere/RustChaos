//! Co-op (#182): a second player in a story mission, Roper next to Darci (or Darci next to
//! Roper, in his own missions). Seat 0 is the mission's own player, in `Game`'s fields as in
//! single player; every seat after it is a [`Partner`] holding the same player-only state
//! of its own. [`Game::focus`] swaps a seat's state into `Game`'s fields, so everything
//! written for "the player" (targeting, the scope, grenades, the hook, searching, using
//! things, driving) works for whichever seat is in focus.
//!
//! The original was built for two players (`MAX_PLAYERS 2`, `NET_PERSON(i)` throughout,
//! see #114): its enemies go for the nearest player, and its script's "player" triggers
//! watch either.

use crate::*;

/// Seconds a partner stays down before getting up again next to seat 0.
const BACK_UP: f32 = 6.0;

/// A seat after the first: who they play, and what of the player's is theirs.
#[derive(Debug, Clone)]
pub(crate) struct Partner {
    player: Option<usize>,
    target: Option<usize>,
    aiming: Aiming,
    search: Option<Search>,
    snipe: Option<game::Snipe>,
    primed: Option<f32>,
    hook: Option<game::hook::Hook>,
    player_name: &'static str,
    gun_drawn: bool,
    /// Seconds down.
    down: f32,
}

impl Game {
    /// Adds a second player next to the first: Roper, or Darci where the mission's player is
    /// Roper. Call it straight after [`Game::new`], the same way on every machine (it rolls
    /// the game's dice for his looks). Returns his index in `people`.
    pub fn add_partner(&mut self, level: &Level) -> Option<usize> {
        self.focus(0);
        let me = self.player?;
        let p = &self.people[me];
        let kind = if p.kind == ptype::ROPER { ptype::DARCI } else { ptype::ROPER };
        let (pos, angle) = (p.fig.pos, p.fig.angle);
        let at = self.beside(level, me).unwrap_or(pos);
        let i = self.spawn(level, kind, true, 0, at, angle)?;
        self.partners.push(Partner {
            player: Some(i),
            target: None,
            aiming: Aiming::default(),
            search: None,
            snipe: None,
            primed: None,
            hook: None,
            player_name: ptype::name(kind),
            gun_drawn: false,
            down: 0.0,
        });
        Some(i)
    }

    /// The seats: 1, or more in co-op.
    pub fn seats(&self) -> usize {
        1 + self.partners.len()
    }

    /// The person seat `seat` plays, whichever seat is in focus.
    pub fn seat_person(&self, seat: usize) -> Option<usize> {
        if seat == self.focused {
            self.player
        } else if seat == 0 {
            self.partners[self.focused - 1].player
        } else {
            self.partners.get(seat - 1)?.player
        }
    }

    /// Everyone playing, seat 0 first.
    pub(crate) fn seat_people(&self) -> Vec<usize> {
        (0..self.seats()).filter_map(|s| self.seat_person(s)).collect()
    }

    /// The seat whose state is in `Game`'s fields: what [`Game::player`], [`Game::target`],
    /// [`Game::sniping`] and the rest answer for. A viewer playing seat 1 focuses it to draw
    /// the camera and the HUD; [`Game::step`] puts seat 0 back in focus while it plays.
    pub fn focus(&mut self, seat: usize) {
        if seat == self.focused || seat >= self.seats() {
            return;
        }
        if self.focused != 0 {
            self.swap_partner(self.focused - 1);
        }
        if seat != 0 {
            self.swap_partner(seat - 1);
        }
        self.focused = seat;
    }

    /// The seat in focus.
    pub fn focused(&self) -> usize {
        self.focused
    }

    fn swap_partner(&mut self, k: usize) {
        use std::mem::swap;
        let s = &mut self.partners[k];
        swap(&mut self.player, &mut s.player);
        swap(&mut self.target, &mut s.target);
        swap(&mut self.aiming, &mut s.aiming);
        swap(&mut self.search, &mut s.search);
        swap(&mut self.snipe, &mut s.snipe);
        swap(&mut self.primed, &mut s.primed);
        swap(&mut self.hook, &mut s.hook);
        swap(&mut self.player_name, &mut s.player_name);
        swap(&mut self.gun_drawn, &mut s.gun_drawn);
    }

    /// Somewhere to stand next to person `i`: a little to their side or behind, on the same
    /// floor and in sight of them.
    fn beside(&self, level: &Level, i: usize) -> Option<Vec3> {
        let p = &self.people[i];
        let (pos, f) = (p.fig.pos, p.facing_dir());
        let side = Vec2::new(-f.y, f.x);
        let eye = Vec3::new(0.0, 100.0, 0.0);
        [side, -side, -f, (side - f).normalize(), (-side - f).normalize()].into_iter().find_map(|d| {
            let at = pos + Vec3::new(d.x, 0.0, d.y) * 100.0;
            let y = level.ground.surface(&level.map, at.x, at.z, pos.y + 64.0);
            let at = Vec3::new(at.x, y, at.z);
            ((y - pos.y).abs() < 32.0 && level.ground.line_of_sight(pos + eye, at + eye)).then_some(at)
        })
    }

    /// Every turn: a partner who's down gets up again next to seat 0 a few seconds later,
    /// whole. Only seat 0 going down loses the mission, as in single player.
    pub(crate) fn partners_back_up(&mut self, dt: f32, level: &Level) {
        let Some(me) = self.player else { return };
        for k in 0..self.partners.len() {
            let Some(i) = self.partners[k].player else { continue };
            let p = &self.people[i];
            let down = p.knocked_out || p.arrested || (p.is_down() && p.health <= 0);
            if !down {
                self.partners[k].down = 0.0;
                continue;
            }
            self.partners[k].down += dt;
            if self.partners[k].down < BACK_UP || self.people[me].is_dead() {
                continue;
            }
            self.partners[k].down = 0.0;
            let at = self.beside(level, me).unwrap_or(self.people[me].fig.pos);
            self.people[i].respawn(at);
        }
    }

    /// A beacon on each other player, for the one in focus to find them by.
    pub(crate) fn coop_beacons(&self) -> Vec<(String, Vec3)> {
        (0..self.seats()).filter(|&s| s != self.focused).filter_map(|s| self.seat_person(s))
            .map(|i| (ptype::name(self.people[i].kind).to_string(), self.people[i].fig.pos))
            .collect()
    }

    /// The nearest playing person to `at` who's on their feet, for the AI to go for.
    pub(crate) fn nearest_seat(&self, at: Vec3) -> Option<usize> {
        let mut seats = self.seat_people();
        if seats.len() == 1 {
            return seats.pop();
        }
        seats.iter().copied()
            .filter(|&s| !self.people[s].is_dead())
            .min_by(|&a, &b| (self.people[a].fig.pos - at).length().total_cmp(&(self.people[b].fig.pos - at).length()))
            .or(seats.first().copied())
    }
}
