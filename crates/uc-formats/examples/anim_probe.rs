// Prints how much each body part moves across an animation (debugging aid).
use uc_formats::all::AnimChunk;
fn main() {
    let g = uc_formats::default_install().expect("install");
    let chunk = AnimChunk::parse(&std::fs::read(uc_formats::resolve(&g, r"data\darci1.all").unwrap()).unwrap()).unwrap();
    for (name, id) in [("stand", 3usize), ("breathe", 124), ("walk", 20), ("run", 119)] {
        let Some(start) = chunk.anims[id] else { println!("{name}: missing"); continue };
        let mut f = start;
        let mut frames = vec![];
        for _ in 0..12 { frames.push(f); match chunk.keyframes[f].next { Some(n) if n != start => f = n, _ => break } }
        let poses: Vec<_> = frames.iter().map(|&f| chunk.pose(f).unwrap().to_vec()).collect();
        let mut maxrot = 0f32; let mut maxoff = 0f32;
        for p in &poses[1..] { for (a, b) in poses[0].iter().zip(p) {
            for r in 0..3 { for c in 0..3 { maxrot = maxrot.max((a.rotation[r][c] - b.rotation[r][c]).abs()); } }
            for k in 0..3 { maxoff = maxoff.max((a.offset[k] - b.offset[k]).abs()); } } }
        let fe: Vec<_> = frames.iter().map(|&f| chunk.keyframes[f].first_element).collect();
        println!("{name}: frames {:?} first_elements {:?} tween_steps {:?} max rot diff {maxrot:.3} max offset diff {maxoff:.1}",
            frames, fe, frames.iter().map(|&f| chunk.keyframes[f].tween_step).collect::<Vec<_>>());
    }
    println!("pelvis rot frame0 walk: {:?}", chunk.pose(chunk.anims[20].unwrap()).unwrap()[0]);
}
