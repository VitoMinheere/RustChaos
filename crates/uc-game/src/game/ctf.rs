//! Capture the flag against a bot (#133): `Capflag1`, the original's two-player network
//! mission on its own map, played single player. Its second `create_player` makes Roper as
//! a person the AI drives. Each player has to reach the other's disk and get back home, as
//! the script's `enemy_radius` points for each seat say; the script announces it and ends
//! the game, and the end it gives the bot's seat is Darci's loss. Knocked out, either comes
//! back at home a few seconds later.

use crate::*;
use uc_formats::ucm::{trigger, waypoint};

/// Seconds down before coming back at home.
const RESPAWN: f32 = 4.0;
/// Within this of the other player (world units, and in sight) a bot fights them; past
/// twice it, it gets on with the game. Whoever has its disk is chased from further.
const FIGHT: f32 = 5.0 * 256.0;
/// How far out of its way a bot goes for something lying about (world units).
const FETCH: f32 = 6.0 * 256.0;
/// Seconds a bot fights before breaking off for the disk (or home) again, and seconds it
/// then keeps going before it'll fight again: the game's won by the disk, not by
/// a brawl, and two bots who block as well as each other would otherwise fight for good.
const BRAWL: f32 = 8.0;
const BREAK_OFF: f32 = 6.0;

/// A seat's places, from the script: the disk to reach and home, each with its radius.
#[derive(Debug, Clone, Copy)]
pub struct Seat {
    pub way: usize,
    pub start: Vec3,
    pub disk: Vec3,
    pub home: Vec3,
    /// How near the disk counts (the script's radius for it, as the trigger rounds it).
    pub reach: f32,
    /// Reached the other's disk.
    pub has_disk: bool,
    /// Seconds knocked out.
    down: f32,
}

#[derive(Debug, Clone)]
pub struct Ctf {
    pub player: Seat,
    pub bot: Seat,
    /// The bot, and Darci.
    pub bot_person: usize,
    pub player_person: usize,
    /// The `end_game_win` points that are the bot's seat's win.
    bot_wins: Vec<usize>,
    /// The squares that can be walked to from the bot's start (the navigation map misses
    /// some of this map's walls).
    grid: game::endless::Grid,
    /// The AI playing each seat, Darci's then the bot's: the bot's always, Darci's only on
    /// autopilot ([`Game::autopilot`]), the seat a second player would otherwise take.
    brains: [Option<Brain>; 2],
}

/// A bot's ways about the map, the next square from each towards its disk and towards its
/// home; where it was when it last got anywhere, and the seconds since; and a sidestep it's
/// taking to get unstuck, with the seconds left of it.
#[derive(Debug, Clone)]
struct Brain {
    to_disk: Option<Ways>,
    to_home: Option<Ways>,
    last: Vec3,
    still: f32,
    detour: Option<(Vec3, f32)>,
    detours: u32,
    /// Seconds into this fight, and seconds left of breaking off from one.
    brawl: f32,
    away: f32,
}

impl Brain {
    fn new(grid: &game::endless::Grid, seat: &Seat, at: Vec3) -> Brain {
        let to_disk = game::endless::towards(grid, seat.disk);
        let to_home = game::endless::towards(grid, seat.home);
        Brain { to_disk, to_home, last: at, still: 0.0, detour: None, detours: 0, brawl: 0.0, away: 0.0 }
    }
}

/// What a bot would pick up: health when hurt, a gun when it has none, a bat or knife
/// when its hands are empty, and rounds for the gun it has.
fn wanted(p: &Person, kind: u8) -> bool {
    match kind {
        3 => p.health < p.max_health * 3 / 4,
        2 | 4 | 6 | SNIPER => !p.weapon.is_gun(),
        5 | 8 => p.weapon == Weapon::None,
        10..=12 => p.weapon == Weapon::GUNS[(kind - 10) as usize],
        _ => false,
    }
}

type Ways = std::collections::BTreeMap<(i32, i32), (i32, i32)>;

fn near_flat(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

/// Whether `level` is a game for two players (two `create_player` points).
pub fn two_players(level: &Level) -> bool {
    level.mission.event_points.iter().filter(|e| e.used != 0 && e.kind == waypoint::CREATE_PLAYER).count() >= 2
}

/// A seat's disk and home: the `enemy_radius` points watching its player, the one furthest
/// from where it starts being the disk.
fn seat(level: &Level, way: usize, start: Vec3) -> Option<Seat> {
    let pos = |e: &uc_formats::ucm::EventPoint| Vec3::new(e.x as f32, e.y as f32, e.z as f32);
    let mut watching: Vec<_> = level.mission.event_points.iter()
        .filter(|e| e.used != 0 && e.trigger == trigger::ENEMY_RADIUS && e.ep_ref as usize == way)
        .map(|e| (pos(e), e.radius as f32))
        .collect();
    watching.sort_by(|a, b| a.0.distance(start).total_cmp(&b.0.distance(start)));
    let (home, _) = *watching.first()?;
    let (disk, r) = *watching.last()?;
    Some(Seat { way, start, disk, home, reach: ((r as i32) / 64 * 64) as f32, has_disk: false, down: 0.0 })
}

/// The `end_game_win` points reached from a trigger watching `way`, through the points they
/// depend on.
fn wins_for(level: &Level, way: usize) -> Vec<usize> {
    let points: std::collections::BTreeMap<usize, &uc_formats::ucm::EventPoint> =
        level.mission.event_points.iter().filter(|e| e.used != 0).map(|e| (e.index, e)).collect();
    let depends_on_seat = |mut i: usize| {
        for _ in 0..16 {
            let Some(e) = points.get(&i) else { return false };
            if e.trigger == trigger::ENEMY_RADIUS {
                return e.ep_ref as usize == way;
            }
            if e.ep_ref == 0 {
                return false;
            }
            i = e.ep_ref as usize;
        }
        false
    };
    points.values().filter(|e| e.kind == waypoint::END_GAME_WIN && depends_on_seat(e.index)).map(|e| e.index).collect()
}

impl Game {
    /// The second player of a two-player mission: Roper (or whoever), driven by the AI.
    pub(crate) fn spawn_bot(&mut self, level: &Level, pos: Vec3, angle: f32, way: usize, kind: u8, has: u8) -> Option<usize> {
        let me = self.player?;
        let i = self.spawn(level, kind, false, way, pos, angle)?;
        let p = &mut self.people[i];
        p.ai = person::AI_GANG;
        p.mv = 1;
        p.skill = 8;
        p.give(Weapon::from_has(has));
        let change = self.difficulty().skill_change();
        self.tune(i, change);
        let player_way = self.people[me].way;
        let player_start = self.people[me].fig.pos;
        if let (Some(player), Some(bot)) = (seat(level, player_way, player_start), seat(level, way, pos)) {
            let grid = game::endless::arena(level, pos, 128.0 * 256.0);
            let brains = [None, Some(Brain::new(&grid, &bot, pos))];
            self.ctf = Some(Ctf { player, bot, bot_person: i, player_person: me, bot_wins: wins_for(level, way), grid, brains });
        }
        Some(i)
    }

    /// Capture the flag, if this is a game of it.
    pub fn ctf(&self) -> Option<&Ctf> {
        self.ctf.as_ref()
    }

    /// Whether `end_game_win` point `way` is the bot's win (so Darci's loss).
    pub(crate) fn bot_won(&self, way: usize) -> bool {
        self.ctf.as_ref().is_some_and(|c| c.bot_wins.contains(&way))
    }

    /// Darci's seat played by the AI as well: bot against bot (#133), the seat a second
    /// player would take filled. The camera stays on her.
    pub fn autopilot(&mut self) -> bool {
        let Some(c) = self.ctf.as_mut() else { return false };
        let me = c.player_person;
        c.brains[0] = Some(Brain::new(&c.grid, &c.player, self.people[me].fig.pos));
        let skill = self.people[c.bot_person].skill;
        let p = &mut self.people[me];
        p.player = false;
        p.ai = person::AI_GANG;
        p.mv = 1;
        p.skill = skill;
        true
    }

    /// Who a seat's player fights: the other seat's. `None` if `i` isn't a bot here.
    pub(crate) fn ctf_foe(&self, i: usize) -> Option<usize> {
        let c = self.ctf.as_ref()?;
        if i == c.bot_person {
            Some(c.player_person)
        } else if i == c.player_person && c.brains[0].is_some() {
            Some(c.bot_person)
        } else {
            None
        }
    }

    /// Every turn: who's reached the other's disk, knocked-out players back at home, and the
    /// bots' plans: fight the other when they're near, else go for its disk, or home with it,
    /// picking up what's lying on the way.
    pub(crate) fn run_ctf(&mut self, dt: f32, level: &Level) {
        let Some(mut c) = self.ctf.take() else { return };
        let near = |a: Vec3, b: Vec3, r: f32| Vec2::new(a.x - b.x, a.z - b.z).length() < r;
        for (seat, i) in [(&mut c.player, c.player_person), (&mut c.bot, c.bot_person)] {
            let p = &mut self.people[i];
            if !seat.has_disk && !p.is_down() && near(p.fig.pos, seat.disk, seat.reach * 0.8) {
                seat.has_disk = true;
            }
            // Knocked out: back at home, whole, after a while.
            if p.knocked_out || (p.is_down() && p.health <= 0) {
                seat.down += dt;
                if seat.down > RESPAWN {
                    seat.down = 0.0;
                    let at = seat.start;
                    let y = level.ground.surface(&level.map, at.x, at.z, at.y + 64.0);
                    p.knocked_out = false;
                    p.out_cold = false;
                    p.health = p.max_health;
                    p.fig.pos = Vec3::new(at.x, y, at.z);
                    p.stand_still();
                }
            } else {
                seat.down = 0.0;
            }
        }
        for s in 0..2 {
            let Some(mut brain) = c.brains[s].take() else { continue };
            let (me, them, seat, theirs) =
                if s == 0 { (c.player_person, c.bot_person, c.player, c.bot) } else { (c.bot_person, c.player_person, c.bot, c.player) };
            self.drive_bot(&mut brain, &c.grid, me, them, &seat, theirs.has_disk, dt, level);
            // (The player's seat picks up as the player always does.)
            if s == 1 {
                self.pick_up_for(me);
            }
            c.brains[s] = Some(brain);
        }
        self.ctf = Some(c);
    }

    /// One bot's turn: after the other player when they're near and in sight (from further
    /// when they have its disk), else to whatever it wants lying nearby, else for the disk or
    /// home with it.
    #[allow(clippy::too_many_arguments)]
    fn drive_bot(&mut self, c: &mut Brain, grid: &game::endless::Grid, me: usize, them: usize, seat: &Seat, they_have_it: bool, dt: f32, level: &Level) {
        let other = self.people[them].fig.pos;
        let other_down = self.people[them].is_down();
        let eye = Vec3::new(0.0, 120.0, 0.0);
        let b = &self.people[me];
        if b.is_down() {
            return;
        }
        let at = b.fig.pos;
        let fight = if they_have_it { FIGHT * 3.0 } else { FIGHT };
        let d = (at - other).length();
        let sees = d < fight && level.ground.line_of_sight(at + eye, other + eye);
        // Something it wants, lying near and in sight.
        let fetch = self.items.iter()
            .filter(|it| wanted(b, it.kind) && (it.pos.y - at.y).abs() < 100.0 && near_flat(it.pos, at) < FETCH)
            .filter(|it| level.ground.line_of_sight(at + eye, it.pos + eye))
            .min_by(|x, y| near_flat(x.pos, at).total_cmp(&near_flat(y.pos, at)))
            .map(|it| it.pos);
        let b = &mut self.people[me];
        c.away = (c.away - dt).max(0.0);
        c.brawl = if b.aggro() { c.brawl + dt } else { 0.0 };
        if c.brawl > BRAWL {
            (c.brawl, c.away) = (0.0, BREAK_OFF);
            b.calm_down();
        }
        // Breaking off, it keeps going whatever it sees (or the AI's own eyes would have it
        // straight back in the fight), unless it's hit.
        b.truce = c.away > 0.0;
        if d > fight * 2.0 || other_down {
            b.calm_down();
        } else if sees && !b.truce {
            b.go_for();
        }
        if b.aggro() {
            return;
        }
        let to = if seat.has_disk { seat.home } else { seat.disk };
        let ways = if seat.has_disk { &c.to_home } else { &c.to_disk };
        let step = ways.as_ref().and_then(|w| game::endless::next_step(grid, w, at, to, level));
        // Stuck (the navigation map and the walker don't always agree about what's in the
        // way): a sidestep of a couple of squares, one way then the other, then on.
        if at.distance(c.last) > 96.0 {
            (c.last, c.still) = (at, 0.0);
        } else {
            c.still += dt;
        }
        if c.still > 1.5 && c.detour.is_none() {
            let ahead = Vec2::new(to.x - at.x, to.z - at.z).normalize_or_zero();
            let side = if c.detours.is_multiple_of(2) { Vec2::new(-ahead.y, ahead.x) } else { Vec2::new(ahead.y, -ahead.x) };
            let back = -ahead * 0.5;
            let d = (side + back).normalize_or_zero() * (2.0 + (c.detours % 3) as f32) * 256.0;
            c.detour = Some((at + Vec3::new(d.x, 0.0, d.y), 1.5));
            c.detours += 1;
            c.still = 0.0;
        }
        b.goal = Some(match (&mut c.detour, fetch) {
            (Some((at, left)), _) => {
                *left -= dt;
                *at
            }
            (None, Some(it)) => it,
            (None, None) => step.unwrap_or(to),
        });
        if c.detour.is_some_and(|(_, left)| left <= 0.0) {
            c.detour = None;
        }
        b.pace = person::Pace::Run;
    }

    /// The beacons: where Darci's going (the disk, then home), and the bot.
    pub(crate) fn ctf_beacons(&self) -> Vec<(String, Vec3)> {
        let Some(c) = &self.ctf else { return Vec::new() };
        let mine = if c.player.has_disk { ("Home with the disk".to_string(), c.player.home) } else { ("The disk".to_string(), c.player.disk) };
        let bot = &self.people[c.bot_person];
        let label = if c.bot.has_disk { "Roper has the disk" } else { "Roper" };
        vec![mine, (label.to_string(), bot.fig.pos)]
    }
}
