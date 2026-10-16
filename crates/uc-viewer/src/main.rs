//! Urban Chaos in Rust: plays a mission (winit + wgpu).
//!
//! Loads a mission's level from your own install and runs its script: Darci, enemies,
//! messages, navigation beacons, fighting and arrests (see the uc-game crate). With
//! `--explore`, Darci just walks around the level. V switches to a free-flying camera.

mod achievements;
mod args;
mod audio;
mod cam;
mod checklist;
mod console;
mod hud;
mod input;
mod menu;
mod pause;
mod progress;
mod sound;
mod world;

use args::*;
use audio::*;
use cam::*;
use console::*;
use hud::*;
use input::*;
use pause::*;
use world::*;

use gilrs::{Axis, Button, Gilrs};
use glam::{vec2, vec3, Vec2, Vec3};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use uc_audio::weather::{Place, Weather, World as Wtype};
use uc_audio::{Audio, Listener};
use uc_game::{Controls, Game, Mood, Outcome};
use uc_render::{wgpu, Camera, Overlay, Renderer, Settings};
use uc_world::{Batch, Layer, Level, LoadOptions};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

fn save_png(path: &Path, size: (u32, u32), rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size.0, size.1);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgba).map_err(|e| e.to_string())
}

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

struct App {
    /// The mission being played (None at the menu).
    world: Option<World>,
    /// The menu, while it's showing.
    menu: Option<menu::Menu>,
    /// Options for loading missions.
    args: Args,
    gfx: Option<Gfx>,
    sound: Option<Sound>,
    pads: Option<Gilrs>,
    input: Input,
    last: Instant,
    title_timer: f32,
    /// The image behind the menu now (to change it when the screen does).
    backdrop: String,
    frames: u32,
    shots: u32,
}

impl App {
    fn init_gfx(&mut self, el: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            el.create_window(Window::default_attributes().with_title("Urban Chaos").with_inner_size(LogicalSize::new(1280, 720))
                .with_fullscreen((!self.args.windowed).then_some(Fullscreen::Borderless(None))))
                .map_err(|e| e.to_string())?,
        );
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("no graphics adapter: {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| format!("no graphics device: {e}"))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface not supported by the adapter")?;
        // Prefer a non-sRGB format so lighting multiplies in gamma space, like the original.
        let caps = surface.get_capabilities(&adapter);
        if let Some(f) = caps.formats.iter().find(|f| !f.is_srgb()) {
            config.format = *f;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);
        let mut renderer = Renderer::new(device, queue, config.format, !self.args.linear);
        self.backdrop = String::new();
        if let Some(w) = &self.world {
            w.upload(&mut renderer);
        }
        self.gfx = Some(Gfx { window, surface, config, renderer });
        self.input.mouse_look = true;
        self.input.focused = true;
        self.capture_cursor(self.want_cursor());
        Ok(())
    }

    fn update(&mut self, dt: f32) {
        self.poll_pad();
        if self.input.hit(KeyCode::KeyM) {
            self.input.mouse_look = !self.input.mouse_look;
            self.capture_cursor(self.want_cursor());
        }
        let inp = &self.input;
        let pad = &inp.pad;
        let Some(w) = self.world.as_mut() else { return };
        let delta = inp.mouse;
        w.checklist = inp.held(KeyCode::Tab);
        if (inp.hit(KeyCode::KeyV) || pad.hit(Button::Select)) && w.game.player.is_some() {
            w.third_person = !w.third_person;
            if let (false, Some(eye)) = (w.third_person, w.eye()) {
                w.fly = w.orbit.cam(eye);
            }
        }
        if inp.hit(KeyCode::KeyH) {
            let shown = w.help && (w.game.time < 20.0 || w.help_forced);
            w.help = !shown;
            w.help_forced = !shown;
        }
        let keys = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6];
        let layers = [Layer::Floor, Layer::Walls, Layer::Props, Layer::Roofs, Layer::Detail, Layer::Character];
        for (k, l) in keys.iter().zip(layers) {
            if inp.hit(*k) {
                if let Some(i) = w.layers.iter().position(|x| *x == l) {
                    w.layers.remove(i);
                } else {
                    w.layers.push(l);
                }
            }
        }
        if inp.hit(KeyCode::KeyR) {
            w.restart();
            // Starting over shouldn't be talked over by the run before it.
            if let Some(s) = self.sound.as_mut() {
                s.audio.hush();
            }
            if !w.third_person {
                w.fly = FlyCam::from(w.start);
            }
        }
        let mut controls = Controls::default();
        if w.third_person {
            if inp.rmb || inp.mouse_look {
                w.orbit.yaw -= delta.x * 0.005;
                w.orbit.pitch = (w.orbit.pitch - delta.y * 0.005).clamp(-1.3, 0.4);
            }
            w.orbit.yaw -= pad.look.x * 2.5 * dt;
            w.orbit.pitch = (w.orbit.pitch + pad.look.y * 1.5 * dt).clamp(-1.3, 0.4);
            w.orbit.dist = (w.orbit.dist * (1.0 - inp.wheel * 0.1)).clamp(0.8, 8.0);
            // Movement relative to the camera; screen right = forward x up.
            let fwd = vec2(w.orbit.yaw.sin(), w.orbit.yaw.cos());
            let right = vec2(-fwd.y, fwd.x);
            let mut wish = Vec2::ZERO;
            if inp.held(KeyCode::KeyW) { wish += fwd; }
            if inp.held(KeyCode::KeyS) { wish -= fwd; }
            if inp.held(KeyCode::KeyD) { wish += right; }
            if inp.held(KeyCode::KeyA) { wish -= right; }
            // The stick walks when pushed partway and runs when pushed fully.
            let stick = fwd * pad.mv.y + right * pad.mv.x;
            let action_held = inp.held(KeyCode::KeyE) || inp.held(KeyCode::KeyL) || pad.held(Button::East);
            controls = Controls {
                wish: if wish != Vec2::ZERO { wish.normalize() } else { stick },
                run: inp.held(KeyCode::ShiftLeft) || inp.held(KeyCode::ShiftRight) || pad.mv.length() > 0.85,
                sprint: action_held,
                punch: inp.hit(KeyCode::KeyJ) || inp.lmb_pressed || pad.hit(Button::West),
                kick: inp.hit(KeyCode::KeyK) || pad.hit(Button::North),
                action: inp.hit(KeyCode::KeyE) || inp.hit(KeyCode::KeyL) || pad.hit(Button::East),
                jump: inp.hit(KeyCode::Space) || pad.hit(Button::South),
                block: inp.held(KeyCode::KeyQ) || pad.held(Button::LeftTrigger) || pad.held(Button::LeftTrigger2),
                fire: inp.held(KeyCode::KeyJ) || inp.lmb || pad.held(Button::West),
                switch: inp.hit(KeyCode::KeyG) || pad.hit(Button::RightTrigger),
                cycle_target: inp.hit(KeyCode::KeyT) || pad.hit(Button::RightThumb),
                scope: inp.hit(KeyCode::KeyZ) || pad.hit(Button::LeftThumb),
                crouch: inp.held(KeyCode::KeyC) || inp.held(KeyCode::ControlLeft) || pad.held(Button::DPadDown),
                ..Default::default()
            };
            // Through the sniper rifle's scope: WASD (or the stick) turn it, punch fires, Z
            // again to come out of it.
            if w.game.sniping().is_some() {
                let key = |k| if inp.held(k) { 1.0 } else { 0.0 };
                let up = key(KeyCode::KeyW) - key(KeyCode::KeyS) + pad.mv.y;
                let turn = key(KeyCode::KeyD) - key(KeyCode::KeyA) + pad.mv.x;
                controls = Controls {
                    scope: controls.scope,
                    punch: controls.punch,
                    drive: uc_game::Drive { throttle: up.clamp(-1.0, 1.0), steer: turn.clamp(-1.0, 1.0), handbrake: false },
                    ..Default::default()
                };
            }
            // Driving: W/S or the triggers (or stick) for speed, A/D or the stick to steer,
            // Space or A for the handbrake. The camera swings round behind the car.
            // On a motorbike: the same keys (Space for a wheelie), and action to get off.
            if w.game.riding().is_some() {
                let key = |k| if inp.held(k) { 1.0 } else { 0.0 };
                let throttle = key(KeyCode::KeyW) - key(KeyCode::KeyS) + pad.mv.y;
                let steer = key(KeyCode::KeyD) - key(KeyCode::KeyA) + pad.mv.x;
                let wheelie = inp.held(KeyCode::Space) || pad.held(Button::South);
                controls = Controls {
                    action: controls.action,
                    drive: uc_game::Drive { throttle: throttle.clamp(-1.0, 1.0), steer: steer.clamp(-1.0, 1.0), handbrake: wheelie },
                    ..Default::default()
                };
            }
            if let Some(car) = w.game.player_vehicle() {
                let key = |k| if inp.held(k) { 1.0 } else { 0.0 };
                let throttle = key(KeyCode::KeyW) - key(KeyCode::KeyS)
                    + if pad.held(Button::RightTrigger2) { 1.0 } else { 0.0 }
                    - if pad.held(Button::LeftTrigger2) { 1.0 } else { 0.0 } + pad.mv.y;
                let steer = key(KeyCode::KeyD) - key(KeyCode::KeyA) + pad.mv.x;
                let handbrake = inp.held(KeyCode::Space) || pad.held(Button::South);
                controls = Controls {
                    action: controls.action,
                    drive: uc_game::Drive { throttle: throttle.clamp(-1.0, 1.0), steer: steer.clamp(-1.0, 1.0), handbrake },
                    ..Default::default()
                };
                if car.speed.abs() > 100.0 && delta.length_squared() < 1.0 && pad.look.length_squared() < 0.01 {
                    let mut d = (car.yaw - w.orbit.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    if car.speed < 0.0 {
                        d = 0.0;
                    }
                    w.orbit.yaw += d * (2.5 * dt).min(1.0);
                }
            }
        } else {
            if inp.rmb || inp.mouse_look {
                w.fly.yaw -= delta.x * 0.004;
                w.fly.pitch = (w.fly.pitch - delta.y * 0.004).clamp(-1.5, 1.5);
            }
            let mut speed = 6.0;
            if inp.held(KeyCode::ShiftLeft) { speed *= 4.0; }
            if inp.held(KeyCode::ControlLeft) { speed *= 0.25; }
            let fwd = w.fly.forward();
            let right = fwd.cross(Vec3::Y).normalize_or_zero();
            let mut mv = Vec3::ZERO;
            if inp.held(KeyCode::KeyW) { mv += fwd; }
            if inp.held(KeyCode::KeyS) { mv -= fwd; }
            if inp.held(KeyCode::KeyD) { mv += right; }
            if inp.held(KeyCode::KeyA) { mv -= right; }
            if inp.held(KeyCode::Space) || inp.held(KeyCode::KeyE) { mv += Vec3::Y; }
            if inp.held(KeyCode::KeyC) || inp.held(KeyCode::KeyQ) { mv -= Vec3::Y; }
            w.fly.yaw -= pad.look.x * 2.0 * dt;
            w.fly.pitch = (w.fly.pitch + pad.look.y * 1.5 * dt).clamp(-1.5, 1.5);
            let pad_mv = fwd * pad.mv.y + right * pad.mv.x;
            w.fly.pos += (mv.normalize_or_zero() + pad_mv) * speed * dt;
            // T: Darci goes where the camera looks, and the camera's back on her.
            if inp.hit(KeyCode::KeyT) && w.game.player.is_some() {
                w.teleport_to_view();
            }
        }
        if w.game.cutscene.is_some() && (inp.hit(KeyCode::Space) || inp.hit(KeyCode::Enter) || pad.hit(Button::South)) {
            w.game.skip_cutscene();
            controls.jump = false;
        }
        w.game.update(dt * w.console.time_scale.unwrap_or(1.0), &controls, &w.level);
    }

    /// Loads mission `name` and starts playing it.
    fn play(&mut self, name: &str) {
        if let Some(w) = self.world.as_mut() {
            w.save_stats();
        }
        let mut args = self.args.clone();
        args.mission = name.to_string();
        match World::load(args) {
            Ok(w) => {
                if let Some(g) = self.gfx.as_mut() {
                    g.renderer.clear_textures();
                    w.upload(&mut g.renderer);
                }
                if let Some(s) = self.sound.as_mut() {
                    s.start(&w);
                }
                self.world = Some(w);
                self.menu = None;
                self.capture_cursor(self.want_cursor());
            }
            Err(e) => eprintln!("{name}: {e}"),
        }
    }

    fn open_menu(&mut self) {
        let mut m = menu::Menu::new(&self.args.game);
        // Back from a mission: straight to the missions, past the title.
        if let Some(w) = self.world.as_mut() {
            w.save_stats();
            m.select(&w.level.name);
            m.screen = menu::Screen::Missions;
        }
        self.world = None;
        self.menu = Some(m);
        if let Some(s) = self.sound.as_mut() {
            s.stop();
        }
        self.capture_cursor(false);
    }

    /// The menu: choose with the arrows (or stick/d-pad), Enter (or A) plays.
    fn menu_frame(&mut self, el: &ActiveEventLoop) {
        self.poll_pad();
        let inp = &self.input;
        let Some(m) = self.menu.as_mut() else { return };
        if let Some(i) = m.loading {
            let name = m.entries[i].mission.clone();
            self.args.endless = false;
            self.args.beat = false;
            self.play(&name);
            self.input.pressed.clear();
            return;
        }
        // Capture the flag against Roper (#133): the original's two-player mission.
        if m.loading_ctf {
            self.args.endless = false;
            self.args.beat = false;
            self.play("Capflag1");
            self.input.pressed.clear();
            return;
        }
        if let Some(i) = m.loading_endless {
            let (name, label, centre, radius) = m.maps[i].clone();
            let beat = m.for_beat;
            self.args.endless = !beat;
            self.args.beat = beat;
            self.args.shift = m.progress.shift();
            let label = if beat { progress::beat_key(&label, self.args.shift) } else { label };
            self.args.arena = (centre, radius);
            self.args.arena_name = Some(label);
            self.play(&name);
            self.input.pressed.clear();
            return;
        }
        let up = inp.hit(KeyCode::ArrowUp) || inp.hit(KeyCode::KeyW) || inp.pad.hit(Button::DPadUp);
        let down = inp.hit(KeyCode::ArrowDown) || inp.hit(KeyCode::KeyS) || inp.pad.hit(Button::DPadDown);
        let go = inp.hit(KeyCode::Enter) || inp.hit(KeyCode::NumpadEnter) || inp.hit(KeyCode::Space)
            || inp.pad.hit(Button::South) || inp.pad.hit(Button::Start);
        let back = inp.hit(KeyCode::Escape) || inp.pad.hit(Button::East);
        match m.screen {
            // Any key from the title.
            menu::Screen::Title => {
                if !inp.pressed.is_empty() || !inp.pad.pressed.is_empty() {
                    m.screen = menu::Screen::Main;
                }
            }
            menu::Screen::Main => {
                if back {
                    m.screen = menu::Screen::Title;
                }
                if up {
                    m.main = (m.main + menu::MAIN.len() - 1) % menu::MAIN.len();
                }
                if down {
                    m.main = (m.main + 1) % menu::MAIN.len();
                }
                if go {
                    match menu::MAIN[m.main] {
                        "START" => m.screen = menu::Screen::Missions,
                        "ENDLESS" => {
                            m.for_beat = false;
                            m.screen = menu::Screen::Endless;
                        }
                        "BEAT COP" => {
                            m.for_beat = true;
                            m.screen = menu::Screen::Endless;
                        }
                        "CAPTURE THE FLAG" => m.loading_ctf = true,
                        "STATS" => m.screen = menu::Screen::Stats,
                        "DIFFICULTY" => {
                            let all = uc_game::Difficulty::ALL;
                            let now = all.iter().position(|&d| d == m.progress.difficulty()).unwrap_or(1);
                            m.progress.set_difficulty(all[(now + 1) % all.len()]);
                        }
                        _ => {
                            el.exit();
                            return;
                        }
                    }
                }
            }
            menu::Screen::Missions => {
                if back {
                    m.screen = menu::Screen::Main;
                }
                if up {
                    m.up();
                }
                if down {
                    m.down();
                }
                if go && !m.entries.is_empty() {
                    m.loading = Some(m.selected);
                }
            }
            menu::Screen::Stats => {
                if back || go {
                    m.screen = menu::Screen::Main;
                }
            }
            menu::Screen::Endless => {
                if back {
                    m.screen = menu::Screen::Main;
                }
                // Left and right pick a shift's length.
                let left = inp.hit(KeyCode::ArrowLeft) || inp.hit(KeyCode::KeyA) || inp.pad.hit(Button::DPadLeft);
                let right = inp.hit(KeyCode::ArrowRight) || inp.hit(KeyCode::KeyD) || inp.pad.hit(Button::DPadRight);
                if m.for_beat && (left || right) {
                    let all = uc_game::game::beat::SHIFTS;
                    let now = all.iter().position(|&s| s == m.progress.shift()).unwrap_or(1);
                    let step = if right { 1 } else { all.len() - 1 };
                    m.progress.set_shift(all[(now + step) % all.len()]);
                }
                if up {
                    m.map_up();
                }
                if down {
                    m.map_down();
                }
                if go && !m.maps.is_empty() {
                    m.loading_endless = Some(m.map_selected);
                }
            }
        }
        self.input.pressed.clear();
        self.input.lmb_pressed = false;
        self.input.mouse = Vec2::ZERO;
        self.input.wheel = 0.0;
        let Some(g) = self.gfx.as_mut() else { return };
        // The screen's image, when it changes.
        let image = m.image();
        if image != self.backdrop {
            if let Some(t) = menu::backdrop(&self.args.game, &image) {
                g.renderer.set_backdrop(&t);
            }
            self.backdrop = image;
        }
        let size = (g.config.width, g.config.height);
        let overlay = m.overlay(size);
        let frame = match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                g.surface.configure(&g.renderer.device, &g.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        g.renderer.render_2d(&view, size, &overlay);
        g.window.pre_present_notify();
        g.renderer.queue.present(frame);
        g.window.set_title("Urban Chaos");
    }

    fn frame(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        let alt = self.input.held(KeyCode::AltLeft) || self.input.held(KeyCode::AltRight);
        if self.input.hit(KeyCode::F11) || (alt && self.input.hit(KeyCode::Enter)) {
            self.input.pressed.remove(&KeyCode::Enter);
            if let Some(g) = &self.gfx {
                let full = g.window.fullscreen().is_some();
                g.window.set_fullscreen((!full).then_some(Fullscreen::Borderless(None)));
            }
        }
        self.sync_cursor(dt);
        if self.menu.is_some() || self.world.is_none() {
            if self.menu.is_none() {
                self.open_menu();
            }
            self.menu_frame(el);
            return;
        }
        if self.world.as_ref().is_some_and(|w| w.pause.is_none()) {
            let w = self.world.as_mut().unwrap();
            w.console_input(&mut self.input, dt);
        }
        let won = self.world.as_ref().is_some_and(|w| w.game.outcome == Some(Outcome::Won));
        // Esc (or Start) pauses (`GAMEMENU_process`, gamemenu.cpp:279): the sound stops and
        // the menu comes up; again, and it's gone. After a win it's back to the mission list;
        // after a loss, the lost menu.
        if self.input.hit(KeyCode::Escape) || (!won && self.input.pad.hit(Button::Start)) {
            self.input.pad.pressed.remove(&Button::Start);
            let w = self.world.as_mut().unwrap();
            let lost = matches!(w.game.outcome, Some(Outcome::Lost(_)));
            match w.pause.map(|p| p.menu) {
                None if won => {
                    self.input.pressed.clear();
                    self.open_menu();
                    return;
                }
                None => {
                    w.pause = Some(Pause::new(if lost { PauseMenu::Lost } else { PauseMenu::Paused }));
                    if let Some(s) = self.sound.as_mut() {
                        s.stop();
                    }
                }
                Some(PauseMenu::Paused) => {
                    w.pause = None;
                    if let Some(s) = self.sound.as_mut() {
                        s.resume();
                    }
                }
                Some(PauseMenu::Sure(lost)) => w.pause = Some(Pause::new(if lost { PauseMenu::Lost } else { PauseMenu::Paused })),
                Some(PauseMenu::Lost) => {
                    self.input.pressed.clear();
                    self.open_menu();
                    return;
                }
            }
            self.input.pressed.remove(&KeyCode::Escape);
        }
        if self.world.as_ref().is_some_and(|w| w.pause.is_some()) && self.pause_frame(dt) {
            return;
        }
        let won = self.world.as_ref().is_some_and(|w| w.game.outcome == Some(Outcome::Won));
        let shift = self.world.as_ref().is_some_and(|w| w.game.beat().is_some());
        if won && !shift && (self.input.hit(KeyCode::Enter) || self.input.pad.hit(Button::Start)) {
            let current = self.world.as_ref().unwrap().level.name.clone();
            let m = menu::Menu::new(&self.args.game);
            if let Some(next) = m.after(&current) {
                let name = m.entries[next].mission.clone();
                self.input.pressed.clear();
                self.play(&name);
                return;
            }
        }
        let paused = self.world.as_ref().is_some_and(|w| w.pause.is_some());
        if !paused {
            self.update(dt);
        }
        let Some(world) = self.world.as_mut() else { return };
        // Remember a win (once) for the menu.
        // Won as Darci with more than one innocent dead: the chief's warning, and at the
        // fourth her badge, which loses the mission (Game.cpp:2783, `X_CIVSDEAD_WARNING_*`).
        let darci = world.game.player().is_some_and(|p| p.kind == uc_world::character::person::DARCI);
        if world.game.outcome == Some(Outcome::Won) && world.result.is_none() && world.civ_warning.is_none() && darci && world.game.red_marks > 1 {
            let n = progress::Progress::load().warn();
            world.civ_warning = Some(civ_warning(&world.args.game, n));
            if n >= 3 {
                world.game.outcome = Some(Outcome::Lost("Too many civilian casualties".into()));
            }
        }
        if let (Some(b), Some(_), None) = (world.game.beat(), &world.game.outcome, world.endless_recorded) {
            let mut p = progress::Progress::load();
            let key = world.args.arena_name.clone().unwrap_or_else(|| progress::beat_key(&world.level.name, world.args.shift));
            let before = p.best_endless(&key);
            let best = p.record_endless(&key, b.score, b.handled);
            world.endless_recorded = Some((best, before));
        }
        if let (Some(e), Some(Outcome::Lost(_)), None) = (world.game.endless(), &world.game.outcome, world.endless_recorded) {
            let mut p = progress::Progress::load();
            let key = world.args.arena_name.clone().unwrap_or_else(|| world.level.name.clone());
            let before = p.best_endless(&key);
            let best = p.record_endless(&key, e.score, e.wave);
            world.endless_recorded = Some((best, before));
        }
        if world.game.outcome == Some(Outcome::Won) && world.result.is_none() && world.game.beat().is_none() {
            let mut p = progress::Progress::load();
            let before = p.best_time(&world.level.name);
            let sides = world.game.sides_done(&world.goals);
            p.record(&world.level.name, world.game.time, &world.game.found, &sides, world.game.difficulty());
            world.kept = p.get(&world.level.name).cloned().unwrap_or_default();
            world.result = Some((world.game.time, before));
            // By the book: an arrest, not a shot fired and no red mark. Thorough: everything
            // in the mission done.
            let s = &mut world.game.record;
            if s.arrests > 0 && s.shots == 0 && world.game.red_marks == 0 {
                s.by_the_book += 1;
            }
            let (badges, sides) = world.tally();
            if badges + sides > 0 && badges == world.goals.badges.len() && sides == world.goals.sides.len() {
                world.game.record.perfect += 1;
            }
        }
        // Over: this game's stats go into the totals.
        if world.game.outcome.is_some() {
            world.save_stats();
        }
        world.check_achievements(dt);
        let shift = self.input.held(KeyCode::ShiftLeft) || self.input.held(KeyCode::ShiftRight);
        let screenshot = self.input.hit(KeyCode::F12) && !shift;
        self.input.pressed.clear();
        self.input.typed.clear();
        self.input.lmb_pressed = false;
        self.input.mouse = Vec2::ZERO;
        self.input.wheel = 0.0;

        // Drawn between the last two turns, so it moves smoothly at any frame rate.
        world.game.begin_draw();
        let cam = world.camera();
        match self.sound.as_mut() {
            Some(s) if !paused => s.update(world, &cam, dt),
            _ => drop(world.game.take_sounds()),
        }
        let settings = world.settings();
        let dynamic = world.dynamic();
        let Some(g) = self.gfx.as_mut() else {
            world.game.end_draw();
            return;
        };
        let size = (g.config.width, g.config.height);
        let mut hud = world.hud(size);
        if world.pause.is_none() {
            draw_sights(&mut hud, &world.game.gun_sights(), &cam, size);
        }
        world.game.end_draw();
        if let Some(p) = &world.pause {
            p.overlay(&mut hud, size);
            let s = (size.1 as f32 / 360.0).round().max(1.0);
            let text = format!("DIFFICULTY: {}", world.game.difficulty().name().to_uppercase());
            hud.text(size.0 as f32 / 2.0 - Overlay::text_width(&text, s) / 2.0, size.1 as f32 * 0.9, s, [200, 200, 190, 220], &text);
        }
        let frame = match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                g.surface.configure(&g.renderer.device, &g.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        g.renderer.render(&view, size, &cam, &settings, &dynamic, &hud);
        g.window.pre_present_notify();
        g.renderer.queue.present(frame);

        if screenshot {
            std::fs::create_dir_all("screenshots").ok();
            self.shots += 1;
            let p = PathBuf::from(format!("screenshots/{}_{:03}.png", world.level.name, self.shots));
            let img = g.renderer.render_to_image(size, &cam, &settings, &dynamic, &hud);
            match save_png(&p, size, &img) {
                Ok(()) => println!("saved {}", p.display()),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
        }

        self.frames += 1;
        self.title_timer += dt;
        if self.title_timer >= 0.5 {
            let fps = self.frames as f32 / self.title_timer;
            let st = g.renderer.stats;
            let mode = match (world.game.player(), world.third_person) {
                (Some(p), true) => format!("Darci at {:.1}, {:.1}", p.fig.pos.x / 256.0, p.fig.pos.z / 256.0),
                _ => format!("free camera at {:.1}, {:.1}, {:.1}", cam.pos.x, cam.pos.y, cam.pos.z),
            };
            g.window.set_title(&format!(
                "Urban Chaos: {}  |  {mode}  |  {fps:.0} fps, {}/{} chunks, {} draws",
                world.level.name, st.chunks_drawn, st.chunks_total, st.draw_calls
            ));
            self.frames = 0;
            self.title_timer = 0.0;
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_none() {
            if let Err(e) = self.init_gfx(el) {
                eprintln!("error: {e}");
                el.exit();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = self.gfx.as_mut() {
                    g.config.width = size.width.max(1);
                    g.config.height = size.height.max(1);
                    g.surface.configure(&g.renderer.device, &g.config);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if let Some(t) = &event.text {
                                self.input.typed.push_str(t);
                            }
                            // Held Backspace repeats, in the console.
                            if !event.repeat || code == KeyCode::Backspace {
                                self.input.pressed.insert(code);
                            }
                            self.input.down.insert(code);
                        }
                        ElementState::Released => {
                            self.input.down.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Right, .. } => {
                self.input.rmb = state == ElementState::Pressed;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                if std::mem::take(&mut self.input.swallow_click) > 0.0 && state == ElementState::Pressed {
                    return;
                }
                self.input.lmb = state == ElementState::Pressed;
                self.input.lmb_pressed |= self.input.lmb;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.wheel += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 50.0,
                };
            }
            WindowEvent::Focused(false) => {
                self.input.down.clear();
                self.input.rmb = false;
                self.input.lmb = false;
                self.input.focused = false;
                self.capture_cursor(false);
            }
            WindowEvent::Focused(true) => {
                self.input.focused = true;
                self.input.swallow_click = 0.3;
                self.capture_cursor(self.want_cursor());
            }
            WindowEvent::RedrawRequested => self.frame(el),
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.mouse += vec2(delta.0 as f32, delta.1 as f32);
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(g) = &self.gfx {
            g.window.request_redraw();
        }
    }
}

/// `--screenshot`: render one frame offscreen, without opening a window.
fn headless_screenshot(mut world: World, path: &Path) -> Result<(), String> {
    if let Some((secs, mode)) = world.args.sim {
        let dt = 1.0 / 30.0;
        let ahead = vec2(world.orbit.yaw.sin(), world.orbit.yaw.cos());
        let c = Controls { wish: if mode.is_some() { ahead } else { Vec2::ZERO }, run: mode == Some(true), ..Default::default() };
        for i in 0..(secs / dt) as usize {
            // Standing still, try out punch, punch, kick, kick first (the tutorial's first task).
            let tap = mode.is_none() && i % 30 == 10 && i < 130;
            let c = Controls { punch: tap && i < 70, kick: tap && i >= 70, fire: tap, ..c };
            world.game.skip_cutscene();
            world.game.update(dt, &c, &world.level);
        }
        // Look at the gun's target, or else the nearest enemy standing.
        if let Some(p) = world.game.player() {
            let enemy = world.game.target.or_else(|| {
                world.game.people.iter().enumerate()
                    .filter(|(_, e)| e.hostile() && !e.is_dead() && !e.is_down())
                    .min_by(|a, b| (a.1.fig.pos - p.fig.pos).length().total_cmp(&(b.1.fig.pos - p.fig.pos).length()))
                    .map(|(i, _)| i)
            });
            if let Some(e) = enemy {
                let d = world.game.people[e].fig.pos - p.fig.pos;
                world.orbit.yaw = d.x.atan2(d.z);
            }
        }
    }
    let mut r = Renderer::headless(!world.args.linear)?;
    world.upload(&mut r);
    let size = world.args.size;
    let cam = world.camera();
    let hud = world.hud(size);
    let img = r.render_to_image(size, &cam, &world.settings(), &world.dynamic(), &hud);
    save_png(path, size, &img)?;
    println!("saved {} ({}/{} chunks drawn, {} draw calls, {} triangles)", path.display(),
             r.stats.chunks_drawn, r.stats.chunks_total, r.stats.draw_calls, r.stats.triangles);
    Ok(())
}

/// `--dump-pages`: write every texture the level uses as page_NNNN.png.
fn dump_pages(world: &World, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let geo = world.level.build();
    let mut pages = geo.pages();
    pages.extend(Game::pages(&world.level));
    let mut n = 0;
    for page in pages {
        if let Ok(Some(t)) = world.level.clump.texture(page) {
            save_png(&dir.join(format!("page_{page:04}.png")), (t.width, t.height), &t.rgba)?;
            n += 1;
        }
    }
    println!("wrote {n} textures to {}", dir.display());
    Ok(())
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let screenshot = args.screenshot.clone();
    let dump = args.dump_pages.clone();
    // `--screenshot` without `--mission`: the menu.
    if let (Some(path), false, None) = (&screenshot, args.mission_given, &dump) {
        let mut r = Renderer::headless(true)?;
        let mut m = menu::Menu::new(&args.game);
        m.screen = menu::Screen::Missions;
        if let Some(t) = menu::backdrop(&args.game, &m.image()) {
            r.set_backdrop(&t);
        }
        let img = r.render_2d_to_image(args.size, &m.overlay(args.size));
        save_png(path, args.size, &img)?;
        println!("saved {}", path.display());
        return Ok(());
    }
    if dump.is_some() || screenshot.is_some() {
        let world = World::load(args)?;
        if let Some(dir) = dump {
            return dump_pages(&world, &dir);
        }
        return headless_screenshot(world, &screenshot.unwrap());
    }
    let el = EventLoop::new().map_err(|e| e.to_string())?;
    let mut sound = Sound::new(&args);
    let pads = Gilrs::new().map_err(|e| eprintln!("gamepads unavailable: {e}")).ok();
    // Straight into a mission with --mission, else the menu.
    let (world, menu) = if args.mission_given {
        let w = World::load(args.clone())?;
        if let Some(s) = sound.as_mut() {
            s.start(&w);
        }
        (Some(w), None)
    } else {
        (None, Some(menu::Menu::new(&args.game)))
    };
    let mut app = App {
        world, menu, args, gfx: None, sound, pads, input: Input::default(), last: Instant::now(), title_timer: 0.0, backdrop: String::new(),
        frames: 0, shots: 0,
    };
    el.run_app(&mut app).map_err(|e| e.to_string())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
