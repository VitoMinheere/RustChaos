//! What the game draws over the level: people, cars, items, effects and beacons.

use uc_formats::units::radians;
use crate::*;

impl Game {
    /// Navigation beacons: (label, position), hiding those whose target is down or gone.
    pub fn beacons(&self) -> Vec<(String, Vec3)> {
        let mut out = self.beat_beacons();
        out.extend(self.endless_beacons());
        out.extend(self.ctf_beacons());
        let Some(s) = &self.script else { return out };
        out.extend(s.beacons()
            .into_iter()
            .filter_map(|b| match b.track {
                Some(way) if self.creature_by_way.contains_key(&way) => {
                    let c = &self.creatures[self.creature_by_way[&way]];
                    (!c.dead).then_some((b.label, c.fig.pos))
                }
                Some(way) => {
                    let p = &self.people[*self.by_way.get(&way)?];
                    (!p.removed && !p.is_down()).then_some((b.label, p.fig.pos))
                }
                None => Some((b.label, b.pos)),
            })
        );
        out
    }

    /// This frame's people meshes, cars, items and effects. `eye` is where the camera is
    /// (world units), which the puffs of an explosion turn to face.
    pub fn batches(&self, level: &Level, eye: Vec3) -> BTreeMap<u32, Batch> {
        let lighter = level.lighter();
        let mut out = BTreeMap::new();
        // Looking through the scope from her head, she isn't drawn.
        let scoped = self.snipe.is_some();
        for p in self.people.iter().filter(|p| !p.removed && !matches!(p.state, State::InCar { .. }) && !(scoped && p.player)) {
            p.fig.append_batches(&mut out, lighter.as_ref());
            shadow(&mut out, p, level);
            // The weapon in hand, and a muzzle flash just after a shot. The Men in Black's own
            // mesh has their gun built into the left arm, so no AK-47 is drawn for them.
            let built_in = p.is_mib() && p.weapon.is_gun();
            if let Some((held, part, flash)) = p.weapon.held().filter(|_| !built_in && (!p.weapon.is_gun() || p.gun_out())) {
                if let Some((rot, at)) = p.fig.part_world(part) {
                    let flash = flash.filter(|_| p.flash > 0.0);
                    for id in std::iter::once(held).chain(flash) {
                        if let Some(prim) = level.weapon_prims.get(&id) {
                            let long = p.weapon == Weapon::Sniper;
                            uc_world::level::prim_batches(prim, |v| at + rot * if long { stretch(prim, v) } else { v }, lighter.as_ref(), &mut out);
                        }
                    }
                }
            }
        }
        // Cars: body and wheels (front wheels turned by the steering), and a long shadow.
        for v in &self.vehicles {
            car(v, level, lighter.as_ref(), &mut out);
            let corner = |x: f32, z: f32| {
                let p = v.to_world(x, z);
                Vec3::new(p.x, level.ground.road(&level.map, p.x, p.y, v.pos.y + 32.0) + 2.0, p.y) / uc_world::UNIT
            };
            out.entry(uc_world::SHADOW).or_default().quad(
                [corner(-170.0, -300.0), corner(170.0, -300.0), corner(-170.0, 310.0), corner(170.0, 310.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [[0, 0, 0, 170]; 4],
            );
        }
        // Mines, lying flat where they were laid (modelled 6 below their origin).
        for m in self.mines.iter().filter(|m| !m.gone) {
            if let Some(prim) = level.item_prims.get(&game::mines::MINE_PRIM) {
                uc_world::level::prim_batches(prim, |p| m.pos + p + Vec3::new(0.0, 6.0, 0.0), lighter.as_ref(), &mut out);
            }
        }
        // Grenades in the air, tumbling.
        for g in &self.grenades {
            if let Some(prim) = level.item_prims.get(&game::grenades::GRENADE_PRIM) {
                let (s, c) = g.pitch.sin_cos();
                uc_world::level::prim_batches(prim, |p| g.pos + Vec3::new(p.x, c * p.y - s * p.z, s * p.y + c * p.z), lighter.as_ref(), &mut out);
            }
        }
        // The grappling hook and its line: the line in short pieces from her hand, going from
        // dark orange to white along it (`AENG_draw`'s `HOOK_pos_point` loop, aeng.cpp:2978).
        if let Some(h) = &self.hook {
            if let Some(prim) = level.item_prims.get(&game::hook::HOOK_PRIM) {
                let (sy, cy) = h.yaw.sin_cos();
                let (sp, cp) = h.pitch.sin_cos();
                uc_world::level::prim_batches(prim, |p| {
                    let p = Vec3::new(p.x, cp * p.y - sp * p.z, sp * p.y + cp * p.z);
                    h.pos + Vec3::new(cy * p.x + sy * p.z, p.y, -sy * p.x + cy * p.z)
                }, lighter.as_ref(), &mut out);
            }
            let mut colour = [0x80u8, 0x20, 0x00];
            for w in h.line.windows(2) {
                let (a, b) = (w[0], w[1]);
                for _ in 0..4 {
                    if colour[0] < 250 {
                        colour[0] += 2;
                    } else if colour[1] < 250 {
                        colour[1] += 2;
                    } else if colour[2] < 250 {
                        colour[2] += 3;
                    }
                }
                let across = (a - eye).cross(b - a).normalize_or(Vec3::X) * 1.5;
                let corner = |p: Vec3, s: f32| (p + across * s) / uc_world::UNIT;
                let [r, g, bl] = colour;
                out.entry(uc_world::UNTEXTURED).or_default().quad(
                    [corner(a, -1.0), corner(a, 1.0), corner(b, -1.0), corner(b, 1.0)],
                    [[0.0, 0.0]; 4],
                    [[r, g, bl, 255]; 4],
                );
            }
        }
        // Barrels and cones: modelled with z up, they topple when knocked.
        for b in self.barrels.iter().filter(|b| !b.gone) {
            let Some(prim) = level.item_prims.get(&b.prim()) else { continue };
            // Standing on its base, wherever the model has it.
            let lift = -uc_world::level::prim_bounds(prim).0.z;
            let (sy, cy) = b.yaw.sin_cos();
            let (st, ct) = b.tilt.sin_cos();
            uc_world::level::prim_batches(prim, |p| {
                // Model z is up; tip over about the model x axis.
                let up = Vec3::new(p.x, p.z + lift, -p.y);
                let t = Vec3::new(up.x, ct * up.y - st * up.z, st * up.y + ct * up.z);
                b.pos + Vec3::new(cy * t.x + sy * t.z, t.y, -sy * t.x + cy * t.z)
            }, lighter.as_ref(), &mut out);
        }
        // Items turn slowly where they lie (barrels stand still).
        let spin = self.time * 1.5;
        for it in &self.items {
            let Some(prim) = uc_world::level::item_prim(it.kind).and_then(|id| level.item_prims.get(&id)) else { continue };
            let (s, c) = if it.kind == 9 { (0.0, 1.0) } else { spin.sin_cos() };
            // Floating a little above the ground (some are modelled well above their origin).
            let lift = if it.kind == 9 { 0.0 } else { 24.0 - uc_world::level::prim_bounds(prim).0.y.max(0.0) };
            let long = it.kind == SNIPER;
            uc_world::level::prim_batches(prim, |v| {
                let v = if long { stretch(prim, v) } else { v };
                it.pos + Vec3::new(c * v.x + s * v.z, v.y + lift, -s * v.x + c * v.z)
            }, lighter.as_ref(), &mut out);
        }
        // Creatures, their fireballs and their lightning.
        for c in self.creatures.iter().filter(|c| c.fig.pos.y > -1.0e4) {
            c.fig.append_batches(&mut out, lighter.as_ref());
        }
        for d in &self.dogs {
            d.fig.append_batches(&mut out, lighter.as_ref());
        }
        let billboard = |out: &mut BTreeMap<u32, Batch>, page: u32, at: Vec3, r: f32, colour: [u8; 4]| {
            let away = at - eye;
            let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * r;
            let up = right.cross(away).normalize_or(Vec3::Y) * r;
            let corner = |x: f32, y: f32| (at + right * x + up * y) / uc_world::UNIT;
            out.entry(page).or_default().quad(
                [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [colour; 4],
            );
        };
        // The mines' red lights (`DRAWXTRA_Special`, drawxtra.cpp:3018, and `BLOOM_draw`): a
        // light 15 above each, turning round at a rate of its own (3 to 10 sixteenths of a
        // 2048 turn a tick), that glows only while it points the camera's way, the brighter
        // the more straight on, so each flashes as it sweeps past. Once set off it's 25 up
        // and spins faster.
        let turn = (self.time * 20.0) as i64;
        for (k, m) in self.mines.iter().enumerate().filter(|(_, m)| !m.gone) {
            let (c0, up) = match m.fuse {
                Some(_) => ((turn << 5) & 2047, 25.0),
                None => ((((turn * (3 + (k as i64 & 7))) + k as i64 * 9) << 4) & 2047, 15.0),
            };
            let a = radians(c0 as f32);
            let at = m.pos + Vec3::new(0.0, up, 0.0);
            let towards = Vec3::new(a.sin(), 0.0, a.cos()).dot((at - eye).normalize_or(Vec3::Z));
            if towards < 0.0 {
                billboard(&mut out, uc_world::GLOW, at, 40.0, [255, 30, 20, (-towards * 255.0) as u8]);
            }
        }
        // A set charge's light the same way, green (`SPECIAL_EXPLOSIVES`, `0x007F5D`), turning
        // with its timer (`timer << 3`; the timer counts down 0x10 a tick).
        for c in &self.charges {
            let timer = (c.fuse * 20.0 * 16.0) as i64;
            let a = radians(((timer << 3) & 2047) as f32);
            let at = c.pos + Vec3::new(0.0, 25.0, 0.0);
            let towards = Vec3::new(a.sin(), 0.0, a.cos()).dot((at - eye).normalize_or(Vec3::Z));
            if towards < 0.0 {
                billboard(&mut out, uc_world::GLOW, at, 40.0, [0, 255, 186, (-towards * 255.0) as u8]);
            }
        }
        for f in &self.fireballs {
            billboard(&mut out, uc_world::GLOW, f.pos, 70.0, [255, 200, 110, 255]);
            billboard(&mut out, uc_world::GLOW, f.pos - f.vel * 2.0, 50.0, [255, 120, 40, 160]);
        }
        for s in &self.sparks {
            // A jagged line of bright points, jumping about from frame to frame.
            let steps = ((s.to - s.from).length() / 24.0).clamp(4.0, 64.0) as usize;
            let mut seed = (self.time * 60.0) as u32 ^ 0x9e37;
            for k in 0..=steps {
                let t = k as f32 / steps as f32;
                let jag = |seed: &mut u32| (rand(seed) as f32 / 32768.0 - 1.0) * 14.0 * (1.0 - (2.0 * t - 1.0).powi(2));
                let at = s.from.lerp(s.to, t) + Vec3::new(jag(&mut seed), jag(&mut seed), jag(&mut seed));
                billboard(&mut out, uc_world::GLOW, at, 9.0, [170, 190, 255, 255]);
            }
        }
        // Platforms, wherever they've got to.
        for (p, prim) in self.platforms.iter().zip(&self.plat_prims) {
            if let Some(prim) = prim {
                let (s, c) = p.yaw.sin_cos();
                uc_world::level::prim_batches(prim, |v| p.pos + Vec3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z), lighter.as_ref(), &mut out);
            }
        }
        // Charges sit still where they were put.
        if let Some(prim) = uc_world::level::item_prim(EXPLOSIVES).and_then(|id| level.item_prims.get(&id)) {
            let lift = 4.0 - uc_world::level::prim_bounds(prim).0.y;
            for c in &self.charges {
                uc_world::level::prim_batches(prim, |v| c.pos + Vec3::new(v.x, v.y + lift, v.z), lighter.as_ref(), &mut out);
            }
        }
        // Ground mist: flat sheets blended over whatever is under them.
        let secs = self.time;
        self.mists.quads(secs, |p, uv| {
            out.entry(uc_world::MIST).or_default().quad(p, uv, [[255, 255, 255, 90]; 4]);
        });
        // Drops: small streaks along their way, added like the original's droplet page, in
        // `DIRT_TYPE_URINE`'s colour.
        for d in &self.drops.drops {
            let along = d.vel.normalize_or(Vec3::NEG_Y) * 6.0;
            let across = (d.pos - eye).cross(along).normalize_or(Vec3::X) * 1.5;
            let corner = |a: f32, b: f32| (d.pos + along * a + across * b) / uc_world::UNIT;
            out.entry(uc_world::GLOW).or_default().quad(
                [corner(-1.0, -1.0), corner(-1.0, 1.0), corner(1.0, -1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [[0x77, 0x55, 0x33, 255]; 4],
            );
        }
        // Steam: soft pale puffs blended over the scene, turned to face the camera. The
        // original draws each with a frame of a wispy cloud (`POLY_PAGE_SMOKECLOUD2`), which
        // covers far less than a solid blob does, so they're thinned to match.
        for puff in &self.steam.puffs {
            let [r, g, b, a] = puff.colour();
            let colour = [r, g, b, (a as f32 * 0.35) as u8];
            let away = puff.pos - eye;
            let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * puff.radius();
            let up = right.cross(away).normalize_or(Vec3::Y) * puff.radius();
            let corner = |x: f32, y: f32| (puff.pos + right * x + up * y) / uc_world::UNIT;
            out.entry(uc_world::SHADOW).or_default().quad(
                [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [colour; 4],
            );
        }
        // Explosions and fire: round puffs turned to face the camera, the bright ones added
        // to the scene and the smoke blended over it.
        for puff in &self.pyro.puffs {
            let away = puff.pos - eye;
            let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * puff.radius();
            let up = right.cross(away).normalize_or(Vec3::Y) * puff.radius();
            let page = if puff.smoke { uc_world::SHADOW } else { uc_world::GLOW };
            let corner = |x: f32, y: f32| (puff.pos + right * x + up * y) / uc_world::UNIT;
            out.entry(page).or_default().quad(
                [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [puff.colour(); 4],
            );
        }
        self.draw_pow(&mut out, eye);
        self.draw_balloons(&mut out, level, eye);
        self.trips.batches(&mut out);
        self.sky(&mut out, level, eye);
        self.draw_choppers(&mut out, level);
        self.draw_dirt(&mut out, level);
        self.draw_tracks(&mut out);
        self.ribbons.draw(&mut out);
        self.draw_bikes(&mut out, level);
        self.draw_sea(&mut out);
        out
    }

    /// The POW sprites (`AENG_draw_pows`, aeng.cpp:4302): each a square of `explode1.tga`
    /// facing the camera, showing its frame, drawn furthest first. The original sizes them
    /// on screen, 650 times the point's perspective `Z`: at 640 x 480, with the lens of 4 and
    /// the 22 squares it draws to, that's about 60 units each way from the middle.
    fn draw_pow(&self, out: &mut BTreeMap<u32, Batch>, eye: Vec3) {
        const HALF: f32 = 60.0;
        let mut sprites: Vec<(Vec3, u8)> = self.pow.sprites().collect();
        sprites.sort_by(|a, b| (b.0 - eye).length_squared().total_cmp(&(a.0 - eye).length_squared()));
        for (pos, frame) in sprites {
            let away = pos - eye;
            let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * HALF;
            let up = right.cross(away).normalize_or(Vec3::Y) * HALF;
            let (u, v) = ((frame & 3) as f32 / 4.0, (frame >> 2) as f32 / 4.0);
            let corner = |x: f32, y: f32| (pos + right * x + up * y) / uc_world::UNIT;
            out.entry(uc_world::EXPLODE_PAGE).or_default().quad(
                [corner(-1.0, 1.0), corner(1.0, 1.0), corner(-1.0, -1.0), corner(1.0, -1.0)],
                [[u, v], [u + 0.25, v], [u, v + 0.25], [u + 0.25, v + 0.25]],
                [[255; 4]; 4],
            );
        }
    }

    /// The sea (`AENG_lower` at a `MAV_SPARE_FLAG_WATER` point, aeng.cpp:7628): each water
    /// point rises and falls by up to 16 units,
    /// `(COS((x << 8) + sea_offset / 2) + SIN((z << 8) + sea_offset / 2 + 700)) >> 13`, a wave
    /// rolling across the squares as `sea_offset` counts the milliseconds.
    pub fn draw_sea(&self, out: &mut BTreeMap<u32, Batch>) {
        let offset = (self.time * 1000.0) as i64 / 2;
        let turn = |a: i64| radians((a & 2047) as f32);
        let wave = |x: usize, z: usize| {
            let c = turn(((x as i64) << 8) + offset).cos();
            let s = turn(((z as i64) << 8) + offset + 700).sin();
            (c + s) * 65536.0 / 8192.0
        };
        for sq in &self.sea {
            let p = [0, 1, 2, 3].map(|i| {
                let (x, z) = sq.corners[i];
                let y = sq.height[i] + if sq.wave[i] { wave(x, z) } else { 0.0 };
                Vec3::new(x as f32, y / uc_world::UNIT, z as f32)
            });
            out.entry(sq.page).or_default().quad(p, sq.uv, sq.colour);
        }
    }

    /// The leaves, cans and snow (`AENG_draw_dirt`, aeng.cpp:3040). A leaf is a triangle 20
    /// to 35 units across, 8 up off the ground, cut from `leaf.tga` and tinted one of four
    /// browns and greens, darker at one corner; tumbling, it's turned by its pitch and roll.
    /// Every sixteenth isn't a leaf but a bit of rubbish, a square from `rubbish.tga` (one of
    /// them money), except on the estate. In the snow it's all snowflakes in pale greys.
    fn draw_dirt(&self, out: &mut BTreeMap<u32, Batch>, level: &Level) {
        // `LEAF_UP`.
        const UP: f32 = 8.0;
        let snow = dirt::Ground::of(level.map.texture_set) == dirt::Ground::Snow;
        let lighter = level.lighter();
        let ambient = lighter.as_ref().map_or([255; 3], |l| l.light([0.0; 3], [0.0, 1.0, 0.0]).map(|c| (c * 255.0) as u8));
        // `leaf_colour_choice_rgb`.
        const LEAVES: [[u8; 3]; 4] = [[0x33, 0x2d, 0x1d], [0x24, 0x32, 0x24], [0x12, 0x33, 0x20], [0x33, 0x2f, 0x07]];
        // `AENG_dirt_uvlookup`: sixteen points round the texture's middle.
        let uv = |k: usize, radius: f32| {
            let a = (k % 16) as f32 * std::f32::consts::TAU / 16.0;
            [0.5 + a.sin() * radius, 0.5 + a.cos() * radius]
        };
        for (i, d) in self.dirt.pieces.iter().enumerate() {
            let Some(d) = d else { continue };
            let at = Vec3::new(d.x as f32, d.y as f32 + UP, d.z as f32);
            // The rotation `AENG_draw_dirt` builds from pitch and roll (no yaw).
            let (sp, cp) = radians(d.pitch as f32).sin_cos();
            let (sr, cr) = radians(d.roll as f32).sin_cos();
            let across = Vec3::new(cr, -cp * sr, sp * sr);
            let along = Vec3::new(0.0, sp, cp);
            match d.kind {
                dirt::Kind::Can => {
                    if let Some(prim) = level.item_prims.get(&uc_world::level::CAN_PRIM) {
                        let rot = glam::Quat::from_rotation_y(radians(d.yaw as f32))
                            * glam::Quat::from_rotation_x(radians(d.pitch as f32));
                        let base = Vec3::new(d.x as f32, d.y as f32, d.z as f32);
                        uc_world::level::prim_batches(prim, |v| base + rot * v, lighter.as_ref(), out);
                    }
                }
                _ if i & 0xf == 0 && !self.estate && !snow => {
                    // Rubbish: 48 units square, lit by the night's ambient light.
                    let (u, v) = if i == 64 { (0.0, 0.5) } else if i & 32 != 0 { (0.0, 0.0) } else { (0.5, 0.0) };
                    let mut c = [ambient[0], ambient[1], ambient[2], 255];
                    if i != 64 && i & 32 == 0 && i & 64 != 0 {
                        // `&= 0xffffff00`: no blue.
                        c[2] = 0;
                    }
                    let (a, b) = (across * 24.0, along * 24.0);
                    out.entry(uc_world::level::RUBBISH_PAGE).or_default().quad(
                        [(at + b + a) / uc_world::UNIT, (at + b - a) / uc_world::UNIT, (at - b + a) / uc_world::UNIT, (at - b - a) / uc_world::UNIT],
                        [[u, v], [u + 0.5, v], [u, v + 0.5], [u + 0.5, v + 0.5]],
                        [c; 4],
                    );
                }
                _ => {
                    // `LEAF_SIZE`.
                    let size = 20.0 + (i & 15) as f32;
                    let (a, b) = (across * size, along * size);
                    let p = [at + b, at - b + a, at - b - a];
                    let (page, c, radius) = if snow {
                        let g = (((i & 0x0f) << 2) + 0xc0) as u8;
                        (uc_world::level::SNOWFLAKE_PAGE, [[g, g, g, 255]; 3], 1.0)
                    } else {
                        let [r, g, b] = LEAVES[i & 3];
                        let tint = |k: u8| [r * k, g * k, b * k, 255];
                        (uc_world::level::LEAF_PAGE, [tint(3), tint(4), tint(5)], 0.5)
                    };
                    out.entry(page).or_default().tri(
                        p.map(|v| v / uc_world::UNIT),
                        [uv(i, radius), uv(i + 16 / 3, radius), uv(i + 32 / 3, radius)],
                        c,
                    );
                }
            }
        }
    }

    /// Texture pages any person in this level might use.
    pub fn pages(level: &Level) -> Vec<u32> {
        let Some(people) = &level.people else { return Vec::new() };
        // The leaves, rubbish and snowflakes the dirt is drawn with.
        let mut out = vec![uc_world::SKY_PAGE, uc_world::MOON_PAGE, uc_world::EXPLODE_PAGE, uc_world::level::LEAF_PAGE, uc_world::level::RUBBISH_PAGE, uc_world::level::SNOWFLAKE_PAGE];
        // And the footprints and tyre tracks.
        out.extend(uc_world::DECAL_PAGES);
        // The sea's floor, which the level leaves to the game.
        out.extend(level.sea().iter().map(|s| s.page));
        // The platforms' props, which the level leaves to the game.
        for &(_, ob) in &level.platforms {
            if let Some(prim) = uc_world::level::load_prim(&level.game, level.map.objects[ob].prim) {
                out.extend(prim.faces.iter().map(|f| f.texture().0));
            }
        }
        // And any creature the mission makes.
        let kinds: BTreeSet<u8> = level.mission.event_points.iter()
            .filter(|e| e.used != 0 && e.kind == uc_formats::ucm::waypoint::CREATE_CREATURE)
            .map(|e| e.data[0] as u8)
            .collect();
        // The bike, if the mission has one.
        if level.mission.event_points.iter().any(|e| e.used != 0 && e.kind == uc_formats::ucm::waypoint::CREATE_VEHICLE && e.data[0] == 5) {
            if let Some(f) = super::bikes::bike_chunk(level).and_then(|c| Figure::anim_prim(c, Vec3::ZERO, 0.0, 1)) {
                out.extend(f.pages());
            }
            // And its exhaust.
            out.push(uc_world::geometry::SMOKER_PAGE);
        }
        for kind in kinds {
            if let Some(f) = creature_chunk(level, kind).and_then(|c| Figure::anim_prim(c, Vec3::ZERO, 0.0, 1)) {
                out.extend(f.pages());
            }
        }
        for t in 0..=ptype::MIB3 {
            for v in 0..4 {
                if let Some(f) = people.figure(t, v, Vec3::ZERO, 0.0) {
                    out.extend(f.pages());
                }
            }
        }
        out.extend(level.item_prims.values().chain(level.weapon_prims.values()).flat_map(|p| p.faces.iter().map(|f| f.texture().0)));
        out.sort_unstable();
        out.dedup();
        out
    }
}

impl Game {
    /// `AENG_NUM_RAINDROPS`: the drops drawn each frame.
    pub const RAINDROPS: usize = 128;

    /// It rains whenever it's night (`GF_RAINING`, set when the lighting isn't
    /// `NIGHT_FLAG_DAYTIME`, elev.cpp:2406).
    pub fn raining(level: &Level) -> bool {
        level.lighting.as_ref().is_some_and(|l| l.flags & uc_formats::lgt::FLAG_DAYTIME == 0)
    }

    /// The rain (`AENG_draw_rain`, aeng.cpp:2448): out of doors on a night mission, a fresh
    /// scatter of drops each frame in front of the camera and up to eight squares off, each
    /// a short streak lit by the light where it is (`SHAPE_droplet`, shape.cpp:1440: a
    /// triangle at the drop narrowing to a point 64 up and 8 back along x and z). The original
    /// scatters them through its own view volume; here it's the camera's, `look` being the way
    /// it faces. They're blended over the scene as the raindrop page is, but with the soft
    /// blob in place of `raindrop.tga`, so they're drawn half the original's ten units wide
    /// and faint to look as thin.
    pub fn rain(&self, level: &Level, eye: Vec3, look: Vec3, out: &mut BTreeMap<u32, Batch>) {
        if !Self::raining(level) || self.indoors.is_some() {
            return;
        }
        let fwd = look.normalize_or(Vec3::Z);
        let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = right.cross(fwd);
        // A 60 degree view at 4:3.
        let (half_h, half_w) = (30f32.to_radians().tan(), 30f32.to_radians().tan() * 4.0 / 3.0);
        let lighter = level.lighter();
        let mut seed = self.time.to_bits() ^ 0x5ca1ab1e;
        let mut unit = || crate::rand(&mut seed) as f32 / 65535.0;
        let batch = out.entry(uc_world::SHADOW).or_default();
        let size = uc_formats::iam::MAP_SIZE as f32 * 256.0;
        for _ in 0..Self::RAINDROPS {
            let (x, y) = (unit() * 2.0 - 1.0, unit() * 2.0 - 1.0);
            let d = (unit() + 0.1) * 256.0 * 8.0;
            let at = eye + fwd * d + right * (x * half_w * d) + up * (y * half_h * d);
            if at.x < 0.0 || at.z < 0.0 || at.x >= size || at.z >= size {
                continue;
            }
            let [r, g, b] = lighter.as_ref().map_or([0.4; 3], |l| l.light(at.into(), [0.0, 1.0, 0.0]));
            let colour = [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 110];
            let tip = at + Vec3::new(-8.0, 64.0, -8.0);
            let across = (at - eye).cross(tip - at).normalize_or(Vec3::X) * 2.5;
            batch.tri(
                [(at + across) / uc_world::UNIT, (at - across) / uc_world::UNIT, tip / uc_world::UNIT],
                [[0.5, 0.2], [0.5, 0.8], [0.95, 0.5]],
                [colour; 3],
            );
        }
    }
}

/// A soft round shadow on the surface under a person, like the original's blob shadows:
/// fainter the higher they are above it, wider when lying down.
/// A car's body and wheels (the front wheels turned by the steering).
pub(crate) fn car(v: &Vehicle, level: &Level, lighter: Option<&uc_formats::lgt::Lighter>, out: &mut BTreeMap<u32, Batch>) {
    let (body, wheel, wx, fz, rz) = vehicle::TYPES[v.kind as usize];
    // A wheel is modelled about its axle, so it rides on its own radius: the vans and
    // the jeep run on a bigger one than the cars do.
    let wheel_prim = level.item_prims.get(&wheel);
    let radius = vehicle::wheel_radius(wheel_prim);
    // The meshes are modelled nose towards -z: turn them round to face the way it drives.
    // The body sits so the wheels are where the original has them in it.
    let lift = radius - vehicle::WHEEL_ABOVE_BODY;
    if let Some(prim) = level.item_prims.get(&body) {
        uc_world::level::prim_batches(prim, |p| v.body_point(p, lift), lighter, out);
    }
    if let Some(prim) = wheel_prim {
        let (ss, cs) = v.wheel_spin.sin_cos();
        // Wheel positions mirrored like the body; the ones towards the nose steer.
        for (x, z, front) in [(wx, -rz, true), (-wx, -rz, true), (wx, -fz, false), (-wx, -fz, false)] {
            let (st, ct) = if front { (v.steer * 0.5).sin_cos() } else { (0.0, 1.0) };
            let place = |p: Vec3| {
                // Spin about the axle (x), then steer about y.
                let r = Vec3::new(p.x, cs * p.y - ss * p.z, ss * p.y + cs * p.z);
                let r = Vec3::new(ct * r.x - st * r.z, r.y, st * r.x + ct * r.z);
                v.place(r + Vec3::new(x, radius, z))
            };
            uc_world::level::prim_batches(prim, place, lighter, out);
        }
    }
}

fn shadow(out: &mut BTreeMap<u32, Batch>, p: &Person, level: &Level) {
    // Under the body, which knockdowns carry away from the figure's origin.
    let pos = p.body_pos();
    let floor = level.ground.surface(&level.map, pos.x, pos.z, pos.y + 8.0);
    let height = (pos.y - floor).max(0.0);
    let strength = (1.0 - height / 512.0).clamp(0.0, 1.0);
    if strength <= 0.0 {
        return;
    }
    let mut r = if p.is_down() { 110.0 } else { 80.0 } * (1.0 + height / 512.0);
    // Someone knocked out for good bleeds: the shadow turns into a spreading red pool.
    let blood = p.blood();
    if let Some(b) = blood {
        r *= 1.0 + 0.5 * b;
    }
    let corner = |dx: f32, dz: f32| {
        let (x, z) = (pos.x + dx * r, pos.z + dz * r);
        Vec3::new(x, level.ground.surface(&level.map, x, z, floor + 16.0) + 2.0, z) / uc_world::UNIT
    };
    let a = (200.0 * strength) as u8;
    out.entry(uc_world::SHADOW).or_default().quad(
        [corner(-1.0, -1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(1.0, 1.0)],
        [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        [match blood {
            Some(b) => [(110.0 * b) as u8, 0, 0, a.max((200.0 * b) as u8)],
            None => [0, 0, 0, a],
        }; 4],
    );
}

impl Game {
    /// The sky round the camera (`SKY_draw_poly_sky_old`, sky.cpp:1058): thirty panels 66
    /// squares out, from the ground up to 12072, the world's `sky.tga` across five at a time.
    /// At night, out of doors, the moon too (`SKY_draw_poly_moon`): an eighth of a turn up to
    /// the north, 22 squares off and half the camera's height up, about as wide as the
    /// original's 15% of the screen. The original's stars and clouds are commented out of it
    /// (aeng.cpp:7720, 9093).
    fn sky(&self, out: &mut BTreeMap<u32, Batch>, level: &Level, eye: Vec3) {
        const STEPS: usize = 30;
        const RADIUS: f32 = 66.0 * 256.0;
        const TOP: f32 = 12072.0;
        let white = [[255, 255, 255, 255]; 4];
        let at = |a: f32, y: f32| Vec3::new(eye.x + a.sin() * RADIUS, y, eye.z + a.cos() * RADIUS) / uc_world::UNIT;
        let sky = out.entry(uc_world::SKY_PAGE).or_default();
        for i in 0..STEPS {
            let (a0, a1) = (i as f32, (i + 1) as f32);
            let (a0, a1) = (a0 * std::f32::consts::TAU / STEPS as f32, a1 * std::f32::consts::TAU / STEPS as f32);
            let u = (i % 5) as f32 * 0.2;
            sky.quad([at(a0, TOP), at(a1, TOP), at(a0, 0.0), at(a1, 0.0)], [[u, 0.0], [u + 0.2, 0.0], [u, 1.0], [u + 0.2, 1.0]], white);
        }
        // `NIGHT_FLAG_DAYTIME`.
        let night = level.lighting.as_ref().is_some_and(|l| l.flags & (1 << 2) == 0);
        if !night || self.indoors.is_some() {
            return;
        }
        let dist = 22.0 * 256.0 - 128.0;
        let pitch = std::f32::consts::PI / 8.0;
        let mid = Vec3::new(eye.x, eye.y * 0.5 + pitch.sin() * dist, eye.z + pitch.cos() * dist);
        let away = (mid - eye).normalize_or(Vec3::Z);
        let radius = dist * 0.3;
        let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * radius;
        let up = right.cross(away).normalize_or(Vec3::Y) * radius;
        let corner = |x: f32, y: f32| (mid + right * x + up * y) / uc_world::UNIT;
        // `SKY_MOON_UV_IN`: the texture's edge left off.
        let (lo, hi) = (0.02, 0.98);
        out.entry(uc_world::MOON_PAGE).or_default().quad(
            [corner(-1.0, 1.0), corner(1.0, 1.0), corner(-1.0, -1.0), corner(1.0, -1.0)],
            [[lo, lo], [hi, lo], [lo, hi], [hi, hi]],
            white,
        );
    }
}

/// The sniper rifle's look: the AK-47's model half as long again, the barrel end (the
/// longer side of its longest axis) drawn out from the grip.
fn stretch(prim: &uc_formats::prm::Prim, v: Vec3) -> Vec3 {
    let (lo, hi) = uc_world::level::prim_bounds(prim);
    let size = hi - lo;
    let axis = if size.x >= size.y && size.x >= size.z { 0 } else if size.y >= size.z { 1 } else { 2 };
    let forward = if hi[axis].abs() >= lo[axis].abs() { 1.0 } else { -1.0 };
    let mut v = v;
    if v[axis] * forward > 0.0 {
        v[axis] *= 1.5;
    }
    v
}
