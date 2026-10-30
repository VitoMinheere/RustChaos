//! Props bent by a knock (`OB_damage`, fallen/Source/ob.cpp:1510): a lamp post or a sign leans
//! away, another prop crumples a little more with each knock (until the fourth, when its count
//! goes round and it's barely crumpled again). How it's bent is kept in the two
//! top bits of the prop's flags ([`Game::prop_bends`](crate::Game)), as the original keeps
//! them, and drawn as `OB_find` and `MESH_draw_poly` draw it. The level leaves these props
//! out of its static geometry for this ([`uc_world::Level::bendable`]).

use glam::{vec3, Vec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uc_formats::prm::Prim;
use uc_formats::units::radians;
use uc_world::level::prim_damage::LEAN;
use uc_world::{Batch, Level};

/// `OB_FLAG_RESERVED1` and `_RESERVED2` (Headers/ob.h): which way a leaning prop leans, and
/// for a crumpling one how crumpled it is.
pub const RESERVED1: u8 = 1 << 6;
pub const RESERVED2: u8 = 1 << 7;

/// `dlean[]` (ob.cpp:357): a leaning prop's own little extra, by its index.
const DLEAN: [i32; 4] = [25, -16, -28, 15];

/// `MESH_NUM_CRUMPLES` and `_CRUMPVALS` (DDEngine/Source/mesh.cpp:51).
const CRUMPLES: usize = 5;
const CRUMPVALS: usize = 8;

/// A knock to prop `index` (`OB_damage`'s leaning and crumpling) with its bits as they were,
/// `yaw` its facing (2048 to a turn) and `from` what the caller passes as where the knock came
/// from. Its bits as they are now. `random` is the game's next random number.
pub fn knock(bits: u8, damage: u8, yaw: i32, from: (f32, f32), random: u32) -> u8 {
    if damage & LEAN != 0 {
        // Which way it leans: by which side of its facing `from` is on, and whether it's in
        // front, then a random lean on top (`flags |= Random() & 0xc0`), so it's mostly
        // forward and to the right.
        let (lx, lz) = radians(yaw as f32).sin_cos();
        let dprod = lx * from.0 + lz * from.1;
        let cprod = lx * from.1 - lz * from.0;
        let mut bits = 0;
        if cprod > 0.0 {
            bits |= RESERVED1;
        }
        if dprod > 0.0 {
            bits |= RESERVED2;
        }
        bits | (random as u8 & 0xc0)
    } else {
        // Crumpling, one more each time. The count is kept in two bits, so 4 is 0 again: the
        // original's turning 5 back into 3 never happens.
        ((bits >> 6) + 1) << 6
    }
}

/// The bendable props, as drawn: all of them in one set of batches, built again only when one
/// is knocked.
#[derive(Default)]
pub struct Bends {
    /// The props ([`Level::bendable`]).
    props: Vec<usize>,
    /// Their prims, by id.
    prims: HashMap<u8, Option<Prim>>,
    /// `MESH_crumple` (mesh.cpp:126): for each amount of crumpling, eight nudges in world
    /// units, handed out to a prim's points in turn.
    crumple: [[Vec3; CRUMPVALS]; CRUMPLES],
    /// Everything, ready to draw.
    batches: BTreeMap<u32, Batch>,
    /// What [`Bends::batches`] was built with.
    built: Option<BTreeMap<usize, u8>>,
}

impl Bends {
    pub fn new(level: &Level) -> Bends {
        let props = level.bendable();
        let mut prims = HashMap::new();
        for &i in &props {
            let id = level.map.objects[i].prim;
            prims.entry(id).or_insert_with(|| uc_world::level::load_prim(&level.game, id));
        }
        // `MESH_init`: up to 2.5 times the amount either way along each axis, at random.
        // Only for drawing, so not from the game's own random numbers.
        let mut seed = 0x1234_5678u32;
        let mut frand = || {
            seed = seed.wrapping_mul(214013).wrapping_add(2531011);
            ((seed >> 16) & 0x7fff) as f32 / 32767.0 * 2.0 - 1.0
        };
        let mut crumple = [[Vec3::ZERO; CRUMPVALS]; CRUMPLES];
        for (i, row) in crumple.iter_mut().enumerate() {
            let amp = i as f32 * 2.5;
            for v in row.iter_mut() {
                *v = vec3(frand(), frand(), frand()) * amp;
            }
        }
        let mut bends = Bends { props, prims, crumple, batches: BTreeMap::new(), built: None };
        // Ready to draw before the first turn (the level explored without its script never
        // has one).
        bends.update(level, &BTreeMap::new());
        bends
    }

    /// The pages the level's bendable props draw with.
    pub fn pages(level: &Level) -> BTreeSet<u32> {
        let ids: BTreeSet<u8> = level.bendable().iter().map(|&i| level.map.objects[i].prim).collect();
        ids.into_iter()
            .filter_map(|id| uc_world::level::load_prim(&level.game, id))
            .flat_map(|p| p.faces.iter().map(|f| f.texture().0).collect::<Vec<_>>())
            .collect()
    }

    /// Builds the batches again if any prop's been knocked since: `bent` is each damaged
    /// prop's bits.
    pub fn update(&mut self, level: &Level, bent: &BTreeMap<usize, u8>) {
        if self.built.as_ref() == Some(bent) {
            return;
        }
        let lighter = level.lighter();
        let mut out = BTreeMap::new();
        for &i in &self.props {
            let o = &level.map.objects[i];
            let Some(prim) = self.prims.get(&o.prim).and_then(Option::as_ref) else { continue };
            let at = vec3(o.x as f32, o.y as f32, o.z as f32);
            let yaw = radians(o.yaw as f32);
            let Some(&bits) = bent.get(&i) else {
                uc_world::level::prim_batches(prim, |v| at + turn(yaw, 0.0, 0.0, v), lighter.as_ref(), &mut out);
                continue;
            };
            if prim.damage & LEAN != 0 {
                // `OB_find` (ob.cpp:360): about 7 degrees each way, and the prop's own extra.
                let lean = |set: bool, k: usize| {
                    radians(if set { 40 } else { -40 } as f32 + DLEAN[(i + k) & 3] as f32)
                };
                let (roll, pitch) = (lean(bits & RESERVED1 != 0, 0), lean(bits & RESERVED2 != 0, 1));
                uc_world::level::prim_batches(prim, |v| at + turn(yaw, pitch, roll, v), lighter.as_ref(), &mut out);
            } else {
                // `OB_find` draws it one more crumpled than its bits say (ob.cpp:375), and
                // `MESH_draw_poly` nudges each point in turn by the next of that amount's
                // eight (mesh.cpp:406).
                let amount = (bits >> 6) as usize + 1;
                let mut crumpled = prim.clone();
                for (k, p) in crumpled.points.iter_mut().enumerate() {
                    let d = self.crumple[amount][k % CRUMPVALS];
                    *p = [p[0] + d.x.round() as i16, p[1] + d.y.round() as i16, p[2] + d.z.round() as i16];
                }
                uc_world::level::prim_batches(&crumpled, |v| at + turn(yaw, 0.0, 0.0, v), lighter.as_ref(), &mut out);
            }
        }
        self.batches = out;
        self.built = Some(bent.clone());
    }

    /// The props, as last built.
    pub fn batches(&self, out: &mut BTreeMap<u32, Batch>) {
        for (&page, b) in &self.batches {
            out.entry(page).or_default().append(b);
        }
    }
}

/// A prim's point `v` turned by `MATRIX_calc(yaw, pitch, roll)` as `MESH_draw_poly` turns it
/// (DDEngine/Source/Matrix.cpp): world = transpose(matrix) * local.
fn turn(yaw: f32, pitch: f32, roll: f32, v: Vec3) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sr, cr) = roll.sin_cos();
    let m = [
        cy * cr + sy * sp * sr,
        -cp * sr,
        -sy * cr + cy * sp * sr,
        cy * sr - sy * sp * cr,
        cp * cr,
        -sy * sr - cy * sp * cr,
        sy * cp,
        sp,
        cy * cp,
    ];
    vec3(
        m[0] * v.x + m[3] * v.y + m[6] * v.z,
        m[1] * v.x + m[4] * v.y + m[7] * v.z,
        m[2] * v.x + m[5] * v.y + m[8] * v.z,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turned by its yaw alone, a prop is placed as the level places its static props.
    #[test]
    fn unbent_is_as_the_level_places_it() {
        let (s, c) = 0.7f32.sin_cos();
        let v = vec3(10.0, 20.0, 30.0);
        let want = vec3(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
        assert!((turn(0.7, 0.0, 0.0, v) - want).length() < 1e-4);
    }

    /// Crumpling goes 1, 2, 3, then round to 0 (two bits), so drawn one more crumpled than
    /// that it's never past `MESH_crumple`'s last.
    #[test]
    fn crumpling_goes_round_in_two_bits() {
        let mut bits = 0;
        let mut seen = Vec::new();
        for _ in 0..7 {
            bits = knock(bits, 4, 0, (0.0, 0.0), 0);
            seen.push(bits >> 6);
        }
        assert_eq!(seen, [1, 2, 3, 0, 1, 2, 3]);
    }

    /// A lean is towards whichever sides `from` is on, with the random lean on top.
    #[test]
    fn a_lean_follows_where_the_knock_came_from() {
        // Facing +z: from ahead on the +x side, then behind on the -x side.
        assert_eq!(knock(0, LEAN, 0, (5.0, 5.0), 0), RESERVED2);
        assert_eq!(knock(0, LEAN, 0, (-5.0, -5.0), 0), RESERVED1);
        assert_eq!(knock(0, LEAN, 0, (-5.0, -5.0), 0x80), RESERVED1 | RESERVED2);
    }
}
