//! WAV decoding for the game's sounds: PCM (8/16-bit), IEEE float and IMA ADPCM (format 17,
//! which 1,886 of the shipped files use, including every mission voice line).

/// A decoded sound: interleaved samples in -1..1.
#[derive(Debug, Clone)]
pub struct Clip {
    pub channels: u16,
    pub rate: u32,
    pub samples: Vec<f32>,
}

impl Clip {
    pub fn seconds(&self) -> f32 {
        self.samples.len() as f32 / self.channels.max(1) as f32 / self.rate.max(1) as f32
    }
}

const IMA_STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118,
    130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060,
    1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484,
    7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const IMA_INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

struct Ima {
    predictor: i32,
    index: i32,
}

impl Ima {
    fn decode(&mut self, nibble: u8) -> i16 {
        let step = IMA_STEPS[self.index as usize];
        let mut diff = step >> 3;
        if nibble & 1 != 0 { diff += step >> 2; }
        if nibble & 2 != 0 { diff += step >> 1; }
        if nibble & 4 != 0 { diff += step; }
        if nibble & 8 != 0 { diff = -diff; }
        self.predictor = (self.predictor + diff).clamp(-32768, 32767);
        self.index = (self.index + IMA_INDEX[(nibble & 7) as usize]).clamp(0, 88);
        self.predictor as i16
    }
}

fn u16le(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32le(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// Microsoft IMA ADPCM: each block starts with a 4-byte header per channel (first sample,
/// step index); then 4-byte groups of 8 nibbles alternate between channels.
fn decode_ima(data: &[u8], channels: usize, block_align: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for block in data.chunks(block_align.max(4 * channels)) {
        if block.len() < 4 * channels {
            break;
        }
        let mut st: Vec<Ima> = (0..channels)
            .map(|c| Ima { predictor: u16le(block, c * 4) as i16 as i32, index: (block[c * 4 + 2] as i32).clamp(0, 88) })
            .collect();
        for s in &st {
            out.push(s.predictor as f32 / 32768.0);
        }
        // Per channel, decode its 8-sample groups into a scratch list, then interleave.
        let body = &block[4 * channels..];
        let groups = body.len() / (4 * channels);
        let mut per: Vec<Vec<i16>> = vec![Vec::with_capacity(groups * 8); channels];
        for g in 0..groups {
            for (c, s) in st.iter_mut().enumerate() {
                for &byte in &body[(g * channels + c) * 4..(g * channels + c) * 4 + 4] {
                    per[c].push(s.decode(byte & 0x0F));
                    per[c].push(s.decode(byte >> 4));
                }
            }
        }
        for i in 0..groups * 8 {
            for p in &per {
                out.push(p[i] as f32 / 32768.0);
            }
        }
    }
    out
}

/// Decodes a RIFF WAVE file.
pub fn decode(bytes: &[u8]) -> Result<Clip, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let (mut fmt, mut data) = (None, None);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32le(bytes, i + 4) as usize;
        let body = &bytes[i + 8..(i + 8 + len).min(bytes.len())];
        match id {
            b"fmt " if body.len() >= 16 => fmt = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        i += 8 + len + (len & 1);
    }
    let fmt = fmt.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    let (tag, channels, rate) = (u16le(fmt, 0), u16le(fmt, 2), u32le(fmt, 4));
    let (block_align, bits) = (u16le(fmt, 12) as usize, u16le(fmt, 14));
    if channels == 0 || rate == 0 {
        return Err("bad format".into());
    }
    let samples = match (tag, bits) {
        // A few car sounds are tagged as float but hold 16-bit PCM.
        (1, 16) | (3, 16) => data.as_chunks::<2>().0.iter().map(|s| i16::from_le_bytes(*s) as f32 / 32768.0).collect(),
        (1, 8) => data.iter().map(|&s| (s as f32 - 128.0) / 128.0).collect(),
        (3, 32) => data.as_chunks::<4>().0.iter().map(|s| f32::from_le_bytes(*s)).collect(),
        (17, 4) => decode_ima(data, channels as usize, block_align),
        _ => return Err(format!("unsupported WAV format {tag} ({bits} bits)")),
    };
    Ok(Clip { channels, rate, samples })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ima_decodes_a_constant_block() {
        // One mono block: first sample 1000, then 8 nibbles of 0 (smallest positive steps).
        let mut b = vec![];
        b.extend_from_slice(&1000i16.to_le_bytes());
        b.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        let s = decode_ima(&b, 1, 8);
        assert_eq!(s.len(), 9);
        assert!((s[0] - 1000.0 / 32768.0).abs() < 1e-6);
        assert!(s.windows(2).all(|w| w[1] >= w[0]), "zero nibbles only step up slowly");
    }
}
