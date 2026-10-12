// Counts facet kinds in a mission's map, and lists cables and fences (debugging aid).
fn main() {
    let m: String = std::env::args().nth(1).unwrap();
    let game = uc_formats::default_install().expect("install");
    let level = uc_world::Level::load(&game, &m, &uc_world::LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let mut counts = std::collections::BTreeMap::new();
    for f in level.map.facets.iter().skip(1) { *counts.entry(f.kind).or_insert(0) += 1; }
    println!("{counts:?}");
    for f in level.map.facets.iter().skip(1).filter(|f| matches!(f.kind, 9 | 10 | 11 | 13)) {
        println!("kind {} ({},{})->({},{}) y {:?} h {} fh {} block {}", f.kind, f.x[0], f.z[0], f.x[1], f.z[1], f.y, f.height, f.fheight, f.block_height);
    }
}
