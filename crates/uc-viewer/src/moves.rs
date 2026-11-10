//! The move list (#547): every move from `uc_game::moves`, with how it's done written in the
//! keys or buttons it's bound to now, and the player's own figure showing it beside the
//! list. Opened from the pause menu, and from the main menu over the sparring room, where
//! Enter on a move asks the room to wait for it. Not in the original.

use crate::*;
use uc_game::moves::{Entry, Group, Tech, MOVES};
use uc_world::Figure;

/// The map the sparring room is on: the combat tutorial's.
pub(crate) const ROOM: &str = "ftutor1";

/// The list up over the paused game.
pub(crate) struct Page {
    pub(crate) sel: usize,
    /// How long it's been up.
    up: f32,
    /// Where it was opened from, and closing it goes back to.
    pub(crate) from: From,
    /// The moves written for a gamepad, as the last thing pressed was one.
    pad: bool,
    /// The figure showing the move, and which.
    fig: Option<(Figure, Tech)>,
}

/// Where the list was opened from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum From {
    /// The pause menu's MOVES.
    Pause,
    /// The main menu's MOVES, over the sparring room.
    Menu,
    /// Tab in the sparring room.
    Game,
}

/// What a frame of the list came to.
pub(crate) enum Left {
    Stay,
    /// Closed: back to where it was opened from.
    Close,
    /// A move chosen in the sparring room, to try.
    Try(Tech),
}

/// The action a name in braces stands for.
fn act(name: &str) -> Option<Act> {
    Some(match name {
        "forward" => Act::Forward,
        "back" => Act::Back,
        "left" => Act::Left,
        "right" => Act::Right,
        "run" => Act::Run,
        "punch" => Act::Punch,
        "kick" => Act::Kick,
        "action" => Act::Action,
        "jump" => Act::Jump,
        "block" => Act::Block,
        "crouch" => Act::Crouch,
        "strafe" => Act::Strafe,
        "weapon" => Act::Weapon,
        "target" => Act::Target,
        "look" => Act::Look,
        "handbrake" => Act::Handbrake,
        "siren" => Act::Siren,
        _ => return None,
    })
}

/// `text` with each `{action}` as the key it's on (or, for a gamepad, the button, the stick
/// for the ways to go). Running needs no key where going forward runs already: the Classic
/// layout, and a stick pushed all the way.
pub(crate) fn keys(text: &str, b: &bindings::Bindings, pad: bool) -> String {
    let text = if pad || b.classic { text.replace("{run}+", "") } else { text.to_string() };
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find('{') {
        out += &rest[..open];
        let Some(close) = rest[open..].find('}') else { break };
        let name = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        let Some(a) = act(name) else { continue };
        let bind = b.of(a);
        out += &match (pad, bind.pads.first(), a) {
            (true, Some(&button), _) => bindings::button_name(button).to_string(),
            (true, None, Act::Forward) => "STICK UP".to_string(),
            (true, None, Act::Back) => "STICK DOWN".to_string(),
            (true, None, Act::Left) => "STICK LEFT".to_string(),
            (true, None, Act::Right) => "STICK RIGHT".to_string(),
            _ => bind.keys.first().map_or("-".to_string(), |&k| bindings::key_name(k)),
        };
    }
    out + rest
}

/// `text` broken into lines no wider than `width` at `scale`.
fn wrap(text: &str, width: f32, scale: f32) -> Vec<String> {
    let mut lines = vec![String::new()];
    for word in text.split_whitespace() {
        let line = lines.last_mut().unwrap();
        let longer = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if Overlay::text_width(&longer, scale) > width && !line.is_empty() {
            lines.push(word.to_string());
        } else {
            *line = longer;
        }
    }
    lines
}

impl Page {
    pub(crate) fn new(from: From, pad: bool) -> Page {
        Page { sel: 0, up: 0.0, from, pad, fig: None }
    }

    fn entry(&self) -> &'static Entry {
        &MOVES[self.sel.min(MOVES.len() - 1)]
    }

    /// A frame of the list: up and down through it, Enter in the sparring room to try the
    /// move, and the figure going through the one chosen.
    pub(crate) fn frame(&mut self, w: &mut World, inp: &Input, dt: f32) -> Left {
        self.up += dt;
        if !inp.pad.pressed.is_empty() || inp.pad.mv.length() > 0.5 {
            self.pad = true;
        } else if !inp.pressed.is_empty() {
            self.pad = false;
        }
        let up = inp.hit(KeyCode::ArrowUp) || inp.hit(KeyCode::KeyW) || inp.pad.hit(Button::DPadUp);
        let down = inp.hit(KeyCode::ArrowDown) || inp.hit(KeyCode::KeyS) || inp.pad.hit(Button::DPadDown);
        let choose = inp.hit(KeyCode::Enter) || inp.hit(KeyCode::NumpadEnter) || inp.hit(KeyCode::Space) || inp.pad.hit(Button::South);
        let back = inp.pad.hit(Button::East) || inp.hit(KeyCode::Backspace);
        let n = MOVES.len();
        if up {
            self.sel = (self.sel + n - 1) % n;
        }
        if down {
            self.sel = (self.sel + 1) % n;
        }
        if inp.hit(KeyCode::PageUp) || inp.pad.hit(Button::LeftTrigger) {
            self.sel = self.section(-1);
        }
        if inp.hit(KeyCode::PageDown) || inp.pad.hit(Button::RightTrigger) {
            self.sel = self.section(1);
        }
        self.show(w, dt);
        // (Not on the press that opened it.)
        if self.up < 0.2 {
            return Left::Stay;
        }
        if back {
            return Left::Close;
        }
        if choose && w.game.spar().is_some() && self.entry().sparred {
            return Left::Try(self.entry().tech);
        }
        Left::Stay
    }

    /// The first move of the section before (`by` -1) or after (1) the one the choice is in.
    fn section(&self, by: i32) -> usize {
        let all = Group::ALL;
        let now = all.iter().position(|&g| g == self.entry().group).unwrap_or(0) as i32;
        let to = all[(now + by).rem_euclid(all.len() as i32) as usize];
        MOVES.iter().position(|m| m.group == to).unwrap_or(0)
    }

    /// The player's figure, where she stands, going through the move chosen over and over;
    /// drawn in her place.
    pub(crate) fn show(&mut self, w: &mut World, dt: f32) {
        let tech = self.entry().tech;
        let Some(p) = w.game.player().filter(|p| !p.aboard()) else {
            (self.fig, w.game.stand_in) = (None, None);
            return;
        };
        if self.fig.as_ref().is_none_or(|f| f.1 != tech) {
            let mut fig = p.fig.clone();
            (fig.lift, fig.cancel_rise, fig.pin_root) = (0.0, false, false);
            // (A character without the animation stands as they were.)
            fig.restart_now(self.entry().anim, uc_world::Play::Loop);
            fig.set_speed(1.0);
            self.fig = Some((fig, tech));
        }
        if let Some((fig, _)) = self.fig.as_mut() {
            fig.advance(dt);
            w.game.stand_in = Some(fig.clone());
        }
    }

    /// The camera on the figure: from in front and a little to one side, from whichever way
    /// round there's most room, with her to the right of the list.
    pub(crate) fn camera(&self, w: &World) -> Option<Camera> {
        let (fig, _) = self.fig.as_ref()?;
        let unit = uc_world::UNIT;
        let at = fig.pos / unit + vec3(0.0, 0.45, 0.0);
        // The first place with nothing between it and her: as far back as there's room for,
        // then round her from the front. With no such place the game's own camera stays.
        let from = |dist: f32, turn: f32| {
            let d = uc_world::character::facing(fig.angle + turn);
            at + vec3(d.x, 0.15, d.y).normalize() * dist
        };
        let clear = |p: Vec3| w.level.ground.line_of_sight(at * unit, p * unit) && w.clear_of_buildings(at, p) == p;
        let pos = [2.1f32, 1.5, 1.1]
            .into_iter()
            .flat_map(|dist| [0.5f32, -0.5, 1.2, -1.2, 2.2, -2.2, std::f32::consts::PI].into_iter().map(move |turn| (dist, turn)))
            .map(|(dist, turn)| from(dist, turn))
            .find(|&p| clear(p))?;
        let forward = (at - pos).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        // Off to the camera's left, so she's to the right of the middle.
        let aside = right * (pos - at).length() * -0.3;
        Some(Camera { pos: pos + aside, target: at + aside, fov_y: 60f32.to_radians(), ortho: None })
    }

    /// The list down the left, a section at a time, and the chosen move's name, how it's
    /// done and when it works at the bottom right, under the figure.
    pub(crate) fn overlay(&self, o: &mut Overlay, size: (u32, u32), w: &World, b: &bindings::Bindings) {
        let (sw, sh) = (size.0 as f32, size.1 as f32);
        let s = (sh / 360.0).round().max(1.0);
        let (gold, white, dim, green) = ([255, 210, 70, 255], [235, 235, 225, 255], [150, 150, 140, 255], [120, 220, 120, 255]);
        let panel = (sw * 0.44).min(300.0 * s);
        o.rect(0.0, 0.0, panel, sh, [0, 0, 0, 215]);
        let x = 14.0 * s;
        o.text(x, 10.0 * s, s * 3.0, gold, "MOVES");
        let spar = w.game.spar();
        if let Some(sp) = spar {
            let tally = format!("{}/{} DONE", sp.done.len(), uc_game::moves::sparred());
            o.text(panel - Overlay::text_width(&tally, s) - 10.0 * s, 20.0 * s, s, green, &tally);
        }
        // The rows: a heading before each section's first move.
        let mut rows: Vec<(Option<usize>, &str)> = Vec::new();
        for (i, m) in MOVES.iter().enumerate() {
            if i == 0 || MOVES[i - 1].group != m.group {
                rows.push((None, m.group.name()));
            }
            rows.push((Some(i), m.name));
        }
        let lh = 11.0 * s;
        let top = 40.0 * s;
        let fit = (((sh - top - 18.0 * s) / lh) as usize).max(3);
        let chosen = rows.iter().position(|r| r.0 == Some(self.sel)).unwrap_or(0);
        let first = chosen.saturating_sub(fit / 2).min(rows.len().saturating_sub(fit));
        for (k, &(i, name)) in rows.iter().enumerate().skip(first).take(fit) {
            let y = top + (k - first) as f32 * lh;
            let Some(i) = i else {
                o.text(x, y, s, gold, name);
                continue;
            };
            if i == self.sel {
                o.rect(x + 4.0 * s, y - 2.0 * s, panel - x - 14.0 * s, lh, [200, 150, 40, 200]);
            }
            let m = &MOVES[i];
            // In the sparring room: a tick by each move done, a dash by those it can't see.
            let mark = match spar {
                Some(sp) if sp.done.contains(&m.tech) => Some(("+", green)),
                Some(_) if !m.sparred => Some(("-", dim)),
                _ => None,
            };
            if let Some((mark, col)) = mark {
                o.text(x + 8.0 * s, y, s, col, mark);
            }
            o.text(x + 18.0 * s, y, s, white, name);
        }
        let keys_line = match (spar.is_some(), self.pad) {
            (true, false) => "Up/Down choose   PgUp/PgDn section   Enter try it   Esc back",
            (true, true) => "D-pad choose   LB/RB section   A try it   B back",
            (false, false) => "Up/Down choose   PgUp/PgDn section   Esc back",
            (false, true) => "D-pad choose   LB/RB section   B back",
        };
        o.text(x, sh - 12.0 * s, s, dim, keys_line);

        // The chosen move.
        let m = self.entry();
        let (bx, bw) = (panel + 12.0 * s, sw - panel - 24.0 * s);
        let how = wrap(&keys(m.input, b, self.pad), bw - 20.0 * s, s * 2.0);
        let when = wrap(&keys(m.when, b, self.pad), bw - 20.0 * s, s);
        let waited = spar.is_some_and(|sp| sp.want == Some(m.tech));
        let note = match spar {
            Some(sp) if sp.done.contains(&m.tech) => Some(("Done.", green)),
            Some(_) if waited => Some(("The room's waiting for this one.", gold)),
            Some(_) if uc_game::game::spar::Spar::takes_two(m.tech) => Some(("A second man comes in for this one, and keeps where it lands.", dim)),
            Some(_) if !m.sparred => Some(("Not one the room can see done.", dim)),
            _ => None,
        };
        let bh = 34.0 * s + how.len() as f32 * 20.0 * s + when.len() as f32 * 10.0 * s + if note.is_some() { 12.0 * s } else { 0.0 };
        let by = sh - bh - 12.0 * s;
        o.rect(bx, by, bw, bh, [0, 0, 0, 200]);
        let mut y = by + 8.0 * s;
        o.text(bx + 10.0 * s, y, s * 2.0, gold, m.name);
        y += 22.0 * s;
        for line in &how {
            o.text(bx + 10.0 * s, y, s * 2.0, white, line);
            y += 20.0 * s;
        }
        for line in &when {
            o.text(bx + 10.0 * s, y, s, [200, 200, 190, 255], line);
            y += 10.0 * s;
        }
        if let Some((text, col)) = note {
            o.text(bx + 10.0 * s, y + 2.0 * s, s, col, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The moves are written in the keys bound now: the viewer's own, a rebound one, the
    /// Classic layout's (where forward runs, so sliding needs no run key) and a gamepad's.
    #[test]
    fn moves_are_written_in_the_bound_keys() {
        let mut b = bindings::Bindings::modern();
        assert_eq!(keys("{punch} {punch} {punch}", &b, false), "J J J");
        assert_eq!(keys("{run}+{forward}, then {kick}", &b, false), "LEFT SHIFT+W, then K");
        assert_eq!(keys("hold {block}, then {kick}", &b, false), "hold Q, then K");
        b.set_key(Act::Kick, KeyCode::KeyP);
        assert_eq!(keys("{back}+{kick}", &b, false), "S+P");
        let classic = bindings::Bindings::classic();
        assert_eq!(keys("{run}+{forward}, then {kick}", &classic, false), "UP, then X");
        assert_eq!(keys("{forward}+{punch}", &classic, false), "UP+Z");
        assert_eq!(keys("{run}+{forward}, then {kick}", &b, true), "STICK UP, then Y");
        assert_eq!(keys("{left}+{jump}  or  {right}+{jump}", &b, true), "STICK LEFT+A  or  STICK RIGHT+A");
        assert_eq!(keys("hold {block}", &b, true), "hold LB");
    }

    /// Every move's text names only actions there are, and none is left in braces.
    #[test]
    fn every_move_can_be_written() {
        for b in [bindings::Bindings::modern(), bindings::Bindings::classic()] {
            for pad in [false, true] {
                for m in &MOVES {
                    for text in [keys(m.input, &b, pad), keys(m.when, &b, pad)] {
                        assert!(!text.contains('{') && !text.contains('}'), "{}: {text}", m.name);
                    }
                }
            }
        }
    }

    #[test]
    fn long_text_is_wrapped() {
        let lines = wrap("one two three four five six seven eight nine ten", Overlay::text_width("one two three", 1.0), 1.0);
        assert!(lines.len() >= 3 && lines.iter().all(|l| !l.is_empty()), "{lines:?}");
        assert_eq!(lines.join(" "), "one two three four five six seven eight nine ten");
    }
}
