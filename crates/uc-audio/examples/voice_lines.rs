//! Every spoken line in the install's missions, for making voices for the ones never recorded:
//! `voice_lines OUT [MISSION...]` writes `OUT/lines.tsv` (mission, point, speech number, line
//! letter, voice, recorded, seconds, text) and each recorded line as 16-bit PCM under
//! `OUT/clips/<voice>/`, the voice's samples to learn from. With no missions, all of them.
//!
//! The voice is who says it, as the game picks (`Script`'s `Speaker`): a conversation's two
//! people take turns; a message is said by whoever was talked to to set it off (a
//! `PERSON_USED` trigger), else by its `Data[2]`, else the radio.

use std::fmt::Write as _;
use std::path::Path;
use uc_formats::ucm::{trigger, waypoint, EventPoint, Mission};

/// The voice a person made by event point `p` speaks with.
fn voice_of(p: &EventPoint) -> &'static str {
    if p.kind == waypoint::CREATE_PLAYER {
        return match p.data[0] {
            2 => "roper",
            3 => "cop",
            4 => "gang",
            _ => "darci",
        };
    }
    if p.kind != waypoint::CREATE_ENEMIES {
        return "radio";
    }
    // `ET_*` (Mission.h), as `enemy_person` reads them.
    match p.data[0] & 0xffff {
        3 | 4 => "slag",
        5 | 17 => "workman",
        6 | 9 => "rasta",
        7 | 10 => "red",
        8 | 11 => "grey",
        12..=15 => "cop",
        16 => "hostage",
        18 => "tramp",
        19..=21 => "mib",
        22 => "darci",
        23 => "roper",
        _ => "civ",
    }
}

/// The 16-bit mono PCM WAV of `clip`, its channels mixed down.
fn pcm(clip: &uc_audio::Clip) -> Vec<u8> {
    let ch = clip.channels.max(1) as usize;
    let data: Vec<u8> = clip.samples.chunks(ch)
        .map(|f| (f.iter().sum::<f32>() / ch as f32 * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .flat_map(i16::to_le_bytes).collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend(b"RIFF");
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(clip.rate.to_le_bytes());
    out.extend((clip.rate * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().expect("usage: voice_lines OUT [MISSION...]"));
    let game = uc_formats::default_install().expect("install");
    let mut missions: Vec<String> = args.collect();
    if missions.is_empty() {
        let levels = uc_formats::resolve(&game, "levels").expect("levels");
        missions = std::fs::read_dir(levels).unwrap().flatten()
            .filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".ucm").map(str::to_string))
            .collect();
        missions.sort();
    }
    let mut tsv = String::from("mission\tpoint\tspeech\tline\tvoice\trecorded\tseconds\ttext\n");
    for m in &missions {
        let Some(mission) = uc_formats::resolve(&game, &format!("levels\\{m}.ucm"))
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| Mission::parse(&b).ok())
        else {
            eprintln!("{m}: can't read");
            continue;
        };
        let texts = mission.point_texts();
        let point = |i: i32| mission.event_points.iter().find(|e| e.used != 0 && e.index as i32 == i);
        let voice = |i: i32| match i {
            0 => Some("radio"),
            0xffff | 0xfffe => None,
            _ => Some(point(i).map_or("radio", voice_of)),
        };
        for e in mission.event_points.iter().filter(|e| e.used != 0) {
            let Some(text) = texts.get(&e.index) else { continue };
            // The recording's number (`Script`'s `speech`).
            let speech = if e.data[9] > 0 { e.data[9] as usize } else { e.index };
            let lines: Vec<(String, Option<&str>, &str)> = match e.kind {
                waypoint::MESSAGE => {
                    let by = if e.trigger == trigger::PERSON_USED { voice(e.ep_ref as i32) } else { voice(e.data[2]) };
                    vec![(String::new(), by, text.as_str())]
                }
                waypoint::CONVERSATION => text.split('|').map(str::trim).filter(|l| !l.is_empty()).enumerate()
                    .map(|(n, l)| (((b'A' + n as u8) as char).to_string(), voice(if n % 2 == 0 { e.data[1] } else { e.data[2] }), l))
                    .collect(),
                _ => continue,
            };
            for (letter, by, line) in lines {
                let Some(by) = by else { continue };
                let name = format!("{m}.ucm{speech}{letter}.wav");
                let clip = uc_formats::resolve(&game, &format!("talk2\\{name}"))
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|b| uc_audio::wav::decode(&b).ok());
                if let Some(c) = &clip {
                    let dir = out.join("clips").join(by);
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(dir.join(&name), pcm(c)).unwrap();
                }
                let secs = clip.as_ref().map_or(0.0, |c| c.seconds());
                let flat = line.replace(['\t', '\n', '\r'], " ");
                writeln!(tsv, "{m}\t{}\t{speech}\t{letter}\t{by}\t{}\t{secs:.2}\t{flat}", e.index, clip.is_some() as u8).unwrap();
            }
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(Path::new(&out).join("lines.tsv"), tsv).unwrap();
}
