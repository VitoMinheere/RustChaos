// Lists the body-part meshes of a character file (debugging aid).
use uc_formats::all::AnimChunk;
fn main() {
    let f: String = std::env::args().nth(1).unwrap_or("data/darci1.all".into());
    let g = uc_formats::default_install().expect("install");
    let c = AnimChunk::parse(&std::fs::read(uc_formats::resolve(&g, &f).unwrap()).unwrap()).unwrap();
    for part in 0..c.element_count {
        let m = c.part_mesh(0, 0, part).unwrap();
        let (mut lo, mut hi) = ([i16::MAX; 3], [i16::MIN; 3]);
        for p in &m.points { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } }
        let off = c.pose(c.anims[3].unwrap()).unwrap()[part].offset;
        println!("{part:2} {:12} pts {:3} box {:?}..{:?} stand offset {:?}", m.name, m.points.len(), lo, hi, off);
    }
}
