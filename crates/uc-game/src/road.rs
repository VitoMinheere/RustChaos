//! The roads cars wander along (fallen/Source/road.cpp): a graph of nodes down the middle of
//! each road, found from the road textures as the level loads (`ROAD_wander_calc`).
//!
//! Roads run along the map's axes only. Each straight stretch of road five squares wide
//! becomes an edge between two nodes, and where two stretches cross both are split at a
//! shared node. A node has at most four neighbours.

use glam::{vec2, Vec2};
use uc_formats::iam::{flags, Map, MAP_SIZE};

/// Where in its lane a car aims, either side of the middle of the road (`ROAD_LANE`).
const LANE: f32 = 256.0;
const MAX_NODES: usize = 256;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Node {
    pub x: u8,
    pub z: u8,
    /// Neighbours, packed at the front; 0 is none (node 0 is never used).
    pub c: [u8; 4],
}

#[derive(Debug, Clone, Default)]
pub struct Roads {
    /// Node 0 is a placeholder, as in the original.
    pub nodes: Vec<Node>,
}

/// `ROAD_is_road`: a square textured as road (pages 323 to 356, or a few more in the two
/// texture sets with dirt roads), and not hidden under a building.
pub fn is_road(map: &Map, x: i32, z: i32) -> bool {
    if !(0..MAP_SIZE as i32).contains(&x) || !(0..MAP_SIZE as i32).contains(&z) {
        return false;
    }
    let sq = map.square(x as usize, z as usize);
    if sq.has(flags::HIDDEN) {
        return false;
    }
    let page = sq.page();
    (323..=356).contains(&page) || (matches!(map.texture_set, Some(7 | 8)) && matches!(page, 35 | 36 | 39))
}

/// `ROAD_is_middle`: road for two squares in every direction.
fn is_middle(map: &Map, x: i32, z: i32) -> bool {
    if !(2..MAP_SIZE as i32 - 2).contains(&x) || !(2..MAP_SIZE as i32 - 2).contains(&z) {
        return false;
    }
    (-2..=2).all(|dx| (-2..=2).all(|dz| is_road(map, x + dx, z + dz)))
}

fn centre(n: &Node) -> Vec2 {
    vec2(n.x as f32 * 256.0 + 128.0, n.z as f32 * 256.0 + 128.0)
}

impl Roads {
    /// `ROAD_wander_calc`. `map_file` is the mission's map, for the one road the original
    /// leaves out.
    pub fn new(map: &Map, map_file: &str) -> Roads {
        let mut r = Roads { nodes: vec![Node::default()] };
        let skip = map_file.to_ascii_lowercase().contains("gpost3.iam");
        let size = MAP_SIZE as i32;
        // Roads along z, then along x: runs of middle squares more than two long.
        for along_z in [true, false] {
            for a in 2..size - 2 {
                let mut run: Option<(i32, i32)> = None;
                for b in 1..size - 1 {
                    let (x, z) = if along_z { (a, b) } else { (b, a) };
                    if is_middle(map, x, z) {
                        run = Some(run.map_or((b, b), |(p1, _)| (p1, b)));
                        continue;
                    }
                    if let Some((p1, p2)) = run.take() {
                        if (p2 - p1).abs() > 2 {
                            if along_z {
                                r.add(a, p1, a, p2, skip);
                            } else {
                                r.add(p1, a, p2, a, skip);
                            }
                        }
                    }
                }
            }
        }
        // Dead ends two squares from the edge lead off the map: move them onto the edge.
        let last = (MAP_SIZE - 1) as u8;
        for n in r.nodes.iter_mut().skip(1) {
            if n.c[0] != 0 && n.c[1] == 0 {
                if n.x == 2 {
                    n.x = 0;
                }
                if n.z == 2 {
                    n.z = 0;
                }
                if n.x == last - 2 {
                    n.x = last;
                }
                if n.z == last - 2 {
                    n.z = last;
                }
            }
        }
        r
    }

    fn find_node(&mut self, x: i32, z: i32) -> usize {
        if let Some(i) = self.nodes.iter().skip(1).position(|n| n.x as i32 == x && n.z as i32 == z) {
            return i + 1;
        }
        self.nodes.push(Node { x: x as u8, z: z as u8, c: [0; 4] });
        self.nodes.len() - 1
    }

    fn connect(&mut self, a: usize, b: usize) {
        for (from, to) in [(a, b), (b, a)] {
            let c = &mut self.nodes[from].c;
            if !c.contains(&(to as u8)) {
                if let Some(slot) = c.iter_mut().find(|s| **s == 0) {
                    *slot = to as u8;
                }
            }
        }
    }

    fn disconnect(&mut self, a: usize, b: usize) {
        for (from, to) in [(a, b), (b, a)] {
            let c = &mut self.nodes[from].c;
            if let Some(i) = c.iter().position(|&s| s as usize == to) {
                c.copy_within(i + 1.., i);
                c[3] = 0;
            }
        }
    }

    /// `ROAD_intersect`: a road crossing the one from (x1, z1) to (x2, z2), and where.
    fn intersect(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> Option<(usize, usize, i32, i32)> {
        let ends = |n: &Node| (n.x as i32 == x1 && n.z as i32 == z1) || (n.x as i32 == x2 && n.z as i32 == z2);
        for (i, rn) in self.nodes.iter().enumerate().skip(1) {
            if ends(rn) {
                continue;
            }
            for &j in rn.c.iter().take_while(|&&j| j != 0) {
                let j = j as usize;
                if j >= i {
                    continue;
                }
                let rm = &self.nodes[j];
                if ends(rm) {
                    continue;
                }
                let within = |v: i32, a: u8, b: u8| (a.min(b) as i32..=a.max(b) as i32).contains(&v);
                if x1 == x2 {
                    if rn.x != rm.x && within(x1, rn.x, rm.x) && (z1.min(z2)..=z1.max(z2)).contains(&(rn.z as i32)) {
                        return Some((i, j, x1, rn.z as i32));
                    }
                } else if rn.z != rm.z && within(z1, rn.z, rm.z) && (x1.min(x2)..=x1.max(x2)).contains(&(rn.x as i32)) {
                    return Some((i, j, rn.x as i32, z1));
                }
            }
        }
        None
    }

    /// `ROAD_add`: a road, split wherever it crosses another.
    fn add(&mut self, x1: i32, z1: i32, x2: i32, z2: i32, skip: bool) {
        if self.nodes.len() >= MAX_NODES - 4 || (x1 == x2 && z1 == z2) {
            return;
        }
        // The original leaves this one out on Cop Killers and Arms Breaker's map.
        if skip && ((x1 == 121 && z1 == 33) || (x2 == 121 && z2 == 33)) {
            return;
        }
        let n1 = self.find_node(x1, z1);
        let n2 = self.find_node(x2, z2);
        match self.intersect(x1, z1, x2, z2) {
            None => self.connect(n1, n2),
            Some((i1, i2, ix, iz)) => {
                let sn = self.find_node(ix, iz);
                if sn != i1 && sn != i2 {
                    self.disconnect(i1, i2);
                    self.connect(i1, sn);
                    self.connect(i2, sn);
                }
                self.add(x1, z1, ix, iz, skip);
                self.add(x2, z2, ix, iz, skip);
            }
        }
    }

    pub fn pos(&self, n: usize) -> Vec2 {
        centre(&self.nodes[n])
    }

    /// `ROAD_signed_dist`: how far `p` is to the right of the middle of road n1 -> n2 (a car
    /// driving on the right is positive), or very far if it's off the road's length.
    pub fn signed_dist(&self, n1: usize, n2: usize, p: Vec2) -> f32 {
        let (a, b) = (&self.nodes[n1], &self.nodes[n2]);
        let (mx, mz) = ((p.x / 256.0).floor() as i32, (p.y / 256.0).floor() as i32);
        if a.x == b.x {
            if (a.z.min(b.z) as i32 - 2..=a.z.max(b.z) as i32 + 2).contains(&mz) {
                let d = p.x - centre(a).x;
                return if a.z < b.z { -d } else { d };
            }
        } else if (a.x.min(b.x) as i32 - 2..=a.x.max(b.x) as i32 + 2).contains(&mx) {
            let d = p.y - centre(a).y;
            return if a.x > b.x { -d } else { d };
        }
        65536.0
    }

    /// `ROAD_find`: the road whose lane `p` is nearest, as (from, to).
    pub fn find(&self, p: Vec2) -> Option<(usize, usize)> {
        let mut best = None;
        let mut best_dist = f32::INFINITY;
        for (i, rn) in self.nodes.iter().enumerate().skip(1) {
            for &j in rn.c.iter().take_while(|&&j| j != 0) {
                let j = j as usize;
                if j < i {
                    let d = (LANE - self.signed_dist(i, j, p)).abs();
                    if d < best_dist {
                        best_dist = d;
                        best = Some((i, j));
                    }
                }
            }
        }
        best
    }

    /// `ROAD_whereto_now`: at the end of road n1 -> n2, which way next. Anything but back the
    /// way it came, picked at random, unless it's a dead end.
    pub fn whereto_now(&self, n1: usize, n2: usize, rand: &mut impl FnMut() -> u32) -> usize {
        let mut best = 0;
        let mut best_score = 0;
        for &c in self.nodes[n2].c.iter().filter(|&&c| c != 0) {
            let score = if c as usize == n1 { 10 } else { (rand() & 0xff) + 20 };
            if score > best_score {
                best_score = score;
                best = c as usize;
            }
        }
        best
    }

    /// `ROAD_bend`: which way the turn n1 -> n2 -> n3 goes (negative is left).
    pub fn bend(&self, n1: usize, n2: usize, n3: usize) -> i32 {
        let (a, b, c) = (&self.nodes[n1], &self.nodes[n2], &self.nodes[n3]);
        let (x12, z12) = (b.x as i32 - a.x as i32, b.z as i32 - a.z as i32);
        let (x23, z23) = (c.x as i32 - b.x as i32, c.z as i32 - b.z as i32);
        x12 * z23 - z12 * x23
    }

    /// `ROAD_get_dest`: where a car going down n1 -> n2 heads for, in its lane at the far end
    /// (and well past it where the road leads off the map).
    pub fn dest(&self, n1: usize, n2: usize) -> Vec2 {
        let (a, b) = (&self.nodes[n1], &self.nodes[n2]);
        let d = vec2((b.x as i32 - a.x as i32).signum() as f32, (b.z as i32 - a.z as i32).signum() as f32) * LANE;
        let mut p = centre(b) + vec2(-d.y, d.x);
        let last = (MAP_SIZE - 1) as u8;
        if b.x == 0 {
            p.x -= 1024.0;
        }
        if b.z == 0 {
            p.y -= 1024.0;
        }
        if b.x == last {
            p.x += 1024.0;
        }
        if b.z == last {
            p.y += 1024.0;
        }
        p
    }
}
