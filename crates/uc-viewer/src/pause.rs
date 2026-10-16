//! The pause menu.

use crate::*;

/// The in-game menus (`GAMEMENU_menu`, fallen/Source/gamemenu.cpp:97): paused, lost, and
/// making sure before abandoning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PauseMenu {
    Paused,
    Lost,
    /// "Are you sure?", and the menu to go back to.
    Sure(bool),
}

impl PauseMenu {
    pub(crate) fn title(self) -> &'static str {
        match self {
            PauseMenu::Paused => "GAME PAUSED",
            PauseMenu::Lost => "LEVEL LOST",
            PauseMenu::Sure(_) => "ARE YOU SURE?",
        }
    }

    pub(crate) fn items(self) -> &'static [&'static str] {
        match self {
            PauseMenu::Paused => &["RESUME LEVEL", "RESTART LEVEL", "ABANDON GAME"],
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
    pub(crate) fn overlay(&self, o: &mut Overlay, size: (u32, u32)) {
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
    /// Back to the mission list.
    /// The in-game menu a frame (`GAMEMENU_process`): the game slowing to a stop over the
    /// first second, then up and down to choose and Enter to pick, once it's been up a fifth
    /// of a second. Returns whether it left the mission.
    pub(crate) fn pause_frame(&mut self, dt: f32) -> bool {
        self.poll_pad();
        let inp = &self.input;
        let up = inp.hit(KeyCode::ArrowUp) || inp.hit(KeyCode::KeyW) || inp.pad.hit(Button::DPadUp);
        let down = inp.hit(KeyCode::ArrowDown) || inp.hit(KeyCode::KeyS) || inp.pad.hit(Button::DPadDown);
        let choose = inp.hit(KeyCode::Enter) || inp.hit(KeyCode::NumpadEnter) || inp.hit(KeyCode::Space) || inp.pad.hit(Button::South);
        let Some(w) = self.world.as_mut() else { return false };
        let Some(mut p) = w.pause else { return false };
        let slow = (1.0 - p.up).max(0.0);
        if slow > 0.0 && w.game.outcome.is_none() {
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
            "RESUME LEVEL" => {
                w.pause = None;
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
            "ABANDON GAME" => w.pause = Some(Pause::new(PauseMenu::Sure(p.menu == PauseMenu::Lost))),
            "OKAY" => {
                self.input.pressed.clear();
                self.open_menu();
                return true;
            }
            _ => {
                let back = if p.menu == PauseMenu::Sure(true) { PauseMenu::Lost } else { PauseMenu::Paused };
                w.pause = Some(Pause::new(back));
            }
        }
        false
    }
}
