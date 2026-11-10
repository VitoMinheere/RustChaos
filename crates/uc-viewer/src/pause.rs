//! The pause menu.

use crate::*;

/// The in-game menus (`GAMEMENU_menu`, fallen/Source/gamemenu.cpp:97): paused, lost, and
/// making sure before abandoning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PauseMenu {
    Paused,
    /// Paused on an unlimited Beat Cop shift (#520), which is ended from here.
    OnShift,
    /// Paused in the sparring room (#547), where the opponent is told what to do.
    Sparring,
    Lost,
    /// "Are you sure?", and the menu to go back to.
    Sure(bool),
}

impl PauseMenu {
    /// The menu for pausing `w`: with END SHIFT on an unlimited shift.
    pub(crate) fn paused(w: &World) -> PauseMenu {
        let clockless = w.game.beat().is_some_and(|b| b.length == uc_game::game::beat::Shift::Unlimited && w.game.outcome.is_none());
        if w.game.spar().is_some() {
            PauseMenu::Sparring
        } else if clockless && w.replay.is_none() {
            PauseMenu::OnShift
        } else {
            PauseMenu::Paused
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            PauseMenu::Paused | PauseMenu::OnShift => "GAME PAUSED",
            PauseMenu::Sparring => "SPARRING",
            PauseMenu::Lost => "LEVEL LOST",
            PauseMenu::Sure(_) => "ARE YOU SURE?",
        }
    }

    pub(crate) fn items(self) -> &'static [&'static str] {
        match self {
            PauseMenu::Paused => &["RESUME LEVEL", "MOVES", "RESTART LEVEL", "ABANDON GAME"],
            PauseMenu::OnShift => &["RESUME LEVEL", "MOVES", "END SHIFT", "RESTART LEVEL", "ABANDON GAME"],
            PauseMenu::Sparring => &["RESUME LEVEL", "MOVES", "OPPONENT", "OPPONENTS", "RESTART LEVEL", "ABANDON GAME"],
            PauseMenu::Lost => &["RESTART LEVEL", "ABANDON GAME"],
            PauseMenu::Sure(_) => &["OKAY", "CANCEL"],
        }
    }
}

/// The in-game menu up: which, the choice, and how long it's been up (the game slows to a
/// stop over its first second and the screen darkens, `GAMEMENU_slowdown` and
/// `GAMEMENU_background`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Pause {
    pub(crate) menu: PauseMenu,
    pub(crate) sel: usize,
    pub(crate) up: f32,
}

impl Pause {
    pub(crate) fn new(menu: PauseMenu) -> Pause {
        Pause { menu, sel: 0, up: 0.0 }
    }

    /// The in-game menu over the game: the screen darkening over 0.64 s, the menu sliding in
    /// once it's under way (`GAMEMENU_fadein_x`), the choice picked out.
    pub(crate) fn overlay(&self, o: &mut Overlay, size: (u32, u32), opponent: Option<(&str, bool)>) {
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).round().max(1.0);
        let dark = (self.up / 0.64).min(1.0);
        o.rect(0.0, 0.0, w, h, [0, 0, 0, (170.0 * dark) as u8]);
        if self.up < 0.4 {
            return;
        }
        let slide = ((self.up - 0.4) / 0.25).min(1.0);
        let x = w * 0.5 - (1.0 - slide) * w * 0.5;
        let (gold, white) = ([255, 200, 60, 255], [235, 235, 225, 255]);
        let title = self.menu.title();
        let big = s * 3.0;
        let mut y = h * 0.35;
        o.text(x - Overlay::text_width(title, big) / 2.0, y, big, gold, title);
        y += 40.0 * s;
        for (i, item) in self.menu.items().iter().enumerate() {
            // What the sparring room's opponent does, beside his row.
            let label = match opponent {
                Some((does, _)) if *item == "OPPONENT" => format!("OPPONENT: {does}"),
                Some((_, two)) if *item == "OPPONENTS" => format!("OPPONENTS: {}", if two { "TWO" } else { "ONE" }),
                _ => item.to_string(),
            };
            let item = &label.as_str();
            let tw = Overlay::text_width(item, s * 2.0);
            if i == self.sel {
                o.rect(x - tw / 2.0 - 6.0 * s, y - 3.0 * s, tw + 12.0 * s, 20.0 * s, [200, 150, 40, 200]);
            }
            o.text(x - tw / 2.0, y, s * 2.0, white, item);
            y += 24.0 * s;
        }
    }
}

impl App {
    /// Closes the move list: back to wherever it was opened from, the pause menu, the main
    /// menu or the sparring. Whether it left the game.
    pub(crate) fn close_moves(&mut self) -> bool {
        let Some(w) = self.world.as_mut() else { return false };
        let Some(page) = w.moves.take() else { return false };
        w.game.stand_in = None;
        match page.from {
            moves::From::Pause => false,
            // Straight back to the sparring.
            moves::From::Game => {
                w.pause = None;
                if let Some(s) = self.sound.as_mut() {
                    s.resume();
                }
                false
            }
            moves::From::Menu => {
                self.input.pressed.clear();
                self.open_menu();
                if let Some(m) = self.menu.as_mut() {
                    m.main = menu::MAIN.iter().position(|&r| r == "MOVES").unwrap_or(0);
                }
                true
            }
        }
    }

    /// The in-game menu a frame (`GAMEMENU_process`): the game slowing to a stop over the
    /// first second, then up and down to choose and Enter to pick, once it's been up a fifth
    /// of a second. Returns whether it left the mission.
    pub(crate) fn pause_frame(&mut self, dt: f32) -> bool {
        self.poll_pad();
        let inp = &self.input;
        let up = inp.hit(KeyCode::ArrowUp) || inp.hit(KeyCode::KeyW) || inp.pad.hit(Button::DPadUp);
        let down = inp.hit(KeyCode::ArrowDown) || inp.hit(KeyCode::KeyS) || inp.pad.hit(Button::DPadDown);
        let choose = inp.hit(KeyCode::Enter) || inp.hit(KeyCode::NumpadEnter) || inp.hit(KeyCode::Space) || inp.pad.hit(Button::South);
        let pad_used = !inp.pad.pressed.is_empty();
        // The move list over it (#547): its own keys, until it's closed.
        if let Some(mut page) = self.world.as_mut().and_then(|w| w.moves.take()) {
            let w = self.world.as_mut().unwrap();
            // (In co-op the game goes on meanwhile, as it does under the pause menu.)
            if let Some(c) = self.coop.as_mut() {
                c.frame(w, dt, &Controls::default(), false);
            }
            match page.frame(w, inp, dt) {
                moves::Left::Stay => w.moves = Some(page),
                moves::Left::Close => {
                    w.moves = Some(page);
                    return self.close_moves();
                }
                // Off to try it: the room waits for the move.
                moves::Left::Try(tech) => {
                    w.game.stand_in = None;
                    w.game.spar_want(Some(tech));
                    w.pause = None;
                    if let Some(s) = self.sound.as_mut() {
                        s.resume();
                    }
                }
            }
            return false;
        }
        let Some(w) = self.world.as_mut() else { return false };
        let Some(mut p) = w.pause else { return false };
        let slow = (1.0 - p.up).max(0.0);
        // In co-op the game goes on for the other player: this one just stands still.
        if let Some(c) = self.coop.as_mut() {
            c.frame(w, dt, &Controls::default(), false);
        } else if slow > 0.0 && w.game.outcome.is_none() {
            w.game.update(dt * slow, &Controls::default(), &w.level);
        }
        p.up += dt;
        let n = p.menu.items().len();
        let ready = p.up > 0.2;
        if ready && up {
            p.sel = (p.sel + n - 1) % n;
        }
        if ready && down {
            p.sel = (p.sel + 1) % n;
        }
        w.pause = Some(p);
        if !(ready && choose) {
            return false;
        }
        match p.menu.items()[p.sel] {
            "MOVES" => w.moves = Some(moves::Page::new(moves::From::Pause, pad_used)),
            // The next thing for him to do.
            "OPPONENT" => {
                use uc_game::game::spar::Stance;
                let now = w.game.spar().map_or(Stance::Still, |s| s.stance);
                let k = Stance::ALL.iter().position(|&s| s == now).unwrap_or(0);
                w.game.spar_stance(Stance::ALL[(k + 1) % Stance::ALL.len()]);
            }
            // One man or two (two for the kicks to the side and behind).
            "OPPONENTS" => {
                let two = w.game.spar().is_some_and(|s| s.two);
                w.game.spar_two(!two);
            }
            "RESUME LEVEL" => {
                w.pause = None;
                if let Some(s) = self.sound.as_mut() {
                    s.resume();
                }
            }
            // In co-op the host starts it over for both.
            "RESTART LEVEL" if self.coop.is_some() => {
                w.pause = None;
                if let Some(c) = self.coop.as_mut() {
                    match (c.is_host(), c.setup.clone()) {
                        (true, Some(s)) => c.client.choose(s),
                        _ => c.notices.push(("Only the host can restart the mission".into(), 4.0)),
                    }
                }
                if let Some(s) = self.sound.as_mut() {
                    s.resume();
                }
            }
            "RESTART LEVEL" => {
                w.restart();
                w.pause = None;
                if !w.third_person {
                    w.fly = FlyCam::from(w.start);
                }
                if let Some(s) = self.sound.as_mut() {
                    s.audio.hush();
                    s.resume();
                }
            }
            // Off duty: the shift over, its summary up.
            "END SHIFT" => {
                w.pause = None;
                w.game.end_shift();
                if let Some(r) = w.recorder.as_mut() {
                    r.event(&mut w.game, uc_net::replay::Event::EndShift);
                }
                if let Some(s) = self.sound.as_mut() {
                    s.resume();
                }
            }
            "ABANDON GAME" => w.pause = Some(Pause::new(PauseMenu::Sure(p.menu == PauseMenu::Lost))),
            "OKAY" => {
                self.input.pressed.clear();
                self.open_menu();
                return true;
            }
            _ => {
                let back = if p.menu == PauseMenu::Sure(true) { PauseMenu::Lost } else { PauseMenu::paused(w) };
                w.pause = Some(Pause::new(back));
            }
        }
        false
    }
}
