// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Interleaved and 16-bit output from a renderer that writes left and
//! right apart, through buffers on the stack: no allocation.

const CHUNK: usize = 256;

/// Fills `out` with interleaved stereo; a stray last sample is silence.
pub(crate) fn interleaved(out: &mut [f32], mut render: impl FnMut(&mut [f32], &mut [f32])) {
    let (mut l, mut r) = ([0f32; CHUNK], [0f32; CHUNK]);
    for chunk in out.chunks_mut(CHUNK * 2) {
        let n = chunk.len() / 2;
        render(&mut l[..n], &mut r[..n]);
        for i in 0..n {
            chunk[2 * i] = l[i];
            chunk[2 * i + 1] = r[i];
        }
        if chunk.len() % 2 == 1 {
            chunk[chunk.len() - 1] = 0.0;
        }
    }
}

/// Fills `out` with interleaved stereo as 16-bit samples, clipped.
pub(crate) fn interleaved_i16(out: &mut [i16], mut render: impl FnMut(&mut [f32], &mut [f32])) {
    let (mut l, mut r) = ([0f32; CHUNK], [0f32; CHUNK]);
    for chunk in out.chunks_mut(CHUNK * 2) {
        let n = chunk.len() / 2;
        render(&mut l[..n], &mut r[..n]);
        for i in 0..n {
            chunk[2 * i] = to_i16(l[i]);
            chunk[2 * i + 1] = to_i16(r[i]);
        }
        if chunk.len() % 2 == 1 {
            chunk[chunk.len() - 1] = 0;
        }
    }
}

fn to_i16(s: f32) -> i16 {
    (s.clamp(-1.0, 1.0) * 32767.0) as i16
}

/// Interleaved stereo as a 16-bit WAV file, clipped at full scale.
pub fn wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&[1, 0, 2, 0]);
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 4).to_le_bytes());
    out.extend_from_slice(&[4, 0, 16, 0]);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&to_i16(*s).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaving_takes_left_then_right_in_any_length() {
        let mut frame = 0f32;
        let mut render = |l: &mut [f32], r: &mut [f32]| {
            for i in 0..l.len() {
                l[i] = frame;
                r[i] = -frame;
                frame += 1.0;
            }
        };
        let mut out = vec![9.0; 1001];
        interleaved(&mut out, &mut render);
        assert_eq!(&out[..4], &[0.0, -0.0, 1.0, -1.0]);
        assert_eq!(out[998], 499.0);
        assert_eq!(out[1000], 0.0);
        let mut ints = vec![0i16; 4];
        interleaved_i16(&mut ints, |l, r| {
            l.fill(0.5);
            r.fill(-2.0);
        });
        assert_eq!(ints, [16383, -32767, 16383, -32767]);
    }
}
