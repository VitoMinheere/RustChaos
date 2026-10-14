//! Loading a mission's level from an install and turning it into geometry.

use crate::geometry::{Builder, Layer, WorldGeometry, UNIT, UNTEXTURED};
use glam::{vec3, Vec3};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::character::{ep_angle, People};
use crate::ground::Ground;
use std::sync::Arc;
use uc_formats::all::AnimChunk;
use uc_formats::facet;
use uc_formats::iam::{facet_type, flags, Map, KERB_HEIGHT, MAP_SIZE};
use uc_formats::lgt::{Lighter, Lighting};
use uc_formats::prm::Prim;
use uc_formats::tma::StyleTable;
use uc_formats::txc::{Clump, Texture};
use uc_formats::ucm::{waypoint, Mission};
use uc_formats::resolve;

/// `ROOF_SHIFT` (Editor/Headers/Prim.h): RoofFace4.DY is in units of 8.
const ROOF_SHIFT: i32 = 3;

/// Floor quad corner k sits at (x + (k & 1), z + (k >> 1)); rotation picks the UV set
/// (`TEXTURE_get_minitexturebits_uvs`, DDEngine/Source/texture.cpp:2292).
pub const FLOOR_UVS: [[[f32; 2]; 4]; 4] = [
    [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
    [[1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 1.0]],
    [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]],
];

pub struct LoadOptions {
    /// Ignore the level's baked lighting.
    pub fullbright: bool,
    /// Load people's models and animations.
    pub character: bool,
    /// Folder of `page_NNNN.png` files that replace clump textures.
    pub overrides: Option<PathBuf>,
}

pub struct Level {
    pub game: PathBuf,
    pub name: String,
    pub mission: Mission,
    pub map: Map,
    pub clump: Clump,
    pub styles: Option<StyleTable>,
    pub lighting: Option<Lighting>,
    /// Surfaces, walls and ladders people move on.
    pub ground: Ground,
    /// Darci's start: feet position in world units and angle in radians (facing (-sin, -cos)).
    pub spawn: (Vec3, f32),
    pub sky: [u8; 3],
    pub people: Option<People>,
    /// Prims for items and barrels that missions place (`ITEM_PRIMS`).
    pub item_prims: HashMap<u8, Prim>,
    /// Weapons as held (256 + `PersonID >> 5` - 1: pistol, knife, shotgun, bat, AK-47) and
    /// muzzle flashes (261 pistol, 262 shotgun, 263 AK-47), modelled in the hand's space.
    pub weapon_prims: HashMap<u16, Prim>,
    /// The mission's moving platforms: (the `link_platform` point, the prop it moves, as an
    /// index into the map's objects). The game draws those props, not the level.
    pub platforms: Vec<(usize, usize)>,
    /// Props that are barrels, which the game turns into barrels it moves and blows up
    /// (`OB_convert_dustbins_to_barrels`, fallen/Source/ob.cpp:1627): as indices into the
    /// map's objects. They're left out of the scenery and of what people walk into.
    pub barrel_props: Vec<usize>,
    overrides: Option<PathBuf>,
}

fn read(game: &Path, rel: &str) -> Result<Vec<u8>, String> {
    let p = resolve(game, rel).ok_or(format!("{rel} not found in {}", game.display()))?;
    std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))
}

/// Whether texture page `p` belongs to the map's world (`TEXTURE_load_needed`,
/// DDEngine/Source/texture.cpp:770): pages 0-255 come from `world<n>\`, 512-575 from
/// `world<n>\insides\`. The rest are shared by every world.
fn world_page(p: u32) -> bool {
    p < 64 * 4 || (64 * 8..64 * 9).contains(&p)
}

/// The world whose textures stand in for a world no shipped mission uses (`freeoncd1.iam`
/// and `training1.iam` are world 2): the city of RTA and the other downtown missions.
const STAND_IN_WORLD: i32 = 3;

/// Textures for a mission the install has no clump for: the shipped missions only have
/// clumps of their own, packed from the loose textures (`server\textures\`) that the
/// install no longer has. The pages come from the clumps of other missions: first those on
/// the same map, then those in the same world; the shared pages from any. A world no
/// clump covers borrows `STAND_IN_WORLD`'s pages, which fit its style table only roughly.
fn borrowed_clump(game: &Path, map_file: &str, texture_set: Option<i32>) -> Clump {
    // (rank, mission): 0 same map, 1 same world, 2 the stand-in world, 3 any other.
    let mut donors: Vec<(u8, String)> = Vec::new();
    let mut sets: HashMap<String, Option<i32>> = HashMap::new();
    let entries = resolve(game, "levels").and_then(|d| std::fs::read_dir(d).ok());
    for e in entries.into_iter().flatten().flatten() {
        let file = e.file_name().to_string_lossy().into_owned();
        let Some(name) = file.strip_suffix(".ucm").or_else(|| file.strip_suffix(".UCM")) else { continue };
        if resolve(game, &format!("clumps\\{name}.txc")).is_none() {
            continue;
        }
        let Some(m) = std::fs::read(e.path()).ok().and_then(|b| Mission::parse(&b).ok()) else { continue };
        let rank = if m.map.eq_ignore_ascii_case(map_file) {
            0
        } else {
            let set = sets.entry(m.map.to_ascii_lowercase()).or_insert_with(|| {
                read(game, &m.map).ok().and_then(|b| Map::parse(&b).ok()).and_then(|map| map.texture_set)
            });
            if *set == texture_set { 1 } else if *set == Some(STAND_IN_WORLD) { 2 } else { 3 }
        };
        donors.push((rank, name.to_string()));
    }
    donors.sort();
    let own_world = donors.iter().any(|&(rank, _)| rank < 2);
    let mut clump = Clump::empty();
    for (rank, name) in donors {
        if let Some(c) = read(game, &format!("clumps\\{name}.txc")).ok().and_then(|b| Clump::parse(b).ok()) {
            clump.fill_from(&c, |p| rank < 2 || (rank == 2 && !own_world) || !world_page(p));
        }
    }
    clump
}

impl Level {
    pub fn load(game: &Path, mission_name: &str, opt: &LoadOptions) -> Result<Level, String> {
        let mission = Mission::parse(&read(game, &format!("levels\\{mission_name}.ucm"))?)
            .map_err(|e| format!("{mission_name}.ucm: {e}"))?;
        let map = Map::parse(&read(game, &mission.map)?).map_err(|e| format!("{}: {e}", mission.map))?;
        // Textures are looked up by mission name, not by map (TEXTURE_initialise_clumping).
        let clump = match read(game, &format!("clumps\\{mission_name}.txc")) {
            Ok(b) => Clump::parse(b).map_err(|e| e.to_string())?,
            Err(_) => borrowed_clump(game, &mission.map, map.texture_set),
        };
        let styles = map
            .texture_set
            .and_then(|w| read(game, &format!("server\\textures\\world{w}\\style.tma")).ok())
            .and_then(|b| StyleTable::parse(&b).ok());
        let lighting = if opt.fullbright { None } else { read(game, &mission.lighting).ok().and_then(|b| Lighting::parse(&b).ok()) };
        let sky = lighting.as_ref().map_or([20, 22, 30], |l| l.sky);

        // The mission's player spawn.
        let platforms = platform_props(&map, &mission);
        let barrel_props: Vec<usize> = map.objects.iter().enumerate().filter(|(_, o)| o.prim == BARREL_PROP).map(|(i, _)| i).collect();
        let moving = platforms.iter().map(|&(_, ob)| ob).chain(barrel_props.iter().copied()).collect();
        let ground = Ground::new(&map, game, &moving);
        let ep = mission.event_points.iter().find(|e| e.kind == waypoint::CREATE_PLAYER);
        let (sx, sy, sz) = ep.map_or((64.0 * UNIT, 0.0, 64.0 * UNIT), |e| (e.x as f32, e.y as f32, e.z as f32));
        let angle = ep.map_or(0.0, |e| ep_angle(e.direction));
        let spawn = (vec3(sx, ground.surface(&map, sx, sz, sy), sz), angle);
        let people = if opt.character {
            let chunk = |name: &str| -> Result<Arc<AnimChunk>, String> {
                let c = AnimChunk::parse(&read(game, &format!("data\\{name}"))?).map_err(|e| format!("{name}: {e}"))?;
                Ok(Arc::new(c))
            };
            // In the combat tutorial the gang meshes are swapped for trainers (setup_people_anims,
            // fallen/Source/Anim.cpp).
            let trainer = if mission_name.eq_ignore_ascii_case("ftutor1") { chunk("trainer.all").ok() } else { None };
            // Roper has a body of his own (`game_chunk[ANIM_TYPE_ROPER]`, Anim.cpp); without it
            // he falls back to a thug's.
            let roper = chunk("roper.all").ok();
            // Everyone animates from Darci's set, with Roper's carrying (and being carried)
            // filled in from his file into the slots from `ANIM_END` up (Anim.cpp:1398).
            let mut darci = AnimChunk::parse(&read(game, "data\\darci1.all")?).map_err(|e| format!("darci1.all: {e}"))?;
            if let Some(r) = &roper {
                for &(from, to) in &crate::character::CARRY_ANIMS {
                    darci.graft(r, from, to);
                }
            }
            Some(People { darci: Arc::new(darci), thug: chunk("rthug.all")?, trainer, roper })
        } else {
            None
        };
        Ok(Level {
            game: game.to_path_buf(), name: mission_name.to_string(), mission, map, clump, styles, lighting,
            ground, spawn, sky, people, overrides: opt.overrides.clone(), platforms, barrel_props,
            item_prims: ITEM_PRIMS.iter().map(|&(_, id)| id).chain(EXTRA_PRIMS)
                .filter_map(|id| Some((id, load_prim(game, id)?))).collect(),
            weapon_prims: (256..=263).filter_map(|id| Some((id, load_prim(game, id)?))).collect(),
        })
    }

    pub fn lighter(&self) -> Option<Lighter<'_>> {
        self.lighting.as_ref().map(|l| Lighter::new(l, &self.map.objects))
    }

    /// The texture for a page: an override PNG if present, else the clump's.
    pub fn texture(&self, page: u32) -> Option<(Texture, bool)> {
        if let Some(dir) = &self.overrides {
            let p = dir.join(format!("page_{page:04}.png"));
            if let Some(t) = load_png(&p) {
                return Some((t, true));
            }
        }
        self.clump.texture(page).ok().flatten().map(|t| (t, false))
    }

    /// Builds all static geometry, lit with the level's baked lighting.
    pub fn build(&self) -> WorldGeometry {
        let lighter = self.lighter();
        let sh = Shader { lighter: lighter.as_ref() };
        let ids: std::collections::HashSet<u32> = self.clump.ids().collect();
        let mut b = Builder::default();
        build_floor(&mut b, &self.map, &sh, |p| !ids.contains(&p));
        build_roof_faces(&mut b, &self.map, &sh);
        if let Some(styles) = &self.styles {
            for i in 1..self.map.facets.len() {
                for q in facet::wall_quads(&self.map, styles, i) {
                    let p = q.corners.map(|c| vec3(c[0], c[1], c[2]) / UNIT);
                    b.batch(p[0], Layer::Walls, q.page).quad(p, q.uv, sh.quad(&p));
                }
            }
        }
        let moving: std::collections::BTreeSet<usize> = self.platforms.iter().map(|&(_, ob)| ob).chain(self.barrel_props.iter().copied()).collect();
        build_props(&mut b, &self.map, &self.game, &sh, &moving);
        build_details(&mut b, &self.map, &sh);
        b.finish()
    }
}

fn load_png(path: &Path) -> Option<Texture> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px.chunks(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Texture { width: info.width, height: info.height, has_alpha: rgba.chunks(4).any(|c| c[3] < 255), rgba })
}

/// Vertex lighting: the level's baked lights if available, else full bright.
pub struct Shader<'a> {
    pub lighter: Option<&'a Lighter<'a>>,
}

impl Shader<'_> {
    /// `p` in square units, `n` any length.
    pub fn at(&self, p: Vec3, n: Vec3) -> [u8; 4] {
        match self.lighter {
            Some(l) => {
                let [r, g, b] = l.light((p * UNIT).to_array(), n.to_array());
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]
            }
            None => [255; 4],
        }
    }
    pub fn quad(&self, p: &[Vec3; 4]) -> [[u8; 4]; 4] {
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        p.map(|v| self.at(v, n))
    }
}

fn scale(c: [[u8; 4]; 4], k: f32) -> [[u8; 4]; 4] {
    c.map(|v| [(v[0] as f32 * k) as u8, (v[1] as f32 * k) as u8, (v[2] as f32 * k) as u8, v[3]])
}

fn build_floor(b: &mut Builder, map: &Map, sh: &Shader, skip_unseen: impl Fn(u32) -> bool) {
    let upper = |x: usize, z: usize| map.corner_alt(x, z) as f32;
    let lower = |x: usize, z: usize| {
        let sink = map.square(x.min(MAP_SIZE - 1), z.min(MAP_SIZE - 1)).has(flags::SINK_POINT);
        upper(x, z) - if sink { KERB_HEIGHT as f32 } else { 0.0 }
    };
    for x in 0..MAP_SIZE - 1 {
        for z in 0..MAP_SIZE - 1 {
            let sq = map.square(x, z);
            let page = sq.page() as u32;
            let uv = FLOOR_UVS[sq.rotation() as usize];
            let corners = [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)];

            // Rooftop: AENG_draw_city's "fake roof", flat at PAP_Hi.Height << 6.
            if sq.has(flags::ROOF_EXISTS) {
                let y = (sq.height as f32 * 64.0) / UNIT;
                let p = corners.map(|(cx, cz)| vec3(cx as f32, y, cz as f32));
                b.batch(p[0], Layer::Roofs, page).quad(p, uv, p.map(|v| sh.at(v, Vec3::Y)));
                continue;
            }
            // Ground under closed buildings is never visible and has no texture in the clump.
            if sq.has(flags::HIDDEN) && skip_unseen(page) {
                continue;
            }
            let sunk = sq.has(flags::SINK_SQUARE);
            let h = |cx: usize, cz: usize| if sunk { lower(cx, cz) } else { upper(cx, cz) };
            let p = corners.map(|(cx, cz)| vec3(cx as f32, h(cx, cz) / UNIT, cz as f32));
            let mut c = sh.quad(&p);
            if sq.has(flags::WATER) {
                c = c.map(|v| [(v[0] as f32 * 0.6) as u8, (v[1] as f32 * 0.7) as u8, v[2], 255]);
            }
            b.batch(p[0], Layer::Floor, page).quad(p, uv, c);

            // Kerb: vertical strip where a sunk square meets a raised neighbour.
            if sunk {
                let neighbours = [((x + 1, z), (x + 1, z), (x + 1, z + 1)),
                                  ((x.wrapping_sub(1), z), (x, z), (x, z + 1)),
                                  ((x, z + 1), (x, z + 1), (x + 1, z + 1)),
                                  ((x, z.wrapping_sub(1)), (x, z), (x + 1, z))];
                for ((nx, nz), a, bb) in neighbours {
                    if nx >= MAP_SIZE || nz >= MAP_SIZE || map.square(nx, nz).has(flags::SINK_SQUARE) {
                        continue;
                    }
                    let (ua, ub, la, lb) = (upper(a.0, a.1), upper(bb.0, bb.1), lower(a.0, a.1), lower(bb.0, bb.1));
                    if ua - la < 1.0 && ub - lb < 1.0 {
                        continue;
                    }
                    let q = [vec3(a.0 as f32, ua / UNIT, a.1 as f32), vec3(bb.0 as f32, ub / UNIT, bb.1 as f32),
                             vec3(a.0 as f32, la / UNIT, a.1 as f32), vec3(bb.0 as f32, lb / UNIT, bb.1 as f32)];
                    let kerb = map.square(nx, nz).page() as u32;
                    b.batch(q[0], Layer::Floor, kerb).quad(q, [[0.0, 0.0], [1.0, 0.0], [0.0, 0.125], [1.0, 0.125]],
                                                          scale(sh.quad(&q), 0.8));
                }
            }
        }
    }
}

/// Walkable roof faces (`FACET_draw_walkable`, facet.cpp:4116). Faces over hidden squares
/// without a rooftop belong to warehouses; the PC engine draws those black from below.
fn build_roof_faces(b: &mut Builder, map: &Map, sh: &Shader) {
    for rf in map.roof_faces.iter().skip(1) {
        let (x, z) = ((rf.rx & 127) as usize, (rf.rz & 127) as usize);
        let sq = map.square(x, z);
        if sq.has(flags::ROOF_EXISTS) {
            continue;
        }
        let y0 = rf.y as i32;
        let ys = [y0, y0 + ((rf.dy[0] as i32) << ROOF_SHIFT), y0 + ((rf.dy[2] as i32) << ROOF_SHIFT),
                  y0 + ((rf.dy[1] as i32) << ROOF_SHIFT)];
        let (xf, zf) = (x as f32, z as f32);
        let p = [vec3(xf, ys[0] as f32 / UNIT, zf), vec3(xf + 1.0, ys[1] as f32 / UNIT, zf),
                 vec3(xf, ys[2] as f32 / UNIT, zf + 1.0), vec3(xf + 1.0, ys[3] as f32 / UNIT, zf + 1.0)];
        if sq.has(flags::HIDDEN) {
            b.batch(p[0], Layer::Roofs, UNTEXTURED).quad(p, [[0.0; 2]; 4], [[10, 10, 12, 255]; 4]);
        } else {
            b.batch(p[0], Layer::Roofs, sq.page() as u32).quad(p, FLOOR_UVS[sq.rotation() as usize], sh.quad(&p));
        }
    }
}

/// Item types (`IT_*`, fallen/Headers/Mission.h) and their prims (`PRIM_OBJ_ITEM_*`,
/// Editor/Headers/Prim.h): the weapons and health, then the things a mission has Darci fetch
/// -- a key, a keycard, files, a floppy, a crowbar, a gas mask, a wrench, a video, gloves and
/// weedkiller. The Fallen's five clues are a keycard, three files and a video.
/// Badges (`create_treasure`) aren't `IT_*` items; they're kinds 100-103 here: stamina,
/// strength, reflexes and constitution, one of whose four models the original picks at random.
pub const ITEM_PRIMS: [(u8, u8); 25] = [
    (1, 36), (2, 143), (3, 142), (4, 127), (5, 166), (6, 120), (8, 205), (9, 141), (10, 255), (11, 253),
    (12, 254), (13, 144), (14, 232), (15, 233), (16, 234), (17, 235), (18, 236), (19, 237), (20, 238),
    (21, 239), (23, 167),
    (100, 81), (101, 94), (102, 71), (103, 39),
];

/// Other prims missions place: traffic cone, barrels, and vehicle bodies and wheels.
/// The prop every map's barrels are (prim 33, `PRIM_OBJ_BIN`, which ob.cpp calls "the
/// barrel prim").
pub const BARREL_PROP: u8 = 33;

pub const EXTRA_PRIMS: [u8; 15] = [33, 67, 145, 88, 87, 108, 220, 155, 150, 159, 90, 91, 131, 27, CAN_PRIM];

/// `TEXTURE_page_leaf`, `_rubbish` and `_snowflake` (`TEXTURE_NUM_STANDARD` + 35, 64 and
/// 74): `leaf.tga`, `rubbish.tga` and `snowflake.tga`, for the dirt.
pub const LEAF_PAGE: u32 = 22 * 64 + 35;
pub const RUBBISH_PAGE: u32 = 22 * 64 + 64;
pub const SNOWFLAKE_PAGE: u32 = 22 * 64 + 74;

/// `PRIM_OBJ_CAN`: the cans lying in the street.
pub const CAN_PRIM: u8 = 76;

/// The prim for item type `kind`.
pub fn item_prim(kind: u8) -> Option<u8> {
    ITEM_PRIMS.iter().find(|&&(k, _)| k == kind).map(|&(_, p)| p)
}

/// A prim's bounding box (world units): (lowest, highest corner).
pub fn prim_bounds(prim: &Prim) -> (Vec3, Vec3) {
    prim.points.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
        let p = vec3(p[0] as f32, p[1] as f32, p[2] as f32);
        (lo.min(p), hi.max(p))
    })
}

/// Adds a prim's triangles to `out` (square units), each point placed by `place` (prim space
/// to world units), lit by `lighter`.
pub fn prim_batches(prim: &Prim, place: impl Fn(Vec3) -> Vec3, lighter: Option<&Lighter>,
                    out: &mut std::collections::BTreeMap<u32, crate::geometry::Batch>) {
    let sh = Shader { lighter };
    for f in &prim.faces {
        let (page, uv) = f.texture();
        let pts: Vec<Vec3> = f.points.iter()
            .map(|&i| { let p = prim.points[i as usize]; place(vec3(p[0] as f32, p[1] as f32, p[2] as f32)) / UNIT })
            .collect();
        let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
        let col: Vec<[u8; 4]> = pts.iter().map(|&p| sh.at(p, n)).collect();
        let batch = out.entry(page).or_default();
        if pts.len() == 4 {
            batch.quad([pts[0], pts[1], pts[2], pts[3]], [uv[0], uv[1], uv[2], uv[3]], [col[0], col[1], col[2], col[3]]);
        } else {
            batch.tri([pts[0], pts[1], pts[2]], [uv[0], uv[1], uv[2]], [col[0], col[1], col[2]]);
        }
    }
}

/// Loads prop prim `id` (`nprimNNN.prm`, else the older `primNNN.prm`).
pub fn load_prim(game: &Path, id: impl Into<u16>) -> Option<Prim> {
    let id = id.into();
    let path = resolve(game, &format!("server\\prims\\nprim{id:03}.prm"))
        .or_else(|| resolve(game, &format!("server\\prims\\prim{id:03}.prm")))?;
    Prim::parse(&std::fs::read(path).ok()?).ok()
}

/// The props a mission's `link_platform` points take (`PLAT_create`): for each, the one
/// nearest it in the map cells within three squares, counting height at a quarter. Each is
/// taken off the map as it's taken (`OB_remove`), so no two points share one.
fn platform_props(map: &Map, mission: &Mission) -> Vec<(usize, usize)> {
    let mut taken = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for e in mission.event_points.iter().filter(|e| e.used != 0 && e.kind == waypoint::LINK_PLATFORM) {
        let cell = |v: i32| (v.clamp(0, 0x7fff)) >> 10;
        let (x1, z1, x2, z2) = (cell(e.x - 0x300), cell(e.z - 0x300), cell(e.x + 0x300), cell(e.z + 0x300));
        let best = map.objects.iter().enumerate()
            .filter(|(i, o)| !taken.contains(i) && o.prim != 0)
            .filter(|(_, o)| (x1..=x2).contains(&(o.x >> 10)) && (z1..=z2).contains(&(o.z >> 10)))
            .min_by_key(|(_, o)| (o.x - e.x).abs() + (o.z - e.z).abs() + ((o.y - e.y).abs() >> 2))
            .map(|(i, _)| i);
        if let Some(ob) = best {
            taken.insert(ob);
            out.push((e.index, ob));
        }
    }
    out
}

fn build_props(b: &mut Builder, map: &Map, game: &Path, sh: &Shader, moving: &std::collections::BTreeSet<usize>) {
    let mut cache: HashMap<u8, Option<Prim>> = HashMap::new();
    for ob in map.objects.iter().enumerate().filter(|(i, _)| !moving.contains(i)).map(|(_, o)| o) {
        let Some(prim) = cache.entry(ob.prim).or_insert_with(|| load_prim(game, ob.prim)) else { continue };
        let a = ob.yaw as f32 * std::f32::consts::TAU / 2048.0;
        let (s, c) = a.sin_cos();
        let origin = vec3(ob.x as f32, ob.y as f32, ob.z as f32);
        // MESH_draw_poly: world = transpose(MATRIX_calc(yaw)) * local.
        let place = |p: [i16; 3]| {
            let (x, y, z) = (p[0] as f32, p[1] as f32, p[2] as f32);
            (origin + vec3(c * x + s * z, y, -s * x + c * z)) / UNIT
        };
        for f in &prim.faces {
            let (page, uv) = f.texture();
            let pts: Vec<Vec3> = f.points.iter().map(|&i| place(prim.points[i as usize])).collect();
            let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
            let col: Vec<[u8; 4]> = pts.iter().map(|&p| sh.at(p, n)).collect();
            let batch = b.batch(origin / UNIT, Layer::Props, page);
            if pts.len() == 4 {
                batch.quad([pts[0], pts[1], pts[2], pts[3]], [uv[0], uv[1], uv[2], uv[3]], [col[0], col[1], col[2], col[3]]);
            } else {
                batch.tri([pts[0], pts[1], pts[2]], [uv[0], uv[1], uv[2]], [col[0], col[1], col[2]]);
            }
        }
    }
}

/// A box between two points (square units) with the given half-width, as four side quads.
fn beam(b: &mut Builder, from: Vec3, to: Vec3, half: f32, color: [u8; 4]) {
    let dir = (to - from).normalize_or_zero();
    let side = if dir.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = dir.cross(side).normalize_or_zero() * half;
    let v = dir.cross(u).normalize_or_zero() * half;
    let corners = [u + v, u - v, -u - v, -u + v];
    let batch = b.batch(from, Layer::Detail, UNTEXTURED);
    for i in 0..4 {
        let (o1, o2) = (corners[i], corners[(i + 1) % 4]);
        let shade = [0.75, 1.0, 0.6, 0.85][i];
        let c = [(color[0] as f32 * shade) as u8, (color[1] as f32 * shade) as u8, (color[2] as f32 * shade) as u8, 255];
        batch.quad([to + o1, to + o2, from + o1, from + o2], [[0.0; 2]; 4], [c; 4]);
    }
}

/// Cables (`cable_draw`, facet.cpp:950) and ladders (`DRAW_ladder`, facet.cpp:5201).
fn build_details(b: &mut Builder, map: &Map, sh: &Shader) {
    for f in map.facets.iter().skip(1) {
        let (x1, z1, x2, z2) = (f.x[0] as f32 * 256.0, f.z[0] as f32 * 256.0, f.x[1] as f32 * 256.0, f.z[1] as f32 * 256.0);
        match f.kind {
            facet_type::CABLE => {
                let pts: Vec<Vec3> = crate::ground::cable_points(f).into_iter().map(|p| p / UNIT).collect();
                let count = pts.len() - 1;
                let col = sh.at(pts[count / 2], Vec3::Y);
                let col = [col[0] / 4, col[1] / 4, col[2] / 4, 255];
                for w in pts.windows(2) {
                    beam(b, w[0], w[1], 2.0 / UNIT, col);
                }
            }
            facet_type::LADDER => {
                // The ladder fills the middle third of the facet, set out from the wall by 1/8 of its length.
                let (dx, dz) = (x2 - x1, z2 - z1);
                let (ax, az) = (x1 + dx / 3.0 + dz / 8.0, z1 + dz / 3.0 - dx / 8.0);
                let (bx, bz) = (x2 - dx / 3.0 + dz / 8.0, z2 - dz / 3.0 - dx / 8.0);
                let y0 = f.y[0] as f32;
                let top = y0 + f.height as f32 * 64.0;
                let col = sh.at(vec3(ax, y0, az) / UNIT, Vec3::Y);
                let col = [(col[0] as f32 * 0.55) as u8, (col[1] as f32 * 0.55) as u8, (col[2] as f32 * 0.6) as u8, 255];
                for (x, z) in [(ax, az), (bx, bz)] {
                    beam(b, vec3(x, y0, z) / UNIT, vec3(x, top, z) / UNIT, 3.0 / UNIT, col);
                }
                for k in 1..=f.height as i32 {
                    let y = y0 + k as f32 * 64.0 - 8.0;
                    beam(b, vec3(ax, y, az) / UNIT, vec3(bx, y, bz) / UNIT, 2.0 / UNIT, col);
                }
            }
            _ => {}
        }
    }
}
