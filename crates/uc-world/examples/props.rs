// Lists the prop prims a mission's map uses: collision type, bounding box (debugging aid).
fn main() {
    let m: String = std::env::args().nth(1).unwrap();
    let game = uc_formats::default_install().expect("install");
    let level = uc_world::Level::load(&game, &m, &uc_world::LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let mut seen = std::collections::BTreeMap::new();
    for ob in &level.map.objects { *seen.entry(ob.prim).or_insert(0) += 1; }
    for (id, n) in seen {
        let Some(p) = uc_world::level::load_prim(&game, id) else { println!("{id}: missing"); continue };
        let (mut lo, mut hi) = ([i16::MAX; 3], [i16::MIN; 3]);
        for pt in &p.points { for k in 0..3 { lo[k] = lo[k].min(pt[k]); hi[k] = hi[k].max(pt[k]); } }
        println!("{id:3} x{n:3} {:20} col {} shadow {} flag {:#x} box {:?}..{:?}", p.name, p.coltype, p.shadowtype, p.flag, lo, hi);
    }
}
