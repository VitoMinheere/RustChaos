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
/// Within this of Darci (world units, and in sight) the bot fights her; past twice it, it
/// gets on with the game.
const FIGHT: f32 = 5.0 * 256.0;

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
    /// The bot.
    pub bot_person: usize,
    /// The `end_game_win` points that are the bot's seat's win.
    bot_wins: Vec<usize>,
    /// The bot's ways about the map (the navigation map misses some of this map's walls):
    /// the squares that can be walked to from its start, and the next square from each
    /// towards Darci's disk and towards its home.
    grid: game::endless::Grid,
    to_disk: Option<Ways>,
    to_home: Option<Ways>,
    /// Where the bot was when it last got anywhere, and the seconds since; and a sidestep
    /// it's taking to get unstuck, with the seconds left of it.
    last: Vec3,
    still: f32,
    detour: Option<(Vec3, f32)>,
    detours: u32,
}

type Ways = std::collections::BTreeMap<(i32, i32), (i32, i32)>;

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
            let to_disk = game::endless::towards(&grid, bot.disk);
            let to_home = game::endless::towards(&grid, bot.home);
            self.ctf = Some(Ctf { player, bot, bot_person: i, bot_wins: wins_for(level, way), grid, to_disk, to_home, last: pos, still: 0.0, detour: None, detours: 0 });
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

    /// Every turn: who's reached the other's disk, knocked-out players back at home, and the
    /// bot's plan: fight Darci when she's near, else go for her disk, or home with it.
    pub(crate) fn run_ctf(&mut self, dt: f32, level: &Level) {
        let Some(mut c) = self.ctf.take() else { return };
        let Some(me) = self.player else {
            self.ctf = Some(c);
            return;
        };
        let near = |a: Vec3, b: Vec3, r: f32| Vec2::new(a.x - b.x, a.z - b.z).length() < r;
        for (seat, i) in [(&mut c.player, me), (&mut c.bot, c.bot_person)] {
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
        // The bot.
        let darci = self.people[me].fig.pos;
        let darci_down = self.people[me].is_down();
        let b = &mut self.people[c.bot_person];
        if !b.is_down() {
            let eye = Vec3::new(0.0, 120.0, 0.0);
            let d = (b.fig.pos - darci).length();
            let sees = d < FIGHT && level.ground.line_of_sight(b.fig.pos + eye, darci + eye);
            if sees && !darci_down {
                b.go_for();
            } else if d > FIGHT * 2.0 || darci_down {
                b.calm_down();
            }
            if !b.aggro() {
                let (to, ways) = if c.bot.has_disk { (c.bot.home, &c.to_home) } else { (c.bot.disk, &c.to_disk) };
                let step = ways.as_ref().and_then(|w| game::endless::next_step(&c.grid, w, b.fig.pos, to, level));
                // Stuck (the navigation map and the walker don't always agree about what's in
                // the way): a sidestep of a couple of squares, one way then the other, then on.
                if b.fig.pos.distance(c.last) > 96.0 {
                    (c.last, c.still) = (b.fig.pos, 0.0);
                } else {
                    c.still += dt;
                }
                if c.still > 1.5 && c.detour.is_none() {
                    let ahead = Vec2::new(to.x - b.fig.pos.x, to.z - b.fig.pos.z).normalize_or_zero();
                    let side = if c.detours.is_multiple_of(2) { Vec2::new(-ahead.y, ahead.x) } else { Vec2::new(ahead.y, -ahead.x) };
                    let back = -ahead * 0.5;
                    let d = (side + back).normalize_or_zero() * (2.0 + (c.detours % 3) as f32) * 256.0;
                    c.detour = Some((b.fig.pos + Vec3::new(d.x, 0.0, d.y), 1.5));
                    c.detours += 1;
                    c.still = 0.0;
                }
                b.goal = Some(match &mut c.detour {
                    Some((at, left)) => {
                        *left -= dt;
                        *at
                    }
                    None => step.unwrap_or(to),
                });
                if c.detour.is_some_and(|(_, left)| left <= 0.0) {
                    c.detour = None;
                }
                b.pace = person::Pace::Run;
            }
        }
        self.ctf = Some(c);
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
