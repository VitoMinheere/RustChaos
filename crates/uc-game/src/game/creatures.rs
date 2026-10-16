//! The creatures' own clock: Bane, the Balrog, gargoyles and bats.

use crate::*;

impl Game {
    /// The creatures go about their business on the original's own 20 ticks a second, and
    /// their fireballs fly.
    pub(crate) fn run_creatures(&mut self, dt: f32, level: &Level, frozen: bool) {
        if self.creatures.is_empty() && self.fireballs.is_empty() {
            return;
        }
        // The Balrog burns as he goes (`PYRO_IMMOLATE` on him).
        for c in self.creatures.iter().filter(|c| c.kind == creature::BALROG && !c.dead) {
            for _ in 0..2 {
                let r = |rng: &mut u32| rand(rng) as f32 / 32768.0 - 1.0;
                let (dx, dz, up) = (r(&mut self.rngs.creature), r(&mut self.rngs.creature), r(&mut self.rngs.creature));
                let at = c.fig.pos + Vec3::new(dx * 90.0, 160.0 + up * 150.0, dz * 90.0);
                self.pyro.lick(at, 50.0);
            }
        }
        self.clocks.creature += dt;
        while self.clocks.creature >= creature::TICK {
            self.clocks.creature -= creature::TICK;
            self.creature_tick(level, frozen);
        }
    }

    pub(crate) fn creature_tick(&mut self, level: &Level, frozen: bool) {
        let seen: Vec<creature::Seen> = self.people.iter().enumerate().map(|(i, p)| {
            let pos = self.thing_pos(i);
            let hips = p.fig.hips();
            creature::Seen {
                pos,
                head: Vec3::new(hips.x, pos.y + 2.0 * (hips.y - pos.y).max(40.0), hips.z),
                alive: !p.removed && !p.is_dead() && !p.is_down(),
                dead: p.is_dead() || p.removed,
                player: p.player,
                ai: p.ai,
                up_high: pos.y - level.map.ground_height(pos.x, pos.z) > 128.0,
            }
        }).collect();
        let floor = |x: f32, z: f32| level.ground.surface(&level.map, x, z, 0x7fff as f32);
        let terrain = |x: f32, z: f32| level.map.ground_height(x, z);
        let clear = |a: Vec3, b: Vec3| level.ground.line_of_sight(a, b);
        let slide = |at: Vec2, y: f32, by: Vec2| level.ground.slide(at, y + 64.0, by, 48.0);
        let around = creature::Around {
            people: &seen, player: self.player, floor: &floor, terrain: &terrain, clear: &clear, slide: &slide, frozen,
        };
        let mut acts = Vec::new();
        let mut rng = self.rngs.creature;
        for (k, c) in self.creatures.iter_mut().enumerate() {
            let before = acts.len();
            c.tick(k, &around, &mut rng, &mut acts);
            // Bolts start from the creature: tag them with where it is.
            for a in &mut acts[before..] {
                if let creature::Act::Spark { .. } = a {
                    self.sparks.push(Spark { from: c.middle() + Vec3::new(0.0, 40.0, 0.0), to: Vec3::ZERO, left: 0.0 });
                }
            }
        }
        self.rngs.creature = rng;
        // Fireballs fall and burst on whatever they meet.
        let mut bursts = Vec::new();
        for f in &mut self.fireballs {
            f.vel.y -= creature::Fireball::GRAVITY;
            f.pos += f.vel;
            f.age += creature::TICK;
            if f.pos.y < level.ground.surface(&level.map, f.pos.x, f.pos.z, f.pos.y + 64.0) {
                bursts.push(f.pos);
                f.age = creature::Fireball::LIFE;
            }
        }
        self.fireballs.retain(|f| f.age < creature::Fireball::LIFE);
        for at in bursts {
            // `PYRO_WHOOMPH`, and a small shockwave that blames nobody.
            self.pyro.burst(at, pyro::FIREBALL | pyro::DUST, 0x140 as f32);
            self.sounds.push((Sfx::Explosion, at));
            self.shockwave(level, at, 0x140 as f32, 80, Blame::Nobody);
        }
        let mut spark = self.sparks.len() - acts.iter().filter(|a| matches!(a, creature::Act::Spark { .. })).count();
        for a in acts {
            match a {
                creature::Act::Shockwave { at, radius, damage, from } => self.shockwave(level, at, radius, damage, Blame::Creature(from)),
                creature::Act::Fireball { at, vel } => self.fireballs.push(creature::Fireball { pos: at, vel, age: 0.0 }),
                creature::Act::Hurt { person, damage, recoil, from } => self.hurt_directly(person, damage, recoil, from),
                creature::Act::Sound { id, at } => self.sounds.push((Sfx::ById { id, replace: false }, at)),
                creature::Act::Float { person } => {
                    let p = &mut self.people[person];
                    if !p.removed && !matches!(p.state, State::InCar { .. }) {
                        let floor = level.ground.surface(&level.map, p.fig.pos.x, p.fig.pos.z, p.fig.pos.y + 32.0);
                        p.state = State::Float { floor };
                    }
                }
                creature::Act::Spark { person, head } => {
                    let p = &self.people[person];
                    let hips = p.fig.hips();
                    let to = if head { Vec3::new(hips.x, p.fig.pos.y + 2.0 * (hips.y - p.fig.pos.y), hips.z) } else { hips };
                    let s = &mut self.sparks[spark];
                    (s.to, s.left) = (to, if head { 0.4 } else { 1.0 });
                    spark += 1;
                }
                creature::Act::Dust { at } => self.pyro.burst(at, pyro::DUST, 300.0),
                creature::Act::BalrogKill => {
                    if let Some(s) = &mut self.script {
                        s.note_balrog_kill();
                    }
                }
            }
        }
    }

    /// Damage straight off someone's health with a cry of pain, as a gargoyle's swoop and
    /// Bane's bolt do: it kills them when there's none left, and otherwise maybe sets them
    /// reeling (`set_person_recoil`).
    pub(crate) fn hurt_directly(&mut self, i: usize, damage: i32, recoil: bool, from: Vec3) {
        let p = &mut self.people[i];
        if p.removed || p.knocked_out || p.invulnerable || matches!(p.state, State::InCar { .. }) {
            return;
        }
        p.health -= damage;
        if p.health <= 0 {
            if p.is_down() {
                p.health = 0;
                p.knocked_out = true;
                p.cues.push(Sfx::KnockedOut { kind: p.kind });
            } else {
                p.take_hit(from, 0, person::ko::FINISHERS, false, false, Weapon::None);
            }
        } else if recoil && matches!(p.state, State::Move | State::Block | State::Attack { .. } | State::Shoot { .. }) {
            p.face_towards(from);
            p.take_hit(from, 0, 0, false, false, Weapon::None);
        } else {
            p.cues.push(Sfx::Hurt { kind: p.kind });
        }
    }
}

/// The creature with its health showing: the one in the player's sights, or else the
/// nearest still standing within twelve squares. (name, health 0..1)
impl Game {
    pub fn creature_health(&self) -> Option<(&'static str, f32)> {
        let me = self.player()?.fig.pos;
        let c = self.aiming.creature.map(|k| &self.creatures[k]).or_else(|| {
            self.creatures.iter()
                .filter(|c| c.targetable() && (c.fig.pos - me).length() < 12.0 * 256.0)
                .min_by(|a, b| (a.fig.pos - me).length().total_cmp(&(b.fig.pos - me).length()))
        })?;
        Some((c.name(), c.health.max(0) as f32 / c.max_health() as f32))
    }
}
