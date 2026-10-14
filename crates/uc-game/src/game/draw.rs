//! What the game draws over the level: people, cars, items, effects and beacons.

use crate::*;

impl Game {
    /// Navigation beacons: (label, position), hiding those whose target is down or gone.
    pub fn beacons(&self) -> Vec<(String, Vec3)> {
        let Some(s) = &self.script else { return Vec::new() };
        s.beacons()
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
            .collect()
    }

    /// This frame's people meshes, cars, items and effects. `eye` is where the camera is
    /// (world units), which the puffs of an explosion turn to face.
    pub fn batches(&self, level: &Level, eye: Vec3) -> BTreeMap<u32, Batch> {
        let lighter = level.lighter();
        let mut out = BTreeMap::new();
        for p in self.people.iter().filter(|p| !p.removed && !matches!(p.state, State::InCar { .. })) {
            p.fig.append_batches(&mut out, lighter.as_ref());
            shadow(&mut out, p, level);
            // The weapon in hand, and a muzzle flash just after a shot.
            if let Some((held, part, flash)) = p.weapon.held().filter(|_| !p.weapon.is_gun() || p.gun_out()) {
                if let Some((rot, at)) = p.fig.part_world(part) {
                    let flash = flash.filter(|_| p.flash > 0.0);
                    for id in std::iter::once(held).chain(flash) {
                        if let Some(prim) = level.weapon_prims.get(&id) {
                            uc_world::level::prim_batches(prim, |v| at + rot * v, lighter.as_ref(), &mut out);
                        }
                    }
                }
            }
        }
        // Cars: body and wheels (front wheels turned by the steering), and a long shadow.
        for v in &self.vehicles {
            let (body, wheel, wx, fz, rz) = vehicle::TYPES[v.kind as usize];
            // A wheel is modelled about its axle, so it rides on its own radius: the vans and
            // the jeep run on a bigger one than the cars do.
            let wheel_prim = level.item_prims.get(&wheel);
            let radius = wheel_prim.map_or(38.0, |prim| {
                let (lo, hi) = uc_world::level::prim_bounds(prim);
                (hi.y - lo.y) / 2.0
            });
            // The meshes are modelled nose towards -z: turn them round to face the way it drives.
            // The body sits so the wheels are where the original has them in it.
            let lift = radius - vehicle::WHEEL_ABOVE_BODY;
            if let Some(prim) = level.item_prims.get(&body) {
                uc_world::level::prim_batches(prim, |p| v.place(Vec3::new(-p.x, p.y + lift, -p.z)), lighter.as_ref(), &mut out);
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
                    uc_world::level::prim_batches(prim, place, lighter.as_ref(), &mut out);
                }
            }
            let corner = |x: f32, z: f32| {
                let p = v.to_world(x, z);
                Vec3::new(p.x, level.ground.surface(&level.map, p.x, p.y, v.pos.y + 32.0) + 2.0, p.y) / uc_world::UNIT
            };
            out.entry(uc_world::SHADOW).or_default().quad(
                [corner(-170.0, -300.0), corner(170.0, -300.0), corner(-170.0, 310.0), corner(170.0, 310.0)],
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                [[0, 0, 0, 170]; 4],
            );
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
            uc_world::level::prim_batches(prim, |v| it.pos + Vec3::new(c * v.x + s * v.z, v.y + lift, -s * v.x + c * v.z),
                                          lighter.as_ref(), &mut out);
        }
        // Creatures, their fireballs and their lightning.
        for c in self.creatures.iter().filter(|c| c.fig.pos.y > -1.0e4) {
            c.fig.append_batches(&mut out, lighter.as_ref());
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
        self.trips.batches(&mut out);
        out
    }

    /// Texture pages any person in this level might use.
    pub fn pages(level: &Level) -> Vec<u32> {
        let Some(people) = &level.people else { return Vec::new() };
        let mut out = Vec::new();
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

/// A soft round shadow on the surface under a person, like the original's blob shadows:
/// fainter the higher they are above it, wider when lying down.
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
