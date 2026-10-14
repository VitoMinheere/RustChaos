// Lists ladders and cables within a box (debugging aid): ladders MISSION X0,Z0,X1,Z1
fn main() {
    let mut a = std::env::args().skip(1);
    let mission = a.next().unwrap();
    let b: Vec<f32> = a.next().unwrap().split(',').map(|s| s.parse().unwrap()).collect();
    let game = uc_formats::default_install().unwrap();
    let level = uc_world::Level::load(&game, &mission, &uc_world::LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    let inside = |x: f32, z: f32| x >= b[0] && x <= b[2] && z >= b[1] && z <= b[3];
    for l in level.ground.ladders.iter().filter(|l| inside(l.foot.x, l.foot.y)) {
        println!("ladder at ({:.0},{:.0}) {:.0}..{:.0} out {:?}", l.foot.x, l.foot.y, l.base, l.top, l.out);
    }
    for c in level.ground.cables.iter() {
        let (p, q) = (c.at(0.0), c.at(1.0));
        if inside(p.x, p.z) || inside(q.x, q.z) {
            println!("cable ({:.0},{:.0},{:.0}) -> ({:.0},{:.0},{:.0})", p.x, p.y, p.z, q.x, q.y, q.z);
        }
    }
}
