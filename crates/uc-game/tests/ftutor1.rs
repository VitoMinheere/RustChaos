//! Plays the combat tutorial with a simple bot and checks that it can be won.
//! Skipped when no Urban Chaos install is found.

use glam::{vec2, Vec2, Vec3};
use uc_game::person::{ko, Grab};
use uc_game::{Controls, Game, Outcome, State};
use uc_world::{Level, LoadOptions};

fn flat(v: Vec3) -> Vec2 {
    vec2(v.x, v.z)
}

/// Decides the bot's controls for this frame.
struct Bot {
    frame: u32,
    warmup: u32,
    /// Position a second ago, to notice being stuck.
    last: Vec3,
    /// Frames left of walking a detour, and its turn (radians).
    detour: (u32, f32),
}

/// Breadth-first search over map squares; walls on square edges block. Returns the next
/// point to walk to from `from` towards `to` (world units), at ground level.
fn next_step(level: &Level, from: Vec3, to: Vec3) -> Vec2 {
    let sq = |v: Vec3| ((v.x / 256.0) as i32, (v.z / 256.0) as i32);
    let (start, goal) = (sq(from), sq(to));
    if start == goal {
        return flat(to);
    }
    // blocked[x][z] bit 0: edge to (x+1, z); bit 1: edge to (x, z+1).
    let mut blocked = vec![[0u8; 128]; 128];
    for w in &level.ground.walls {
        // Walls she can't reach the top of block; low ledges get climbed.
        if from.y + 20.0 < w.y0 || from.y + 20.0 > w.y1 || w.y1 - from.y <= 150.0 {
            continue;
        }
        let (a, b) = (Vec2::from(w.a) / 256.0, Vec2::from(w.b) / 256.0);
        let n = ((b - a).length() * 4.0).ceil().max(1.0) as i32;
        for k in 0..n {
            let p = a + (b - a) * ((k as f32 + 0.5) / n as f32);
            let (x, z) = (p.x.round() as i32, p.y.round() as i32);
            if (b.x - a.x).abs() < 0.01 && (1..128).contains(&x) {
                // A wall along x = const separates (x-1, z) and (x, z).
                let zz = p.y.floor() as i32;
                if (0..128).contains(&zz) { blocked[(x - 1) as usize][zz as usize] |= 1; }
            } else if (b.y - a.y).abs() < 0.01 && (1..128).contains(&z) {
                let xx = p.x.floor() as i32;
                if (0..128).contains(&xx) { blocked[xx as usize][(z - 1) as usize] |= 2; }
            } else {
                let (xx, zz) = (p.x.floor() as i32, p.y.floor() as i32);
                if (1..127).contains(&xx) && (1..127).contains(&zz) {
                    blocked[xx as usize][zz as usize] |= 3;
                    blocked[xx as usize - 1][zz as usize] |= 1;
                    blocked[xx as usize][zz as usize - 1] |= 2;
                }
            }
        }
    }
    let h = |x: i32, z: i32| level.map.ground_height(x as f32 * 256.0 + 128.0, z as f32 * 256.0 + 128.0);
    let open = |(x, z): (i32, i32), (nx, nz): (i32, i32)| -> bool {
        if !(0..128).contains(&nx) || !(0..128).contains(&nz) { return false; }
        if (h(nx, nz) - h(x, z)).abs() > 40.0 { return false; }
        if nx != x { blocked[x.min(nx) as usize][z as usize] & 1 == 0 } else { blocked[x as usize][z.min(nz) as usize] & 2 == 0 }
    };
    let mut prev = vec![[None::<(i32, i32)>; 128]; 128];
    let mut queue = std::collections::VecDeque::from([start]);
    prev[start.0 as usize][start.1 as usize] = Some(start);
    while let Some(c) = queue.pop_front() {
        if c == goal { break; }
        for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (c.0 + d.0, c.1 + d.1);
            if open(c, n) && prev[n.0 as usize][n.1 as usize].is_none() {
                prev[n.0 as usize][n.1 as usize] = Some(c);
                queue.push_back(n);
            }
        }
    }
    if prev[goal.0 as usize][goal.1 as usize].is_none() {
        if std::env::var("BOT_MAP").is_ok() {
            println!("no path {start:?} -> {goal:?}");
            for z in (36..80).rev() {
                let row: String = (30..90).rev().map(|x| if (x, z) == start { 'S' } else if (x, z) == goal { 'G' } else if prev[x as usize][z as usize].is_some() { '.' } else { '#' }).collect();
                println!("{z:3} {row}");
            }
            std::process::exit(1);
        }
        return flat(to);
    }
    // Walk back to the square after the start; aim a little further for smoother paths.
    let mut path = vec![goal];
    while let Some(p) = prev[path.last().unwrap().0 as usize][path.last().unwrap().1 as usize] {
        if p == start { break; }
        path.push(p);
    }
    let next = path[path.len().saturating_sub(1)];
    vec2(next.0 as f32 + 0.5, next.1 as f32 + 0.5) * 256.0
}

fn rotate(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

impl Bot {
    /// Steers around obstacles: when walking but not getting anywhere, turn for a while.
    fn controls(&mut self, g: &Game, level: &Level) -> Controls {
        let mut c = self.plan(g, level);
        let pos = g.player().unwrap().fig.pos;
        if self.frame.is_multiple_of(30) {
            let moved = flat(pos - self.last).length();
            // Stuck: jump (onto a walkway or platform rim), and walk round for a while.
            if c.wish.length() > 0.5 && moved < 40.0 && matches!(g.player().unwrap().state, State::Move) {
                c.jump = true;
            }
            if c.wish.length() > 0.5 && moved < 40.0 && self.detour.0 == 0 && matches!(g.player().unwrap().state, State::Move) {
                let side = if (self.frame / 30) % 4 < 2 { 1.2 } else { -1.2 };
                self.detour = (45, side);
            }
            self.last = pos;
        }
        if self.detour.0 > 0 && c.wish.length() > 0.5 {
            self.detour.0 -= 1;
            c.wish = rotate(c.wish, self.detour.1);
        }
        c
    }

    fn plan(&mut self, g: &Game, level: &Level) -> Controls {
        self.frame += 1;
        let me = g.player().unwrap();
        let pos = me.fig.pos;
        let mut c = Controls::default();
        let tap = self.frame.is_multiple_of(6);

        // Step 1: try out punches and kicks.
        if self.warmup < 4 {
            if tap && matches!(me.state, State::Move) {
                if self.warmup.is_multiple_of(2) { c.punch = true } else { c.kick = true }
                self.warmup += 1;
            }
            return c;
        }
        // Keep climbing a ladder once on it.
        if let State::Climb { ladder, .. } = me.state {
            c.wish = -level.ground.ladders[ladder].out;
            return c;
        }
        // Finish a grapple: knee, knee.
        if let State::Grapple { grab, .. } = me.state {
            c.punch = tap && matches!(grab, Grab::Hold | Grab::Knee1);
            return c;
        }

        // Arrest anyone down and not yet arrested whose arrest the script wants (has a beacon
        // history): the arrest target is the one that any move knocks out.
        if let Some(p) = g.people.iter().find(|p| !p.player && !p.removed && p.is_down() && !p.arrested && p.ko_moves == 127) {
            // Go to where the body lies (a knockdown carries it away from where he stood).
            let d = flat(p.body_pos() - pos);
            if d.length() > 90.0 {
                c.wish = d.normalize();
            } else {
                c.action = tap;
            }
            return c;
        }

        // The nearest beacon is the next target.
        let beacons = g.beacons();
        let Some((_, target)) = beacons.iter().min_by(|a, b| (a.1 - pos).length().total_cmp(&(b.1 - pos).length())) else {
            return c;
        };
        let target = *target;
        let person = g.people.iter().find(|p| !p.player && !p.removed && (p.fig.pos - target).length() < 1.0);

        // Up on a roof: climb the nearest ladder first.
        if target.y - pos.y > 200.0 {
            if let State::Climb { ladder, .. } = me.state {
                c.wish = -level.ground.ladders[ladder].out;
                return c;
            }
            let l = level.ground.ladders.iter().min_by(|a, b| (a.foot - flat(target)).length().total_cmp(&(b.foot - flat(target)).length())).unwrap();
            let stand = l.foot + l.out * 70.0;
            let d = stand - flat(pos);
            c.wish = if d.length() > 400.0 {
                (next_step(level, pos, Vec3::new(stand.x, pos.y, stand.y)) - flat(pos)).normalize_or_zero()
            } else if d.length() > 30.0 { d.normalize() } else { -l.out };
            c.run = d.length() > 300.0;
            return c;
        }

        let Some(p) = person else { return c };
        let d = flat(p.fig.pos - pos);
        let dist = d.length();
        if dist > 400.0 && !matches!(me.state, State::Slide { .. }) {
            c.wish = (next_step(level, pos, p.fig.pos) - flat(pos)).normalize_or_zero();
            c.run = true;
            return c;
        }
        let k = p.ko_moves;
        if k & ko::SLIDE != 0 && k != 127 {
            // Sliding tackle: back off, then run in and kick.
            if dist < 330.0 && !matches!(me.state, State::Slide { .. }) && self.frame % 300 < 150 {
                c.wish = -d.normalize();
                c.run = true;
            } else {
                c.wish = d.normalize();
                c.run = true;
                c.kick = dist < 300.0 && tap;
            }
            return c;
        }
        if k & ko::GRAPPLE != 0 && k != 127 {
            c.wish = d.normalize();
            c.punch = dist < 115.0 && tap;
            return c;
        }
        if k & (ko::SIDE_KICK | ko::BACK_KICK) != 0 && k != 127 {
            // Kick whoever we're not facing, sideways or backwards. If they're all in front,
            // walk towards the farthest so the nearest ends up beside or behind.
            let f = me.facing_dir();
            let others: Vec<_> = g.people.iter().filter(|o| !o.player && !o.removed && !o.is_down() && o.ko_moves == k && (o.fig.pos - pos).length() < 900.0).collect();
            let off: Vec<_> = others.iter().filter(|o| flat(o.fig.pos - pos).normalize().dot(f) < 0.3).collect();
            if let Some(o) = off.iter().min_by(|a, b| (a.fig.pos - pos).length().total_cmp(&(b.fig.pos - pos).length())) {
                let to = flat(o.fig.pos - pos);
                c.wish = to.normalize();
                c.kick = to.length() < 150.0 && tap;
            } else if let Some(o) = others.iter().max_by(|a, b| (a.fig.pos - pos).length().total_cmp(&(b.fig.pos - pos).length())) {
                c.wish = flat(o.fig.pos - pos).normalize_or_zero();
            }
            return c;
        }
        // Combos: three punches (or kicks) in time.
        if dist > 110.0 {
            c.wish = d.normalize();
            c.run = dist > 400.0;
        } else if k & ko::COMBO_KKK != 0 && k != 127 {
            c.kick = tap;
        } else {
            c.punch = tap;
        }
        c
    }
}

#[test]
fn bot_completes_the_combat_tutorial() {
    let Some(game) = uc_formats::default_install() else {
        eprintln!("no Urban Chaos install found (set UC_GAME_DIR); skipping");
        return;
    };
    let level = Level::load(&game, "ftutor1", &LoadOptions { fullbright: true, character: true, overrides: None }).unwrap();
    let mut g = Game::new(&level, true);
    assert!(g.unsupported().is_empty(), "unsupported: {:?}", g.unsupported());
    let mut bot = Bot { frame: 0, warmup: 0, last: Vec3::ZERO, detour: (0, 0.0) };
    let dt = 1.0 / 30.0;
    let mut seen = Vec::new();
    for _ in 0..(600.0 / dt) as usize {
        let c = bot.controls(&g, &level);
        g.update(dt, &c, &level);
        if let Ok(range) = std::env::var("BOT_TRACE") {
            let (t0, t1) = range.split_once('-').map(|(a, b)| (a.parse().unwrap_or(0.0), b.parse().unwrap_or(1e9))).unwrap_or((0.0, 1e9));
            if g.time > t0 && g.time < t1 && (std::env::var("BOT_EVERY").is_ok() || ((g.time * 30.0) as u32).is_multiple_of(10)) {
                let p = g.player().unwrap();
                let near: Vec<String> = g.people.iter().filter(|o| !o.player && !o.removed && (o.fig.pos - p.fig.pos).length() < 600.0)
                    .map(|o| format!("[{} {:?} d{:.0} ko{}]", o.way, o.state, (o.fig.pos - p.fig.pos).length(), o.ko_moves)).collect();
                println!("{:.2} hp{} {:?} anim {} pos {:.0?} wish {:.1?} p{} k{} | {}", g.time, p.health, p.state, p.fig.anim(), p.fig.pos, c.wish, c.punch as u8, c.kick as u8, near.join(" "));
            }
        }
        for m in &g.messages {
            if !seen.contains(&m.text) {
                println!("[{:6.1}s] {}", g.time, m.text);
                seen.push(m.text.clone());
            }
        }
        if g.outcome.is_some() {
            break;
        }
    }
    let p = g.player().unwrap();
    println!("t={:.1}s ended at {:?}, Darci at {:?} in {:?}, health {}", g.time, g.outcome, p.fig.pos, p.state, p.health);
    assert_eq!(g.outcome, Some(Outcome::Won));
}
