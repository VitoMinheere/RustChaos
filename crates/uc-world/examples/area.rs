// Lists props and roof faces in a rectangle of map squares (debugging aid).
// cargo run --release -p uc-world --example area -- MISSION X0 Z0 X1 Z1
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let game = uc_formats::default_install().expect("install");
    let level = uc_world::Level::load(&game, &a[0], &uc_world::LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let r: Vec<f32> = a[1..5].iter().map(|s| s.parse().unwrap()).collect();
    let inside = |x: f32, z: f32| x >= r[0] && x <= r[2] && z >= r[1] && z <= r[3];
    for ob in &level.map.objects {
        if inside(ob.x as f32 / 256.0, ob.z as f32 / 256.0) {
            let p = uc_world::level::load_prim(&game, ob.prim);
            println!("prim {:3} {:12} at ({:.1},{},{:.1}) yaw {} col {:?} flag {:?}", ob.prim, p.as_ref().map_or("?", |p| p.name.as_str()),
                     ob.x as f32 / 256.0, ob.y, ob.z as f32 / 256.0, ob.yaw, p.as_ref().map(|p| p.coltype), p.as_ref().map(|p| p.flag));
        }
    }
    for rf in level.map.roof_faces.iter().skip(1) {
        let (x, z) = ((rf.rx & 127) as f32, (rf.rz & 127) as f32);
        if inside(x, z) { println!("roof face ({x},{z}) y {} dy {:?} flags {:#x}", rf.y, rf.dy, rf.draw_flags); }
    }
}
