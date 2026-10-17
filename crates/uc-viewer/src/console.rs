//! The developer console (#106, the original's F9 `parse_console`, Controls.cpp:238): `~`
//! or F9 opens a line to type a command into; Enter runs it, Esc closes it. Also the
//! free camera's teleport (T puts Darci where it looks), the original's full-kit button
//! combination, and the position readout (Shift+F12, the original's `cheat = 2`).

use crate::*;

const HELP: &str = "telw N (to event point N)  tels / telr (store / return)  wpt (nearest point)  \
win  lose  restart  roper  darci  kit  boo  pos  god  ammo  freeze  give WHAT  kill [near]  \
crime N  time X  wave N  bonus  telb N  car  call WHAT  autopilot";

#[derive(Default)]
pub(crate) struct Console {
    /// The line being typed, while the console is open.
    pub(crate) line: Option<String>,
    /// What the console has said, with seconds left on screen.
    pub(crate) log: Vec<(String, f32)>,
    /// Where `tels` stored Darci.
    pub(crate) stored: Option<Vec3>,
    /// The position readout is on.
    pub(crate) show_pos: bool,
    /// `bonus`: the bonus objectives left are on the map.
    pub(crate) show_bonus: bool,
    /// `time`: the game runs this much faster (or slower) than normal.
    pub(crate) time_scale: Option<f32>,
}

impl Console {
    fn say(&mut self, s: impl Into<String>) {
        self.log.push((s.into(), 6.0));
        if self.log.len() > 6 {
            self.log.remove(0);
        }
    }
}

impl World {
    /// The console's keys this frame. While it's open it takes the keyboard: whatever was
    /// pressed is used up here, so the game sees nothing. Returns whether it's open.
    pub(crate) fn console_input(&mut self, inp: &mut Input, dt: f32) -> bool {
        for l in &mut self.console.log {
            l.1 -= dt;
        }
        self.console.log.retain(|l| l.1 > 0.0);
        let shift = inp.held(KeyCode::ShiftLeft) || inp.held(KeyCode::ShiftRight);
        if shift && inp.hit(KeyCode::F12) {
            inp.pressed.remove(&KeyCode::F12);
            self.console.show_pos = !self.console.show_pos;
        }
        // The original's full kit: punch, kick, action, jump and right held together.
        let kit = [KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyE, KeyCode::Space, KeyCode::KeyD];
        if kit.iter().all(|&k| inp.held(k)) && kit.iter().any(|&k| inp.hit(k)) {
            self.game.full_kit(&self.level);
            self.console.say("Full kit");
        }
        let toggle = inp.hit(KeyCode::Backquote) || inp.hit(KeyCode::F9);
        let Some(mut line) = self.console.line.take() else {
            if toggle {
                self.console.line = Some(String::new());
                inp.pressed.clear();
                inp.typed.clear();
                return true;
            }
            return false;
        };
        if toggle || inp.hit(KeyCode::Escape) {
            inp.pressed.clear();
            inp.down.clear();
            return false;
        }
        line.extend(inp.typed.chars().filter(|c| !c.is_control() && *c != '`' && *c != '~'));
        if inp.hit(KeyCode::Backspace) {
            line.pop();
        }
        let run = inp.hit(KeyCode::Enter) || inp.hit(KeyCode::NumpadEnter);
        inp.pressed.clear();
        inp.down.clear();
        if run {
            let cmd = line.trim().to_string();
            if !cmd.is_empty() {
                self.console.say(format!("> {cmd}"));
                self.run_command(&cmd);
            }
            return false;
        }
        self.console.line = Some(line);
        true
    }

    fn run_command(&mut self, cmd: &str) {
        let mut words = cmd.split_whitespace();
        let name = words.next().unwrap_or("").to_ascii_lowercase();
        let arg = words.next();
        let here = self.game.player().map(|p| p.fig.pos);
        match name.as_str() {
            "help" | "?" => self.console.say(HELP),
            "telw" => match arg.and_then(|a| a.parse::<usize>().ok()) {
                Some(n) => match Game::point_pos(&self.level, n) {
                    Some(at) if self.game.teleport(at, &self.level) => self.console.say(format!("At event point {n}")),
                    Some(_) => self.console.say("Not while driving"),
                    None => self.console.say(format!("No event point {n}")),
                },
                None => self.console.say("telw N: teleport to event point N"),
            },
            "tels" => {
                self.console.stored = here;
                self.console.say("Position stored");
            }
            "telr" => match self.console.stored {
                Some(at) if self.game.teleport(at, &self.level) => self.console.say("Back to the stored position"),
                Some(_) => self.console.say("Not while driving"),
                None => self.console.say("Nothing stored: tels first"),
            },
            "wpt" => match self.game.nearest_point(&self.level) {
                Some((n, d)) => self.console.say(format!("Nearest event point {n}, {:.1} squares away", d / uc_world::UNIT)),
                None => self.console.say("No event points"),
            },
            "win" => self.game.end_mission(true),
            "lose" => self.game.end_mission(false),
            "restart" => self.restart(),
            "roper" | "darci" => {
                let kind = if name == "roper" { uc_world::character::person::ROPER } else { uc_world::character::person::DARCI };
                if self.game.play_as(kind, &self.level) {
                    self.console.say(format!("Playing as {name}"));
                } else {
                    self.console.say(format!("Can't play as {name} here (his body isn't in this mission)"));
                }
            }
            "kit" => {
                self.game.full_kit(&self.level);
                self.console.say("Full kit");
            }
            "boo" => self.game.boo(),
            "god" => {
                self.game.cheats.god = !self.game.cheats.god;
                self.console.say(if self.game.cheats.god { "God mode on" } else { "God mode off" });
            }
            "ammo" => {
                self.game.cheats.ammo = !self.game.cheats.ammo;
                self.console.say(if self.game.cheats.ammo { "Unlimited ammo on" } else { "Unlimited ammo off" });
            }
            "freeze" => {
                self.game.cheats.freeze = !self.game.cheats.freeze;
                self.console.say(if self.game.cheats.freeze { "Everyone else frozen" } else { "Unfrozen" });
            }
            "give" => match arg.map(str::to_ascii_lowercase) {
                Some(what) if self.game.give(&what) => self.console.say(format!("Given {what}")),
                _ => self.console.say("give pistol|shotgun|ak|sniper|knife|bat|grenades|explosives|health|key"),
            },
            "kill" => {
                let near = arg.is_some_and(|a| a.eq_ignore_ascii_case("near"));
                let n = self.game.knock_out_enemies(near.then_some(10.0 * uc_world::UNIT));
                self.console.say(format!("{n} knocked out"));
            }
            "crime" => match arg.and_then(|a| a.parse::<u8>().ok()) {
                Some(n) => {
                    self.game.set_crime_rate(n);
                    self.console.say(format!("Crime rate {}", self.game.crime_rate));
                }
                None => self.console.say("crime N: set the crime rate (0-100)"),
            },
            "wave" => match (arg.and_then(|a| a.parse::<u32>().ok()), self.game.endless_mut()) {
                (Some(n), Some(e)) if n > 0 => {
                    e.wave = n - 1;
                    e.phase = uc_game::game::endless::Phase::Breather(0.1);
                    self.console.say(format!("On to wave {n}"));
                }
                (_, None) => self.console.say("wave N: only in an endless run"),
                _ => self.console.say("wave N: jump to wave N"),
            },
            "time" => match arg.and_then(|a| a.parse::<f32>().ok()) {
                Some(x) => {
                    self.console.time_scale = Some(x.clamp(0.1, 4.0)).filter(|&x| x != 1.0);
                    self.console.say(format!("Time x{:.2}", x.clamp(0.1, 4.0)));
                }
                None => self.console.say("time X: game speed, 0.1 to 4 (1 is normal)"),
            },
            "bonus" => {
                self.console.show_bonus = !self.console.show_bonus;
                let open = self.bonus_points();
                if open.is_empty() {
                    self.console.say("No bonus objectives or badges left here");
                }
                for (n, (label, at)) in open.iter().enumerate().take(5) {
                    let d = here.map_or(0.0, |h| (*at - h).length() / uc_world::UNIT);
                    self.console.say(format!("{}: {label} ({d:.0} squares)", n + 1));
                }
                self.console.say(if self.console.show_bonus { "Shown on the map in light blue; telb N to go there" } else { "Hidden from the map" });
            }
            "telb" => match arg.and_then(|a| a.parse::<usize>().ok()) {
                Some(n) => match self.bonus_points().get(n.wrapping_sub(1)).cloned() {
                    Some((label, at)) if self.game.teleport(at, &self.level) => self.console.say(format!("At {label}")),
                    Some(_) => self.console.say("Not while driving"),
                    None => self.console.say(format!("No bonus objective {n}: bonus lists them")),
                },
                None => self.console.say("telb N: teleport to bonus objective N (see bonus)"),
            },
            "car" => {
                let made = self.game.spawn_car(&self.level);
                self.console.say(if made { "A car, just ahead" } else { "Only on foot" });
            }
            "call" => {
                use uc_game::game::beat::Kind;
                match (arg.and_then(Kind::named), self.game.beat().is_some()) {
                    (_, false) => self.console.say("call WHAT: only on a Beat Cop shift"),
                    (Some(kind), true) if self.game.call_in_now(kind, &self.level) => self.console.say(format!("{} called in", kind.label())),
                    (Some(kind), true) => self.console.say(format!("Nowhere for a {} round here", kind.label().to_ascii_lowercase())),
                    (None, true) => {
                        let names: Vec<_> = Kind::ALL.iter().map(|k| k.label().split(' ').next().unwrap_or("").to_ascii_lowercase()).collect();
                        self.console.say(format!("call {}", names.join("|")));
                    }
                }
            }
            "autopilot" => {
                let on = self.game.autopilot();
                self.console.say(if on { "Darci's on autopilot: bot against bot" } else { "autopilot: only in capture the flag" });
            }
            "pos" => self.console.show_pos = !self.console.show_pos,
            _ => self.console.say(format!("Unknown command '{name}': try help")),
        }
    }

    /// The bonus objectives still open, for `bonus` and `telb`: the side objectives not yet
    /// done (at the point whose triggering does them) and the badges not yet found (where
    /// their point is).
    pub(crate) fn bonus_points(&self) -> Vec<(String, Vec3)> {
        let done = self.game.sides_done(&self.goals);
        let sides = self.goals.sides.iter().filter(|g| !done.contains(&g.id))
            .filter_map(|g| Some((g.label.clone(), Game::point_pos(&self.level, g.id)?)));
        let badges = self.goals.badges.iter().filter(|b| !self.game.found.contains(b) && !self.kept.badges.contains(b))
            .filter_map(|&b| Some((format!("Badge {b}"), Game::point_pos(&self.level, b)?)));
        sides.chain(badges).collect()
    }

    /// The free camera's teleport: Darci goes where the camera is looking. Along the line
    /// of sight, the first wall it meets puts her on whatever is on top just beyond it (a
    /// building's roof), and the first ground or roof it reaches puts her there. Looking at
    /// nothing within 60 squares, she goes under the camera.
    pub(crate) fn teleport_to_view(&mut self) {
        let eye = self.fly.pos * uc_world::UNIT;
        let dir = self.fly.forward();
        let flat = vec3(dir.x, 0.0, dir.z).normalize_or_zero();
        let step = 32.0;
        let mut prev = eye;
        let mut to = None;
        for k in 1..(60.0 * uc_world::UNIT / step) as usize {
            let p = eye + dir * step * k as f32;
            if let Some(t) = self.level.ground.first_hit(prev, p) {
                let beyond = prev + (p - prev) * t + flat * 48.0;
                let top = self.level.ground.surface(&self.level.map, beyond.x, beyond.z, 1.0e5);
                to = Some(vec3(beyond.x, top, beyond.z));
                break;
            }
            let s = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 1.0);
            if p.y <= s {
                to = Some(vec3(p.x, s, p.z));
                break;
            }
            prev = p;
        }
        let to = to.unwrap_or_else(|| {
            let s = self.level.ground.surface(&self.level.map, eye.x, eye.z, eye.y);
            vec3(eye.x, s, eye.z)
        });
        if self.game.teleport(to, &self.level) {
            self.third_person = true;
            self.console.say("Teleported");
        } else {
            self.console.say("Not while driving");
        }
    }

    /// The console's line, what it has said, and the position readout.
    pub(crate) fn draw_console(&self, o: &mut Overlay, w: f32, h: f32, s: f32) {
        let white = [235, 235, 225, 255];
        let line_h = 9.0 * s;
        let mut y = h - line_h * 2.5;
        if let Some(line) = &self.console.line {
            o.rect(0.0, y - s * 2.0, w, line_h + s * 4.0, [0, 0, 0, 200]);
            o.text(s * 6.0, y, s, [120, 230, 120, 255], &format!("> {line}_"));
        }
        for (text, _) in self.console.log.iter().rev() {
            y -= line_h * 1.2;
            o.text(s * 6.0, y, s, white, text);
        }
        if self.console.show_pos {
            if let Some(p) = self.game.player() {
                let at = p.fig.pos / uc_world::UNIT;
                let near = self.game.nearest_point(&self.level).map_or(String::new(), |(n, _)| format!("   nearest point {n}"));
                let text = format!("x {:.2}  y {:.2}  z {:.2}{near}   {:.0}s", at.x, at.y, at.z, self.game.time);
                o.text(s * 6.0, h * 0.42, s, [255, 230, 120, 255], &text);
            }
        }
    }
}
