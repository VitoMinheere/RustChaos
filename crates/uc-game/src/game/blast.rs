//! Explosions: barrels going up, shockwaves, damaged props and explosive charges.

use crate::*;

impl Game {
    /// A blast that knocks people about: damage falls off from `damage` at the middle to
    /// nothing at `radius`, and over 30 points knocks them down (`create_shockwave`,
    /// fallen/Source/collide.cpp). People in cars are shielded, as everywhere else here.
    /// `BARREL_shoot` (fallen/Source/barrel.cpp:1824): a bang, and a barrel that explodes
    /// goes up in a firebomb, with a shockwave of 0x200 doing up to 250 that's the shooter's
    /// doing, and knocks the barrels round it away. A cone or a bin is only knocked about.
    /// Only a shot does this: a blast, a charge's included, leaves barrels alone
    /// (`create_shockwave` doesn't look for them).
    /// Puts down a barrel (`BARREL_alloc`, barrel.cpp:1299): on top of the highest other
    /// barrel within `BARREL_STACK_RADIUS` (45) of it, `BARREL_HEIGHT` (93) up, if that's
    /// higher than the ground. Cones don't stack.
    pub(crate) fn add_barrel(&mut self, mut b: Barrel) {
        if b.kind != 1 {
            let at = Vec2::new(b.pos.x, b.pos.z);
            let top = self.barrels.iter().enumerate()
                .filter(|(_, o)| !o.gone && o.kind != 1 && Vec2::new(o.pos.x, o.pos.z).distance(at) < 45.0)
                .map(|(j, o)| (j, o.pos.y + 93.0))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((j, y)) = top.filter(|&(_, y)| y > b.pos.y) {
                b.pos.y = y;
                b.on = Some(j);
            }
        }
        self.barrels.push(b);
    }

    pub fn shoot_barrel(&mut self, k: usize, by: usize, level: &Level) {
        let b = &self.barrels[k];
        if b.gone {
            return;
        }
        let (at, explodes) = (b.pos, b.explodes);
        self.sounds.push((Sfx::Explosion, at));
        if !explodes {
            let a = (self.rand() % 628) as f32 / 100.0;
            self.barrels[k].vel = Vec2::new(a.sin(), a.cos()) * 150.0;
            return;
        }
        self.barrels[k].gone = true;
        if self.aim_barrel == Some(k) {
            self.aim_barrel = None;
        }
        self.pyro.burst(at, pyro::FIREBOMB, 256.0);
        self.bang(by, at);
        self.shockwave(level, at, 0x200 as f32, 250, Blame::Person(by));
        // `BARREL_hit_with_sphere` 0x20 down, 0x60 across: the barrels beside it fly.
        for (j, o) in self.barrels.iter_mut().enumerate() {
            let d = Vec2::new(o.pos.x - at.x, o.pos.z - at.z);
            if j != k && !o.gone && d.length() < 0x60 as f32 + 40.0 && (o.pos.y - at.y).abs() < 128.0 {
                o.vel = d.normalize_or(Vec2::X) * 600.0;
            }
        }
    }

    pub(crate) fn shockwave(&mut self, level: &Level, at: Vec3, radius: f32, damage: i32, by: Blame) {
        self.shock_props(level, at, radius, damage);
        self.light_mines(at);
        // Creatures: a little more than people get, as far out as the edge, and four times
        // that for the Balrog, who is "so hard that we'd better hurt them more here".
        let mut acts = Vec::new();
        for (k, c) in self.creatures.iter_mut().enumerate() {
            let d = (c.fig.pos - at).abs();
            let dist = d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0;
            if dist >= radius || by == Blame::Creature(k) {
                continue;
            }
            let mut hit = (damage as f32 * (radius - dist + 0x80 as f32) / radius) as i32;
            if c.kind == creature::BALROG {
                hit <<= 2;
            }
            let who = match by {
                Blame::Person(p) => Some(p),
                _ => None,
            };
            c.hit(Some((at, who)), hit, &mut acts);
        }
        self.sounds.extend(acts.into_iter().filter_map(|a| match a {
            creature::Act::Sound { id, at } => Some((Sfx::ById { id, replace: false }, at)),
            _ => None,
        }));
        // Cars: the whole of it within 0x100 of the middle, less further out
        // (`create_shockwave`, collide.cpp), of which `VEH_reduce_health` takes half.
        for v in self.vehicles.iter_mut().filter(|v| !v.wrecked()) {
            let dist = (v.pos - at).length();
            if dist >= radius {
                continue;
            }
            let dist = dist - 0x100 as f32;
            let hit = if dist <= 0.0 { damage } else { (damage as f32 * (radius - dist) / radius) as i32 };
            v.reduce_health(hit);
        }
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if p.removed || p.is_dead() {
                continue;
            }
            // QDIST3: the largest axis plus a quarter of each of the others.
            let d = (p.fig.pos - at).abs();
            let dist = d.max_element() + (d.x + d.y + d.z - d.max_element()) / 4.0;
            if dist >= radius {
                continue;
            }
            let hit = (damage as f32 * (radius - dist) / radius) as i32;
            let ko = if hit > 30 { person::ko::FINISHERS } else { 0 };
            self.people[i].take_hit(at, hit, ko, false, false, Weapon::None);
        }
    }

    /// The props a shockwave catches: those that can be damaged, within `radius` summing the
    /// three axes, that take more than 50 of its `damage` (`create_shockwave`). The original
    /// steps through each map cell's props two at a time (`oi += 1` on top of the loop's own
    /// `oi++`), so only every other one is ever looked at; that is kept. Where a cell has an
    /// odd number it also runs on past the end of the list into whatever was left in the
    /// buffer from before, which isn't.
    pub(crate) fn shock_props(&mut self, level: &Level, at: Vec3, radius: f32, damage: i32) {
        let cell = |v: f32| (v as i32) >> 10;
        let (lo, hi) = ((cell(at.x - radius), cell(at.z - radius)), (cell(at.x + radius), cell(at.z + radius)));
        let mut hit = Vec::new();
        let mut last = None;
        let mut nth = 0;
        for (i, o) in level.map.objects.iter().enumerate() {
            let c = (o.x >> 10, o.z >> 10);
            if last != Some(c) {
                (last, nth) = (Some(c), 0);
            }
            nth += 1;
            if nth % 2 == 0 || c.0 < lo.0 || c.0 > hi.0 || c.1 < lo.1 || c.1 > hi.1 {
                continue;
            }
            if level.ground.prop_damage.get(i).is_none_or(|d| d.0 & DAMAGABLE == 0) {
                continue;
            }
            let dist = (o.x as f32 - at.x).abs() + (o.y as f32 - at.y).abs() + (o.z as f32 - at.z).abs();
            if dist < radius && damage as f32 * (radius - dist) / radius > 50.0 {
                hit.push(i);
            }
        }
        for i in hit {
            self.damage_prop(level, i);
        }
    }

    /// `OB_damage`: the prop is marked damaged, and one that explodes (or a lamppost) goes up
    /// in a firebomb the first time, with a shockwave of its own.
    pub(crate) fn damage_prop(&mut self, level: &Level, i: usize) {
        let (damage, flag) = level.ground.prop_damage.get(i).copied().unwrap_or_default();
        if damage & DAMAGABLE == 0 {
            return;
        }
        // Marked before the blast, so the shockwave it sets off doesn't set it off again.
        let first = self.damaged.insert(i);
        if first && (damage & EXPLODES != 0 || flag & LAMPOST != 0) {
            let o = &level.map.objects[i];
            let at = Vec3::new(o.x as f32, o.y as f32, o.z as f32);
            self.pyro.burst(at, pyro::FIREBOMB, 256.0);
            self.sounds.push((Sfx::Explosion, at));
            self.shockwave(level, at, 0x300 as f32, 150, Blame::Nobody);
        }
    }

    /// Whether the prop a `prim_damaged` point at `pos` watches has been damaged. It watches
    /// the nearest prop in its own map cell, damaged or not (`EWAY_create_cond`).
    pub(crate) fn prim_damaged(&self, level: &Level, pos: Vec3) -> bool {
        let cell = ((pos.x as i32) >> 10, (pos.z as i32) >> 10);
        level.map.objects.iter().enumerate()
            .filter(|(_, o)| (o.x >> 10, o.z >> 10) == cell)
            .map(|(i, o)| (i, (o.x as f32 - pos.x).abs() + (o.y as f32 - pos.y).abs() + (o.z as f32 - pos.z).abs()))
            .filter(|&(_, d)| d < 0x7800 as f32)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .is_some_and(|(i, _)| self.damaged.contains(&i))
    }

    /// Charges burn down and go off: a flash, dust, streamers and a firebomb, and a shockwave
    /// five squares across that does 500 at the middle (`SPECIAL_EXPLOSIVES` in
    /// `special_normal`).
    pub(crate) fn charges(&mut self, dt: f32, level: &Level) {
        for c in &mut self.charges {
            c.fuse -= dt;
        }
        let (gone, left): (Vec<Charge>, Vec<Charge>) = self.charges.iter().partition(|c| c.fuse <= 0.0);
        self.charges = left;
        for c in gone {
            let at = c.pos + Vec3::new(0.0, 16.0, 0.0);
            self.pyro.burst(at, pyro::FLASH | pyro::DUST | pyro::STREAMERS | pyro::FIREBOMB, 0xa0 as f32);
            self.sounds.push((Sfx::Explosion, at));
            let by = self.player.map_or(Blame::Nobody, Blame::Person);
            self.shockwave(level, at, 0x500 as f32, 500, by);
        }
    }
}
