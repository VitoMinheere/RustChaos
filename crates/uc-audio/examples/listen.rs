// Plays a few of the game's sounds through the default device (a manual check).
// cargo run --release -p uc-audio --example listen
use std::{thread::sleep, time::Duration};
fn main() {
    let game = uc_formats::default_install().expect("install");
    let mut a = uc_audio::Audio::new(&game).expect("audio device");
    println!("music world for ftutor1: {}", uc_audio::music_world(&game, "ftutor1", 0));
    a.music(Some("music01/darci/Fight1.wav"));
    for name in ["f_PUNCH1.wav", "Footsteps/HardStep1.wav", "darci/DARCI-ARREST01.wav", "f_PUNCH3.wav"] {
        a.play(name, 1.0);
        for _ in 0..10 {
            a.update(0.05);
            sleep(Duration::from_millis(50));
        }
    }
    println!("fight1 voice line 12: {}", a.voice("fight1", 12));
    for _ in 0..40 {
        a.update(0.05);
        sleep(Duration::from_millis(50));
    }
    a.music(None);
    for _ in 0..40 {
        a.update(0.05);
        sleep(Duration::from_millis(50));
    }
    println!("done");
}
