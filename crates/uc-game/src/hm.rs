//! Hypermatter (fallen/Source/hm.cpp, Headers/hm.h): a soft-body prototype. A prim is boxed
//! in a lattice of up to 8x8x8 points (`HM_create`, hm.cpp:474), each lattice point joined
//! to its neighbours, across faces and through the middle of each cube, by springs that want
//! their starting length (`HM_process`, hm.cpp:2362). The points fall, bounce off the ground
//! and off "colvects", infinitely tall walls the caller hands over, and the prim is drawn
//! through the bent lattice: each of its points is kept as a corner of its cube plus a
//! fraction of the cube's three edges from that corner (`HM_find_point_inside_cube`,
//! hm.cpp:385, and `HM_find_mesh_point`, hm.cpp:1302). Two objects can be made to push each
//! other apart (`HM_collide`, hm.cpp:2011) when a point of one gets inside a cube of the
//! other.
//!
//! The shipped game never runs any of it. The only `HM_process` call, eight times a turn
//! (Controls.cpp:2986), is commented out, as are the debug key that drops prim 6 above Darci
//! (Controls.cpp:4269) and the one that turns furniture into hypermatter
//! (`FURN_hypermatterise`, Furn.cpp:950). `HM_init` and `HM_load` are never called either, so
//! every prim gets [`DEFAULT_PRIMGRID`]. This is here as an extra, off unless asked for.
//!
//! Units are the original's: world units, y up, angles in 2048ths of a turn. A step is an
//! eighth of a turn ([`STEPS_PER_TURN`]).

// The index loops follow the original's, over the lattice's x, y and z.
#![allow(clippy::needless_range_loop)]

use glam::{vec3, Vec3};
use uc_formats::prm::Prim;

/// `HM_MAX_RES`: the most lattice points along an axis.
pub const MAX_RES: usize = 8;
/// `HM_MAX_OBJECTS`.
pub const MAX_OBJECTS: usize = 8;
/// `HM_COLS_PER_OBJECT`.
const COLS_PER_OBJECT: usize = 64;
/// `HM_GRAVITY`, a step.
const GRAVITY: f32 = -0.015;
/// `AT_TWENTY` (Controls.cpp:2980): `HM_process` runs eight times a 20 Hz turn (four with
/// shift held).
pub const STEPS_PER_TURN: usize = 8;
/// `PRIM_MIN_BBOX` (Prim.cpp:23).
const PRIM_MIN_BBOX: i32 = 0x58;
/// `INFINITY` (MFStdLib.h:319), an integer: `float(INFINITY)` is the "no answer" of
/// `HM_collide`'s entry search.
const INFINITY: f32 = 0x7fff_ffff as f32;

/// What hypermatter needs from the world.
pub trait HmWorld {
    /// The ground under (x, z), world units. The original's `HM_process` takes it as 0
    /// everywhere (hm.cpp:2581); `HM_height_at` (`PAP_calc_height_at`, hm.cpp:1287) is
    /// written but never called.
    fn height(&self, x: f32, z: f32) -> f32;
}

/// `HM_Primgrid` (hm.h:32): where the lattice planes go along each axis, as fractions of
/// the prim's bounding box (0 its lowest side, 0x10000 its highest), and how mass varies
/// across the object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Primgrid {
    /// `x_res`, `y_res`, `z_res`: points along each axis, 2 to [`MAX_RES`].
    pub res: [usize; 3],
    /// `x_point`, `y_point`, `z_point`.
    pub points: [[i32; MAX_RES]; 3],
    /// `x_dgrav`, `y_dgrav`, `z_dgrav`: how much heavier each step along an axis makes a
    /// point. Beyond 1 in size, part of the object weighs less than nothing.
    pub dgrav: Vec3,
}

/// `HM_default_primgrid` (hm.cpp:349): a single cube round the whole prim, its far z end a
/// quarter lighter.
pub const DEFAULT_PRIMGRID: Primgrid = Primgrid {
    res: [2, 2, 2],
    points: [[0, 0x10000, 0, 0, 0, 0, 0, 0]; 3],
    dgrav: Vec3::new(0.0, 0.0, -0.25),
};

/// `HM_get_primgrid` (hm.cpp:366). Nothing loads the editor's grids (`HM_load` is never
/// called), so it's always [`DEFAULT_PRIMGRID`].
pub fn primgrid(_prim: u16) -> Primgrid {
    DEFAULT_PRIMGRID
}

/// How the object behaves, each 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// How hard the springs pull.
    pub elasticity: f32,
    /// How much of a point's fall it keeps when it hits the ground.
    pub bounciness: f32,
    /// How much of its sideways speed a point keeps when it hits the ground.
    pub friction: f32,
    /// How much of its speed every point keeps each step (so 1 is no damping at all).
    pub damping: f32,
}

/// What the debug key gives prim 6 (Controls.cpp:4292).
pub const DEBUG_KEY: Params = Params { elasticity: 0.050, bounciness: 0.900, friction: 0.000, damping: 0.999 };
/// What `FURN_hypermatterise` gives furniture (Furn.cpp:1024).
pub const FURNITURE: Params = Params { elasticity: 0.010, bounciness: 0.950, friction: 0.850, damping: 0.997 };

/// `HM_Point`: a lattice point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub pos: Vec3,
    /// A step's movement.
    pub vel: Vec3,
    /// How much gravity it gets.
    pub mass: f32,
}

/// `HM_Edge`: a spring between two points, wanting the length whose square is
/// `size[len]`.
#[derive(Debug, Clone, Copy)]
struct Edge {
    p1: usize,
    p2: usize,
    len: usize,
}

/// `HM_Mesh`: a prim point as a lattice point plus fractions of the edges to three others.
#[derive(Debug, Clone, Copy)]
struct Mesh {
    origin: usize,
    p: [usize; 3],
    along: [f32; 3],
}

/// `HM_Col`: a wall, infinitely tall, from (x1, z1) to (x2, z2).
#[derive(Debug, Clone, Copy)]
struct Col {
    x1: f32,
    z1: f32,
    x2: f32,
    z2: f32,
}

/// `HM_Bump`: a point of this object that has got inside a cube of another, and where on
/// the cube's surface it went in (in the cube's own space, 0 to 1 along each edge).
#[derive(Debug, Clone, Copy)]
struct Bump {
    point: usize,
    other: usize,
    cube: [usize; 3],
    rel: Vec3,
}

/// `MUL64`: 16.16 multiply.
fn mul64(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> 16) as i32
}

/// `MATRIX_calc` (DDEngine/Source/Matrix.cpp:19), angles in radians.
fn matrix_calc(yaw: f32, pitch: f32, roll: f32) -> [f32; 9] {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sr, cr) = roll.sin_cos();
    let mut m = [0.0; 9];
    m[0] = cy * cr + sy * sp * sr;
    m[3] = cy * sr - sy * sp * cr;
    m[6] = sy * cp;
    m[1] = -cp * sr;
    m[4] = cp * cr;
    m[7] = sp;
    m[2] = -sy * cr + cy * sp * sr;
    m[5] = -sy * sr - cy * sp * cr;
    m[8] = cy * cp;
    m
}

/// `MATRIX_MUL_BY_TRANSPOSE`.
fn mul_by_transpose(m: &[f32; 9], v: Vec3) -> Vec3 {
    vec3(
        v.x * m[0] + v.y * m[3] + v.z * m[6],
        v.x * m[1] + v.y * m[4] + v.z * m[7],
        v.x * m[2] + v.y * m[5] + v.z * m[8],
    )
}

/// `MATHS_seg_intersect` (maths.cpp:67): whether segments v and w cross, ends included. In
/// 64 bits where the original's 32 could overflow on long segments.
#[allow(clippy::too_many_arguments)]
fn seg_intersect(vx1: i64, vz1: i64, vx2: i64, vz2: i64, wx1: i64, wz1: i64, wx2: i64, wz2: i64) -> bool {
    // Bounding boxes first.
    if vx1 < vx2 {
        if wx1 < vx1 && wx2 < vx1 { return false; }
        if wx1 > vx2 && wx2 > vx2 { return false; }
    } else {
        if wx1 > vx1 && wx2 > vx1 { return false; }
        if wx1 < vx2 && wx2 < vx2 { return false; }
    }
    if vz1 < vz2 {
        if wz1 < vz1 && wz2 < vz1 { return false; }
        if wz1 > vz2 && wz2 > vz2 { return false; }
    } else {
        if wz1 > vz1 && wz2 > vz1 { return false; }
        if wz1 < vz2 && wz2 < vz2 { return false; }
    }
    let (ax, az) = (vx2 - vx1, vz2 - vz1);
    let (bx, bz) = (wx1 - vx1, wz1 - vz1);
    let (dx, dz) = (wx2 - wx1, wz2 - wz1);
    // `MATH_DIFFERENT_SIGN`: the sign bits differ (so 0 counts as positive).
    let different = |a: i64, b: i64| (a < 0) != (b < 0);
    let dcrossa = dx * az - dz * ax;
    let dcrossb = dx * bz - dz * bx;
    if different(dcrossb, dcrossa) || dcrossb.abs() > dcrossa.abs() {
        return false;
    }
    let acrossb = ax * bz - az * bx;
    !(different(acrossb, dcrossa) || acrossb.abs() > dcrossa.abs())
}

/// The bounding box `calc_prim_info` (Prim.cpp:877) gives a prim, (lowest, highest): y over
/// all its points, but x and z only over those in its bottom eighth (all of them for the
/// step, prim 41), widened to at least `PRIM_MIN_BBOX`. The centre of gravity is always the
/// origin.
fn prim_bounds(prim: &Prim, id: u16) -> Option<([i32; 3], [i32; 3])> {
    let miny = prim.points.iter().map(|p| p[1] as i32).min()?;
    let maxy = prim.points.iter().map(|p| p[1] as i32).max()?;
    let below = miny + ((maxy - miny) >> 3);
    let bottom = |p: &&[i16; 3]| id == 41 || (p[1] as i32) < below;
    // A prim less than 8 high has no point below `below`, and the original's box is left
    // inside out at +-INFINITY; take all the points instead.
    let pts: Vec<&[i16; 3]> = if prim.points.iter().any(|p| bottom(&p)) {
        prim.points.iter().filter(bottom).collect()
    } else {
        prim.points.iter().collect()
    };
    let mut lo = [i32::MAX, miny, i32::MAX];
    let mut hi = [i32::MIN, maxy, i32::MIN];
    for p in pts {
        for a in [0, 2] {
            lo[a] = lo[a].min(p[a] as i32);
            hi[a] = hi[a].max(p[a] as i32);
        }
    }
    for a in [0, 2] {
        let dist = hi[a] - lo[a];
        if dist < PRIM_MIN_BBOX {
            hi[a] += (PRIM_MIN_BBOX - dist) >> 1;
            lo[a] -= (PRIM_MIN_BBOX - dist) >> 1;
        }
    }
    if id == 181 || id == 182 {
        // The estate gate, made bigger so you can't run through it.
        for a in [0, 2] {
            lo[a] -= 0x40;
            hi[a] += 0x40;
        }
    }
    Some((lo, hi))
}

/// `HM_Object`: one lump of hypermatter.
#[derive(Debug, Clone)]
pub struct Object {
    res: [usize; 3],
    /// `index`: the point at each lattice position, 0 for none.
    index: Vec<usize>,
    /// Point 0 is the original's null point and is never used.
    points: Vec<Point>,
    edges: Vec<Edge>,
    mesh: Vec<Mesh>,
    cols: Vec<Col>,
    bumps: Vec<Bump>,
    /// `cubex`, `cubey`, `cubez`: the lattice planes in prim space, where the points
    /// started before being turned and moved into place.
    cube: [[i32; MAX_RES]; 3],
    /// The squares of the springs' lengths (`size`), and their reciprocals (`oversize`).
    size: Vec<f32>,
    oversize: Vec<f32>,
    pub params: Params,
}

impl Object {
    /// `HM_create` (hm.cpp:474): a lattice round `prim` (number `prim_id`, for its bounding
    /// box), turned by `angles` (yaw, pitch, roll, 2048ths) about the prim's origin, put at
    /// `pos` and moving at `vel` a step. `None` if the lattice has no whole cube in it
    /// (as the original gives up), or the grid or prim makes no sense.
    pub fn new(prim: &Prim, prim_id: u16, pos: Vec3, angles: [i32; 3], vel: Vec3, grid: &Primgrid, params: Params) -> Option<Object> {
        let res = grid.res;
        if res.iter().any(|r| !(2..=MAX_RES).contains(r)) {
            return None;
        }
        let (lo, hi) = prim_bounds(prim, prim_id)?;
        let mut cube = [[0; MAX_RES]; 3];
        for a in 0..3 {
            for i in 0..res[a] {
                cube[a][i] = lo[a] + mul64(grid.points[a][i], hi[a] - lo[a]);
            }
            // A cube of no size would make every spring along it NaN.
            if (0..res[a] - 1).any(|i| cube[a][i + 1] <= cube[a][i]) {
                return None;
            }
        }
        let mut ho = Object {
            res,
            index: vec![0; res[0] * res[1] * res[2]],
            points: vec![Point { pos: Vec3::ZERO, vel: Vec3::ZERO, mass: 0.0 }],
            edges: Vec::new(),
            mesh: Vec::new(),
            cols: Vec::new(),
            bumps: Vec::new(),
            cube,
            size: Vec::new(),
            oversize: Vec::new(),
            params,
        };

        // The cubes with a point of the prim in them, and so the lattice points they use.
        let mut full = vec![false; (res[0] - 1) * (res[1] - 1) * (res[2] - 1)];
        let cube_no = |c: [usize; 3]| c[0] + c[1] * (res[0] - 1) + c[2] * (res[0] - 1) * (res[1] - 1);
        for p in &prim.points {
            full[cube_no(ho.cell(vec3(p[0] as f32, p[1] as f32, p[2] as f32)))] = true;
        }
        for x in 0..res[0] - 1 {
            for y in 0..res[1] - 1 {
                for z in 0..res[2] - 1 {
                    if full[cube_no([x, y, z])] {
                        for i in 0..8 {
                            let at = ho.idx(x + (i & 1), y + ((i >> 1) & 1), z + ((i >> 2) & 1));
                            ho.index[at] = usize::MAX;
                        }
                    }
                }
            }
        }

        // Number the points, x outermost as the original does, and weigh them.
        let mid = vec3(res[0] as f32, res[1] as f32, res[2] as f32) * 0.5;
        for x in 0..res[0] {
            for y in 0..res[1] {
                for z in 0..res[2] {
                    let at = ho.idx(x, y, z);
                    if ho.index[at] != 0 {
                        ho.index[at] = ho.points.len();
                        let pos = vec3(cube[0][x] as f32, cube[1][y] as f32, cube[2][z] as f32);
                        let off = vec3(x as f32, y as f32, z as f32) - mid;
                        let mass = 1.0 + off.x * grid.dgrav.x + off.y * grid.dgrav.y + off.z * grid.dgrav.z;
                        ho.points.push(Point { pos, vel, mass });
                    }
                }
            }
        }
        // So that every object weighs the same in all.
        let n = ho.points.len() - 1;
        let grav_av = ho.points[1..].iter().map(|p| p.mass).sum::<f32>() / n as f32;
        if grav_av >= 0.01 {
            let adjust = 1.0 / grav_av;
            for p in &mut ho.points[1..] {
                p.mass *= adjust;
            }
        }

        // The springs: to the next point along x, and to the three ahead in y and the nine
        // ahead in z, so each pair is joined once.
        for z in 0..res[2] {
            for y in 0..res[1] {
                for x in 0..res[0] {
                    let p1 = ho.index[ho.idx(x, y, z)];
                    if p1 == 0 {
                        continue;
                    }
                    for dz in 0..=1i32 {
                        for dy in -1..=1i32 {
                            for dx in -1..=1i32 {
                                if !(dz == 1 || dy == 1 || (dx == 1 && dy == 0)) {
                                    continue;
                                }
                                let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);
                                if !(0..res[0] as i32).contains(&nx) || !(0..res[1] as i32).contains(&ny) || !(0..res[2] as i32).contains(&nz) {
                                    continue;
                                }
                                let p2 = ho.index[ho.idx(nx as usize, ny as usize, nz as usize)];
                                if p2 == 0 {
                                    continue;
                                }
                                let size = (ho.points[p2].pos - ho.points[p1].pos).length_squared();
                                let len = match ho.size.iter().position(|&s| s == size) {
                                    Some(i) => i,
                                    None => {
                                        ho.size.push(size);
                                        ho.oversize.push(1.0 / size);
                                        ho.size.len() - 1
                                    }
                                };
                                ho.edges.push(Edge { p1, p2, len });
                            }
                        }
                    }
                }
            }
        }

        // Where each prim point is in its cube.
        ho.mesh = prim.points.iter().map(|p| ho.locate(vec3(p[0] as f32, p[1] as f32, p[2] as f32))).collect();

        // `HM_create` looks for the cube corner nearest the prim's origin with all three
        // edges out of it, for `HM_find_mesh_pos`, and gives up without one.
        let any_corner = (0..res[0] - 1).any(|x| (0..res[1] - 1).any(|y| (0..res[2] - 1).any(|z| {
            [ho.idx(x, y, z), ho.idx(x + 1, y, z), ho.idx(x, y + 1, z), ho.idx(x, y, z + 1)].iter().all(|&i| ho.index[i] != 0)
        })));
        if !any_corner {
            return None;
        }

        // Turn about the centre of gravity (the prim's origin) and put in place.
        let tau = std::f32::consts::TAU / 2048.0;
        let m = matrix_calc(angles[0] as f32 * tau, angles[1] as f32 * tau, angles[2] as f32 * tau);
        for p in &mut ho.points[1..] {
            p.pos = mul_by_transpose(&m, p.pos) + pos;
        }
        Some(ho)
    }

    /// `HM_index`.
    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.res[0] + z * self.res[0] * self.res[1]
    }

    /// The cube a prim-space point is in: along each axis, the first whose planes it's on or
    /// between. The original only looks inside the lattice and leaves a point outside it
    /// unplaced (hm.cpp:988, an `ASSERT`); here it goes in the nearest cube, so the parts of
    /// a prim outside its bounding box (which only spans its bottom eighth in x and z) follow
    /// along.
    fn cell(&self, p: Vec3) -> [usize; 3] {
        let mut c = [0; 3];
        for a in 0..3 {
            let v = p[a];
            let n = self.res[a] - 1;
            c[a] = (0..n)
                .find(|&i| self.cube[a][i] as f32 <= v && v <= self.cube[a][i + 1] as f32)
                .unwrap_or(if v < self.cube[a][0] as f32 { 0 } else { n - 1 });
        }
        c
    }

    /// `HM_cube_exists` (hm.cpp:1363): all eight corners of the cube are points.
    fn cube_exists(&self, c: [i32; 3]) -> bool {
        if (0..3).any(|a| !(0..self.res[a] as i32 - 1).contains(&c[a])) {
            return false;
        }
        (0..8).all(|i| self.index[self.idx(c[0] as usize + (i & 1), c[1] as usize + ((i >> 1) & 1), c[2] as usize + ((i >> 2) & 1))] != 0)
    }

    /// `HM_find_point_inside_cube` (hm.cpp:385), from where the lattice started: how far a
    /// prim point is along the three edges out of its cube's nearer corner (the far corner
    /// if the fractions add up to more than 1.5). A point outside the lattice gets fractions
    /// beyond 0 to 1 and goes where the cube's edges would carry it on to.
    fn locate(&self, p: Vec3) -> Mesh {
        let mut c = self.cell(p);
        let ci = |c: [usize; 3]| [c[0] as i32, c[1] as i32, c[2] as i32];
        if !self.cube_exists(ci(c)) {
            // Only an arbitrary point, never one of the prim's, can be in a cube that isn't
            // there: take the nearest that is.
            let mut best = None;
            for x in 0..self.res[0] - 1 {
                for y in 0..self.res[1] - 1 {
                    for z in 0..self.res[2] - 1 {
                        if self.cube_exists([x as i32, y as i32, z as i32]) {
                            let d = x.abs_diff(c[0]) + y.abs_diff(c[1]) + z.abs_diff(c[2]);
                            if best.is_none_or(|(bd, _)| d < bd) {
                                best = Some((d, [x, y, z]));
                            }
                        }
                    }
                }
            }
            if let Some((_, b)) = best {
                c = b;
            }
        }
        let [x, y, z] = c;
        let o = vec3(self.cube[0][x] as f32, self.cube[1][y] as f32, self.cube[2][z] as f32);
        let far = vec3(self.cube[0][x + 1] as f32, self.cube[1][y + 1] as f32, self.cube[2][z + 1] as f32);
        let along = (p - o) / (far - o);
        let at = |x, y, z| self.index[self.idx(x, y, z)];
        if along.x + along.y + along.z > 1.5 {
            // The other corner is nearer.
            Mesh {
                origin: at(x + 1, y + 1, z + 1),
                p: [at(x, y + 1, z + 1), at(x + 1, y, z + 1), at(x + 1, y + 1, z)],
                along: [1.0 - along.x, 1.0 - along.y, 1.0 - along.z],
            }
        } else {
            Mesh { origin: at(x, y, z), p: [at(x + 1, y, z), at(x, y + 1, z), at(x, y, z + 1)], along: along.to_array() }
        }
    }

    /// A mesh's point where the lattice is now.
    fn eval(&self, m: &Mesh) -> Vec3 {
        let o = self.points[m.origin].pos;
        o + (0..3).map(|i| m.along[i] * (self.points[m.p[i]].pos - o)).sum::<Vec3>()
    }

    /// `HM_find_mesh_point` (hm.cpp:1302): where point `i` of the prim the object was made
    /// round is now.
    pub fn mesh_point(&self, i: usize) -> Vec3 {
        self.eval(&self.mesh[i])
    }

    /// The prim's points, how many [`Object::mesh_point`] takes.
    pub fn mesh_len(&self) -> usize {
        self.mesh.len()
    }

    /// Where a prim-space point (unturned, at the prim's origin) is now, through the bent
    /// lattice, in the same way as [`Object::mesh_point`]: for drawing with
    /// `prim_batches(prim, |v| obj.place(v), ..)`.
    pub fn place(&self, p: Vec3) -> Vec3 {
        self.eval(&self.locate(p))
    }

    /// The lattice points.
    pub fn points(&self) -> &[Point] {
        &self.points[1..]
    }

    /// The springs' ends, to draw the lattice by (`HM_draw`, hm.cpp:2676, draws only the
    /// outside of it, each point half a step back).
    pub fn springs(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        self.edges.iter().map(|e| (self.points[e.p1].pos, self.points[e.p2].pos))
    }

    /// `HM_find_cog` (hm.cpp:1164): the centre of gravity.
    pub fn cog(&self) -> Vec3 {
        let pts = self.points();
        let tot: f32 = pts.iter().map(|p| p.mass).sum();
        pts.iter().map(|p| p.pos * p.mass).sum::<Vec3>() / tot
    }

    /// `HM_stationary` (hm.cpp:2997): the average speed along each axis, added up, is under
    /// a quarter of a unit a step. The average is over the null point too.
    pub fn stationary(&self) -> bool {
        let sum = self.points().iter().fold(Vec3::ZERO, |s, p| s + p.vel.abs());
        let av = sum / self.points.len() as f32;
        av.x + av.y + av.z < 0.25
    }

    /// `HM_shockwave` (hm.cpp:3033): pushes every point within `range` of `at` away from it,
    /// harder the nearer it is, and twice as hard upwards. A point right on `at` is left
    /// alone, where the original divides by nothing.
    pub fn shockwave(&mut self, at: Vec3, range: f32, force: f32) {
        for p in &mut self.points[1..] {
            let d = p.pos - at;
            let dist = d.length();
            if dist < range && dist > 0.0 {
                let push = ((range - dist) / range) * force / dist;
                p.vel += vec3(d.x * push, d.y * push * 2.0, d.z * push);
            }
        }
    }

    /// `HM_colvect_clear` (hm.cpp:1207).
    pub fn colvect_clear(&mut self) {
        self.cols.clear();
    }

    /// `HM_colvect_add` (hm.cpp:1214): a wall from `a` to `b` (x, z), infinitely tall, for
    /// the points to bounce off. The ends are whole units, as the original takes them, and a
    /// wall it already has, or one past the 64th, is left out. `FURN` (Furn.cpp:609) gave a
    /// moving piece of furniture every wall within four map squares whose bottom is on the
    /// ground there, each turn.
    pub fn colvect_add(&mut self, a: [f32; 2], b: [f32; 2]) {
        let c = Col { x1: a[0] as i32 as f32, z1: a[1] as i32 as f32, x2: b[0] as i32 as f32, z2: b[1] as i32 as f32 };
        if self.cols.len() >= COLS_PER_OBJECT || self.cols.iter().any(|h| h.x1 == c.x1 && h.z1 == c.z1 && h.x2 == c.x2 && h.z2 == c.z2) {
            return;
        }
        self.cols.push(c);
    }

    /// Whether any of its points are inside another object.
    pub fn bumping(&self) -> bool {
        !self.bumps.is_empty()
    }

    /// `HM_process` (hm.cpp:2362) for this object alone: one step. Bumps against other
    /// objects are dropped; [`Hypermatter::process`] keeps them.
    pub fn process(&mut self, world: &impl HmWorld) {
        self.step(world, &[]);
    }

    /// One step of `HM_process`, `others` being every object (this one's own slot is
    /// empty while it steps).
    fn step(&mut self, world: &impl HmWorld, others: &[Option<Object>]) {
        let Params { elasticity, bounciness, friction, damping } = self.params;

        // The springs: equal and opposite pulls, in proportion to how far the square of each
        // one's length is from the square of what it wants.
        for e in &self.edges {
            let d = self.points[e.p2].pos - self.points[e.p1].pos;
            let squaredist = d.x * d.x + d.y * d.y + d.z * d.z;
            if squaredist > 0.001 {
                let squash = (squaredist - self.size[e.len]) * elasticity * self.oversize[e.len];
                let f = d * squash;
                self.points[e.p1].vel += f;
                self.points[e.p2].vel -= f;
            }
        }

        // `HM_process_bump` (hm.cpp:1836): a point inside another object is pulled back to
        // where it went in, by the cube of how far in it is.
        for b in &self.bumps {
            let Some(Some(ho2)) = others.get(b.other) else { continue };
            let hp = &mut self.points[b.point];
            let d = ho2.rel_cube_to_world(b.cube, b.rel) - hp.pos;
            let squash = ho2.params.elasticity * d.length_squared() * 0.0003;
            hp.vel += squash * d;
        }

        // Move the points.
        for hp in &mut self.points[1..] {
            hp.vel *= damping;
            hp.vel.y += GRAVITY * hp.mass;

            // Under the ground: bounce, and reflect it back up. The original's ground is
            // always 0, and it reflects with `y = gy - y`, which is the same thing there.
            let gy = world.height(hp.pos.x, hp.pos.z);
            if hp.pos.y < gy {
                hp.vel.y = -hp.vel.y * bounciness;
                hp.vel.x *= friction;
                hp.vel.z *= friction;
                hp.pos.y = gy + (gy - hp.pos.y);
            }

            // Bounce off a wall it would cross within two steps, and stay put this one.
            let hit = self.cols.iter().find(|c| {
                seg_intersect(
                    c.x1 as i64, c.z1 as i64, c.x2 as i64, c.z2 as i64,
                    hp.pos.x as i64, hp.pos.z as i64,
                    (hp.pos.x + hp.vel.x + hp.vel.x) as i64, (hp.pos.z + hp.vel.z + hp.vel.z) as i64,
                )
            });
            if let Some(c) = hit {
                if c.x1 == c.x2 {
                    hp.vel.x = -hp.vel.x;
                } else if c.z1 == c.z2 {
                    hp.vel.z = -hp.vel.z;
                } else {
                    hp.vel.x = -hp.vel.x;
                    hp.vel.z = -hp.vel.z;
                }
                continue;
            }
            hp.pos += hp.vel;
        }

        // `HM_bump_dead` (hm.cpp:1940): forget points that are out again.
        let points = &self.points;
        self.bumps.retain(|b| {
            let Some(Some(ho2)) = others.get(b.other) else { return false };
            let p = &points[b.point];
            ho2.point_in_cube(p.pos, b.cube).is_some() || ho2.cubes().any(|c| c != b.cube && ho2.point_in_cube(p.pos, c).is_some())
        });
    }

    /// Every cube position, x outermost.
    fn cubes(&self) -> impl Iterator<Item = [usize; 3]> {
        let [rx, ry, rz] = self.res;
        (0..rx - 1).flat_map(move |x| (0..ry - 1).flat_map(move |y| (0..rz - 1).map(move |z| [x, y, z])))
    }

    /// The corner of a cube and its three edges out of it, `back` steps ago.
    fn cube_axes(&self, c: [usize; 3], back: f32) -> (Vec3, [Vec3; 3]) {
        let at = |x, y, z| {
            let p = &self.points[self.index[self.idx(x, y, z)]];
            p.pos - p.vel * back
        };
        let o = at(c[0], c[1], c[2]);
        (o, [at(c[0] + 1, c[1], c[2]) - o, at(c[0], c[1] + 1, c[2]) - o, at(c[0], c[1], c[2] + 1) - o])
    }

    /// `p` in a cube's own space, assuming it's still square: each edge divided by its
    /// length squared, so the cube runs 0 to 1 along each.
    fn cube_space(o: Vec3, axes: [Vec3; 3], p: Vec3) -> Vec3 {
        let r = p - o;
        vec3(axes[0].dot(r) / axes[0].length_squared(), axes[1].dot(r) / axes[1].length_squared(), axes[2].dot(r) / axes[2].length_squared())
    }

    /// `HM_is_point_in_cube` (hm.cpp:1428): where `p` is in the cube, if it's inside.
    fn point_in_cube(&self, p: Vec3, c: [usize; 3]) -> Option<Vec3> {
        if !self.cube_exists([c[0] as i32, c[1] as i32, c[2] as i32]) {
            return None;
        }
        let (o, axes) = self.cube_axes(c, 0.0);
        let r = Self::cube_space(o, axes, p);
        (0.0..=1.0).contains(&r.x).then_some(r)
            .filter(|r| (0.0..=1.0).contains(&r.y) && (0.0..=1.0).contains(&r.z))
    }

    /// `HM_rel_cube_to_world` (hm.cpp:1708): a point in a cube's space back in the world.
    fn rel_cube_to_world(&self, c: [usize; 3], rel: Vec3) -> Vec3 {
        let (o, axes) = self.cube_axes(c, 0.0);
        o + axes[0] * rel.x + axes[1] * rel.y + axes[2] * rel.z
    }

    /// `HM_collide` (hm.cpp:2011): notes each point of this object that has got into a cube
    /// of `ho2` (object number `other`) through one of its outside faces, to be pushed back
    /// out by the next steps.
    fn collide_with(&mut self, other: usize, ho2: &Object) {
        let mut already = vec![false; self.points.len()];
        for b in self.bumps.iter().filter(|b| b.other == other) {
            already[b.point] = true;
        }
        for i in 1..self.points.len() {
            if already[i] {
                continue;
            }
            let hp = self.points[i];
            for c in ho2.cubes() {
                let Some(rel) = ho2.point_in_cube(hp.pos, c) else { continue };

                // `HM_last_point_in_last_cube` (hm.cpp:1564): where it was in the cube a step
                // ago.
                let (o, axes) = ho2.cube_axes(c, 1.0);
                let last = Self::cube_space(o, axes, hp.pos - hp.vel);

                // How far along its move it crossed each face it came in by, that isn't
                // shared with another cube.
                let ci = [c[0] as i32, c[1] as i32, c[2] as i32];
                let mut along = [INFINITY; 3];
                for a in 0..3 {
                    let mut up = ci;
                    up[a] += 1;
                    let mut down = ci;
                    down[a] -= 1;
                    if last[a] > 1.0 && rel[a] <= 1.0 && !ho2.cube_exists(up) {
                        along[a] = (1.0 - last[a]) / (rel[a] - last[a]);
                    } else if last[a] < 0.0 && rel[a] >= 0.0 && !ho2.cube_exists(down) {
                        along[a] = (0.0 - last[a]) / (rel[a] - last[a]);
                    }
                }
                let along_enter = along.iter().fold(INFINITY, |m, &a| if a < m { a } else { m });
                if along_enter >= INFINITY {
                    // It didn't come in through an outside face (it was in already, or came
                    // from a neighbouring cube). The original carries on with `float(INFINITY)`
                    // and puts the entry point some two billion cube-widths off, where the
                    // pull back is enough to fling the object out of the world.
                    continue;
                }
                let enter = last + along_enter * (rel - last);
                // The original also keeps the eight points of `ho2` nearest the entry point,
                // to push back on, but only under `WE_WANT_TO_APPLY_THE_EQUAL_FORCE_TO_THE_CUBE`,
                // which is never defined.
                self.bumps.push(Bump { point: i, other, cube: c, rel: enter });
            }
        }
    }
}

/// `HM_object`: every object, up to [`MAX_OBJECTS`].
#[derive(Debug, Clone, Default)]
pub struct Hypermatter {
    pub objects: Vec<Option<Object>>,
}

impl Hypermatter {
    /// Puts a new object (from [`Object::new`]) in the first free slot, as `HM_create` does;
    /// `None` (`HM_NO_MORE_OBJECTS`) if all eight are taken.
    pub fn add(&mut self, obj: Object) -> Option<usize> {
        if let Some(i) = self.objects.iter().position(Option::is_none) {
            self.objects[i] = Some(obj);
            return Some(i);
        }
        if self.objects.len() >= MAX_OBJECTS {
            return None;
        }
        self.objects.push(Some(obj));
        Some(self.objects.len() - 1)
    }

    /// `HM_destroy` (hm.cpp:1136).
    pub fn destroy(&mut self, i: usize) {
        if let Some(o) = self.objects.get_mut(i) {
            *o = None;
        }
    }

    pub fn get(&self, i: usize) -> Option<&Object> {
        self.objects.get(i).and_then(Option::as_ref)
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut Object> {
        self.objects.get_mut(i).and_then(Option::as_mut)
    }

    /// `HM_process` (hm.cpp:2362): one step of every object, in turn, so a later one's
    /// bumps see where an earlier one has just moved to. Run [`STEPS_PER_TURN`] a turn.
    pub fn process(&mut self, world: &impl HmWorld) {
        for i in 0..self.objects.len() {
            let Some(mut ho) = self.objects[i].take() else { continue };
            ho.step(world, &self.objects);
            self.objects[i] = Some(ho);
        }
    }

    /// `HM_collide` (hm.cpp:2011): object `i`'s points against object `j`'s cubes.
    pub fn collide(&mut self, i: usize, j: usize) {
        if i == j || self.get(i).is_none() || self.get(j).is_none() {
            return;
        }
        let mut ho1 = self.objects[i].take().unwrap();
        ho1.collide_with(j, self.objects[j].as_ref().unwrap());
        self.objects[i] = Some(ho1);
    }

    /// `HM_collide_all` (hm.cpp:1678): every pair, both ways. (The original checks that the
    /// first of each pair is in use where it means the second.)
    pub fn collide_all(&mut self) {
        for i in 0..self.objects.len() {
            for j in i + 1..self.objects.len() {
                self.collide(i, j);
                self.collide(j, i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat ground at height 0.
    struct Flat;

    impl HmWorld for Flat {
        fn height(&self, _: f32, _: f32) -> f32 {
            0.0
        }
    }

    /// A box 128 on a side, its base on its origin.
    fn crate_prim() -> Prim {
        let mut points = Vec::new();
        for y in [0, 128] {
            for x in [-64, 64] {
                for z in [-64, 64] {
                    points.push([x, y, z]);
                }
            }
        }
        // And one in the middle of the top, to place by.
        points.push([0, 128, 0]);
        Prim { save_type: 0, name: String::new(), points, faces: Vec::new(), coltype: 0, damage: 0, shadowtype: 0, flag: 0 }
    }

    fn make(pos: Vec3, yaw: i32, vel: Vec3) -> Object {
        Object::new(&crate_prim(), 1, pos, [yaw, 0, 0], vel, &primgrid(1), DEBUG_KEY).unwrap()
    }

    fn finite(o: &Object) -> bool {
        o.points().iter().all(|p| p.pos.is_finite() && p.vel.is_finite())
    }

    #[test]
    fn the_default_grid_is_one_cube_round_the_prim() {
        let o = make(Vec3::ZERO, 0, Vec3::ZERO);
        assert_eq!(o.points().len(), 8);
        // Every pair of the 8 corners, once.
        assert_eq!(o.edges.len(), 28);
        // Lengths 128 along an edge, across a face and through the middle.
        assert_eq!(o.size.len(), 3);
        // The far z end is lighter, and the masses average 1.
        let m: f32 = o.points().iter().map(|p| p.mass).sum();
        assert!((m - 8.0).abs() < 1e-4);
        assert!(o.points().iter().filter(|p| p.pos.z > 0.0).all(|p| p.mass < 1.0));
    }

    #[test]
    fn mesh_points_start_where_the_prim_is_put() {
        let prim = crate_prim();
        let at = vec3(1000.0, 300.0, 2000.0);
        // A quarter turn: x goes to -z (`MESH_draw_poly`'s transpose of `MATRIX_calc`).
        let o = make(at, 512, Vec3::ZERO);
        for (i, p) in prim.points.iter().enumerate() {
            let (x, y, z) = (p[0] as f32, p[1] as f32, p[2] as f32);
            let want = at + vec3(z, y, -x);
            assert!((o.mesh_point(i) - want).length() < 0.01, "point {i}: {} vs {want}", o.mesh_point(i));
            assert!((o.place(vec3(x, y, z)) - want).length() < 0.01);
        }
        assert!((o.cog() - (at + vec3(0.0, 64.0, 0.0))).length() < 16.0);
    }

    #[test]
    fn a_dropped_box_lands_and_settles() {
        let mut o = make(vec3(5000.0, 512.0, 5000.0), 100, Vec3::ZERO);
        let mut landed = false;
        let mut still_at = None;
        for turn in 0..20 * 60 {
            for _ in 0..STEPS_PER_TURN {
                o.process(&Flat);
            }
            assert!(finite(&o), "turn {turn}");
            landed |= o.points().iter().any(|p| p.pos.y < 10.0);
            if landed && o.stationary() {
                still_at = Some(turn);
                break;
            }
        }
        assert!(landed);
        assert!(still_at.is_some(), "never came to rest");
        for p in o.points() {
            assert!((-1.0..200.0).contains(&p.pos.y), "point at {}", p.pos);
        }
        // Still roughly a box: its edges near their 128.
        for (a, b) in o.springs() {
            let d = (a - b).length();
            assert!((100.0..240.0).contains(&d), "spring {d}");
        }
        let c = o.cog();
        // With no friction it slides off a little, the light end leading.
        assert!(vec3(c.x - 5000.0, 0.0, c.z - 5000.0).length() < 500.0, "{c}");
    }

    #[test]
    fn a_shockwave_pushes_points_away() {
        let mut o = make(vec3(0.0, 100.0, 0.0), 0, Vec3::ZERO);
        let at = vec3(-200.0, 100.0, 0.0);
        o.shockwave(at, 400.0, 20.0);
        for p in o.points() {
            let d = p.pos - at;
            if d.length() < 400.0 {
                assert!(p.vel.dot(d) > 0.0);
            }
        }
        let before = o.cog();
        for _ in 0..STEPS_PER_TURN * 4 {
            o.process(&Flat);
        }
        assert!(o.cog().x > before.x + 20.0, "{} -> {}", before, o.cog());
    }

    #[test]
    fn a_box_does_not_go_through_a_colvect() {
        // Sliding along at 4 units a step towards a wall at x = 300.
        let mut o = make(vec3(0.0, 1.0, 0.0), 0, vec3(4.0, 0.0, 0.0));
        o.params.friction = 1.0;
        o.colvect_add([300.0, -1000.0], [300.0, 1000.0]);
        o.colvect_add([300.0, -1000.0], [300.0, 1000.0]);
        assert_eq!(o.cols.len(), 1, "the same wall twice is once");
        let mut reached = false;
        for _ in 0..20 * 10 {
            for _ in 0..STEPS_PER_TURN {
                o.process(&Flat);
                assert!(finite(&o));
                assert!(o.points().iter().all(|p| p.pos.x < 300.0));
                reached |= o.points().iter().any(|p| p.pos.x > 280.0);
            }
        }
        assert!(reached, "never got to the wall");
        // It bounced back.
        assert!(o.points().iter().map(|p| p.vel.x).sum::<f32>() < 0.0 || o.stationary());
    }

    #[test]
    fn colvects_cap_at_sixty_four() {
        let mut o = make(Vec3::ZERO, 0, Vec3::ZERO);
        for i in 0..100 {
            o.colvect_add([i as f32, 0.0], [i as f32, 10.0]);
        }
        assert_eq!(o.cols.len(), COLS_PER_OBJECT);
        o.colvect_clear();
        assert!(o.cols.is_empty());
    }

    #[test]
    fn only_eight_objects() {
        let mut hm = Hypermatter::default();
        for i in 0..MAX_OBJECTS {
            assert_eq!(hm.add(make(Vec3::ZERO, 0, Vec3::ZERO)), Some(i));
        }
        assert_eq!(hm.add(make(Vec3::ZERO, 0, Vec3::ZERO)), None);
        hm.destroy(3);
        assert_eq!(hm.add(make(Vec3::ZERO, 0, Vec3::ZERO)), Some(3));
    }

    /// One box dropped on another: its bottom gets into the lower one, is pushed back out,
    /// and the pair stays in one piece.
    #[test]
    fn dropped_on_another_it_bumps() {
        let mut hm = Hypermatter::default();
        let low = hm.add(make(vec3(0.0, 0.0, 0.0), 0, Vec3::ZERO)).unwrap();
        let high = hm.add(make(vec3(10.0, 160.0, 10.0), 0, vec3(0.0, -3.0, 0.0))).unwrap();
        let mut bumped = false;
        for _ in 0..20 * 3 {
            for _ in 0..STEPS_PER_TURN {
                hm.process(&Flat);
                hm.collide_all();
                bumped |= hm.get(high).unwrap().bumping();
            }
            assert!(finite(hm.get(low).unwrap()) && finite(hm.get(high).unwrap()));
        }
        assert!(bumped);
        let (l, h) = (hm.get(low).unwrap().cog(), hm.get(high).unwrap().cog());
        assert!(h.y > l.y, "the top box went through: {l} {h}");
    }

    #[test]
    fn segments() {
        assert!(seg_intersect(0, 0, 10, 0, 5, -5, 5, 5));
        assert!(!seg_intersect(0, 0, 10, 0, 5, 1, 5, 5));
        assert!(!seg_intersect(0, 0, 10, 0, 11, -5, 11, 5));
    }
}
