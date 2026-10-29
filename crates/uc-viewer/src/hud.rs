//! The HUD drawn over the game: health, weapons, messages, the map, sights and the rest.

use uc_formats::units::radians;
use crate::*;

/// The gun sights (`PANEL_draw_gun_sight`, DDEngine/Source/panel.cpp:728): four strokes
/// round the point, 84 to 164 out (times the sight's scale over 256), at `accuracy` (in
/// 2048ths of a turn) either side of left and right, so they close up into two level strokes
/// as the aim steadies; faint, from orange towards red as it does (`col = acc << 8 |
/// (255 - acc) << 16`), 30 wide. Drawn as runs of small squares, the overlay having no lines.
pub(crate) fn draw_sights(o: &mut Overlay, sights: &[uc_game::Sight], cam: &Camera, size: (u32, u32)) {
    let (w, h) = (size.0 as f32, size.1 as f32);
    let vp = cam.view_proj(size);
    let screen = |p: Vec3| -> Option<Vec2> {
        let c = vp * p.extend(1.0);
        (c.w > 0.01).then(|| vec2((c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h))
    };
    let right = (cam.target - cam.pos).cross(Vec3::Y).normalize_or(Vec3::X);
    for s in sights {
        let at = s.at / uc_world::UNIT;
        let (Some(c), Some(e)) = (screen(at), screen(at + right / uc_world::UNIT)) else { continue };
        let px = (e - c).length(); // pixels a world unit
        let k = s.scale / 256.0;
        let (r_in, r_out) = (84.0 * k * px, 164.0 * k * px);
        let dot = (30.0 * k * px).clamp(2.0, 12.0);
        let acc = s.accuracy.clamp(0.0, 100.0);
        let colour = [(255.0 - acc) as u8, acc as u8, 0, 140];
        let a = radians(acc);
        for angle in [a, -a, std::f32::consts::PI + a, std::f32::consts::PI - a] {
            let dir = vec2(angle.cos(), angle.sin());
            let n = ((r_out - r_in) / dot).ceil().max(1.0) as usize;
            for j in 0..=n {
                let p = c + dir * (r_in + (r_out - r_in) * j as f32 / n as f32);
                o.rect(p.x - dot / 2.0, p.y - dot / 2.0, dot, dot, colour);
            }
        }
    }
}

/// The chief's `n`th warning about civilian casualties (`X_CIVSDEAD_WARNING_1` to `_4`, the
/// game's strings 151 to 154: `text\lang_english.txt`'s lines 152 to 155).
pub(crate) fn civ_warning(game: &Path, n: u8) -> String {
    let line = 151 + n.min(3) as usize;
    uc_formats::resolve(game, "text\\lang_english.txt")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| String::from_utf8_lossy(&b).lines().nth(line).map(|l| l.trim().to_string()))
        .unwrap_or_else(|| "Too many civilian casualties.".to_string())
}

impl World {
    /// The HUD: health, mission messages, beacons, the outcome and help.
    pub(crate) fn hud(&self, size: (u32, u32)) -> Overlay {
        let mut o = Overlay::new();
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).round().max(1.0);
        let white = [235, 235, 225, 255];

        // A tutorial message: the whole screen darkened, the text in a bubble at the top left,
        // and nothing else of the panel (`PANEL_last`, DDEngine/Source/panel.cpp:4327).
        if let Some((text, _)) = &self.game.tutorial {
            o.rect(0.0, 0.0, w, h, [0, 0, 0, 0x88]);
            let k = h / 480.0;
            let (x, y) = (16.0 * k, 16.0 * k);
            let lines = Overlay::wrap(text, w - 2.0 * x - 12.0 * k, s);
            let lh = 11.0 * s;
            let right = lines.iter().map(|l| Overlay::text_width(l, s)).fold(0.0, f32::max);
            o.rect(x - 6.0 * k, y - 4.0 * k, right + 12.0 * k, lines.len() as f32 * lh + 8.0 * k, [250, 250, 240, 60]);
            for (n, line) in lines.iter().enumerate() {
                o.text(x, y + n as f32 * lh, s, [255, 255, 255, 255], line);
            }
            return o;
        }

        // The scope: dark round the edges, and cross hairs.
        if self.game.sniping().is_some() {
            let (cx, cy, r) = (w / 2.0, h / 2.0, h * 0.42);
            let dark = [0, 0, 0, 255];
            o.rect(0.0, 0.0, cx - r, h, dark);
            o.rect(cx + r, 0.0, w - cx - r, h, dark);
            o.rect(cx - r, 0.0, 2.0 * r, cy - r, dark);
            o.rect(cx - r, cy + r, 2.0 * r, h - cy - r, dark);
            let line = [20, 20, 20, 220];
            o.rect(cx - r, cy - s * 0.5, 2.0 * r, s, line);
            o.rect(cx - s * 0.5, cy - r, s, 2.0 * r, line);
        }
        if let Some(p) = self.game.player() {
            // Health bar.
            let frac = (p.health.max(0) as f32 / p.max_health as f32).clamp(0.0, 1.0);
            o.text(12.0 * s, 10.0 * s, s, white, &self.game.player_name().to_uppercase());
            let (bx, by, bw, bh) = (12.0 * s, 20.0 * s, 80.0 * s, 5.0 * s);
            o.rect(bx - s, by - s, bw + 2.0 * s, bh + 2.0 * s, [0, 0, 0, 160]);
            let col = if frac > 0.5 { [90, 200, 90, 255] } else if frac > 0.25 { [230, 190, 60, 255] } else { [220, 60, 50, 255] };
            o.rect(bx, by, bw * frac, bh, col);
            // A grim reaper for each innocent killed (`PANEL_IC_GRIMREAPER`, panel.cpp:3315).
            for k in 0..self.game.red_marks as usize {
                let (x, y) = (bx + k as f32 * 9.0 * s, by + bh + 4.0 * s);
                o.rect(x, y + 2.0 * s, 6.0 * s, 9.0 * s, [30, 30, 34, 230]);
                o.rect(x + 2.0 * s, y + 3.0 * s, 2.0 * s, 2.0 * s, [225, 220, 200, 255]);
                o.rect(x + 7.0 * s, y, 1.0 * s, 11.0 * s, [190, 190, 190, 255]);
                o.rect(x + 4.0 * s, y, 4.0 * s, 1.0 * s, [190, 190, 190, 255]);
            }
            // What's in hand, and the rounds left in a gun.
            let weapon = match (p.rounds(), self.game.primed) {
                // A primed grenade: the seconds to it going off.
                (Some(n), Some(t)) if p.weapon == uc_game::Weapon::Grenade => format!("{} {n}  PRIMED {:.0}", p.weapon.name(), t.ceil()),
                (Some(n), _) => format!("{} {n}", p.weapon.name()),
                (None, _) => p.weapon.name().to_string(),
            };
            let col = if p.rounds() == Some(0) { [240, 90, 70, 255] } else { white };
            o.text(bx + bw + 8.0 * s, by - 2.0 * s, s, col, &weapon);
            // The inventory, for a few seconds after a weapon change (`PANEL_inventory`,
            // DDEngine/Source/panel.cpp:4173): everything that can be taken out, in the order
            // the weapon key goes through them, the one in hand picked out, and a key if she
            // has one. It fades in and out (`INVENTORY_FADE_SPEED`, 32 of 256 a tick); not
            // while driving.
            if p.inventory > 0.0 && self.game.player_vehicle().is_none() {
                let fade = (p.inventory.min(3.0 - p.inventory) / 0.4).clamp(0.0, 1.0);
                let a = |c: [u8; 4]| [c[0], c[1], c[2], (c[3] as f32 * fade) as u8];
                // On the left, below the controls help.
                let (x, mut y) = (bx, h * 0.32);
                let mut rows: Vec<(String, bool)> = p.carried().into_iter().map(|w| {
                    let name = if w == uc_game::Weapon::None { "FISTS" } else { w.name() };
                    let rounds = p.rounds_of(w).map_or(String::new(), |n| format!(" {n}"));
                    (format!("{name}{rounds}"), w == p.weapon)
                }).collect();
                if p.has_key {
                    rows.push(("KEY".to_string(), false));
                }
                let width = rows.iter().map(|(t, _)| Overlay::text_width(t, s)).fold(0.0, f32::max) + 12.0 * s;
                for (text, held) in rows {
                    o.rect(x - 2.0 * s, y - 2.0 * s, width, 11.0 * s, a(if held { [200, 150, 40, 200] } else { [0, 0, 0, 150] }));
                    o.text(x + 4.0 * s, y, s, a(if held { white } else { [200, 200, 190, 255] }), &text);
                    y += 12.0 * s;
                }
            }
            // The info line (`PANEL_new_info_message`, panel.cpp:5235): slid out to the right
            // over its first 0.4 s, right-justified, fading in over a quarter second with a
            // shadow that glows then goes.
            if let Some((text, t)) = &self.game.info {
                let ms = t * 1000.0;
                let main = ms.min(255.0) as u8;
                let shad = if ms < 255.0 { ms } else if ms < 768.0 { 255.0 - (ms - 256.0) / 2.0 } else { 0.0 }.clamp(0.0, 255.0) as u8;
                let k = h / 480.0;
                let right = bx + ((ms / 2.0).clamp(16.0, 205.0) - 16.0) * k + Overlay::text_width(text, s);
                let x = right - Overlay::text_width(text, s);
                let y = by + bh + 18.0 * s;
                o.text(x + 2.0 * k, y + 2.0 * k, s, [shad, shad, shad, shad], text);
                o.text(x, y, s, [255, 255, 255, main], text);
            }
            // Badge stats (gold for those raised in this mission; kept when it's won). In
            // co-op they're the host's, and only Darci's.
            let mut x = bx;
            let stats: &[&str] = if self.seat.unwrap_or(0) == 0 { &["STA", "STR", "REF", "CON"] } else { &[] };
            for (k, name) in stats.iter().enumerate() {
                let text = format!("{name} {}", self.game.stats[k]);
                let col = if self.game.badges[k] > 0 { [255, 210, 70, 255] } else { [190, 190, 180, 200] };
                x += o.text(x, by + bh + 4.0 * s, s, col, &text) + 8.0 * s;
            }
            // How much of the mission is done: badges and side objectives (with earlier wins).
            let (badges, sides) = self.tally();
            let (nb, ns) = (self.goals.badges.len(), self.goals.sides.len());
            // Not in an endless run: its map's mission isn't being played.
            if nb + ns > 0 && self.game.endless().is_none() && self.game.beat().is_none() {
                let text = format!("  BADGES {badges}/{nb}   SIDE {sides}/{ns}");
                let all = badges == nb && sides == ns;
                o.text(x, by + bh + 4.0 * s, s, if all { [255, 210, 70, 255] } else { [190, 190, 180, 200] }, &text);
            }
            // The crime rate, for the missions that show it, pulsing red from 95%
            // (`GF_SHOW_CRIMERATE`, DDEngine/Source/panel.cpp:4789).
            if self.game.show_crime_rate {
                let rate = self.game.crime_rate;
                let col = if rate >= 95 {
                    let g = (128.0 + 127.0 * (self.game.time * 9.0).sin()) as u8;
                    [255, g, g, 255]
                } else {
                    white
                };
                o.text(bx, by + bh + 16.0 * s, s, col, &format!("CRIME RATE {rate}%"));
            }
            // Co-op: the other player's name and health, smaller, underneath.
            if let Some(me) = self.seat {
                let mut y = by + bh + 28.0 * s;
                for i in (0..self.game.seats()).filter(|&k| k != me).filter_map(|k| self.game.seat_person(k)) {
                    let q = &self.game.people[i];
                    let frac = (q.health.max(0) as f32 / q.max_health as f32).clamp(0.0, 1.0);
                    let name = uc_world::character::person::name(q.kind).to_uppercase();
                    let tw = o.text(bx, y, s, [200, 200, 190, 230], &name);
                    let (x, w) = (bx + tw + 6.0 * s, 40.0 * s);
                    o.rect(x - s, y, w + 2.0 * s, 3.0 * s + 2.0 * s, [0, 0, 0, 160]);
                    let col = if q.knocked_out || q.health <= 0 { [120, 120, 120, 255] } else { [90, 170, 220, 255] };
                    o.rect(x, y + s, w * frac, 3.0 * s, col);
                    y += 10.0 * s;
                }
            }
        }

        // Driving: speed and the car's damage.
        if let Some(v) = self.game.player_vehicle() {
            let mph = (v.speed.abs() / 256.0 * 2.0 * 2.237) as u32;
            let text = format!("{mph} MPH");
            let big = s * 2.0;
            o.text(w - Overlay::text_width(&text, big) - 14.0 * s, h - 30.0 * s, big, white, &text);
            let frac = (v.health.max(0) as f32 / 300.0).clamp(0.0, 1.0);
            let (bx, by, bw, bh) = (w - 94.0 * s, h - 38.0 * s, 80.0 * s, 4.0 * s);
            o.rect(bx - s, by - s, bw + 2.0 * s, bh + 2.0 * s, [0, 0, 0, 160]);
            o.rect(bx, by, bw * frac, bh, if frac > 0.3 { [120, 170, 230, 255] } else { [220, 60, 50, 255] });
        }
        // A road sign the mission flashes.
        if let Some((text, _)) = &self.game.sign {
            let big = s * 3.0;
            let tw = Overlay::text_width(text, big);
            o.rect(w / 2.0 - tw / 2.0 - 10.0 * s, h * 0.2 - 6.0 * s, tw + 20.0 * s, 36.0 * s, [200, 30, 30, 220]);
            o.text(w / 2.0 - tw / 2.0, h * 0.2, big, white, text);
        }

        // A search under way: the panel's box in the middle of the screen, with its bar.
        if let Some(search) = &self.game.search {
            let (bw, bh) = (160.0 * s, 32.0 * s);
            let (bx, by) = (w / 2.0 - bw / 2.0, h * 0.6 - bh / 2.0);
            o.rect(bx, by, bw, bh, [0, 0, 0, 170]);
            let text = "SEARCHING";
            o.text(w / 2.0 - Overlay::text_width(text, s) / 2.0, by + 5.0 * s, s, white, text);
            let (fx, fy, fw, fh) = (bx + 10.0 * s, by + 18.0 * s, bw - 20.0 * s, 6.0 * s);
            o.rect(fx - s, fy - s, fw + 2.0 * s, fh + 2.0 * s, [60, 60, 60, 200]);
            o.rect(fx, fy, fw * search.progress(), fh, [230, 190, 60, 255]);
        }

        // The health of whoever she's fighting or aiming at, over their feet
        // (`PANEL_draw_local_health`, panel.cpp:602): 60 towards the camera and 10 down, a
        // dark blue strip 54 by 4 of the original's 640 by 480, and in it a red bar 2 high,
        // half a pixel to each per cent.
        if let Some((at, frac)) = self.game.enemy_health() {
            let cam = self.camera();
            let eye = cam.pos * uc_world::UNIT;
            let toward = (eye - at).normalize_or_zero() * 60.0;
            let (p, ahead) = cam.project((at + toward - Vec3::new(0.0, 10.0, 0.0)) / uc_world::UNIT, size);
            if ahead {
                let k = h / 480.0;
                o.rect(p.x - 27.0 * k, p.y, 54.0 * k, 4.0 * k, [0, 0, 0x0f, 255]);
                o.rect(p.x - 25.0 * k, p.y + k, 50.0 * frac * k, 2.0 * k, [0xff, 0, 0, 255]);
            }
        }

        // A creature's health, under the clock: the one in the sights, or the nearest.
        if let Some((name, frac)) = self.game.creature_health() {
            let (bw, bh) = (160.0 * s, 5.0 * s);
            let (bx, by) = (w / 2.0 - bw / 2.0, 44.0 * s);
            o.text(w / 2.0 - Overlay::text_width(name, s) / 2.0, by - 11.0 * s, s, white, name);
            o.rect(bx - s, by - s, bw + 2.0 * s, bh + 2.0 * s, [0, 0, 0, 160]);
            o.rect(bx, by, bw * frac, bh, [220, 80, 40, 255]);
        }

        // The mission clock.
        if let Some(t) = self.game.countdown() {
            let secs = t.ceil() as u32;
            let text = format!("{}:{:02}", secs / 60, secs % 60);
            let col = if t < 30.0 { [240, 90, 70, 255] } else { white };
            let big = s * 2.0;
            o.text(w / 2.0 - Overlay::text_width(&text, big) / 2.0, 10.0 * s, big, col, &text);
        }

        // The ghost race (#186), under the clock: the best run's time, and how this run stood
        // against it at the last checkpoint, green ahead and red behind.
        if let (Some(r), None) = (&self.race, &self.game.outcome) {
            let best = match (&r.best, self.ghost_on) {
                (Some(b), true) => format!("GHOST {}", progress::clock(b.time)),
                (Some(b), false) => format!("BEST {}  (B: ghost)", progress::clock(b.time)),
                (None, _) => "NO GHOST YET".to_string(),
            };
            let split = r.split.map(|(d, _)| (format!("  {}", ghosts::split_text(d)), if d <= 0.0 { [110, 230, 110, 255] } else { [240, 90, 70, 255] }));
            let tw = Overlay::text_width(&best, s) + split.as_ref().map_or(0.0, |(t, _)| Overlay::text_width(t, s * 2.0));
            let (x, y) = (w / 2.0 - tw / 2.0, 30.0 * s);
            let x = x + o.text(x, y + 3.0 * s, s, [200, 210, 235, 230], &best);
            if let Some((text, col)) = split {
                o.text(x, y, s * 2.0, col, &text);
            }
        }

        // Fly-bys: letterbox and a hint.
        if self.game.cutscene.is_some() {
            o.rect(0.0, 0.0, w, h * 0.1, [0, 0, 0, 255]);
            o.rect(0.0, h * 0.9, w, h * 0.1, [0, 0, 0, 255]);
            let hint = "Space to skip";
            o.text(w - Overlay::text_width(hint, s) - 12.0 * s, h * 0.95 - 4.0 * s, s, [200, 200, 190, 200], hint);
        }

        // The minimap (top right): the city from above around Darci, north up, with the
        // mission's beacons as gold dots (pinned to the edge when further off), enemies in a
        // fight as red dots and Darci as a white one; the beacons are listed underneath.
        if self.game.cutscene.is_none() {
            if let Some(pl) = self.game.player() {
                let driving = self.game.player_vehicle();
                let (me, facing) = match driving {
                    Some(v) => (v.pos, v.forward()),
                    None => (pl.fig.pos, pl.facing_dir()),
                };
                let me = vec2(me.x, me.z) / uc_world::UNIT;
                // Squares from the middle to the edge.
                let reach = if driving.is_some() { 24.0 } else { 12.0 };
                let size_px = 100.0 * s;
                let (mx, my) = (w - size_px - 10.0 * s, 10.0 * s);
                let span = uc_formats::iam::MAP_SIZE as f32;
                // A frame (rectangles are drawn over the map image, so only round its edge).
                let (b, frame) = (2.0 * s, [0, 0, 0, 220]);
                o.rect(mx - b, my - b, size_px + 2.0 * b, b, frame);
                o.rect(mx - b, my + size_px, size_px + 2.0 * b, b, frame);
                o.rect(mx - b, my, b, size_px, frame);
                o.rect(mx + size_px, my, b, size_px, frame);
                o.minimap(mx, my, size_px, size_px,
                          [(me.x - reach) / span, (me.y - reach) / span, (me.x + reach) / span, (me.y + reach) / span],
                          [255, 255, 255, 255]);
                // A point (squares) on the map, pinned inside its edge.
                let at = |p: Vec2| {
                    let d = (p - me) / reach;
                    let k = d.abs().max_element().max(1.0);
                    (vec2(mx, my) + (d / k * 0.94 + Vec2::ONE) * 0.5 * size_px, k > 1.0)
                };
                let dot = |o: &mut Overlay, p: Vec2, r: f32, col: [u8; 4]| {
                    o.rect(p.x - r - s, p.y - r - s, 2.0 * (r + s), 2.0 * (r + s), [0, 0, 0, 220]);
                    o.rect(p.x - r, p.y - r, 2.0 * r, 2.0 * r, col);
                };
                for q in self.game.people.iter().filter(|q| q.aggro() && !q.is_dead()) {
                    let (p, outside) = at(vec2(q.fig.pos.x, q.fig.pos.z) / uc_world::UNIT);
                    if !outside {
                        dot(&mut o, p, 1.5 * s, [230, 60, 50, 255]);
                    }
                }
                let gold = [255, 210, 70, 255];
                let mut y = my + size_px + 6.0 * s;
                for (label, pos) in self.game.beacons() {
                    let (p, _) = at(vec2(pos.x, pos.z) / uc_world::UNIT);
                    dot(&mut o, p, 2.0 * s, gold);
                    let dist = (pl.fig.pos - pos).length() / uc_world::UNIT;
                    let text = format!("{} {dist:.0}", label.trim_end_matches('.'));
                    o.text(w - Overlay::text_width(&text, s) - 10.0 * s, y, s, gold, &text);
                    y += 10.0 * s;
                }
                // The console's `bonus`: what's left to find, in light blue, numbered for `telb`.
                if self.console.show_bonus {
                    let blue = [120, 200, 255, 255];
                    for (n, (label, pos)) in self.bonus_points().into_iter().enumerate() {
                        let (p, _) = at(vec2(pos.x, pos.z) / uc_world::UNIT);
                        dot(&mut o, p, 2.0 * s, blue);
                        let dist = (pl.fig.pos - pos).length() / uc_world::UNIT;
                        let text = format!("{}: {label} {dist:.0}", n + 1);
                        o.text(w - Overlay::text_width(&text, s) - 10.0 * s, y, s, blue, &text);
                        y += 10.0 * s;
                    }
                }
                // Darci, with a tick the way she (or her car) faces.
                let (c, _) = at(me);
                for k in 1..=3 {
                    let t = c + facing * (k as f32 * 1.5 * s);
                    o.rect(t.x - s * 0.75, t.y - s * 0.75, 1.5 * s, 1.5 * s, [255, 255, 255, 255]);
                }
                dot(&mut o, c, 2.0 * s, [255, 255, 255, 255]);
            }
        }

        // Mission messages, newest at the bottom, each with the speaker's face beside it
        // (`PANEL_new_text_draw`). A filmed conversation has the widescreen panel instead.
        let ms = s;
        let max_w = (w * 0.7).min(900.0 * s);
        let mut lines = Vec::new();
        let mut faces = Vec::new();
        if self.game.wide.is_none() {
            let shown: Vec<_> = self.game.messages.iter().filter(|m| !m.filmed).collect();
            for m in &shown[shown.len().saturating_sub(2)..] {
                if let Some(f) = m.face {
                    faces.push((lines.len(), f));
                }
                lines.extend(Overlay::wrap(&m.text, max_w, ms));
                lines.push(String::new());
            }
            lines.pop();
        }
        if !lines.is_empty() {
            let lh = 11.0 * ms;
            let total = lines.len() as f32 * lh;
            let y0 = h - total - 24.0 * s;
            let bx = w / 2.0 - max_w / 2.0 - 10.0 * s;
            o.rect(bx, y0 - 6.0 * s, max_w + 20.0 * s, total + 10.0 * s, [0, 0, 0, 150]);
            for (k, line) in lines.iter().enumerate() {
                o.text(w / 2.0 - Overlay::text_width(line, ms) / 2.0, y0 + k as f32 * lh, ms, white, line);
            }
            // The small faces, 32 pixels on a 480-line screen.
            let fs = 32.0 * h / 480.0;
            for (k, f) in faces {
                let (sheet, uv) = uc_game::talk::face_uv(f, false);
                o.face(sheet, bx - fs - 4.0 * s, y0 + k as f32 * lh - 6.0 * s, fs, fs, uv);
            }
        }

        // A filmed conversation: widescreen bars, one speaker's face top right and the other's
        // bottom left, and the line beside whoever is saying it (`PANEL_new_widescreen`).
        if let Some(wd) = &self.game.wide {
            let k = h / 480.0;
            let bar = 80.0 * k;
            o.rect(0.0, 0.0, w, bar, [0, 0, 0, 255]);
            o.rect(0.0, h - bar, w, bar, [0, 0, 0, 255]);
            let size = 64.0 * k;
            if let Some(f) = wd.top {
                let (sheet, uv) = uc_game::talk::face_uv(f, true);
                o.face(sheet, w - 72.0 * k, 8.0 * k, size, size, uv);
            }
            if let Some(f) = wd.bottom {
                let (sheet, uv) = uc_game::talk::face_uv(f, true);
                o.face(sheet, 8.0 * k, h - 72.0 * k, size, size, uv);
            }
            let room = w - 80.0 * k - 80.0 * k;
            let lines = Overlay::wrap(&wd.text, room, s);
            let lh = 11.0 * s;
            let col = [255, 255, 255, 255];
            if wd.top_talking {
                // Right-aligned against the face, its last line at the foot of the top bar.
                let y0 = bar - 5.0 * k - lines.len() as f32 * lh;
                for (n, line) in lines.iter().enumerate() {
                    o.text(w - 80.0 * k - Overlay::text_width(line, s), y0 + n as f32 * lh, s, col, line);
                }
            } else {
                for (n, line) in lines.iter().enumerate() {
                    o.text(80.0 * k, h - bar + 5.0 * k + n as f32 * lh, s, col, line);
                }
            }
        }

        // An endless run: the wave, who's left of it, the score; or the breather's countdown.
        if let Some(e) = self.game.endless() {
            let gold = [255, 210, 70, 255];
            let line = match e.breather() {
                Some(t) => format!("WAVE {} IN {:.0}", e.wave + 1, t.ceil()),
                None => format!("WAVE {}   LEFT {}", e.wave, e.left(&self.game)),
            };
            let big = s * 2.0;
            o.text(w / 2.0 - Overlay::text_width(&line, big) / 2.0, 8.0 * s, big, gold, &line);
            let score = format!("SCORE {}", e.score);
            o.text(w / 2.0 - Overlay::text_width(&score, s) / 2.0, 26.0 * s, s, white, &score);
        }

        // Achievements just earned.
        for (k, (text, _)) in self.popups.iter().enumerate() {
            let y = h * 0.16 + k as f32 * 16.0 * s;
            let tw = Overlay::text_width(text, s);
            o.rect(w / 2.0 - tw / 2.0 - 8.0 * s, y - 4.0 * s, tw + 16.0 * s, 14.0 * s, [0, 0, 0, 190]);
            o.text(w / 2.0 - tw / 2.0, y, s, [255, 210, 70, 255], text);
        }

        // A shift: its clock, the score and the tally, and what's on.
        if let Some(b) = self.game.beat() {
            let gold = [255, 210, 70, 255];
            // Time left, or for an open-ended shift the time on duty.
            let (what, secs) = match b.shift {
                Some(left) => ("SHIFT", left.max(0.0) as u32),
                None => ("ON DUTY", b.on as u32),
            };
            let line = format!("{what} {}:{:02}   SCORE {}", secs / 60, secs % 60, b.score);
            let big = s * 2.0;
            o.text(w / 2.0 - Overlay::text_width(&line, big) / 2.0, 8.0 * s, big, gold, &line);
            let tally = format!("HANDLED {}   MISSED {}", b.handled, b.missed);
            o.text(w / 2.0 - Overlay::text_width(&tally, s) / 2.0, 26.0 * s, s, white, &tally);
            for (k, inc) in b.incidents.iter().enumerate() {
                let t = format!("{}  {:.0}s", inc.kind.label().to_uppercase(), inc.left.max(0.0));
                let col = if inc.left < 20.0 { [255, 120, 100, 255] } else { white };
                o.text(w / 2.0 - Overlay::text_width(&t, s) / 2.0, 38.0 * s + k as f32 * 10.0 * s, s, col, &t);
            }
        }

        // Outcome. (In a replay, without the keys to play on.)
        let keys = |k: &'static str| if self.replay.is_some() { "" } else { k };
        if let (Some(b), Some(out)) = (self.game.beat(), &self.game.outcome) {
            let (title, col) = match out {
                Outcome::Won => ("SHIFT OVER".to_string(), [120, 230, 120, 255]),
                Outcome::Lost(why) => (format!("SHIFT LOST: {}", why.to_uppercase()), [240, 90, 70, 255]),
            };
            let best = match self.endless_recorded {
                Some((true, _)) => "A new best!".to_string(),
                Some((false, Some((sc, n)))) => format!("Best {sc} ({n} handled)"),
                _ => String::new(),
            };
            let sub = format!("Handled {}   Missed {}   Score {}   {best}   {}", b.handled, b.missed, b.score, keys("R: again   Esc: menu"));
            let big = s * 3.0;
            o.rect(0.0, h * 0.33, w, 50.0 * s, [0, 0, 0, 160]);
            o.text(w / 2.0 - Overlay::text_width(&title, big) / 2.0, h * 0.33 + 8.0 * s, big, col, &title);
            o.text(w / 2.0 - Overlay::text_width(&sub, s) / 2.0, h * 0.33 + 36.0 * s, s, white, &sub);
        } else if let (Some(e), Some(Outcome::Lost(why))) = (self.game.endless(), &self.game.outcome) {
            let title = format!("RUN OVER: WAVE {}", e.wave);
            let best = match self.endless_recorded {
                Some((true, _)) => "A new best!".to_string(),
                Some((false, Some((b, bw)))) => format!("Best {b} (wave {bw})"),
                _ => String::new(),
            };
            let sub = format!("{why}. Score {}   Arrests {}   Knockouts {}   {best}   {}", e.score, e.arrests, e.downed, keys("R: again   Esc: menu"));
            let big = s * 3.0;
            o.rect(0.0, h * 0.33, w, 50.0 * s, [0, 0, 0, 160]);
            o.text(w / 2.0 - Overlay::text_width(&title, big) / 2.0, h * 0.33 + 8.0 * s, big, [255, 210, 70, 255], &title);
            o.text(w / 2.0 - Overlay::text_width(&sub, s) / 2.0, h * 0.33 + 36.0 * s, s, white, &sub);
        } else if let Some(out) = &self.game.outcome {
            let (title, sub, col) = match out {
                Outcome::Won => {
                    // The time, and how it compares with the best before.
                    let time = match self.result {
                        Some((t, Some(b))) if t < b => format!("Time {}: a new best (was {})   ", progress::clock(t), progress::clock(b)),
                        Some((t, Some(b))) => format!("Time {} (best {})   ", progress::clock(t), progress::clock(b)),
                        Some((t, None)) => format!("Time {}   ", progress::clock(t)),
                        None => String::new(),
                    };
                    let time = match &self.race {
                        Some(r) if r.saved => format!("{time}New ghost saved   "),
                        _ => time,
                    };
                    let (badges, sides) = self.tally();
                    let found = if self.goals.badges.is_empty() && self.goals.sides.is_empty() {
                        String::new()
                    } else {
                        format!("Badges {badges}/{}   Side objectives {sides}/{}   ", self.goals.badges.len(), self.goals.sides.len())
                    };
                    ("MISSION COMPLETE".to_string(), format!("{time}{found}{}", keys("Enter: next mission   R: play again   Esc: menu")), [120, 230, 120, 255])
                }
                Outcome::Lost(why) => ("MISSION FAILED".to_string(), format!("{why}. {}", keys("Press R to try again")), [240, 90, 70, 255]),
            };
            let big = s * 3.0;
            o.rect(0.0, h * 0.33, w, 50.0 * s, [0, 0, 0, 160]);
            o.text(w / 2.0 - Overlay::text_width(&title, big) / 2.0, h * 0.33 + 8.0 * s, big, col, &title);
            o.text(w / 2.0 - Overlay::text_width(&sub, s) / 2.0, h * 0.33 + 36.0 * s, s, white, &sub);
            // The chief's word on the civilians killed (`deadcivs.tga`'s message).
            if let Some(warning) = &self.civ_warning {
                let lines = Overlay::wrap(warning, w * 0.6, s);
                let top = h * 0.33 + 56.0 * s;
                o.rect(w * 0.18, top - 4.0 * s, w * 0.64, (lines.len() as f32 * 11.0 + 8.0) * s, [60, 0, 0, 190]);
                for (k, l) in lines.iter().enumerate() {
                    o.text(w / 2.0 - Overlay::text_width(l, s) / 2.0, top + k as f32 * 11.0 * s, s, [255, 220, 200, 255], l);
                }
            }
        }

        // The run's replay, saved once it's over.
        if let (Some((path, kept)), Some(_)) = (&self.replay_saved, &self.game.outcome) {
            let name = path.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
            let text = if *kept { format!("Replay kept: {name}") } else { format!("Replay saved: {name}   K: keep it") };
            o.text(w / 2.0 - Overlay::text_width(&text, s) / 2.0, h * 0.33 - 12.0 * s, s, [200, 200, 190, 220], &text);
        }

        // Tab held: the mission's checklist, ticked as things get done (side objectives and
        // badges from earlier wins count too).
        if self.checklist {
            let won = self.game.outcome == Some(Outcome::Won);
            let (badges, _) = self.tally();
            let done = self.game.sides_done(&self.goals);
            let items = checklist::items(&self.objectives, won, &self.goals, badges,
                                         |id| self.kept.sides.contains(&id) || done.contains(&id));
            let pw = (w * 0.5).max(300.0 * s).min(w - 20.0 * s);
            let bw = pw - 24.0 * s;
            let ph = 52.0 * s + checklist::height(&items, bw, s);
            let (px, py) = (w / 2.0 - pw / 2.0, (h * 0.18).min(h - ph - 10.0 * s));
            o.rect(px, py, pw, ph, [0, 0, 0, 200]);
            let gold = [255, 210, 70, 255];
            o.text(px + 12.0 * s, py + 10.0 * s, s * 2.0, gold, &self.title);
            let time = progress::clock(self.game.time);
            o.text(px + pw - Overlay::text_width(&time, s) - 12.0 * s, py + 14.0 * s, s, white, &time);
            checklist::draw(&mut o, px + 12.0 * s, py + 34.0 * s, bw, s, &items);
        }

        // The controls, for the first 20 seconds (H toggles them).
        if self.help && !self.checklist && self.game.cutscene.is_none() && (self.game.time < 20.0 || self.help_forced) && self.replay.is_none() {
            for (k, line) in self.help_lines.iter().enumerate() {
                o.text(12.0 * s, 44.0 * s + k as f32 * 10.0 * s, s, [200, 200, 190, 200], line);
            }
        }
        self.replay_overlay(&mut o, w, h, s);
        self.draw_console(&mut o, w, h, s);
        o
    }
}
