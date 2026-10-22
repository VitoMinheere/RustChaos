//! `.iam` maps: `load_game_map` (fallen/Source/io.cpp:617), `load_super_map` and
//! `load_walkables` (fallen/Source/supermap.cpp:2256, :1745). See docs/ARCHITECTURE.md §5.

use crate::prm::Prim;
use crate::reader::{count, Reader, Result};

/// Hi-res map squares per side (`PAP_SIZE_HI`).
pub const MAP_SIZE: usize = 128;
/// Lo-res cells per side (`PAP_SIZE_LO`, `OB_SIZE`).
pub const LO_SIZE: usize = 32;
/// World units per hi-res square (`PAP_SHIFT_HI` = 8).
pub const SQUARE: i32 = 256;
/// World height of a corner = `Alt << ALT_SHIFT` (`PAP_ALT_SHIFT`).
pub const ALT_SHIFT: i32 = 3;
/// How far `PAP_FLAG_SINK_POINT` corners drop for kerbs (`KERB_HEIGHT`, DDEngine/Headers/aeng.h).
pub const KERB_HEIGHT: i32 = 32;

/// `PAP_Hi.Flags` bits (fallen/Headers/pap.h).
pub mod flags {
    pub const SHADOW_1: u16 = 1 << 0;
    pub const REFLECTIVE: u16 = 1 << 3;
    pub const HIDDEN: u16 = 1 << 4;
    pub const SINK_SQUARE: u16 = 1 << 5;
    pub const SINK_POINT: u16 = 1 << 6;
    pub const NOUPPER: u16 = 1 << 7;
    pub const NOGO: u16 = 1 << 8;
    pub const ROOF_EXISTS: u16 = 1 << 9;
    pub const WATER: u16 = 1 << 15;
}

/// `STOREY_TYPE_*` (fallen/Headers/building.h:169).
pub mod facet_type {
    pub const NORMAL: u8 = 1;
    pub const ROOF: u8 = 2;
    pub const WALL: u8 = 3;
    pub const ROOF_QUAD: u8 = 4;
    pub const FIRE_ESCAPE: u8 = 6;
    pub const STAIRCASE: u8 = 7;
    pub const CABLE: u8 = 9;
    pub const FENCE: u8 = 10;
    pub const FENCE_BRICK: u8 = 11;
    pub const LADDER: u8 = 12;
    pub const FENCE_FLAT: u8 = 13;
    pub const TRENCH: u8 = 14;
    pub const DOOR: u8 = 18;
    pub const INSIDE_DOOR: u8 = 19;
    pub const OUTSIDE_DOOR: u8 = 21;
}

/// `FACET_FLAG_*` for `DFacet.FacetFlags` (fallen/Headers/building.h:102).
pub mod facet_flags {
    pub const INVISIBLE: u16 = 1 << 0;
    pub const INSIDE: u16 = 1 << 3;
    pub const HUG_FLOOR: u16 = 1 << 5;
    pub const TWO_SIDED: u16 = 1 << 7;
    pub const SEETHROUGH: u16 = 1 << 11;
    pub const TWO_TEXTURED: u16 = 1 << 14;
}

/// `BUILDING_TYPE_*`.
pub const BUILDING_TYPE_WAREHOUSE: u8 = 1;

/// `PAP_Hi`: one hi-res map square (6 bytes).
#[derive(Debug, Clone, Copy, Default)]
pub struct Square {
    /// Bits 0-9 texture page, 10-11 rotation (12-15 unused on PC).
    pub texture: u16,
    pub flags: u16,
    pub alt: i8,
    pub height: i8,
}

impl Square {
    pub fn page(&self) -> u16 {
        self.texture & 0x3FF
    }
    pub fn rotation(&self) -> u8 {
        ((self.texture >> 10) & 3) as u8
    }
    pub fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }
}

/// `DBuilding` (24 bytes).
#[derive(Debug, Clone, Copy)]
pub struct Building {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub start_facet: u16,
    pub end_facet: u16,
    pub walkable: u16,
    pub ware: u8,
    pub kind: u8,
}

/// `DFacet` (26 bytes): a vertical wall strip between two map-grid points.
#[derive(Debug, Clone, Copy)]
pub struct Facet {
    pub kind: u8,
    /// In quarter storeys: one row of wall per 4.
    pub height: u8,
    pub x: [u8; 2],
    pub y: [i16; 2],
    pub z: [u8; 2],
    pub flags: u16,
    pub style_index: u16,
    pub building: u16,
    pub storey: u16,
    pub fheight: u8,
    /// Row height in units of 16 world units (16 = one 256-unit storey).
    pub block_height: u8,
    pub open: u8,
}

/// `DStorey` (6 bytes): a "painted" storey, overriding pages per wall segment.
#[derive(Debug, Clone, Copy)]
pub struct Storey {
    pub style: u16,
    pub index: u16,
    pub count: i8,
}

/// `RoofFace4` (10 bytes).
#[derive(Debug, Clone, Copy)]
pub struct RoofFace4 {
    pub y: i16,
    pub dy: [i8; 3],
    pub draw_flags: u8,
    pub rx: u8,
    pub rz: u8,
    pub next: i16,
}

/// A static object (`OB_Ob` resolved through `OB_mapwho`) with world coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub prim: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// 0..2047 per full turn.
    pub yaw: i32,
    pub flags: u8,
    pub inside_index: u8,
}

/// `LoadGameThing` (44 bytes): animated prims placed on the map.
#[derive(Debug, Clone, Copy)]
pub struct Thing {
    pub kind: u16,
    pub subtype: u16,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub flags: u32,
    pub index_other: u16,
    pub angle: [u16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkablesLayout {
    /// The layout the shipped code reads.
    Current,
    /// The older layout (commented out in `load_walkables`), used by 3 unshipped retail maps
    /// (`jumper1`, `submiss1a`, `supply1`). These also keep a facet's x and z as 16-bit world
    /// coordinates rather than bytes of map squares, and their roofs and walkable tops as prim
    /// faces ([`Map::walkables`]) rather than [`RoofFace4`]s.
    Old,
}

#[derive(Debug, Clone)]
pub struct Map {
    pub save_type: i32,
    /// x-major: `squares[x * MAP_SIZE + z]`.
    pub squares: Vec<Square>,
    pub things: Vec<Thing>,
    /// Record 0 is an unused dummy.
    pub buildings: Vec<Building>,
    /// Record 0 is an unused dummy.
    pub facets: Vec<Facet>,
    /// Per facet row: >= 0 is a style in `style.tma`, < 0 is `-index` into `storeys`.
    pub dstyles: Vec<i16>,
    pub paint_mem: Vec<u8>,
    pub storeys: Vec<Storey>,
    pub roof_faces: Vec<RoofFace4>,
    pub objects: Vec<Object>,
    pub texture_set: Option<i32>,
    pub walkables_layout: WalkablesLayout,
    /// In the older layout, the roofs and walkable tops: points in world units and the faces
    /// between them (`PrimPoint`, `PrimFace3`, `PrimFace4`), as a prim placed at the origin.
    pub walkables: Option<Prim>,
}

impl Map {
    pub fn square(&self, x: usize, z: usize) -> &Square {
        &self.squares[x * MAP_SIZE + z]
    }

    /// Height of the upper surface at grid corner (x, z), clamped to the map.
    pub fn corner_alt(&self, x: usize, z: usize) -> i32 {
        let x = x.min(MAP_SIZE - 1);
        let z = z.min(MAP_SIZE - 1);
        (self.square(x, z).alt as i32) << ALT_SHIFT
    }

    /// Height of the drawn ground at grid corner (x, z) for square (sx, sz): squares flagged
    /// `SINK_SQUARE` (roads) use the "lower" corners, which drop `KERB_HEIGHT` at `SINK_POINT`s.
    fn ground_corner(&self, sx: usize, sz: usize, x: usize, z: usize) -> i32 {
        let up = self.corner_alt(x, z);
        if self.square(sx, sz).has(flags::SINK_SQUARE)
            && self.square(x.min(MAP_SIZE - 1), z.min(MAP_SIZE - 1)).has(flags::SINK_POINT)
        {
            up - KERB_HEIGHT
        } else {
            up
        }
    }

    /// Ground height at world (x, z), split into two triangles per square like
    /// `PAP_calc_height_at` (fallen/Source/pap.cpp). Rooftops are not included.
    pub fn ground_height(&self, x: f32, z: f32) -> f32 {
        let (mx, mz) = ((x / SQUARE as f32).floor(), (z / SQUARE as f32).floor());
        if mx < 0.0 || mz < 0.0 || mx >= (MAP_SIZE - 1) as f32 || mz >= (MAP_SIZE - 1) as f32 {
            return 0.0;
        }
        let (sx, sz) = (mx as usize, mz as usize);
        let (fx, fz) = (x / SQUARE as f32 - mx, z / SQUARE as f32 - mz);
        let h = |cx: usize, cz: usize| self.ground_corner(sx, sz, cx, cz) as f32;
        let (h0, h1, h2, h3) = (h(sx, sz), h(sx + 1, sz), h(sx, sz + 1), h(sx + 1, sz + 1));
        if fx + fz < 1.0 {
            h0 + (h1 - h0) * fx + (h2 - h0) * fz
        } else {
            h3 + (h2 - h3) * (1.0 - fx) + (h1 - h3) * (1.0 - fz)
        }
    }

    /// Parse a map, trying the shipped walkables layout first and then the older one.
    /// The two can't be told apart by `save_type` (docs/ARCHITECTURE.md §5).
    pub fn parse(data: &[u8]) -> Result<Map> {
        match parse_with(data, WalkablesLayout::Current) {
            Ok(m) => Ok(m),
            Err(first) => parse_with(data, WalkablesLayout::Old).map_err(|_| first),
        }
    }
}

fn objects(r: &mut Reader) -> Result<Vec<Object>> {
    let n = r.i32()?;
    let n = count(r, n)?;
    let mut obs = Vec::with_capacity(n);
    for _ in 0..n {
        let y = r.i16()?;
        let (x, z, prim, yaw, flags, inside) = (r.u8()?, r.u8()?, r.u8()?, r.u8()?, r.u8()?, r.u8()?);
        obs.push((y, x, z, prim, yaw, flags, inside));
    }
    let mut out = Vec::new();
    for lx in 0..LO_SIZE {
        for lz in 0..LO_SIZE {
            let v = r.u16()?;
            let (index, num) = ((v & 0x7FF) as usize, (v >> 11) as usize);
            for k in 0..num {
                let Some(&(y, ox, oz, prim, yaw, flags, inside)) = obs.get(index + k) else {
                    return r.error(format!("object mapwho points past the object list ({index}+{k})"));
                };
                out.push(Object {
                    prim,
                    x: ((lx as i32) << 10) + ((ox as i32) << 2),
                    y: y as i32,
                    z: ((lz as i32) << 10) + ((oz as i32) << 2),
                    yaw: (yaw as i32) << 3,
                    flags,
                    inside_index: inside,
                });
            }
        }
    }
    Ok(out)
}

fn parse_with(data: &[u8], layout: WalkablesLayout) -> Result<Map> {
    let mut r = Reader::new(data);
    let save_type = r.i32()?;
    if save_type > 23 {
        r.i32()?; // ob_size: read but unused by the loader
    }
    let mut squares = Vec::with_capacity(MAP_SIZE * MAP_SIZE);
    for _ in 0..MAP_SIZE * MAP_SIZE {
        squares.push(Square { texture: r.u16()?, flags: r.u16()?, alt: r.i8()?, height: r.i8()? });
    }
    if save_type == 18 {
        return r.error("save_type 18 (MapThingPSX records) is not supported");
    }
    let mut things = Vec::new();
    if save_type > 18 {
        for _ in 0..r.u16()? {
            let t = Thing {
                kind: r.u16()?,
                subtype: r.u16()?,
                x: r.i32()?,
                y: r.i32()?,
                z: r.i32()?,
                flags: r.u32()?,
                index_other: r.u16()?,
                angle: [r.u16()?, r.u16()?, r.u16()?],
            };
            r.skip(16)?;
            things.push(t);
        }
    }
    let mut obs = Vec::new();
    if save_type < 23 {
        obs = objects(&mut r)?;
    }

    // Super map.
    let (nb, nf, ns) = (r.u16()? as usize, r.u16()? as usize, r.u16()? as usize);
    let (mut npaint, mut nstorey) = (0, 0);
    if save_type >= 17 {
        npaint = r.u16()? as usize;
        nstorey = r.u16()? as usize;
    }
    let mut buildings = Vec::with_capacity(nb);
    for _ in 0..nb {
        let (x, y, z) = (r.i32()?, r.i32()?, r.i32()?);
        let (start_facet, end_facet, walkable) = (r.u16()?, r.u16()?, r.u16()?);
        r.skip(4)?; // Counter[2], Padding
        buildings.push(Building { x, y, z, start_facet, end_facet, walkable, ware: r.u8()?, kind: r.u8()? });
    }
    let mut facets = Vec::with_capacity(nf);
    for _ in 0..nf {
        let kind = r.u8()?;
        let height = r.u8()?;
        // The older layout keeps x and z as world units, a square being 256 of them, and has
        // three bytes less after `Open` (the record is the same size).
        let (x, y, z) = if layout == WalkablesLayout::Old {
            let x = [r.i16()?, r.i16()?];
            let y = [r.i16()?, r.i16()?];
            let z = [r.i16()?, r.i16()?];
            (x.map(|v| (v >> 8) as u8), y, z.map(|v| (v >> 8) as u8))
        } else {
            ([r.u8()?, r.u8()?], [r.i16()?, r.i16()?], [r.u8()?, r.u8()?])
        };
        let (flags, style_index, building, storey) = (r.u16()?, r.u16()?, r.u16()?, r.u16()?);
        let (fheight, block_height, open) = (r.u8()?, r.u8()?, r.u8()?);
        r.skip(if layout == WalkablesLayout::Old { 1 } else { 5 })?; // Dfcache, Shake, CutHole, Counter[2]
        facets.push(Facet { kind, height, x, y, z, flags, style_index, building, storey, fheight, block_height, open });
    }
    let mut dstyles = Vec::with_capacity(ns);
    for _ in 0..ns {
        dstyles.push(r.i16()?);
    }
    let mut paint_mem = Vec::new();
    let mut storeys = Vec::new();
    if save_type >= 17 {
        paint_mem = r.take(npaint)?.to_vec();
        for _ in 0..nstorey {
            let (style, index, count) = (r.u16()?, r.u16()?, r.i8()?);
            r.skip(1)?;
            storeys.push(Storey { style, index, count });
        }
    }
    if save_type >= 21 {
        let (nis, nst) = (r.u16()? as usize, r.u16()? as usize);
        let nblk = r.i32()?;
        let nblk = count(&mut r, nblk)?;
        r.skip(nis * 22 + nst * 10 + nblk)?; // InsideStorey, Staircase, inside_block
    }
    let mut roof_faces = Vec::new();
    let mut walkables = None;
    match layout {
        WalkablesLayout::Current => {
            let (nw, nr) = (r.u16()? as usize, r.u16()? as usize);
            r.skip(nw * 22)?; // DWalkable
            for _ in 0..nr {
                roof_faces.push(RoofFace4 {
                    y: r.i16()?,
                    dy: [r.i8()?, r.i8()?, r.i8()?],
                    draw_flags: r.u8()?,
                    rx: r.u8()?,
                    rz: r.u8()?,
                    next: r.i16()?,
                });
            }
        }
        WalkablesLayout::Old => {
            // `DWalkable`s, then the prim points and faces they index (supermap.cpp:1758,
            // commented out there).
            let (nw, npt, nf3, nf4) = (r.u16()? as usize, r.u16()? as usize, r.u16()? as usize, r.u16()? as usize);
            r.skip(nw * 22)?;
            let mut points = Vec::with_capacity(npt);
            for _ in 0..npt {
                points.push([r.i16()?, r.i16()?, r.i16()?]);
            }
            let mut faces = Vec::with_capacity(nf3 + nf4);
            for _ in 0..nf3 {
                faces.push(crate::prm::face(&mut r, 3, 0)?);
            }
            for _ in 0..nf4 {
                faces.push(crate::prm::face(&mut r, 4, 0)?);
            }
            faces.retain(|f| f.points.iter().all(|&p| (p as usize) < points.len()));
            walkables = Some(Prim { save_type: 0, name: "walkables".into(), points, faces, coltype: 0, damage: 0, shadowtype: 0, flag: 0 });
        }
    }
    if save_type >= 23 {
        obs = objects(&mut r)?;
    }
    let texture_set = if save_type >= 20 { Some(r.i32()?) } else { None };
    if save_type >= 25 {
        r.skip(2 * 200 * 5)?; // psx_textures_xy
    }
    r.expect_end()?;
    Ok(Map {
        save_type,
        squares,
        things,
        buildings,
        facets,
        dstyles,
        paint_mem,
        storeys,
        roof_faces,
        objects: obs,
        texture_set,
        walkables_layout: layout,
        walkables,
    })
}
