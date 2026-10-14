// Prints walkable heights at a few points (debugging aid): probe MISSION X,Z [X,Z...]
fn main() {
    let mut a = std::env::args().skip(1);
    let mission = a.next().unwrap();
    let game = uc_formats::default_install().unwrap();
    let level = uc_world::Level::load(&game, &mission, &uc_world::LoadOptions { fullbright: true, character: false, overrides: None }).unwrap();
    for xz in a {
        let v: Vec<f32> = xz.split(',').map(|s| s.parse().unwrap()).collect();
        let hs: Vec<String> = [2200.0, 2300.0, 2400.0, 2500.0, 2600.0, 2700.0]
            .iter().map(|&h| format!("{h}->{}", level.ground.surface(&level.map, v[0], v[1], h))).collect();
        println!("{xz}: {}", hs.join("  "));
    }
}
