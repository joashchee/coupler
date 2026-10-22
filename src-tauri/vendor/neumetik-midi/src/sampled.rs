// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! A note played from a sound bank's sample: one region of a SoundFont or
//! DLS instrument, read at the note's pitch (between samples by straight
//! lines), its loop gone round, then the low-pass filter, the volume
//! envelope and the region's level. As for Neumetik's own voices, the
//! envelopes, LFOs, pitch and filter are worked out once a block; only
//! the sample, the filter and the level's ramp run every sample.

use crate::bank::{seconds, Envelope, Looping, Region};
use crate::voice::{Coef, Controls, Svf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Delay,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
    Done,
}

/// The quietest level that counts: -96 dB, in centibels.
const SILENT: f32 = 960.0;

/// An envelope as it plays: the volume envelope's level is an
/// attenuation (centibels down from the peak), straight lines in decibels
/// as both formats have it, but for its attack, a straight line up in
/// level; the modulation envelope's is a level, 0 to 1, all straight.
#[derive(Clone, Copy, Debug)]
struct Env {
    volume: bool,
    stage: Stage,
    /// Seconds into the stage.
    t: f32,
    delay: f32,
    attack: f32,
    hold: f32,
    /// How fast the decay and release fall: centibels (or level) a second.
    decay: f32,
    release: f32,
    sustain: f32,
    /// The attenuation in the decay and release (the volume envelope's),
    /// or the level (the modulation envelope's).
    at: f32,
    /// Its output, 0 to 1.
    out: f32,
}

impl Env {
    fn new(e: &Envelope, volume: bool, key: f32, vel: f32) -> Env {
        let below = e.key_center - key;
        // The shortest release that doesn't click.
        let release = seconds(e.release).max(if volume { 0.004 } else { 0.001 });
        let full = if volume { e.range } else { 1.0 };
        Env {
            volume,
            stage: Stage::Delay,
            t: 0.0,
            delay: seconds(e.delay),
            attack: seconds(e.attack + e.vel_attack * vel),
            hold: seconds(e.hold + e.key_hold * below),
            decay: full / seconds(e.decay + e.key_decay * below).max(0.001),
            release: full / release,
            sustain: e.sustain,
            at: if volume { 0.0 } else { 1.0 },
            out: 0.0,
        }
    }

    fn level(&self) -> f32 {
        if self.volume {
            if self.at >= SILENT {
                0.0
            } else {
                10f32.powf(-self.at / 200.0)
            }
        } else {
            self.at.max(0.0)
        }
    }

    fn release(&mut self) {
        if self.stage == Stage::Done || self.stage == Stage::Release {
            return;
        }
        if self.volume {
            // From where it's got to, in decibels.
            self.at = if self.out > 1e-5 { -200.0 * self.out.log10() } else { SILENT };
        } else {
            self.at = self.out;
        }
        self.stage = Stage::Release;
        self.t = 0.0;
    }

    /// Steps `dt` seconds on; its output then.
    fn step(&mut self, dt: f32) -> f32 {
        self.t += dt;
        loop {
            let next = match self.stage {
                Stage::Delay if self.t >= self.delay => Stage::Attack,
                Stage::Attack if self.t >= self.attack => Stage::Hold,
                Stage::Hold if self.t >= self.hold => Stage::Decay,
                _ => break,
            };
            self.t -= match self.stage {
                Stage::Delay => self.delay,
                Stage::Attack => self.attack,
                _ => self.hold,
            };
            self.stage = next;
            if next == Stage::Decay {
                self.at = if self.volume { 0.0 } else { 1.0 };
            }
        }
        self.out = match self.stage {
            Stage::Delay => 0.0,
            Stage::Attack => (self.t / self.attack.max(1e-6)).min(1.0),
            Stage::Hold => 1.0,
            Stage::Decay => {
                let fall = self.decay * dt.min(self.t);
                if self.volume {
                    self.at += fall;
                    if self.at >= self.sustain {
                        self.at = self.sustain;
                        self.stage = Stage::Sustain;
                    }
                } else {
                    self.at -= fall;
                    if self.at <= self.sustain {
                        self.at = self.sustain;
                        self.stage = Stage::Sustain;
                    }
                }
                self.level()
            }
            Stage::Sustain => self.level(),
            Stage::Release => {
                if self.volume {
                    self.at += self.release * dt;
                    if self.at >= SILENT {
                        self.stage = Stage::Done;
                    }
                } else {
                    self.at -= self.release * dt;
                    if self.at <= 0.0 {
                        self.stage = Stage::Done;
                    }
                }
                self.level()
            }
            Stage::Done => 0.0,
        };
        if self.volume && self.stage == Stage::Sustain && self.sustain >= SILENT {
            self.stage = Stage::Done;
        }
        self.out
    }
}

/// A triangle, 0 at its start and rising: SoundFont's and DLS's LFOs.
fn triangle(phase: f32) -> f32 {
    let p = phase + 0.25;
    1.0 - 4.0 * (p - p.floor() - 0.5).abs()
}

/// A sampled note.
#[derive(Clone, Copy, Debug)]
pub struct Sampled {
    r: Region,
    key: f32,
    vel: f32,
    /// The region's level and the velocity's, before the envelope.
    gain: f32,
    /// Where in the bank's samples it's reading.
    pos: f64,
    released: bool,
    vol: Env,
    modenv: Env,
    /// Seconds since it started.
    t: f32,
    filter: Svf,
    coef: Coef,
    /// The level at the end of the last block, ramped from in the next.
    level: f32,
    pub done: bool,
}

impl Sampled {
    pub fn new(r: &Region, key: u8, vel: u8) -> Sampled {
        let key = f32::from(r.fixed_key.unwrap_or(key));
        let v = f32::from(r.fixed_vel.unwrap_or(vel).max(1)) / 127.0;
        Sampled {
            r: *r,
            key,
            vel: v,
            // A bank can ask for anything: up to 24 dB of boost.
            gain: 10f32.powf(-r.attenuation.clamp(-240.0, 1440.0) / 200.0) * v.powf(r.vel_curve.clamp(0.0, 8.0)),
            pos: f64::from(r.start),
            released: false,
            vol: Env::new(&r.vol_env, true, key, v),
            modenv: Env::new(&r.mod_env, false, key, v),
            t: 0.0,
            filter: Svf::default(),
            coef: Coef::default(),
            level: 0.0,
            done: false,
        }
    }

    pub fn release(&mut self) {
        self.released = true;
        self.vol.release();
        self.modenv.release();
    }

    /// How loud it is now, for choosing which voice to steal.
    pub fn loudness(&self) -> f32 {
        self.level
    }

    pub fn pan(&self) -> f32 {
        self.r.pan
    }

    pub fn reverb(&self) -> f32 {
        self.r.reverb
    }

    pub fn chorus(&self) -> f32 {
        self.r.chorus
    }

    pub fn exclusive(&self) -> u16 {
        self.r.exclusive
    }

    /// Renders `out.len()` samples (a block at most) into `out`, which is
    /// zeroed, at `rate`, reading `samples`.
    pub fn render(&mut self, out: &mut [f32], ctl: &Controls, samples: &[i16], rate: f32) {
        let n = out.len();
        let dt = n as f32 / rate;
        let r = &self.r;
        let lfo = |(delay, hz): (f32, f32), t: f32| {
            let d = seconds(delay);
            if t < d {
                0.0
            } else {
                triangle((t - d) * hz)
            }
        };
        let mod_lfo = lfo(r.mod_lfo, self.t);
        let vib_lfo = lfo(r.vib_lfo, self.t);
        let menv = self.modenv.step(dt);
        let env = self.vol.step(dt);
        if self.vol.stage == Stage::Done {
            self.done = true;
        }

        let cents = (self.key - r.root) * r.scale
            + r.tune
            + ctl.bend * 100.0
            + menv * r.mod_env_pitch
            + mod_lfo * (r.mod_lfo_pitch + ctl.modulation * r.wheel_mod_lfo)
            + vib_lfo * (r.vib_lfo_pitch + ctl.modulation * r.wheel_vib_lfo);
        // Ten octaves either way, which no bank needs, keep it finite.
        let inc = f64::from(r.rate.clamp(1.0, 1e6) / rate * 2f32.powf(cents.clamp(-12_000.0, 12_000.0) / 1200.0));

        let filtered = r.filtered || ctl.brightness < 0.0;
        if filtered {
            let cents = menv * r.mod_env_cutoff + mod_lfo * r.mod_lfo_cutoff + self.vel * r.vel_cutoff + self.key * r.key_cutoff + ctl.brightness * 1200.0;
            let cutoff = crate::bank::hz((r.cutoff + cents).clamp(1500.0, 13_500.0));
            self.coef = Coef::lowpass(cutoff, r.q + ctl.resonance * 24.0, rate);
        }

        let tremolo = if r.mod_lfo_volume != 0.0 { 10f32.powf((mod_lfo * r.mod_lfo_volume).clamp(-960.0, 240.0) / 200.0) } else { 1.0 };
        let target = if self.done { 0.0 } else { env * self.gain * tremolo };
        let step = (target - self.level) / n as f32;
        let mut level = self.level;

        let looping = r.looping == Looping::On || (r.looping == Looping::UntilRelease && !self.released);
        let (start, end) = (f64::from(r.loop_start), f64::from(r.loop_end));
        let length = end - start;
        let sample = |i: usize| f32::from(samples.get(i).copied().unwrap_or(0)) * (1.0 / 32_768.0);
        for o in out.iter_mut() {
            level += step;
            let i = self.pos as usize;
            let f = (self.pos - i as f64) as f32;
            let a = sample(i);
            let b = if looping && i + 1 >= r.loop_end as usize {
                sample(r.loop_start as usize)
            } else if i + 1 < r.end as usize {
                sample(i + 1)
            } else {
                0.0
            };
            let mut s = a + (b - a) * f;
            if filtered {
                s = self.filter.tick(&self.coef, crate::patch::Filter::Low, s);
            }
            *o = s * level;
            self.pos += inc;
            if looping {
                if self.pos >= end {
                    self.pos = start + (self.pos - end) % length;
                }
            } else if self.pos >= f64::from(r.end) {
                self.done = true;
                break;
            }
        }
        self.level = level;
        self.t += dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_triangle_starts_at_zero_and_rises() {
        assert!(triangle(0.0).abs() < 1e-6);
        assert!((triangle(0.25) - 1.0).abs() < 1e-6);
        assert!(triangle(0.5).abs() < 1e-6);
        assert!((triangle(0.75) + 1.0).abs() < 1e-6);
        assert!(triangle(0.1) > 0.0);
    }

    fn env(attack: f32, decay: f32, sustain: f32, release: f32) -> Envelope {
        let s = |x: f32| 1200.0 * x.log2();
        Envelope { delay: f32::NEG_INFINITY, attack: s(attack), hold: f32::NEG_INFINITY, decay: s(decay), sustain, release: s(release), key_hold: 0.0, key_decay: 0.0, key_center: 60.0, vel_attack: 0.0, range: 1000.0 }
    }

    #[test]
    fn the_volume_envelope_falls_in_decibels_to_its_sustain_then_releases() {
        // 10 ms attack, a second to fall 100 dB, sustain at -20 dB.
        let mut e = Env::new(&env(0.01, 1.0, 200.0, 0.5), true, 60.0, 1.0);
        let mut t = 0.0;
        while t < 0.01 {
            e.step(0.001);
            t += 0.001;
        }
        assert!(e.out > 0.9, "{}", e.out);
        // 0.1 s into the decay: 10 dB down.
        for _ in 0..100 {
            e.step(0.001);
        }
        assert!((e.out - 10f32.powf(-0.5)).abs() < 0.02, "{}", e.out);
        for _ in 0..400 {
            e.step(0.001);
        }
        assert_eq!(e.stage, Stage::Sustain);
        assert!((e.out - 0.1).abs() < 1e-3);
        e.release();
        // From -20 dB, 76 dB to go at 200 dB a second.
        for _ in 0..390 {
            e.step(0.001);
        }
        assert_eq!(e.stage, Stage::Done);
    }
}
