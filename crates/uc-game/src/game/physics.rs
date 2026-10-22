//! The original's unused physics (#111), as console extras: sphere-matter (`sm`), the dike
//! (`dike`) and cloth (`flag`). Each was made only by a debug key the shipped game
//! comments out; here each is made the way its key made it, run a turn at a time, and
//! drawn as the original drew it.

use crate::*;

/// What the extras see of the level: the floor (roofs and prop tops too) and the roads.
struct Floor<'a>(&'a Level);

impl Floor<'_> {
    /// The floor under (x, z) for something at height `y`: whatever's no more than a step
    /// above it.
    fn at(&self, x: f32, z: f32, y: f32) -> f32 {
        self.0.ground.surface(&self.0.map, x, z, y + uc_world::ground::STEP_HEIGHT)
    }
}

impl sm::Ground for Floor<'_> {
    fn height(&self, x: f32, z: f32, y: f32) -> f32 {
        self.at(x, z, y)
    }
}

impl cloth::Ground for Floor<'_> {
    fn height(&self, x: f32, z: f32, y: f32) -> f32 {
        self.at(x, z, y)
    }
}

impl dike::Road for Floor<'_> {
    fn height(&self, x: f32, z: f32, y: f32) -> f32 {
        self.at(x, z, y)
    }
    fn is_road(&self, x: f32, z: f32) -> bool {
        road::is_road(&self.0.map, (x / 256.0) as i32, (z / 256.0) as i32)
    }
}

impl Game {
    /// `sm`: clears the sphere-matter and drops a cube of it, 0x80 a side, 0x80 above where
    /// the player stands (the debug key, Controls.cpp:4233).
    pub fn sphere_matter_test(&mut self) {
        let Some(at) = self.player().map(|p| p.fig.pos) else { return };
        self.sm.clear();
        let lo = at + Vec3::new(0.0, 128.0, 0.0);
        self.sm.create_cube(lo, lo + Vec3::splat(128.0), 3, 0x10000, 0x10000);
    }

    /// `dike`: a new dike where the player stands, in place of the last (Controls.cpp:2217).
    pub fn dike_test(&mut self, level: &Level) {
        let Some(at) = self.player().map(|p| p.fig.pos) else { return };
        self.dike = Some(dike::Dike::new(at.x, at.y, at.z, &Floor(level)));
    }

    /// `flag`: a pink flag (`0x00ff8888`) half a square off from the player each way and
    /// half a square up, sixteen units between its points, hung by its left corners
    /// (Controls.cpp:4070). Whether there was room for another.
    pub fn flag_test(&mut self) -> bool {
        let Some(at) = self.player().map(|p| p.fig.pos) else { return false };
        let Some(c) = self.cloths.create(cloth::Kind::Flag, at + Vec3::splat(128.0), Vec3::new(16.0, 0.0, 0.0), Vec3::new(0.0, 16.0, 0.0), 16.0, 0xff8888) else {
            return false;
        };
        self.cloths.lock(c, 0, 0);
        self.cloths.lock(c, 0, cloth::HEIGHT - 1);
        true
    }

    /// A turn of the extras.
    pub(crate) fn run_physics(&mut self, dt: f32, level: &Level) {
        let floor = Floor(level);
        self.sm.update(dt, &floor);
        self.cloths.update(dt, &floor);
        if let Some(d) = self.dike.as_mut() {
            d.control = self.dike_control;
            self.dike_clock += dt;
            while self.dike_clock >= 1.0 / dike::STEPS_PER_SECOND {
                self.dike_clock -= 1.0 / dike::STEPS_PER_SECOND;
                d.step(&floor);
            }
        }
    }

    /// Draws the extras.
    pub(crate) fn draw_physics(&self, out: &mut BTreeMap<u32, Batch>, level: &Level, eye: Vec3) {
        let lighter = level.lighter();
        let shader = uc_world::level::Shader { lighter: lighter.as_ref() };
        for (at, r, colour) in self.sm.spheres() {
            sphere(out, &shader, at, r, colour);
        }
        // `DIKE_draw`: the frame, from nothing at the front wheel to 32 wide at the back, and
        // the bars, nothing to 16 wide.
        if let Some(d) = &self.dike {
            let (front, back) = d.wheels();
            line(out, eye, front, 0.0, [0xff, 0xee, 0xee, 255], back, 32.0, [255; 4]);
            let (a, b) = d.bars();
            line(out, eye, a, 0.0, [0x00, 0xff, 0xee, 255], b, 16.0, [255; 4]);
        }
        for c in &self.cloths.cloths {
            flag(out, c);
        }
    }
}

/// `SHAPE_sphere` (shape.cpp:617): eight rings of eight round from top to bottom, each point
/// lit as the light falls on it there and tinted `colour`.
fn sphere(out: &mut BTreeMap<u32, Batch>, shader: &uc_world::level::Shader, at: Vec3, r: f32, colour: [u8; 4]) {
    const UPDOWN: usize = 8;
    const AROUND: usize = 8;
    let unit = |n: Vec3| at + n * r;
    let tint = |p: Vec3, n: Vec3| {
        let l = shader.at(p / uc_world::UNIT, n);
        [0, 1, 2].map(|k| (l[k] as u32 * colour[k] as u32 / 255) as u8)
    };
    let point = |n: Vec3| {
        let p = unit(n);
        let [r, g, b] = tint(p, n);
        (p / uc_world::UNIT, [r, g, b, 255])
    };
    let dir = |i: usize, j: usize| {
        let pitch = std::f32::consts::FRAC_PI_2 - std::f32::consts::PI * i as f32 / UPDOWN as f32;
        let yaw = std::f32::consts::TAU * j as f32 / AROUND as f32;
        Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos())
    };
    let b = out.entry(uc_world::UNTEXTURED).or_default();
    for i in 0..UPDOWN {
        for j in 0..AROUND {
            let k = (j + 1) % AROUND;
            let (p00, p01, p10, p11) = (point(dir(i, j)), point(dir(i, k)), point(dir(i + 1, j)), point(dir(i + 1, k)));
            b.quad([p00.0, p01.0, p10.0, p11.0], [[0.0; 2]; 4], [p00.1, p01.1, p10.1, p11.1]);
        }
    }
}

/// `AENG_world_line`: a line from `a` to `b`, `wa` and `wb` wide at its ends and shaded
/// from one colour to the other, turned to face the camera.
#[allow(clippy::too_many_arguments)]
fn line(out: &mut BTreeMap<u32, Batch>, eye: Vec3, a: Vec3, wa: f32, ca: [u8; 4], b: Vec3, wb: f32, cb: [u8; 4]) {
    let side = (b - a).cross((a + b) * 0.5 - eye).normalize_or_zero();
    // Nothing wide still shows, a unit across.
    let (wa, wb) = (wa.max(1.0) / 2.0, wb.max(1.0) / 2.0);
    let p = [a + side * wa, a - side * wa, b + side * wb, b - side * wb].map(|v| v / uc_world::UNIT);
    out.entry(uc_world::UNTEXTURED).or_default().quad(p, [[0.0; 2]; 4], [ca, ca, cb, cb]);
}

/// `AENG_draw_cloth` (aeng.cpp:2742): a quad between each four neighbouring points, each
/// point as bright as its normal faces (0.55, 0.55, 0.55), either side, tinting the
/// cloth's colour.
fn flag(out: &mut BTreeMap<u32, Batch>, c: &cloth::Cloth) {
    let [_, r, g, b] = c.colour.to_be_bytes();
    let colour = |i: usize| {
        let bright = (c.normal(i).dot(Vec3::splat(0.55)).abs() * 255.0) as u32;
        [r, g, b].map(|v| ((bright.min(255) * v as u32) >> 8) as u8)
    };
    let batch = out.entry(uc_world::UNTEXTURED).or_default();
    for x in 0..cloth::WIDTH - 1 {
        for y in 0..cloth::HEIGHT - 1 {
            let i = [cloth::index(x, y), cloth::index(x + 1, y), cloth::index(x, y + 1), cloth::index(x + 1, y + 1)];
            let p = i.map(|i| c.pos[i] / uc_world::UNIT);
            let col = i.map(|i| {
                let [r, g, b] = colour(i);
                [r, g, b, 255]
            });
            batch.quad(p, [[0.0; 2]; 4], col);
        }
    }
}
