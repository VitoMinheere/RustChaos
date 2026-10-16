//! The developer console (#106, the original's F9 `parse_console`, Controls.cpp:238): `~`
//! or F9 opens a line to type a command into; Enter runs it, Esc closes it. Also the
//! free camera's teleport (T puts Darci where it looks), the original's full-kit button
//! combination, and the position readout (Shift+F12, the original's `cheat = 2`).

use crate::*;

const HELP: &str = "telw N (to event point N)  tels / telr (store / return)  wpt (nearest point)  \
win  lose  restart  roper  darci  kit  boo  pos";

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
            "pos" => self.console.show_pos = !self.console.show_pos,
            _ => self.console.say(format!("Unknown command '{name}': try help")),
        }
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
