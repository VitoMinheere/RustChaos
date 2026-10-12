// Prints the bounding boxes of the given prims (debugging aid).
fn main() {
    let game = uc_formats::default_install().expect("install");
    for id in std::env::args().skip(1).map(|a| a.parse::<u8>().unwrap()) {
        let Some(p) = uc_world::level::load_prim(&game, id) else { println!("{id}: missing"); continue };
        let (mut lo, mut hi) = ([i16::MAX; 3], [i16::MIN; 3]);
        for pt in &p.points { for k in 0..3 { lo[k] = lo[k].min(pt[k]); hi[k] = hi[k].max(pt[k]); } }
        println!("{id:3} {:16} pts {:4} faces {:4} box {lo:?}..{hi:?}", p.name, p.points.len(), p.faces.len());
    }
}
