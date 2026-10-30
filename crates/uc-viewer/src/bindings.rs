//! The controls, remappable (#202; the manual's Controller Configuration, p. 24): which keys
//! and gamepad buttons do what, kept in `bindings.txt` beside the progress file. Two layouts
//! to start from: the viewer's own (WASD and the mouse) and the original's (the arrows, Z X C
//! and Space, `keybrd_button_use`, fallen/Source/interfac.cpp:340).

use crate::*;

/// What a key does in play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Act {
    Forward,
    Back,
    Left,
    Right,
    /// Run held (Modern), or walk held (Classic, where moving runs).
    Run,
    Punch,
    Kick,
    Action,
    Jump,
    Block,
    Crouch,
    /// Held: the ways to go are hers, not the camera's (#309): sidesteps, walking
    /// backwards, the back flip and turning on the spot.
    Strafe,
    Weapon,
    Target,
    Scope,
    /// Held: look round (the original's first-person key).
    Look,
    CamLeft,
    CamRight,
    CamBehind,
    /// The camera behind her or flying free.
    View,
    Handbrake,
    Siren,
}

pub(crate) const ACTS: [Act; 22] = [
    Act::Forward, Act::Back, Act::Left, Act::Right, Act::Run, Act::Punch, Act::Kick, Act::Action, Act::Jump, Act::Block,
    Act::Crouch, Act::Strafe, Act::Weapon, Act::Target, Act::Scope, Act::Look, Act::CamLeft, Act::CamRight, Act::CamBehind, Act::View,
    Act::Handbrake, Act::Siren,
];

/// The controls page's rows: the layout, each action, and back.
pub(crate) const ROWS: usize = ACTS.len() + 2;

impl Act {
    pub(crate) fn name(self, classic: bool) -> &'static str {
        match self {
            Act::Forward => "FORWARD",
            Act::Back => "BACK",
            Act::Left => if classic { "TURN LEFT" } else { "LEFT" },
            Act::Right => if classic { "TURN RIGHT" } else { "RIGHT" },
            Act::Run => if classic { "WALK" } else { "RUN" },
            Act::Punch => "PUNCH / FIRE",
            Act::Kick => "KICK",
            Act::Action => "ACTION",
            Act::Jump => "JUMP",
            Act::Block => "BLOCK",
            Act::Crouch => "CROUCH",
            Act::Strafe => "STRAFE",
            Act::Weapon => "INVENTORY",
            Act::Target => "NEXT TARGET",
            Act::Scope => "SNIPER SCOPE",
            Act::Look => "LOOK AROUND",
            Act::CamLeft => "CAMERA LEFT",
            Act::CamRight => "CAMERA RIGHT",
            Act::CamBehind => "CAMERA BEHIND",
            Act::View => "FREE CAMERA",
            Act::Handbrake => "CAR: HANDBRAKE",
            Act::Siren => "CAR: SIREN",
        }
    }

    /// Its name in the file.
    fn key(self) -> &'static str {
        match self {
            Act::Forward => "forward",
            Act::Back => "back",
            Act::Left => "left",
            Act::Right => "right",
            Act::Run => "run",
            Act::Punch => "punch",
            Act::Kick => "kick",
            Act::Action => "action",
            Act::Jump => "jump",
            Act::Block => "block",
            Act::Crouch => "crouch",
            Act::Strafe => "strafe",
            Act::Weapon => "inventory",
            Act::Target => "target",
            Act::Scope => "scope",
            Act::Look => "look",
            Act::CamLeft => "cam_left",
            Act::CamRight => "cam_right",
            Act::CamBehind => "cam_behind",
            Act::View => "view",
            Act::Handbrake => "handbrake",
            Act::Siren => "siren",
        }
    }
}

/// An action's keys and buttons.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Bind {
    pub(crate) keys: Vec<KeyCode>,
    pub(crate) pads: Vec<Button>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Bindings {
    /// The original's way of moving: Left and Right turn, Forward runs and Run held walks.
    pub(crate) classic: bool,
    binds: Vec<Bind>,
}

impl Default for Bindings {
    fn default() -> Bindings {
        Bindings::modern()
    }
}

use KeyCode as K;

impl Bindings {
    fn from(classic: bool, keys: [&[KeyCode]; 22]) -> Bindings {
        let pads: [&[Button]; 22] = [
            &[], &[], &[], &[], &[], &[Button::West], &[Button::North], &[Button::East], &[Button::South],
            &[Button::LeftTrigger, Button::LeftTrigger2], &[Button::DPadDown], &[Button::RightTrigger2], &[Button::RightTrigger], &[Button::RightThumb],
            &[Button::LeftThumb], &[], &[], &[], &[], &[Button::Select], &[Button::South], &[Button::North],
        ];
        let binds = keys.iter().zip(pads).map(|(k, p)| Bind { keys: k.to_vec(), pads: p.to_vec() }).collect();
        Bindings { classic, binds }
    }

    /// The viewer's own layout: WASD to move about the camera, the mouse to look.
    pub(crate) fn modern() -> Bindings {
        Bindings::from(false, [
            &[K::KeyW], &[K::KeyS], &[K::KeyA], &[K::KeyD], &[K::ShiftLeft, K::ShiftRight], &[K::KeyJ], &[K::KeyK],
            &[K::KeyE, K::KeyL], &[K::Space], &[K::KeyQ], &[K::KeyC, K::ControlLeft], &[K::KeyX], &[K::KeyG], &[K::KeyT], &[K::KeyZ],
            &[K::KeyF], &[K::Delete], &[K::PageDown], &[K::End], &[K::KeyV], &[K::Space], &[K::KeyK, K::KeyY],
        ])
    }

    /// The original's (the manual's pp. 11-13, 16-17 and 21): the arrows to run and turn,
    /// Z punch, X kick, C action, Space jump, Enter the inventory, A held to look round and
    /// Space in a car for pursuit mode; V walks (`keyboard_run`), and End, Delete and Page
    /// Down turn the camera. Left Shift strafes (not the original's: #309).
    pub(crate) fn classic() -> Bindings {
        Bindings::from(true, [
            &[K::ArrowUp], &[K::ArrowDown], &[K::ArrowLeft], &[K::ArrowRight], &[K::KeyV], &[K::KeyZ], &[K::KeyX], &[K::KeyC],
            &[K::Space], &[K::KeyQ], &[K::ControlLeft], &[K::ShiftLeft], &[K::Enter], &[K::KeyT], &[K::KeyF], &[K::KeyA], &[K::Delete],
            &[K::PageDown], &[K::End], &[K::Home], &[K::KeyX], &[K::Space],
        ])
    }

    pub(crate) fn of(&self, a: Act) -> &Bind {
        &self.binds[ACTS.iter().position(|&x| x == a).unwrap_or(0)]
    }

    fn of_mut(&mut self, a: Act) -> &mut Bind {
        &mut self.binds[ACTS.iter().position(|&x| x == a).unwrap_or(0)]
    }

    /// `a` bound to `key` alone, its buttons kept.
    pub(crate) fn set_key(&mut self, a: Act, key: KeyCode) {
        self.of_mut(a).keys = vec![key];
    }

    /// `a` bound to `button` alone, its keys kept.
    pub(crate) fn set_pad(&mut self, a: Act, button: Button) {
        self.of_mut(a).pads = vec![button];
    }

    /// Some action is on `key`, which the viewer's own keys then leave to it.
    pub(crate) fn uses(&self, key: KeyCode) -> bool {
        self.binds.iter().any(|b| b.keys.contains(&key))
    }

    /// `a` shares a key with another action used at the same time: the car's two are only
    /// used driving, the fighting and the like only on foot.
    pub(crate) fn clashes(&self, a: Act) -> bool {
        let car_only = |x: Act| matches!(x, Act::Handbrake | Act::Siren);
        let foot_only = |x: Act| matches!(x, Act::Run | Act::Punch | Act::Kick | Act::Jump | Act::Block | Act::Crouch | Act::Strafe | Act::Weapon
            | Act::Target | Act::Scope | Act::Look);
        let mine = &self.of(a).keys;
        ACTS.iter().any(|&b| {
            let apart = (car_only(a) && foot_only(b)) || (foot_only(a) && car_only(b));
            b != a && !apart && self.of(b).keys.iter().any(|k| mine.contains(k))
        })
    }

    /// `a`'s first key, as the help names it.
    fn first(&self, a: Act) -> String {
        self.of(a).keys.first().map_or("-".into(), |&k| key_name(k))
    }

    /// The help's three lines, with the keys as they're bound.
    pub(crate) fn help(&self) -> Vec<String> {
        let k = |a| self.first(a);
        let (mv, run) = if self.classic {
            (format!("{} run   {}/{} turn   {} back", k(Act::Forward), k(Act::Left), k(Act::Right), k(Act::Back)), format!("{} walk", k(Act::Run)))
        } else {
            (format!("{}{}{}{} move", k(Act::Forward), k(Act::Left), k(Act::Back), k(Act::Right)), format!("{} run", k(Act::Run)))
        };
        vec![
            format!("{mv}   {run}   {} jump   {} action (hold: sprint)   {} crouch (move: crawl)", k(Act::Jump), k(Act::Action), k(Act::Crouch)),
            format!("{} / left mouse punch   {} kick   {} block   {} strafe   {} look around   M mouse look on/off", k(Act::Punch), k(Act::Kick), k(Act::Block), k(Act::Strafe), k(Act::Look)),
            format!("{} next weapon   {} next target   {} scope (sniper rifle)   Tab checklist   {} free camera (T: go there)   B ghost   ~ console   H hide help",
                    k(Act::Weapon), k(Act::Target), k(Act::Scope), k(Act::View)),
        ]
    }

    fn path() -> Option<PathBuf> {
        Some(progress::Progress::folder()?.join("bindings.txt"))
    }

    pub(crate) fn load() -> Bindings {
        Bindings::parse(&Bindings::path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default())
    }

    pub(crate) fn save(&self) {
        let Some(path) = Bindings::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.text());
    }

    /// From the file: `layout=classic` (or `modern`) first, the layout the rest change, then
    /// `punch=KeyZ Mouse... pad:West` lines, keys by their names and buttons after `pad:`.
    fn parse(text: &str) -> Bindings {
        let lines: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim(), v.trim())).collect();
        let classic = lines.iter().any(|&(k, v)| k == "layout" && v == "classic");
        let mut b = if classic { Bindings::classic() } else { Bindings::modern() };
        for (key, value) in lines {
            let Some(&a) = ACTS.iter().find(|a| a.key() == key) else { continue };
            let mut bind = Bind::default();
            for word in value.split_whitespace() {
                if let Some(p) = word.strip_prefix("pad:") {
                    bind.pads.extend(BUTTONS.iter().find(|b| format!("{b:?}") == p));
                } else {
                    bind.keys.extend(KEYS.iter().find(|k| format!("{k:?}") == word));
                }
            }
            *b.of_mut(a) = bind;
        }
        b
    }

    fn text(&self) -> String {
        let mut s = format!("layout={}\n", if self.classic { "classic" } else { "modern" });
        for (a, b) in ACTS.iter().zip(&self.binds) {
            let words: Vec<String> = b.keys.iter().map(|k| format!("{k:?}")).chain(b.pads.iter().map(|p| format!("pad:{p:?}"))).collect();
            s += &format!("{}={}\n", a.key(), words.join(" "));
        }
        s
    }
}

impl Input {
    /// `a`'s key or button held.
    pub(crate) fn on(&self, a: Act) -> bool {
        let b = self.bindings.of(a);
        b.keys.iter().any(|&k| self.held(k)) || b.pads.iter().any(|&p| self.pad.held(p))
    }

    /// `a`'s key or button pressed this frame.
    pub(crate) fn tap(&self, a: Act) -> bool {
        let b = self.bindings.of(a);
        b.keys.iter().any(|&k| self.hit(k)) || b.pads.iter().any(|&p| self.pad.hit(p))
    }

    /// 1 with `a` held, else 0.
    pub(crate) fn axis(&self, a: Act) -> f32 {
        if self.bindings.of(a).keys.iter().any(|&k| self.held(k)) { 1.0 } else { 0.0 }
    }

    /// One of the viewer's own keys (help, restart and the like) pressed, unless an action
    /// has been put on it.
    pub(crate) fn own(&self, k: KeyCode) -> bool {
        self.hit(k) && !self.bindings.uses(k)
    }

    /// One of the free camera's keys (T, its teleport) pressed. Flying free she takes no
    /// controls, so what's bound for play doesn't stand in its way (both layouts put the
    /// next target on T): only the key back to her does.
    pub(crate) fn free_cam(&self, k: KeyCode) -> bool {
        self.hit(k) && !self.bindings.of(Act::View).keys.contains(&k)
    }
}

/// A key's name on screen: `KeyZ` as Z, `ArrowUp` as UP, `ShiftLeft` as LEFT SHIFT.
pub(crate) fn key_name(k: KeyCode) -> String {
    let s = format!("{k:?}");
    let s = s.strip_prefix("Key").or(s.strip_prefix("Digit")).unwrap_or(&s).to_string();
    let s = s.strip_prefix("Arrow").map(str::to_string).unwrap_or(s);
    for side in ["Left", "Right"] {
        if let Some(base) = s.strip_suffix(side).filter(|b| !b.is_empty() && *b != "Page") {
            return format!("{} {}", side.to_uppercase(), base.to_uppercase());
        }
    }
    s.to_uppercase()
}

/// A button's name on screen, as on an Xbox pad.
pub(crate) fn button_name(b: Button) -> &'static str {
    match b {
        Button::South => "A",
        Button::East => "B",
        Button::West => "X",
        Button::North => "Y",
        Button::LeftTrigger => "LB",
        Button::RightTrigger => "RB",
        Button::LeftTrigger2 => "LT",
        Button::RightTrigger2 => "RT",
        Button::LeftThumb => "L3",
        Button::RightThumb => "R3",
        Button::Select => "BACK",
        Button::Start => "START",
        Button::DPadUp => "D-UP",
        Button::DPadDown => "D-DOWN",
        Button::DPadLeft => "D-LEFT",
        Button::DPadRight => "D-RIGHT",
        _ => "?",
    }
}

/// The buttons that can be bound (Start stays the pause).
pub(crate) const BUTTONS: [Button; 15] = [
    Button::South, Button::East, Button::West, Button::North, Button::LeftTrigger, Button::RightTrigger, Button::LeftTrigger2,
    Button::RightTrigger2, Button::LeftThumb, Button::RightThumb, Button::Select, Button::DPadUp, Button::DPadDown,
    Button::DPadLeft, Button::DPadRight,
];

/// The keys that can be bound: Escape stays the pause, the F keys and the backquote the
/// viewer's.
pub(crate) const KEYS: [KeyCode; 79] = [
    K::KeyA, K::KeyB, K::KeyC, K::KeyD, K::KeyE, K::KeyF, K::KeyG, K::KeyH, K::KeyI, K::KeyJ, K::KeyK, K::KeyL, K::KeyM,
    K::KeyN, K::KeyO, K::KeyP, K::KeyQ, K::KeyR, K::KeyS, K::KeyT, K::KeyU, K::KeyV, K::KeyW, K::KeyX, K::KeyY, K::KeyZ,
    K::Digit0, K::Digit1, K::Digit2, K::Digit3, K::Digit4, K::Digit5, K::Digit6, K::Digit7, K::Digit8, K::Digit9,
    K::ArrowUp, K::ArrowDown, K::ArrowLeft, K::ArrowRight, K::Space, K::Enter, K::Tab, K::Backspace, K::ShiftLeft,
    K::ShiftRight, K::ControlLeft, K::ControlRight, K::AltLeft, K::AltRight, K::Insert, K::Delete, K::Home, K::End,
    K::PageUp, K::PageDown, K::Minus, K::Equal, K::BracketLeft, K::BracketRight, K::Semicolon, K::Quote, K::Comma,
    K::Period, K::Slash, K::Backslash, K::Numpad0, K::Numpad1, K::Numpad2, K::Numpad3, K::Numpad4, K::Numpad5, K::Numpad6,
    K::Numpad7, K::Numpad8, K::Numpad9, K::NumpadEnter, K::NumpadAdd, K::NumpadSubtract,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// T teleports from the free camera in either layout, though both have the next
    /// target on it in play; with the free camera itself put on T, T is the way back.
    #[test]
    fn t_teleports_from_the_free_camera_in_either_layout() {
        for b in [Bindings::modern(), Bindings::classic()] {
            assert!(b.uses(K::KeyT), "the next target is on T");
            let mut inp = Input { bindings: b, ..Default::default() };
            inp.pressed.insert(K::KeyT);
            assert!(!inp.own(K::KeyT));
            assert!(inp.free_cam(K::KeyT));
            inp.bindings.set_key(Act::View, K::KeyT);
            assert!(!inp.free_cam(K::KeyT));
        }
    }

    #[test]
    fn saved_and_read_back_the_same() {
        for start in [Bindings::modern(), Bindings::classic()] {
            let mut b = start.clone();
            b.set_key(Act::Punch, K::KeyP);
            b.set_pad(Act::Kick, Button::DPadUp);
            assert_eq!(Bindings::parse(&b.text()), b);
            assert_ne!(b, start);
        }
        assert_eq!(Bindings::parse(""), Bindings::modern());
        assert_eq!(Bindings::parse("layout=classic\njump=Nonsense pad:West"), {
            let mut c = Bindings::classic();
            c.of_mut(Act::Jump).keys.clear();
            c.of_mut(Act::Jump).pads = vec![Button::West];
            c
        });
    }

    #[test]
    fn the_presets_bind_everything_to_keys_the_file_knows() {
        for b in [Bindings::modern(), Bindings::classic()] {
            for &a in &ACTS {
                assert!(!b.of(a).keys.is_empty(), "{a:?} has no key");
                for k in &b.of(a).keys {
                    assert!(KEYS.contains(k), "{k:?} can't be saved");
                }
                assert!(!b.clashes(a), "{a:?} shares a key ({})", b.classic);
            }
        }
    }

    #[test]
    fn keys_read_well() {
        assert_eq!(key_name(K::KeyZ), "Z");
        assert_eq!(key_name(K::ArrowUp), "UP");
        assert_eq!(key_name(K::ShiftLeft), "LEFT SHIFT");
        assert_eq!(key_name(K::PageDown), "PAGEDOWN");
        assert_eq!(key_name(K::Digit4), "4");
    }
}
