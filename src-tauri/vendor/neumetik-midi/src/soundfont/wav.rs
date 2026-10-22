// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Samples from WAV files: PCM of 8, 16, 24 or 32 bits, or 32-bit float,
//! mono or stereo, with the key and loop a sampler wrote in its `smpl`
//! chunk, if it did.

use super::{Link, Sample};
use crate::bank::riff::{chunks, u16_at, u32_at};
use crate::Error;

impl Sample {
    /// A WAV file's sound as samples: one if it's mono, a left and right
    /// pair if it's stereo (more channels are mixed to two), at its own
    /// rate, named `name`. Its root key and loop come from its `smpl`
    /// chunk, else `root` and no loop.
    ///
    /// Add a pair with [`SoundFont::add_stereo`](super::SoundFont::add_stereo).
    pub fn from_wav(name: &str, bytes: &[u8], root: u8) -> Result<Vec<Sample>, Error> {
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(Error::NotWav);
        }
        let body = &bytes[12..(8 + u32_at(bytes, 4) as usize).clamp(12, bytes.len())];
        let fmt = chunks(body).find(|c| &c.id == b"fmt ").map(|c| c.data).ok_or(Error::NotWav)?;
        let data = chunks(body).find(|c| &c.id == b"data").map(|c| c.data).ok_or(Error::NotWav)?;
        let mut format = u16_at(fmt, 0);
        let channels = usize::from(u16_at(fmt, 2));
        let rate = u32_at(fmt, 4);
        let bits = u16_at(fmt, 14);
        if format == 0xfffe && fmt.len() >= 26 {
            format = u16_at(fmt, 24);
        }
        let width = match (format, bits) {
            (1, 8) => 1,
            (1, 16) => 2,
            (1, 24) => 3,
            (1, 32) | (3, 32) => 4,
            _ => return Err(Error::NotWav),
        };
        if channels == 0 || rate == 0 {
            return Err(Error::NotWav);
        }
        let value = |c: &[u8]| match (format, width) {
            (1, 1) => (f32::from(c[0]) - 128.0) / 128.0,
            (1, 2) => f32::from(i16::from_le_bytes([c[0], c[1]])) / 32_768.0,
            (1, 3) => (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0,
            (1, _) => i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0,
            _ => f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
        };
        let to16 = |x: f32| (x.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
        let sides = channels.min(2);
        let mut out: Vec<Vec<i16>> = vec![Vec::with_capacity(data.len() / (width * channels)); sides];
        for frame in data.chunks_exact(width * channels) {
            if channels <= 2 {
                for (side, c) in out.iter_mut().zip(frame.chunks_exact(width)) {
                    side.push(to16(value(c)));
                }
            } else {
                // Odd channels to the left, even to the right.
                let mut sum = [0f32; 2];
                for (i, c) in frame.chunks_exact(width).enumerate() {
                    sum[i % 2] += value(c);
                }
                let per = [channels.div_ceil(2) as f32, (channels / 2) as f32];
                out[0].push(to16(sum[0] / per[0]));
                out[1].push(to16(sum[1] / per[1]));
            }
        }

        let mut root = root.min(127);
        let mut correction = 0i8;
        let mut looped = None;
        if let Some(smpl) = chunks(body).find(|c| &c.id == b"smpl").map(|c| c.data) {
            if smpl.len() >= 36 {
                root = (u32_at(smpl, 12).min(127)) as u8;
                // A fraction of a semitone up, as 2³² parts.
                correction = (f64::from(u32_at(smpl, 16)) / 4_294_967_296.0 * 100.0).round().min(99.0) as i8;
                if u32_at(smpl, 28) > 0 && smpl.len() >= 60 {
                    // The loop's last sample, not the one past it.
                    let (s, e) = (u32_at(smpl, 44), u32_at(smpl, 48).saturating_add(1));
                    let len = out[0].len() as u32;
                    if s < e && e <= len {
                        looped = Some((s, e));
                    }
                }
            }
        }
        let count = out.len();
        Ok(out
            .into_iter()
            .enumerate()
            .map(|(i, data)| {
                let side = match (count, i) {
                    (1, _) => "",
                    (_, 0) => " L",
                    _ => " R",
                };
                let mut s = Sample::new(&format!("{name}{side}"), data, rate, root);
                s.correction = -correction;
                if let Some((a, b)) = looped {
                    s = s.looped(a, b);
                }
                s.link = Link::Mono;
                s
            })
            .collect())
    }
}
