// Lists which animation ids a character file has, with frame counts (debugging aid).
// cargo run --example anim_list -- data\rthug.all 3 19 35
use uc_formats::all::AnimChunk;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let g = uc_formats::default_install().expect("install");
    let chunk = AnimChunk::parse(&std::fs::read(uc_formats::resolve(&g, &args[0]).unwrap()).unwrap()).unwrap();
    println!("{} bodies, {} people types, {} parts", chunk.bodies.len(), chunk.people_types.len(), chunk.element_count);
    for b in &chunk.bodies { println!("  body: {} parts", b.parts.len()); }
    let ids: Vec<usize> = if args.len() > 1 { args[1..].iter().map(|s| s.parse().unwrap()).collect() } else { (0..chunk.anims.len()).collect() };
    for id in ids {
        let Some(start) = chunk.anims.get(id).copied().flatten() else { continue };
        let (mut f, mut n, mut ticks) = (start, 1, 0u32);
        loop {
            ticks += 256 / (chunk.keyframes[f].tween_step.max(1) as u32 * 2);
            match chunk.keyframes[f].next { Some(x) if x != start && n < 200 => { f = x; n += 1 } _ => break }
        }
        let looped = chunk.keyframes[f].next == Some(start);
        print!("{id}:{n}f/{ticks}t{} ", if looped { "L" } else { "" });
    }
    println!();
}
