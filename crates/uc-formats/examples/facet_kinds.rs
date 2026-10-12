// How many facets of each kind each map has (debugging aid).
fn main() {
    let game = uc_formats::default_install().expect("install");
    for m in std::env::args().skip(1) {
        let rel = "Data\\".to_string() + &m + ".iam";
        let Some(p) = uc_formats::resolve(&game, &rel) else { println!("{m}: missing"); continue };
        let map = uc_formats::iam::Map::parse(&std::fs::read(p).unwrap()).unwrap();
        let mut n = std::collections::BTreeMap::new();
        for f in map.facets.iter().skip(1) { *n.entry(f.kind).or_insert(0usize) += 1; }
        println!("{m:12} {n:?}");
    }
}
