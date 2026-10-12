// Prints a prim's faces with their flags and heights (debugging aid).
fn main() {
    let id: u8 = std::env::args().nth(1).unwrap().parse().unwrap();
    let game = uc_formats::default_install().expect("install");
    let p = uc_world::level::load_prim(&game, id).unwrap();
    for f in &p.faces {
        let ys: Vec<i16> = f.points.iter().map(|&i| p.points[i as usize][1]).collect();
        println!("flags {:#06x} ys {ys:?}", f.face_flags);
    }
}
