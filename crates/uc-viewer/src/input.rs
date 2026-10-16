//! The keyboard, mouse and gamepad.

use crate::*;

/// Gamepad state (the first connected pad), with deadzones applied.
#[derive(Default)]
pub(crate) struct Pad {
    /// Left stick, y up.
    pub(crate) mv: Vec2,
    /// Right stick, y up.
    pub(crate) look: Vec2,
    pub(crate) held: HashSet<Button>,
    pub(crate) pressed: HashSet<Button>,
}

impl Pad {
    pub(crate) fn held(&self, b: Button) -> bool {
        self.held.contains(&b)
    }
    pub(crate) fn hit(&self, b: Button) -> bool {
        self.pressed.contains(&b)
    }
}

pub(crate) fn deadzone(v: Vec2) -> Vec2 {
    let len = v.length();
    if len < 0.2 { Vec2::ZERO } else { v / len * ((len - 0.2) / 0.8).min(1.0) }
}

#[derive(Default)]
pub(crate) struct Input {
    pub(crate) down: HashSet<KeyCode>,
    pub(crate) pressed: HashSet<KeyCode>,
    pub(crate) rmb: bool,
    /// The mouse turns the camera without a button held (the cursor is captured).
    pub(crate) mouse_look: bool,
    pub(crate) lmb_pressed: bool,
    pub(crate) lmb: bool,
    pub(crate) mouse: Vec2,
    pub(crate) wheel: f32,
    pub(crate) pad: Pad,
    /// The window has the keyboard and mouse.
    pub(crate) focused: bool,
    /// The cursor is captured now.
    pub(crate) captured: bool,
    /// Seconds to capturing it again anyway: Windows lets go of a confined cursor when the
    /// window moves or another program confines one, without telling.
    pub(crate) regrab: f32,
    /// Seconds a click is still taken as the one that gave the window back its focus, which
    /// mustn't also punch or shoot.
    pub(crate) swallow_click: f32,
}

impl Input {
    pub(crate) fn held(&self, k: KeyCode) -> bool {
        self.down.contains(&k)
    }
    pub(crate) fn hit(&self, k: KeyCode) -> bool {
        self.pressed.contains(&k)
    }
}

impl App {
    /// Whether the cursor should be captured: playing (not at the menu), with mouse look on,
    /// in the window that has the focus.
    pub(crate) fn want_cursor(&self) -> bool {
        self.input.focused && self.input.mouse_look && self.menu.is_none() && self.world.is_some()
    }

    /// Captures the cursor or gives it back as `want_cursor` says, and captures it again
    /// every second while it should be, in case the system has let go of it.
    pub(crate) fn sync_cursor(&mut self, dt: f32) {
        let want = self.want_cursor();
        self.input.regrab -= dt;
        self.input.swallow_click = (self.input.swallow_click - dt).max(0.0);
        if want != self.input.captured || (want && self.input.regrab <= 0.0) {
            self.capture_cursor(want);
        }
    }

    /// Captures (hides and locks) the cursor for mouse look, or gives it back.
    pub(crate) fn capture_cursor(&mut self, on: bool) {
        self.input.captured = on;
        self.input.regrab = 1.0;
        let Some(g) = &self.gfx else { return };
        if on {
            // Locked keeps the cursor in place; some platforms only support confining it.
            if g.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                let _ = g.window.set_cursor_grab(CursorGrabMode::Confined);
            }
        } else {
            let _ = g.window.set_cursor_grab(CursorGrabMode::None);
        }
        g.window.set_cursor_visible(!on);
    }

    /// Reads the gamepad: button presses since last frame, sticks and held buttons.
    pub(crate) fn poll_pad(&mut self) {
        let Some(gilrs) = self.pads.as_mut() else { return };
        let pad = &mut self.input.pad;
        pad.pressed.clear();
        while let Some(ev) = gilrs.next_event() {
            if let gilrs::EventType::ButtonPressed(b, _) = ev.event {
                pad.pressed.insert(b);
            }
        }
        pad.held.clear();
        (pad.mv, pad.look) = (Vec2::ZERO, Vec2::ZERO);
        if let Some((_, g)) = gilrs.gamepads().next() {
            pad.mv = deadzone(vec2(g.value(Axis::LeftStickX), g.value(Axis::LeftStickY)));
            pad.look = deadzone(vec2(g.value(Axis::RightStickX), g.value(Axis::RightStickY)));
            for b in [Button::South, Button::East, Button::North, Button::West, Button::RightTrigger, Button::RightTrigger2,
                      Button::LeftTrigger, Button::LeftTrigger2] {
                if g.is_pressed(b) {
                    pad.held.insert(b);
                }
            }
            // The d-pad moves too.
            let d = vec2(g.is_pressed(Button::DPadRight) as i32 as f32 - g.is_pressed(Button::DPadLeft) as i32 as f32,
                         g.is_pressed(Button::DPadUp) as i32 as f32 - g.is_pressed(Button::DPadDown) as i32 as f32);
            if d != Vec2::ZERO {
                pad.mv = d.normalize() * 0.6;
            }
        }
    }
}
