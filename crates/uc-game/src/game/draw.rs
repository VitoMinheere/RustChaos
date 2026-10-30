//! What the game draws over the level: people, cars, items, effects and beacons.

use uc_formats::units::radians;
use crate::*;

/// Someone on the radar ([`Game::radar`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadarDot {
    pub pos: Vec3,
    /// One of the Men in Black, drawn white; everyone else red.
    pub mib: bool,
    /// Out to get her: the dot pulses.
    pub after_her: bool,
}

impl Game {
    /// Who shows on the radar (the scanner's thugs, panel.cpp:5329): within 16 squares of the
    /// player, living gang members (red) and Men in Black (white), whatever they're doing,
    /// and anyone at all who's out to get her (`PCOM_person_wants_to_kill`), pulsing.
    pub fn radar(&self) -> Vec<RadarDot> {
        use uc_world::character::person as ptype;
        let Some(pi) = self.player else { return Vec::new() };
        let me = self.people[pi].fig.pos;
        self.people.iter().enumerate()
            .filter(|&(i, q)| i != pi && !q.removed && !q.is_dead())
            .filter(|(_, q)| Vec2::new(q.fig.pos.x - me.x, q.fig.pos.z - me.z).length() < 16.0 * 256.0)
            .filter_map(|(_, q)| {
                let gang = matches!(q.kind, ptype::THUG_RASTA | ptype::THUG_GREY | ptype::THUG_RED);
                let after_her = q.aggro() && q.foe == Some(pi);
                (gang || q.is_mib() || after_her).then_some(RadarDot { pos: q.fig.pos, mib: q.is_mib(), after_her })
            })
            .collect()
    }

    /// Navigation beacons: (label, position), hiding those whose target is down or gone.
    pub fn beacons(&self) -> Vec<(String, Vec3)> {
        let mut out = self.beat_beacons();
        out.extend(self.endless_beacons());
        out.extend(self.ctf_beacons());
        out.extend(self.coop_beacons());
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
        let show = self.show;
        // Looking through the scope from her head, she isn't drawn.
        let scoped = self.snipe.is_some();
        for p in self.people.iter().filter(|p| !p.removed && !matches!(p.state, State::InCar { .. }) && !(scoped && p.player)) {
            p.fig.append_batches(&mut out, lighter.as_ref());
            // People cast their own shapes (below); the knocked out still bleed a pool.
            if show.shadows && p.blood().is_some() {
                shadow(&mut out, p, level);
            }
            // The weapon in hand, and a muzzle flash just after a shot. The Men in Black's own
            // mesh has their gun built into the left arm, so no AK-47 is drawn for them.
            let built_in = (p.is_mib() && p.weapon.is_gun()) || !p.in_hand();
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
        if show.shadows {
            self.cast_shadows(&mut out, level);
        }
        self.reflections(&mut out, level, eye);
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
        // The light bars of cars with their sirens going (vehicle.cpp:1268): two lights, each
        // flaring as it turns past the camera, a turn every 16 ticks. The second turns the
        // other way (its direction is the first's with x and z swapped), so they flash out of
        // step; it's red, the first blue (red on the ambulance). Each car starts at a turn of
        // its own.
        for (k, v) in self.vehicles.iter().enumerate().filter(|(_, v)| v.siren) {
            let (_, wheel, ..) = vehicle::TYPES[v.kind as usize];
            let lift = vehicle::wheel_radius(level.item_prims.get(&wheel)) - vehicle::WHEEL_ABOVE_BODY;
            let Some(([first, second], red)) = v.light_bar(lift) else { continue };
            let a = radians((((k as i64 * 331) + (turn << 7)) & 2047) as f32);
            let (sin, cos) = a.sin_cos();
            let blue = if red { [223, 0, 0] } else { [0, 0, 223] };
            for (at, dir, [r, g, b]) in [(first, Vec3::new(sin, 0.0, cos), blue), (second, Vec3::new(cos, 0.0, sin), [223, 0, 0])] {
                let towards = dir.dot((at - eye).normalize_or(Vec3::Z));
                if towards < 0.0 {
                    billboard(&mut out, uc_world::GLOW, at, 60.0, [r, g, b, (-towards * 255.0) as u8]);
                }
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
        // Switches, green once pressed.
        for (k, o) in level.map.objects.iter().enumerate().filter(|(_, o)| o.prim == uc_world::level::SWITCH_OFF) {
            let Some(prim) = &self.switch_prims[self.pressed.contains(&k) as usize] else { continue };
            let (s, c) = radians(o.yaw as f32).sin_cos();
            let at = Vec3::new(o.x as f32, o.y as f32, o.z as f32);
            uc_world::level::prim_batches(prim, |v| at + Vec3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z), lighter.as_ref(), &mut out);
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
        if show.mist {
            self.mists.quads(secs, |p, uv| {
                out.entry(uc_world::MIST).or_default().quad(p, uv, [[255, 255, 255, 90]; 4]);
            });
        }
        // Drops: small streaks along their way, added like the original's droplet page, in
        // `DIRT_TYPE_URINE`'s colour or `DIRT_TYPE_WATER`'s (aeng.cpp:3749).
        for d in &self.drops.drops {
            let along = d.vel.normalize_or(Vec3::NEG_Y) * 6.0;
            let across = (d.pos - eye).cross(along).normalize_or(Vec3::X) * 1.5;
            let corner = |a: f32, b: f32| (d.pos + along * a + across * b) / uc_world::UNIT;
            out.entry(uc_world::GLOW).or_default().quad(
                [corner(-1.0, -1.0), corner(-1.0, 1.0), corner(1.0, -1.0), corner(1.0, 1.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [if d.water { [0x22, 0x44, 0x55, 255] } else { [0x77, 0x55, 0x33, 255] }; 4],
            );
        }
        // Puddles (`AENG_draw_puddles`, aeng.cpp:8600): each corner as the original numbers
        // them, bit 0 picking x1 or x2 and bit 1 z1 or z2, with the texture turned a quarter
        // round for those that run across (`rotate_uvs`). Only those within the fog.
        for pd in &self.puddles.puddles {
            let mid = Vec3::new((pd.a.0 + pd.b.0) / 2.0, pd.y, (pd.a.1 + pd.b.1) / 2.0);
            if (mid - eye).length() > 32.0 * 256.0 {
                continue;
            }
            let ([u1, v1], [u2, v2]) = pd.uvs();
            let corner = |i: usize| Vec3::new(if i & 1 != 0 { pd.a.0 } else { pd.b.0 }, pd.y, if i & 2 != 0 { pd.a.1 } else { pd.b.1 }) / uc_world::UNIT;
            let mut uv: [[f32; 2]; 4] = std::array::from_fn(|i| [if i & 1 != 0 { u1 } else { u2 }, if i & 2 != 0 { v1 } else { v2 }]);
            if pd.rotate {
                uv.swap(0, 1);
                uv.swap(1, 3);
                uv.swap(3, 2);
            }
            out.entry(puddle::PUDDLE_PAGE).or_default().quad(std::array::from_fn(corner), uv, [[255, 255, 255, 255]; 4]);
        }
        // Drips: a ring of `drip.tga` spreading and fading, added to the scene, 8 above the
        // puddle (`AENG_draw_drips`).
        for d in &self.puddles.drips {
            let r = d.size;
            let corner = |i: usize| Vec3::new(d.pos.x + if i & 1 != 0 { r } else { -r }, d.pos.y + 8.0, d.pos.z + if i & 2 != 0 { r } else { -r }) / uc_world::UNIT;
            let f = d.fade.clamp(0.0, 255.0) as u8;
            out.entry(puddle::DRIP_PAGE).or_default().quad(std::array::from_fn(corner), [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], [[f, f, f, 255]; 4]);
        }
        // Gunfire: the muzzle's sparks and where bullets land flash bright, cooling as they
        // go; blood and casings are small blended spots, dark red and brass.
        for s in &self.gunfx.sparks {
            let t = gunfx::GunFx::spark_t(s);
            billboard(&mut out, uc_world::GLOW, s.pos, 5.0, [255, (220.0 - 120.0 * t) as u8, 80, (255.0 * (1.0 - t)) as u8]);
        }
        for s in &self.gunfx.spangs {
            let t = gunfx::GunFx::spang_t(s);
            billboard(&mut out, uc_world::GLOW, s.pos, 12.0 + 22.0 * t, [255, 240, 200, (255.0 * (1.0 - t)) as u8]);
        }
        for d in &self.gunfx.blood {
            billboard(&mut out, uc_world::SHADOW, d.pos, 4.0, [150, 0, 0, 255]);
        }
        for b in &self.gunfx.brass {
            billboard(&mut out, uc_world::SHADOW, b.pos + Vec3::new(0.0, 1.5, 0.0), 2.5, [210, 170, 60, 255]);
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
        self.doors.batches(&mut out);
        if show.skyline {
            self.sky(&mut out, level, eye);
        }
        self.draw_choppers(&mut out, level);
        if show.dirt {
            self.draw_dirt(&mut out, level);
        }
        self.draw_tracks(&mut out);
        self.ribbons.draw(&mut out);
        self.draw_bikes(&mut out, level);
        self.draw_sea(&mut out);
        self.draw_physics(&mut out, level, eye);
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
                dirt::Kind::Can | dirt::Kind::HeldCan | dirt::Kind::ThrownCan => {
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
        // And the footprints, tyre tracks and puddles, and the drips in the puddles.
        out.extend(uc_world::DECAL_PAGES);
        out.push(puddle::DRIP_PAGE);
        // The sea's floor, which the level leaves to the game.
        out.extend(level.sea().iter().map(|s| s.page));
        // The doors, which the level leaves to the game.
        out.extend(level.doors().iter().flatten().map(|q| q.page));
        // The platforms' props, which the level leaves to the game.
        for &(_, ob) in &level.platforms {
            if let Some(prim) = uc_world::level::load_prim(&level.game, level.map.objects[ob].prim) {
                out.extend(prim.faces.iter().map(|f| f.texture().0));
            }
        }
        // The switches, off and on, which it leaves to the game too.
        if level.map.objects.iter().any(|o| o.prim == uc_world::level::SWITCH_OFF) {
            for id in [uc_world::level::SWITCH_OFF, uc_world::level::SWITCH_ON] {
                if let Some(prim) = uc_world::level::load_prim(&level.game, id) {
                    out.extend(prim.faces.iter().map(|f| f.texture().0));
                }
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

/// Which of the extras are drawn, as the Options screen sets them (#201; the original's
/// detail toggles, `ENV_get_value_number("draw_*")`): people's shadows, the leaves and litter,
/// ground mist, the skyline round the city and the moon. All on by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Show {
    pub shadows: bool,
    pub dirt: bool,
    pub mist: bool,
    pub skyline: bool,
    pub moon: bool,
}

impl Default for Show {
    fn default() -> Show {
        Show { shadows: true, dirt: true, mist: true, skyline: true, moon: true }
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
        if !night || self.indoors.is_some() || !self.show.moon {
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
        // And upside down in the puddles (`SKY_draw_moon_reflection`, sky.cpp:714): as far
        // under the camera's half height as the moon is over it, squashed to half its height,
        // dimmer and yellower (`0xaaaa88`), and wobbling side to side a strip at a time. The
        // renderer shows it only where a puddle does.
        if self.no_floor {
            return;
        }
        let mid = Vec3::new(eye.x, eye.y * 0.5 - pitch.sin() * dist, eye.z + pitch.cos() * dist);
        let away = (mid - eye).normalize_or(Vec3::Z);
        let right = away.cross(Vec3::Y).normalize_or(Vec3::X) * radius;
        let up = right.cross(away).normalize_or(Vec3::Y) * radius;
        let corner = |x: f32, y: f32| (mid + right * x + up * y) / uc_world::UNIT;
        // Sixteen strips, each shifted by two waves down the screen and in time, in the
        // original's pixels of a 640-wide screen (`SKY_wibble_*`, the moon 96 across).
        const STRIPS: usize = 16;
        let turn = (self.time * 20.0) as i32;
        let wibble = |k: usize| {
            let y = (k * 96 / STRIPS) as i32;
            let a1 = ((y * 62 + turn * 17) & 2047) as f32 * std::f32::consts::TAU / 2048.0;
            let a2 = ((y * 137 + turn * 78) & 2047) as f32 * std::f32::consts::TAU / 2048.0;
            (a1.sin() * 40.0 + a2.cos() * 45.0) / 8.0 / 96.0
        };
        let tint = [[0xaa, 0xaa, 0x88, 255]; 4];
        let page = out.entry(uc_world::REFLECT + uc_world::MOON_PAGE).or_default();
        for k in 0..STRIPS {
            let (t0, t1) = (k as f32 / STRIPS as f32, (k + 1) as f32 / STRIPS as f32);
            let (y0, y1) = (0.5 - t0, 0.5 - t1);
            let (w0, w1) = (wibble(k), wibble(k + 1));
            let (v0, v1) = (lo + (hi - lo) * t0, lo + (hi - lo) * t1);
            page.quad(
                [corner(-1.0 + w0, y0), corner(1.0 + w0, y0), corner(-1.0 + w1, y1), corner(1.0 + w1, y1)],
                [[lo, v0], [hi, v0], [lo, v1], [hi, v1]],
                tint,
            );
        }
    }

    /// The people casting their own shadows this frame (aeng.cpp:7770): the four nearest the
    /// camera's focus (Darci, or her car), across plus along, within six squares of it, and
    /// their maps.
    pub fn shadow_casters(&self) -> Vec<(usize, smap::Smap)> {
        let Some(focus) = self.player_vehicle().map(|v| v.pos).or(self.player().map(|p| p.fig.pos)) else { return Vec::new() };
        let scoped = self.snipe.is_some();
        let mut near: Vec<(usize, f32)> = self.people.iter().enumerate()
            .filter(|(_, p)| !p.removed && !matches!(p.state, State::InCar { .. }) && !(scoped && p.player))
            .map(|(i, p)| (i, (p.fig.pos.x - focus.x).abs() + (p.fig.pos.z - focus.z).abs()))
            .filter(|&(_, d)| d < smap::RANGE)
            .collect();
        near.sort_by(|a, b| a.1.total_cmp(&b.1));
        near.truncate(smap::MAX);
        near.into_iter().filter_map(|(i, _)| {
            let p = &self.people[i];
            let mut parts = BTreeMap::new();
            p.fig.append_batches(&mut parts, None);
            let tris: Vec<[Vec3; 3]> = parts.values()
                .flat_map(|b| b.indices.as_chunks::<3>().0.iter().map(|t| t.map(|k| Vec3::from(b.vertices[k as usize].pos) * uc_world::UNIT)))
                .collect();
            smap::Smap::new(&tris, p.body_pos()).map(|m| (i, m))
        }).collect()
    }

    /// The shadow maps to draw with this frame, by page: each pixel's coverage.
    pub fn shadow_maps(&self) -> Vec<(u32, Vec<u8>)> {
        if !self.show.shadows {
            return Vec::new();
        }
        self.shadow_casters().into_iter().enumerate().map(|(k, (_, m))| (uc_world::SMAP - k as u32, m.pixels)).collect()
    }

    /// Each caster's map laid over what it falls on (aeng.cpp:7958-8300): the ground of the
    /// three by three squares round them (or the roof they're on), and the walls beside them,
    /// each polygon clipped to the map so its fade from the person comes out right.
    fn cast_shadows(&self, out: &mut BTreeMap<u32, Batch>, level: &Level) {
        use uc_formats::iam::{facet_type, flags};
        for (k, (i, m)) in self.shadow_casters().into_iter().enumerate() {
            let p = &self.people[i];
            let at = p.body_pos();
            let page = out.entry(uc_world::SMAP - k as u32).or_default();
            let mut lay = |poly: &[Vec3], normal: Vec3| {
                if !m.falls_on(poly, normal) {
                    return;
                }
                let clipped = clip(&m, poly);
                for t in 1..clipped.len().saturating_sub(1) {
                    let tri = [clipped[0], clipped[t], clipped[t + 1]];
                    // A touch off the surface, towards the light.
                    let lift = normal.normalize_or_zero() * 1.5;
                    page.tri(tri.map(|q| (q + lift) / uc_world::UNIT), tri.map(|q| m.uv(q)), tri.map(|q| [255, 255, 255, m.fade(q)]));
                }
            };
            let (mx, mz) = ((at.x / 256.0).floor() as i32, (at.z / 256.0).floor() as i32);
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let (sx, sz) = (mx + dx, mz + dz);
                    if !(0..127).contains(&sx) || !(0..127).contains(&sz) || level.map.square(sx as usize, sz as usize).has(flags::WATER) {
                        continue;
                    }
                    // The surface they're on, each corner sampled just inside the square.
                    let (x0, z0) = (sx as f32 * 256.0, sz as f32 * 256.0);
                    let corner = |cx: f32, cz: f32| {
                        let (x, z) = (x0 + cx * 256.0, z0 + cz * 256.0);
                        let (ix, iz) = (x0 + 1.0 + cx * 254.0, z0 + 1.0 + cz * 254.0);
                        Vec3::new(x, level.ground.surface(&level.map, ix, iz, at.y + 64.0), z)
                    };
                    let (a, b, c, d) = (corner(0.0, 0.0), corner(1.0, 0.0), corner(1.0, 1.0), corner(0.0, 1.0));
                    for tri in [[a, d, c], [a, c, b]] {
                        let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
                        lay(&tri, if n.y < 0.0 { -n } else { n });
                    }
                }
            }
            // The walls beside them (`STOREY_TYPE_NORMAL` facets), each the height of its
            // storeys and, with foundations, going down into the ground.
            for f in level.map.facets.iter().skip(1).filter(|f| f.kind == facet_type::NORMAL) {
                let (e0, e1) = (Vec3::new(f.x[0] as f32 * 256.0, 0.0, f.z[0] as f32 * 256.0), Vec3::new(f.x[1] as f32 * 256.0, 0.0, f.z[1] as f32 * 256.0));
                let along = e1 - e0;
                let t = ((Vec3::new(at.x, 0.0, at.z) - e0).dot(along) / along.length_squared().max(1.0)).clamp(0.0, 1.0);
                let side = Vec3::new(at.x, 0.0, at.z) - (e0 + along * t);
                if side.length() > 320.0 {
                    continue;
                }
                let h = ((f.block_height as u32) << 4) as f32 * (f.height >> 2) as f32;
                let deep = if f.fheight != 0 { 512.0 } else { 0.0 };
                let (y0, y1) = (f.y[0] as f32, f.y[1] as f32);
                let poly = [
                    Vec3::new(e1.x, y1 - deep, e1.z), Vec3::new(e1.x, y1 + h, e1.z),
                    Vec3::new(e0.x, y0 + h, e0.z), Vec3::new(e0.x, y0 - deep, e0.z),
                ];
                // Facing the side they're on.
                let mut n = Vec3::new(-along.z, 0.0, along.x);
                if n.dot(side) < 0.0 {
                    n = -n;
                }
                lay(&poly, n);
            }
        }
    }

    /// People standing on a puddle, upside down in it (`FIGURE_draw_reflection`, aeng.cpp:8393):
    /// those in view within 0x600 of the camera (as the original measures, across plus along)
    /// whose square has a puddle on it, mirrored in the puddle's surface and lit as they are.
    /// The renderer shows them only where the puddle does.
    fn reflections(&self, out: &mut BTreeMap<u32, Batch>, level: &Level, eye: Vec3) {
        let lighter = level.lighter();
        let scoped = self.snipe.is_some();
        for p in self.people.iter().filter(|p| !p.removed && !matches!(p.state, State::InCar { .. }) && !(scoped && p.player)) {
            let at = p.fig.pos;
            if (at.x - eye.x).abs() + (at.z - eye.z).abs() >= 0x600 as f32 {
                continue;
            }
            // `PAP_FLAG_REFLECTIVE`: a puddle on the square they're on.
            let (sx, sz) = ((at.x / 256.0).floor() * 256.0, (at.z / 256.0).floor() * 256.0);
            let Some(pd) = self.puddles.puddles.iter().find(|pd| {
                pd.a.0.min(pd.b.0) < sx + 256.0 && pd.a.0.max(pd.b.0) > sx && pd.a.1.min(pd.b.1) < sz + 256.0 && pd.a.1.max(pd.b.1) > sz
            }) else { continue };
            let mirror = 2.0 * pd.y / uc_world::UNIT;
            // A rippling puddle wobbles what it shows (`WIBBLE_simple`, wibble.cpp:13): each
            // line shifted by two waves running down it and on in time, by up to a sixteenth
            // of their strengths each. A line here is two units of height, and a shift a
            // couple of units across the camera's view.
            let (s1, s2) = pd.ripple;
            let right = (at - eye).cross(Vec3::Y).normalize_or_zero() / uc_world::UNIT;
            let [y1, y2, g1, g2, ..] = puddle::RIPPLE.map(|k| k as f32);
            let turn = self.time * 20.0;
            let wobble = |y: f32| {
                let line = y * uc_world::UNIT / 2.0;
                let a1 = (line * y1 + turn * g1) * std::f32::consts::TAU / 2048.0;
                let a2 = (line * y2 + turn * g2) * std::f32::consts::TAU / 2048.0;
                (a1.sin() * s1 as f32 + a2.cos() * s2 as f32) / 8.0 * 2.0
            };
            let mut mine = BTreeMap::new();
            p.fig.append_batches(&mut mine, lighter.as_ref());
            for (page, b) in mine {
                let into = out.entry(uc_world::REFLECT + page).or_default();
                let base = into.vertices.len() as u32;
                into.vertices.extend(b.vertices.into_iter().map(|mut v| {
                    v.pos[1] = mirror - v.pos[1];
                    if s1 | s2 != 0 {
                        let shift = right * wobble(v.pos[1]);
                        v.pos[0] += shift.x;
                        v.pos[2] += shift.z;
                    }
                    v
                }));
                into.indices.extend(b.indices.into_iter().map(|i| i + base));
            }
        }
    }
}

/// `poly` clipped to where shadow map `m` covers (`SMAP_project_onto_poly`'s clipping to
/// `SMAP_u_min` and the rest), a point added where an edge crosses.
fn clip(m: &smap::Smap, poly: &[Vec3]) -> Vec<Vec3> {
    let mut pts = poly.to_vec();
    for (axis, keep_above) in [(0, true), (0, false), (1, true), (1, false)] {
        if pts.is_empty() {
            break;
        }
        // The map's edge a pixel in, where the figure stops.
        let (lo, hi) = (1.0 / smap::RES as f32, 1.0 - 1.0 / smap::RES as f32);
        let inside = |q: Vec3| {
            let c = m.uv(q)[axis];
            if keep_above { c >= lo } else { c <= hi }
        };
        let mut next = Vec::with_capacity(pts.len() + 2);
        for j in 0..pts.len() {
            let (a, b) = (pts[j], pts[(j + 1) % pts.len()]);
            let (ia, ib) = (inside(a), inside(b));
            if ia {
                next.push(a);
            }
            if ia != ib {
                let (ca, cb) = (m.uv(a)[axis], m.uv(b)[axis]);
                let edge = if keep_above { lo } else { hi };
                next.push(a + (b - a) * ((edge - ca) / (cb - ca)));
            }
        }
        pts = next;
    }
    pts
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
