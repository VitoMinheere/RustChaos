//! The mission editor (#185): a mission's event points shown in the level, moved, changed,
//! added and deleted from the free camera, saved as the mission's JSON (`uc mission export`'s
//! form) to a missions folder next to the progress, and played from there.
//!
//! The mission being edited is kept as its JSON, the form `docs/MISSIONS.md` describes:
//! each point carries its own message or cutscene there, so adding and deleting points can't
//! put the messages out of step with them, and the game's form is made from it again
//! (`from_json`) to check it and to play it.

use crate::*;
use uc_formats::ucm::{has_text, names, waypoint, Mission, MAX_EVENT_POINTS};
use uc_formats::ucm_json::{from_json, to_json, EventPointJson, MissionJson, Named};

/// How many edits undo goes back.
const UNDO: usize = 200;
/// A square, in world units.
const SQUARE: f32 = 256.0;
/// How tall a point's marker stands, in world units.
const MARKER: f32 = 160.0;
/// How far away markers are drawn, and labelled, in squares.
const SEEN: f32 = 40.0;
const LABELLED: f32 = 8.0;
/// How near the middle of the screen (in pixels at 360 high) a marker is aimed at.
const AIM: f32 = 14.0;
/// The editor's keys, as the screen shows them.
const KEYS: [&str; 16] = [
    "WASD, Space and C: fly", "click or Enter: select", "Tab: next point", "G: put it here", "[ ]: turn", "N: new", "Ins: copy",
    "Del: delete", "F: filter", "Ctrl+Z / Ctrl+Y: undo / redo", "Ctrl+S: save", "P: next problem", "F5: play",
    "Shift+F5: play from here", "~ then ep {json}: change fields", "Esc: leave",
];
/// Triggers that go off within the point's radius: their circle is drawn.
const RADIUS_TRIGGERS: [&str; 5] = ["radius", "enemy_radius", "player_uses_radius", "thing_radius_dir", "move_radius_dir"];

/// The folder edited missions are saved in: `missions` next to the progress. Nothing is
/// ever written into the game's install.
pub(crate) fn folder() -> Option<PathBuf> {
    Some(progress::Progress::folder()?.join("missions"))
}

/// Where mission `name`'s edit is saved.
pub(crate) fn path(name: &str) -> Option<PathBuf> {
    Some(folder()?.join(format!("{}.json", name.to_ascii_lowercase())))
}

/// Mission `name`'s saved edit, as JSON, if there is one that reads.
pub(crate) fn saved(name: &str) -> Option<MissionJson> {
    let text = std::fs::read(path(name)?).ok()?;
    serde_json::from_slice(&text).map_err(|e| eprintln!("{name}: the saved edit doesn't read: {e}")).ok()
}

/// Which markers are drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Filter {
    All,
    /// The selected point, the points it depends on, and the points that depend on it.
    Linked,
    /// Points of the selected point's type.
    Kind,
}

/// Something wrong with the mission: the point it's about (by index), if it's about one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Problem {
    pub(crate) point: Option<usize>,
    pub(crate) text: String,
}

/// A mission being edited.
pub(crate) struct Editor {
    /// The mission's name, as the install has it.
    pub(crate) name: String,
    pub(crate) doc: MissionJson,
    undo: Vec<MissionJson>,
    redo: Vec<MissionJson>,
    /// The selected point, by its index.
    pub(crate) selected: Option<usize>,
    pub(crate) filter: Filter,
    /// Saved as it stands.
    pub(crate) saved: bool,
    pub(crate) problems: Vec<Problem>,
    /// What the editor last said, with seconds left on screen.
    pub(crate) said: Option<(String, f32)>,
    /// Esc was pressed once with the edits unsaved: again leaves without saving.
    pub(crate) leaving: bool,
    /// The last problem gone to (P goes to the next).
    next_problem: usize,
}

impl Editor {
    /// Starts editing mission `name`: its saved edit if there is one, else `shipped`.
    pub(crate) fn new(name: &str, shipped: &Mission) -> Editor {
        let (doc, from) = match saved(name) {
            Some(d) => (d, "the saved edit"),
            None => (to_json(shipped, false), "the shipped mission"),
        };
        let mut e = Editor::with(name, doc);
        e.say(format!("Editing {name}: {from}"));
        e
    }

    pub(crate) fn with(name: &str, doc: MissionJson) -> Editor {
        let mut e = Editor {
            name: name.to_string(), doc, undo: Vec::new(), redo: Vec::new(), selected: None, filter: Filter::All, saved: true,
            problems: Vec::new(), said: None, leaving: false, next_problem: 0,
        };
        e.problems = e.check();
        e
    }

    pub(crate) fn say(&mut self, s: impl Into<String>) {
        self.said = Some((s.into(), 5.0));
    }

    /// The mission in the game's form, to play.
    pub(crate) fn mission(&self) -> Result<Mission, String> {
        from_json(&self.doc)
    }

    pub(crate) fn point(&self, index: usize) -> Option<&EventPointJson> {
        self.doc.event_points.iter().find(|e| e.index == index)
    }

    /// Changes the mission, as one step undo takes back.
    pub(crate) fn edit(&mut self, f: impl FnOnce(&mut MissionJson)) {
        let before = self.doc.clone();
        f(&mut self.doc);
        if self.doc == before {
            return;
        }
        self.undo.push(before);
        if self.undo.len() > UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.saved = false;
        self.leaving = false;
        self.problems = self.check();
    }

    fn edit_point(&mut self, index: usize, f: impl FnOnce(&mut EventPointJson)) {
        self.edit(|d| {
            if let Some(e) = d.event_points.iter_mut().find(|e| e.index == index) {
                f(e);
            }
        });
    }

    pub(crate) fn undo(&mut self) -> bool {
        let Some(d) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(&mut self.doc, d));
        self.after_history();
        true
    }

    pub(crate) fn redo(&mut self) -> bool {
        let Some(d) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(&mut self.doc, d));
        self.after_history();
        true
    }

    fn after_history(&mut self) {
        self.saved = false;
        self.problems = self.check();
        if self.selected.is_some_and(|i| self.point(i).is_none()) {
            self.selected = None;
        }
    }

    /// Puts point `index` at `at` (world units).
    pub(crate) fn move_to(&mut self, index: usize, at: Vec3) {
        self.edit_point(index, |e| e.pos = [at.x.round() as i32, at.y.round() as i32, at.z.round() as i32]);
    }

    /// Turns point `index` by `by` 256ths of a turn.
    pub(crate) fn turn(&mut self, index: usize, by: i32) {
        self.edit_point(index, |e| e.direction = (e.direction as i32 + by).rem_euclid(256) as u8);
    }

    /// The lowest index no point has, nor refers to (a point deleted from under another
    /// mustn't come back as something else). Index 0 isn't used: a reference of 0 means none.
    fn free_index(&self) -> Option<usize> {
        let referred = |i: usize| self.doc.event_points.iter().any(|e| e.ep_ref as usize == i || e.ep_ref_bool as usize == i);
        (1..MAX_EVENT_POINTS).find(|&i| self.point(i).is_none() && !referred(i))
    }

    /// A new point at `at`, a copy of point `like` if given, else a plain one (`simple`, set
    /// off by nothing): its index, or None when all the slots are taken.
    pub(crate) fn add(&mut self, at: Vec3, like: Option<usize>) -> Option<usize> {
        let index = self.free_index()?;
        let mut e = like.and_then(|i| self.point(i).cloned()).unwrap_or_else(|| blank(index));
        e.index = index;
        e.pos = [at.x.round() as i32, at.y.round() as i32, at.z.round() as i32];
        e.next = 0;
        e.prev = 0;
        self.edit(|d| {
            d.event_points.push(e);
            d.event_points.sort_by_key(|e| e.index);
        });
        self.selected = Some(index);
        Some(index)
    }

    /// Deletes point `index`. Points that depend on it are left as they are, and listed as
    /// problems.
    pub(crate) fn delete(&mut self, index: usize) {
        self.edit(|d| d.event_points.retain(|e| e.index != index));
        if self.selected == Some(index) {
            self.selected = None;
        }
    }

    /// Changes point `index`'s fields to those in `json`, an object with any of the fields
    /// `uc mission export` writes (`{"radius": 512, "trigger": "timer"}`). Its index stays.
    /// A field the JSON doesn't have is refused, not ignored.
    pub(crate) fn merge(&mut self, index: usize, json: &str) -> Result<(), String> {
        let change: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("not JSON: {e}"))?;
        let serde_json::Value::Object(change) = change else { return Err("give an object: {\"field\": value, ...}".into()) };
        if change.contains_key("index") {
            return Err("a point's index can't be changed".into());
        }
        // Every field a point can have, the optional ones included.
        let full = EventPointJson { summary: Some(String::new()), text: Some(String::new()), cutscene: None, ..blank(0) };
        let mut fields: Vec<String> = match serde_json::to_value(&full) {
            Ok(serde_json::Value::Object(o)) => o.keys().cloned().collect(),
            _ => Vec::new(),
        };
        fields.push("cutscene".into());
        if let Some(k) = change.keys().find(|k| !fields.contains(k)) {
            return Err(format!("no field {k:?}: a point has {}", fields.join(", ")));
        }
        let e = self.point(index).ok_or_else(|| format!("no event point {index}"))?;
        let mut v = serde_json::to_value(e).map_err(|e| e.to_string())?;
        if let serde_json::Value::Object(o) = &mut v {
            for (k, x) in change {
                o.insert(k, x);
            }
        }
        let e: EventPointJson = serde_json::from_value(v).map_err(|e| e.to_string())?;
        // Names and numbers alike have to be ones the game knows.
        e.kind.clone().resolve_in(&names::WAYPOINT).ok_or("unknown event point type")?;
        e.trigger.clone().resolve_in(&names::TRIGGER).ok_or("unknown trigger")?;
        e.on_trigger.clone().resolve_in(&names::ON_TRIGGER).ok_or("unknown on_trigger")?;
        self.edit_point(index, |p| *p = e);
        Ok(())
    }

    /// What's wrong with the mission: what keeps it from loading, then each point's problems.
    pub(crate) fn check(&self) -> Vec<Problem> {
        let mut out = Vec::new();
        if let Err(e) = from_json(&self.doc) {
            out.push(Problem { point: None, text: e });
        }
        let limit = (uc_formats::iam::MAP_SIZE as f32 * SQUARE) as i32;
        for e in &self.doc.event_points {
            let mut say = |text: String| out.push(Problem { point: Some(e.index), text: format!("{}: {text}", e.index) });
            for r in [e.ep_ref, e.ep_ref_bool] {
                if r != 0 && self.point(r as usize).is_none() {
                    say(format!("depends on point {r}, which isn't there"));
                }
            }
            let kind = e.kind.clone().resolve_in(&names::WAYPOINT);
            if kind.is_some_and(has_text) && e.text.as_deref().is_none_or(|t| t.trim().is_empty()) {
                say("a message with no text".into());
            }
            if !(0..limit).contains(&e.pos[0]) || !(0..limit).contains(&e.pos[2]) {
                say("off the map".into());
            }
        }
        out
    }

    /// Writes the mission's JSON to the missions folder, and checks it.
    pub(crate) fn save(&mut self) -> Result<PathBuf, String> {
        let path = path(&self.name).ok_or("no folder to save in")?;
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&self.doc).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        self.saved = true;
        self.leaving = false;
        self.problems = self.check();
        Ok(path)
    }

    /// The point after `from` (by index), round to the first.
    pub(crate) fn next_point(&self, from: Option<usize>, back: bool) -> Option<usize> {
        let ids: Vec<usize> = self.doc.event_points.iter().map(|e| e.index).collect();
        let at = from.and_then(|i| ids.iter().position(|&x| x == i));
        let n = ids.len();
        if n == 0 {
            return None;
        }
        let k = match (at, back) {
            (None, false) => 0,
            (None, true) => n - 1,
            (Some(k), false) => (k + 1) % n,
            (Some(k), true) => (k + n - 1) % n,
        };
        Some(ids[k])
    }

    /// The point of the next problem that has one.
    pub(crate) fn next_problem_point(&mut self) -> Option<usize> {
        let points: Vec<usize> = self.problems.iter().filter_map(|p| p.point).collect();
        if points.is_empty() {
            return None;
        }
        self.next_problem = (self.next_problem + 1) % points.len();
        Some(points[self.next_problem])
    }

    /// Whether point `e` is shown, by the filter.
    fn shown(&self, e: &EventPointJson) -> bool {
        let Some(s) = self.selected.and_then(|i| self.point(i)) else { return true };
        match self.filter {
            Filter::All => true,
            Filter::Kind => e.kind == s.kind,
            Filter::Linked => {
                let refs = |p: &EventPointJson, i: usize| p.ep_ref as usize == i || p.ep_ref_bool as usize == i;
                e.index == s.index || refs(s, e.index) || refs(e, s.index)
            }
        }
    }
}

/// A plain new point: `simple`, set off by nothing.
fn blank(index: usize) -> EventPointJson {
    EventPointJson {
        index, kind: Named::Name("simple".into()), trigger: Named::Name("none".into()), on_trigger: Named::Name("none".into()), summary: None,
        colour: 0, group: 0, direction: 0, flags: 0, ep_ref: 0, ep_ref_bool: 0, after_timer: 0, data: [0; 10], radius: 0, pos: [0; 3],
        next: 0, prev: 0, used: 1, text: None, cutscene: None,
    }
}

/// A named value's number in `table`, by name or as it is.
trait Resolve {
    fn resolve_in(self, table: &[&str]) -> Option<u8>;
}

impl Resolve for Named {
    fn resolve_in(self, table: &[&str]) -> Option<u8> {
        match self {
            Named::Number(n) => Some(n),
            Named::Name(s) => names::lookup(table, &s),
        }
    }
}

/// A point type's name, as JSON writes it.
fn kind_name(e: &EventPointJson) -> String {
    match &e.kind {
        Named::Name(s) => s.clone(),
        Named::Number(n) => names::WAYPOINT.get(*n as usize).map_or(n.to_string(), |s| s.to_string()),
    }
}

/// A marker's colour, by what the point does.
fn colour(e: &EventPointJson) -> [u8; 4] {
    let kind = e.kind.clone().resolve_in(&names::WAYPOINT).unwrap_or(0);
    match kind {
        waypoint::CREATE_PLAYER => [90, 200, 255, 255],
        waypoint::CREATE_ENEMIES => [255, 90, 70, 255],
        waypoint::END_GAME_WIN => [90, 255, 110, 255],
        waypoint::END_GAME_LOSE => [230, 80, 230, 255],
        waypoint::CUT_SCENE | waypoint::CREATE_CAMERA | waypoint::CAMERA_WAYPOINT => [180, 140, 255, 255],
        k if has_text(k) => [245, 245, 235, 255],
        waypoint::CREATE_VEHICLE | waypoint::CREATE_ITEM | waypoint::CREATE_BARREL | waypoint::CREATE_TREASURE
        | waypoint::CREATE_CREATURE | waypoint::CREATE_TRAP | waypoint::CREATE_MIST => [255, 210, 70, 255],
        _ => [170, 170, 165, 255],
    }
}

/// Where a point is in the world.
fn at(e: &EventPointJson) -> Vec3 {
    vec3(e.pos[0] as f32, e.pos[1] as f32, e.pos[2] as f32)
}

impl World {
    /// The point the free camera looks at within 60 squares, on the ground or a roof, or
    /// under the camera looking at nothing (the editor's).
    pub(crate) fn look_point(&self) -> Vec3 {
        self.look_at(60.0).unwrap_or_else(|| self.under_camera())
    }

    /// The point the free camera looks at within `reach` squares, on the ground or a roof:
    /// along the line of sight, the first wall it meets puts it on whatever is on top just
    /// beyond it (a building's roof), and the first ground or roof it reaches there. None
    /// looking at the sky, or off the map.
    pub(crate) fn look_at(&self, reach: f32) -> Option<Vec3> {
        let eye = self.fly.pos * uc_world::UNIT;
        let dir = self.fly.forward();
        let flat = vec3(dir.x, 0.0, dir.z).normalize_or_zero();
        let step = 32.0;
        let edge = (uc_formats::iam::MAP_SIZE - 1) as f32 * uc_world::UNIT;
        let on_map = |p: Vec3| (0.0..edge).contains(&p.x) && (0.0..edge).contains(&p.z);
        let mut prev = eye;
        for k in 1..(reach * uc_world::UNIT / step) as usize {
            let p = eye + dir * step * k as f32;
            // Gone off the edge of the map from over it: nothing more to see.
            if !on_map(p) && on_map(prev) {
                return None;
            }
            if let Some(t) = self.level.ground.first_hit(prev, p) {
                let beyond = prev + (p - prev) * t + flat * 48.0;
                let top = self.level.ground.surface(&self.level.map, beyond.x, beyond.z, 1.0e5);
                return Some(vec3(beyond.x, top, beyond.z));
            }
            let s = self.level.ground.surface(&self.level.map, p.x, p.z, p.y + 1.0);
            if p.y <= s && on_map(p) {
                return Some(vec3(p.x, s, p.z));
            }
            prev = p;
        }
        None
    }

    /// The ground or roof under the free camera.
    pub(crate) fn under_camera(&self) -> Vec3 {
        let eye = self.fly.pos * uc_world::UNIT;
        let s = self.level.ground.surface(&self.level.map, eye.x, eye.z, eye.y);
        vec3(eye.x, s, eye.z)
    }

    /// Brings the free camera round to look at point `index` from a few squares off.
    pub(crate) fn go_to_point(&mut self, index: usize) {
        let Some(e) = self.editor.as_ref().and_then(|ed| ed.point(index)) else { return };
        let target = at(e) / uc_world::UNIT + vec3(0.0, 0.3, 0.0);
        let back = vec3(-self.fly.yaw.sin(), 0.0, -self.fly.yaw.cos());
        self.fly.pos = target + back * 4.0 + vec3(0.0, 2.5, 0.0);
        let d = target - self.fly.pos;
        self.fly.yaw = d.x.atan2(d.z);
        self.fly.pitch = (d.y / vec2(d.x, d.z).length()).atan();
    }

    /// The point whose marker is at the middle of the screen, nearest first.
    fn aimed(&self, cam: &Camera, size: (u32, u32)) -> Option<usize> {
        let ed = self.editor.as_ref()?;
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).max(1.0);
        let vp = cam.view_proj(size);
        let centre = vec2(w / 2.0, h / 2.0);
        let eye = cam.pos * uc_world::UNIT;
        ed.doc.event_points.iter().filter(|e| ed.shown(e)).filter_map(|e| {
            let p = at(e) + vec3(0.0, MARKER * 0.5, 0.0);
            let c = vp * (p / uc_world::UNIT).extend(1.0);
            if c.w <= 0.01 || (p - eye).length() > SEEN * SQUARE {
                return None;
            }
            let sp = vec2((c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h);
            ((sp - centre).length() < AIM * s).then_some((e.index, (p - eye).length()))
        }).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
    }

    /// The editor's keys this frame (the free camera moves as usual).
    pub(crate) fn editor_frame(&mut self, inp: &Input, cam: &Camera, size: (u32, u32), dt: f32) {
        let ctrl = inp.held(KeyCode::ControlLeft) || inp.held(KeyCode::ControlRight);
        let shift = inp.held(KeyCode::ShiftLeft) || inp.held(KeyCode::ShiftRight);
        let aimed = self.aimed(cam, size);
        let look = self.look_point();
        let Some(ed) = self.editor.as_mut() else { return };
        if let Some(s) = ed.said.as_mut() {
            s.1 -= dt;
        }
        ed.said = ed.said.take().filter(|s| s.1 > 0.0);
        let mut go = None;
        if inp.lmb_pressed || inp.hit(KeyCode::Enter) {
            ed.selected = aimed;
        }
        if inp.hit(KeyCode::Tab) {
            ed.selected = ed.next_point(ed.selected, shift);
            go = ed.selected;
        }
        if inp.hit(KeyCode::KeyP) {
            match ed.next_problem_point() {
                Some(i) => {
                    ed.selected = Some(i);
                    go = Some(i);
                }
                None if ed.problems.is_empty() => ed.say("No problems"),
                None => ed.say(ed.problems[0].text.clone()),
            }
        }
        if inp.hit(KeyCode::KeyF) {
            ed.filter = match ed.filter {
                Filter::All => Filter::Linked,
                Filter::Linked => Filter::Kind,
                Filter::Kind => Filter::All,
            };
            ed.say(format!("Showing {}", match ed.filter {
                Filter::All => "every point",
                Filter::Linked => "the selected point and its links",
                Filter::Kind => "points of the selected point's type",
            }));
        }
        if ctrl && inp.hit(KeyCode::KeyZ) {
            let done = ed.undo();
            ed.say(if done { "Undone" } else { "Nothing to undo" });
        }
        if ctrl && inp.hit(KeyCode::KeyY) {
            let done = ed.redo();
            ed.say(if done { "Redone" } else { "Nothing to redo" });
        }
        if ctrl && inp.hit(KeyCode::KeyS) {
            match ed.save() {
                Ok(p) => {
                    let n = ed.problems.len();
                    ed.say(if n == 0 { format!("Saved to {}", p.display()) } else { format!("Saved, with {n} problems (P goes to each)") });
                }
                Err(e) => ed.say(format!("Not saved: {e}")),
            }
        }
        if inp.hit(KeyCode::KeyN) && !ctrl {
            match ed.add(look, None) {
                Some(i) => ed.say(format!("New point {i}")),
                None => ed.say("Every slot is taken"),
            }
        }
        if let Some(i) = ed.selected {
            if inp.hit(KeyCode::KeyG) {
                ed.move_to(i, look);
            }
            if inp.hit(KeyCode::BracketLeft) {
                ed.turn(i, -16);
            }
            if inp.hit(KeyCode::BracketRight) {
                ed.turn(i, 16);
            }
            if inp.hit(KeyCode::Insert) || (ctrl && inp.hit(KeyCode::KeyD)) {
                match ed.add(look, Some(i)) {
                    Some(j) => ed.say(format!("Point {i} copied as {j}")),
                    None => ed.say("Every slot is taken"),
                }
            }
            if inp.hit(KeyCode::Delete) {
                ed.delete(i);
                ed.say(format!("Point {i} deleted"));
            }
        }
        if let Some(i) = go {
            self.go_to_point(i);
        }
    }

    /// The console's `ep {json}`: changes the selected point's fields.
    pub(crate) fn editor_command(&mut self, json: &str) {
        let Some(ed) = self.editor.as_mut() else {
            self.console.say("ep: only in the editor");
            return;
        };
        let Some(i) = ed.selected else {
            self.console.say("ep: select a point first");
            return;
        };
        match ed.merge(i, json) {
            Ok(()) => self.console.say(format!("Point {i} changed")),
            Err(e) => self.console.say(format!("ep: {e}")),
        }
    }

    /// What the editor draws: the markers, links and circles, the selected point's panel,
    /// the problems, and the keys.
    pub(crate) fn editor_overlay(&self, cam: &Camera, size: (u32, u32)) -> Overlay {
        let mut o = Overlay::new();
        let Some(ed) = &self.editor else { return o };
        let (w, h) = (size.0 as f32, size.1 as f32);
        let s = (h / 360.0).round().max(1.0);
        let vp = cam.view_proj(size);
        let eye = cam.pos * uc_world::UNIT;
        let screen = |p: Vec3| -> Option<Vec2> {
            let c = vp * (p / uc_world::UNIT).extend(1.0);
            (c.w > 0.01).then(|| vec2((c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h))
        };
        let dots = |o: &mut Overlay, a: Vec2, b: Vec2, c: [u8; 4]| {
            let n = ((b - a).length() / (6.0 * s)).min(400.0) as usize;
            for k in 0..=n {
                let p = a + (b - a) * (k as f32 / n.max(1) as f32);
                o.rect(p.x - s * 0.5, p.y - s * 0.5, s, s, c);
            }
        };
        let aimed = self.aimed(cam, size);
        let mut shown: Vec<&EventPointJson> =
            ed.doc.event_points.iter().filter(|e| ed.shown(e) && (at(e) - eye).length() < SEEN * SQUARE).collect();
        // Far first, so the near ones are on top.
        shown.sort_by(|a, b| (at(b) - eye).length().total_cmp(&(at(a) - eye).length()));
        // Links: from each point to the points it depends on.
        for e in &shown {
            for (r, c) in [(e.ep_ref, [255, 255, 255, 120]), (e.ep_ref_bool, [120, 200, 255, 120])] {
                let Some(t) = (r != 0).then(|| ed.point(r as usize)).flatten() else { continue };
                if let (Some(a), Some(b)) = (screen(at(e) + vec3(0.0, MARKER, 0.0)), screen(at(t) + vec3(0.0, MARKER, 0.0))) {
                    dots(&mut o, a, b, c);
                    o.rect(b.x - 2.0 * s, b.y - 2.0 * s, 4.0 * s, 4.0 * s, c);
                }
            }
        }
        for e in &shown {
            let sel = ed.selected == Some(e.index);
            let col = colour(e);
            let p = at(e);
            // The circle of a trigger that goes off within a radius.
            let trig = match &e.trigger { Named::Name(n) => n.clone(), Named::Number(n) => names::TRIGGER.get(*n as usize).unwrap_or(&"").to_string() };
            if RADIUS_TRIGGERS.contains(&trig.as_str()) && e.radius > 0 {
                let r = (e.radius & 0xffff) as f32;
                let ring: Vec<Option<Vec2>> = (0..=48).map(|k| {
                    let a = k as f32 / 48.0 * std::f32::consts::TAU;
                    screen(p + vec3(a.cos() * r, 8.0, a.sin() * r))
                }).collect();
                for pair in ring.windows(2) {
                    if let [Some(a), Some(b)] = pair {
                        dots(&mut o, *a, *b, [col[0], col[1], col[2], 110]);
                    }
                }
            }
            let (Some(foot), Some(top)) = (screen(p), screen(p + vec3(0.0, MARKER, 0.0))) else { continue };
            let width = if sel { 4.0 * s } else { 2.0 * s };
            o.rect(foot.x - width / 2.0, top.y, width, (foot.y - top.y).max(s), col);
            let head = if sel || aimed == Some(e.index) { 7.0 * s } else { 5.0 * s };
            if sel {
                o.rect(top.x - head / 2.0 - s, top.y - head / 2.0 - s, head + 2.0 * s, head + 2.0 * s, [255, 255, 255, 255]);
            }
            o.rect(top.x - head / 2.0, top.y - head / 2.0, head, head, col);
            // Which way it faces, for the points that do.
            let angle = uc_world::character::ep_angle(e.direction);
            if let Some(f) = screen(p + vec3(-angle.sin(), 0.0, -angle.cos()) * 96.0) {
                dots(&mut o, foot, f, [col[0], col[1], col[2], 160]);
            }
            if sel || aimed == Some(e.index) || (p - eye).length() < LABELLED * SQUARE {
                let label = format!("{} {}", e.index, kind_name(e));
                let lw = Overlay::text_width(&label, s);
                o.rect(top.x - lw / 2.0 - s, top.y - head - 9.0 * s, lw + 2.0 * s, 9.0 * s, [0, 0, 0, 140]);
                o.text(top.x - lw / 2.0, top.y - head - 8.0 * s, s, col, &label);
            }
        }
        // The cross hair.
        let c = if aimed.is_some() { [255, 220, 90, 255] } else { [255, 255, 255, 160] };
        o.rect(w / 2.0 - 5.0 * s, h / 2.0 - s * 0.5, 10.0 * s, s, c);
        o.rect(w / 2.0 - s * 0.5, h / 2.0 - 5.0 * s, s, 10.0 * s, c);
        let white = [235, 235, 225, 255];
        let dim = [200, 200, 190, 220];
        // The heading and the keys, as many lines as the screen's width needs.
        let title = format!("EDITING {}{}", ed.name.to_uppercase(), if ed.saved { "" } else { " (unsaved)" });
        let keys = KEYS.iter().fold(vec![String::new()], |mut lines, k| {
            let last = lines.last_mut().unwrap();
            if last.is_empty() {
                last.push_str(k);
            } else if Overlay::text_width(&format!("{last}   {k}"), s) <= w - 20.0 * s {
                last.push_str("   ");
                last.push_str(k);
            } else {
                lines.push(k.to_string());
            }
            lines
        });
        let top = 16.0 * s + keys.len() as f32 * 9.0 * s + 2.0 * s;
        o.rect(0.0, 0.0, w, top, [0, 0, 0, 150]);
        o.text(10.0 * s, 6.0 * s, s, [255, 210, 70, 255], &title);
        for (k, l) in keys.iter().enumerate() {
            o.text(10.0 * s, 16.0 * s + k as f32 * 9.0 * s, s, dim, l);
        }
        if let Some((text, _)) = &ed.said {
            o.text(w / 2.0 - Overlay::text_width(text, s) / 2.0, top + 8.0 * s, s, white, text);
        }
        if ed.leaving {
            let text = "Unsaved changes: Esc again leaves without saving, Ctrl+S saves";
            o.text(w / 2.0 - Overlay::text_width(text, s) / 2.0, top + 18.0 * s, s, [255, 140, 120, 255], text);
        }
        // The problems, bottom left.
        if !ed.problems.is_empty() {
            let shown = ed.problems.len().min(5);
            let y0 = h - (shown as f32 + 1.5) * 9.0 * s - 30.0 * s;
            o.text(10.0 * s, y0, s, [255, 140, 120, 255], &format!("{} problem{}:", ed.problems.len(), if ed.problems.len() == 1 { "" } else { "s" }));
            for (k, p) in ed.problems.iter().take(shown).enumerate() {
                let line = crate::menu::fit(&p.text, w * 0.45, s);
                o.text(10.0 * s, y0 + (k as f32 + 1.0) * 9.0 * s, s, [255, 180, 160, 230], &line);
            }
        }
        // The selected point: its fields as JSON, on the right.
        if let Some(e) = ed.selected.and_then(|i| ed.point(i)) {
            let pw = (w * 0.34).max(160.0 * s);
            let px = w - pw - 8.0 * s;
            let bw = pw - 16.0 * s;
            // A field a line, its value as compact JSON (wrapped if it's long).
            let fields = match serde_json::to_value(e) {
                Ok(serde_json::Value::Object(o)) => o.into_iter().collect(),
                _ => Vec::new(),
            };
            let mut lines: Vec<String> = fields.iter().flat_map(|(k, v)| {
                let v = serde_json::to_string(v).unwrap_or_default().replace(',', ", ");
                Overlay::wrap(&format!("{k}: {v}"), bw, s)
            }).collect();
            let line = 9.0 * s;
            let py = top + 30.0 * s;
            let room = ((h - py - 60.0 * s) / line).max(4.0) as usize;
            if lines.len() > room {
                lines.truncate(room - 1);
                lines.push("...".into());
            }
            let ph = 26.0 * s + lines.len() as f32 * line;
            o.rect(px, py, pw, ph, [0, 0, 0, 170]);
            let head = crate::menu::fit(&format!("EVENT POINT {}: {}", e.index, kind_name(e).to_uppercase()), bw, s);
            o.text(px + 8.0 * s, py + 6.0 * s, s, colour(e), &head);
            for (k, l) in lines.iter().enumerate() {
                o.text(px + 8.0 * s, py + 20.0 * s + k as f32 * line, s, white, l);
            }
        }
        self.draw_console(&mut o, w, h, s);
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shipped mission to edit, or None without an install.
    fn shipped(name: &str) -> Option<Mission> {
        let game = uc_formats::default_install()?;
        let bytes = std::fs::read(game.join("levels").join(format!("{name}.ucm"))).ok()?;
        Some(Mission::parse(&bytes).unwrap())
    }

    #[test]
    fn edits_undo_and_check() {
        let Some(m) = shipped("police1") else { return };
        let mut ed = Editor::with("police1", to_json(&m, false));
        assert!(ed.problems.is_empty(), "{:?}", ed.problems);
        let n = ed.doc.event_points.len();
        let first = ed.doc.event_points[0].index;
        // A copy goes in the first free slot, where it's put.
        let i = ed.add(vec3(1000.0, 0.0, 2000.0), Some(first)).unwrap();
        assert_eq!(ed.doc.event_points.len(), n + 1);
        assert_eq!(ed.point(i).unwrap().pos, [1000, 0, 2000]);
        assert_eq!(ed.point(i).unwrap().kind, ed.point(first).unwrap().kind);
        ed.move_to(i, vec3(1500.4, 64.0, 2500.6));
        assert_eq!(ed.point(i).unwrap().pos, [1500, 64, 2501]);
        ed.turn(i, -16);
        assert_eq!(ed.point(i).unwrap().direction, (ed.point(first).unwrap().direction as i32 - 16).rem_euclid(256) as u8);
        // Fields changed as JSON, and names checked.
        ed.merge(i, r#"{"radius": 512, "trigger": "timer"}"#).unwrap();
        assert_eq!(ed.point(i).unwrap().radius, 512);
        assert_eq!(ed.point(i).unwrap().trigger, Named::Name("timer".into()));
        assert!(ed.merge(i, r#"{"trigger": "no such trigger"}"#).is_err());
        assert!(ed.merge(i, r#"{"radiuss": 5}"#).is_err());
        assert!(ed.merge(i, r#"{"index": 3}"#).is_err());
        assert!(ed.merge(i, "radius 5").is_err());
        // The game's form is made from it again.
        let played = ed.mission().unwrap();
        assert_eq!(played.event_points.len(), n + 1);
        // Undo takes it all back, a step at a time, and redo puts it back.
        for _ in 0..4 {
            assert!(ed.undo());
        }
        assert_eq!(ed.doc.event_points.len(), n);
        assert!(!ed.undo());
        assert!(ed.redo());
        assert_eq!(ed.doc.event_points.len(), n + 1);
    }

    #[test]
    fn an_edit_plays_and_stays_out_of_the_progress() {
        let (Some(game), Some(m)) = (uc_formats::default_install(), shipped("police1")) else { return };
        let args = Args { game, mission: "police1".into(), mission_given: true, ..Default::default() };
        let mut ed = Editor::with("police1", to_json(&m, false));
        // Darci's start moved a few squares.
        let p = ed.doc.event_points.iter().find(|e| e.kind == Named::Name("create_player".into())).unwrap();
        let (i, to) = (p.index, vec3(p.pos[0] as f32 + 768.0, p.pos[1] as f32, p.pos[2] as f32 + 512.0));
        ed.move_to(i, to);
        // In the editor the game isn't played: it's the edit's level, but nothing counts.
        let w = World::load_edit(args.clone(), Some(ed)).unwrap();
        assert!(w.editor.is_some() && !w.counts() && w.recorder.is_none());
        let start = w.level.mission.event_points.iter().find(|e| e.index == i).unwrap();
        assert_eq!([start.x, start.z], [to.x as i32, to.z as i32]);
        // Played, she starts where the edit put her, and it's kept out of the progress and
        // the replays, even started over.
        let mut w = World::load_edited(args, w.editor.unwrap().mission().unwrap()).unwrap();
        let at = w.game.player().unwrap().fig.pos;
        assert!((at.x - to.x).abs() < 1.0 && (at.z - to.z).abs() < 1.0, "Darci at {at}, not {to}");
        assert!(!w.counts() && w.recorder.is_none() && w.race.is_none());
        w.restart();
        assert!(!w.counts() && w.recorder.is_none());
    }

    #[test]
    fn problems_are_listed() {
        let Some(m) = shipped("police1") else { return };
        let mut ed = Editor::with("police1", to_json(&m, false));
        // Deleting a point another depends on leaves that one pointing at nothing, and a new
        // point doesn't take its place.
        let used = ed.doc.event_points.iter().map(|e| e.ep_ref).find(|&r| r != 0).unwrap() as usize;
        ed.delete(used);
        let depends = format!("depends on point {used}");
        assert!(ed.problems.iter().any(|p| p.text.contains(&depends)), "{:?}", ed.problems);
        // A message point needs its text, and every point has to be on the map.
        let i = ed.add(vec3(-10.0, 0.0, 100.0), None).unwrap();
        assert_ne!(i, used);
        assert!(ed.problems.iter().any(|p| p.text.contains(&depends)));
        assert!(ed.problems.iter().any(|p| p.point == Some(i) && p.text.contains("off the map")));
        assert!(ed.merge(i, r#"{"kind": "message"}"#).is_err(), "JSON calls it type");
        ed.merge(i, r#"{"type": "message"}"#).unwrap();
        assert!(ed.problems.iter().any(|p| p.point == Some(i) && p.text.contains("no text")), "{:?} {:?}", ed.problems, ed.point(i));
        ed.merge(i, r#"{"text": "Hello"}"#).unwrap();
        assert!(!ed.problems.iter().any(|p| p.point == Some(i) && p.text.contains("no text")));
        // The message goes with its point: the game's form has it.
        assert!(ed.mission().unwrap().messages().any(|t| t == "Hello"));
        // P goes round the points with problems.
        assert!(ed.next_problem_point().is_some());
    }
}
