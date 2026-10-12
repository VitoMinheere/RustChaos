// Prints how far each animation moves the body root (part 0 offset), first to last frame.
// cargo run --release -p uc-formats --example root_motion -- 332 55 170
use uc_formats::all::AnimChunk;
fn main() {
    let g = uc_formats::default_install().expect("install");
    let chunk = AnimChunk::parse(&std::fs::read(uc_formats::resolve(&g, r"data\darci1.all").unwrap()).unwrap()).unwrap();
    for id in std::env::args().skip(1).map(|a| a.parse::<usize>().unwrap()) {
        let Some(start) = chunk.anims[id] else { println!("{id}: missing"); continue };
        let mut frames = vec![start];
        while let Some(n) = chunk.keyframes[*frames.last().unwrap()].next {
            if n == start || frames.len() > 64 { break; }
            frames.push(n);
        }
        let ys: Vec<String> = frames.iter().map(|&f| { let o = chunk.pose(f).unwrap()[0].offset; format!("({:.0},{:.0},{:.0})", o[0], o[1], o[2]) }).collect();
        println!("{id}: {}", ys.join(" "));
    }
}
