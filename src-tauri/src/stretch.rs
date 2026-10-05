//! Pitch and speed for a voice that has neither (Pocket TTS, `synth.rs`),
//! each on its own: pure and unit-tested.
//!
//! Speed is a time stretch by WSOLA (waveform-similarity overlap-add):
//! the sound is cut into overlapping windows and laid down again closer
//! together or further apart, each window nudged to where it best
//! continues the last, so the pitch stays. Pitch is a stretch by the
//! pitch's factor, then a resample that brings the length back and
//! raises or lowers every frequency by that factor.

/// Window, in samples at 24 kHz: 40 ms.
const WINDOW: usize = 960;
/// Laid down every half window, so the Hann windows sum to one.
const HOP: usize = WINDOW / 2;
/// How far a window may move to fit: 10 ms either way.
const SEEK: usize = 240;
/// Similarity is measured on every 4th sample: plenty for speech, and quick.
const STRIDE: usize = 4;

/// `samples` with every frequency times `pitch` and played `speed` times
/// as fast (both 1.0 for unchanged).
pub fn reshape(samples: &[f32], pitch: f32, speed: f32) -> Vec<f32> {
    let (pitch, speed) = (pitch.clamp(0.25, 4.0), speed.clamp(0.25, 4.0));
    if (pitch - 1.0).abs() < 0.01 && (speed - 1.0).abs() < 0.01 {
        return samples.to_vec();
    }
    let stretched = if ((pitch / speed) - 1.0).abs() < 0.01 { samples.to_vec() } else { wsola(samples, pitch / speed) };
    if (pitch - 1.0).abs() < 0.01 { stretched } else { resample(&stretched, pitch) }
}

/// `samples` made `factor` times as long, the pitch kept.
pub fn wsola(samples: &[f32], factor: f32) -> Vec<f32> {
    let n = samples.len();
    if n < WINDOW * 2 {
        return resample(samples, 1.0 / factor);
    }
    let hann: Vec<f32> = (0..WINDOW).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / WINDOW as f32).cos()).collect();
    let out_len = (n as f32 * factor) as usize;
    let mut out = vec![0.0f32; out_len + WINDOW];
    let mut weight = vec![0.0f32; out_len + WINDOW];
    let read = |at: isize| if at >= 0 && (at as usize) < n { samples[at as usize] } else { 0.0 };
    // Where the last window was taken from: the next should continue it.
    let mut last: isize = 0;
    let mut k = 0;
    while k * HOP < out_len {
        let nominal = (k as f32 * HOP as f32 / factor) as isize;
        let from = if k == 0 {
            0
        } else {
            // The best fit near `nominal` for what follows `last` naturally.
            let natural = last + HOP as isize;
            let mut best = (f32::MIN, nominal);
            let mut d = -(SEEK as isize);
            while d <= SEEK as isize {
                let at = nominal + d;
                let mut sum = 0.0;
                let mut i = 0;
                while i < WINDOW {
                    sum += read(at + i as isize) * read(natural + i as isize);
                    i += STRIDE;
                }
                if sum > best.0 {
                    best = (sum, at);
                }
                d += 2;
            }
            best.1
        };
        let start = k * HOP;
        for i in 0..WINDOW {
            out[start + i] += read(from + i as isize) * hann[i];
            weight[start + i] += hann[i];
        }
        last = from;
        k += 1;
    }
    out.truncate(out_len);
    for (s, w) in out.iter_mut().zip(&weight) {
        if *w > 1e-3 {
            *s /= w;
        }
    }
    out
}

/// `samples` read `step` samples at a time (linearly between): shorter
/// and higher above 1.0.
pub fn resample(samples: &[f32], step: f32) -> Vec<f32> {
    if samples.is_empty() || step <= 0.0 {
        return Vec::new();
    }
    let len = (samples.len() as f32 / step) as usize;
    (0..len)
        .map(|i| {
            let at = i as f32 * step;
            let j = at as usize;
            let t = at - j as f32;
            let a = samples[j.min(samples.len() - 1)];
            let b = samples[(j + 1).min(samples.len() - 1)];
            a + (b - a) * t
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 24_000.0;

    fn tone(hz: f32, seconds: f32) -> Vec<f32> {
        (0..(RATE * seconds) as usize).map(|i| (std::f32::consts::TAU * hz * i as f32 / RATE).sin() * 0.5).collect()
    }

    /// The tone's frequency, from its upward zero crossings (the edges left out).
    fn frequency(s: &[f32]) -> f32 {
        let s = &s[s.len() / 10..s.len() * 9 / 10];
        let ups = s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        ups as f32 * RATE / s.len() as f32
    }

    fn near(got: f32, want: f32, share: f32) -> bool {
        (got - want).abs() <= want * share
    }

    #[test]
    fn unchanged_is_unchanged() {
        let t = tone(220.0, 0.5);
        assert_eq!(reshape(&t, 1.0, 1.0), t);
    }

    #[test]
    fn speed_changes_length_not_pitch() {
        let t = tone(220.0, 1.0);
        for speed in [0.6, 1.5, 2.5] {
            let out = reshape(&t, 1.0, speed);
            assert!(near(out.len() as f32, t.len() as f32 / speed, 0.03), "speed {speed}: {} samples", out.len());
            assert!(near(frequency(&out), 220.0, 0.04), "speed {speed}: {} Hz", frequency(&out));
        }
    }

    #[test]
    fn pitch_changes_pitch_not_length() {
        let t = tone(220.0, 1.0);
        for pitch in [0.7, 1.4] {
            let out = reshape(&t, pitch, 1.0);
            assert!(near(out.len() as f32, t.len() as f32, 0.03), "pitch {pitch}: {} samples", out.len());
            assert!(near(frequency(&out), 220.0 * pitch, 0.04), "pitch {pitch}: {} Hz", frequency(&out));
        }
        // Both at once.
        let out = reshape(&t, 1.25, 2.0);
        assert!(near(out.len() as f32, t.len() as f32 / 2.0, 0.03));
        assert!(near(frequency(&out), 275.0, 0.04));
    }

    #[test]
    fn short_and_empty_sounds_survive() {
        assert!(reshape(&[], 1.5, 0.5).is_empty());
        let blip = tone(440.0, 0.02);
        let out = reshape(&blip, 1.0, 2.0);
        assert!(near(out.len() as f32, blip.len() as f32 / 2.0, 0.05));
    }
}
