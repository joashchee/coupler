// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! The send effects every voice shares: a chorus and a reverb.

use std::f32::consts::FRAC_PI_2;

/// A stereo chorus: one delay line, read at two slowly moving places.
pub(crate) struct Chorus {
    line: Vec<f32>,
    at: usize,
    phase: f32,
    rate: f32,
}

impl Chorus {
    pub(crate) fn new(rate: f32) -> Chorus {
        Chorus { line: vec![0.0; ((rate * 0.03) as usize).next_power_of_two()], at: 0, phase: 0.0, rate }
    }

    pub(crate) fn memory_bytes(&self) -> usize {
        self.line.capacity() * 4
    }

    fn read(&self, delay: f32) -> f32 {
        let mask = self.line.len() - 1;
        let pos = self.at as f32 - delay;
        let i = pos.floor();
        let f = pos - i;
        let i = i as isize as usize;
        self.line[i & mask] * (1.0 - f) + self.line[i.wrapping_add(1) & mask] * f
    }

    pub(crate) fn process(&mut self, input: &[f32], left: &mut [f32], right: &mut [f32]) {
        let mask = self.line.len() - 1;
        let step = 0.6 / self.rate;
        for i in 0..input.len() {
            self.line[self.at & mask] = input[i];
            let wobble = (self.phase * std::f32::consts::TAU).sin();
            let wobble2 = (self.phase * std::f32::consts::TAU + FRAC_PI_2).sin();
            left[i] += self.read((0.007 + 0.0025 * wobble) * self.rate) * 0.8;
            right[i] += self.read((0.008 + 0.0025 * wobble2) * self.rate) * 0.8;
            self.phase = (self.phase + step).fract();
            self.at = (self.at + 1) & mask;
        }
    }
}

/// Freeverb, slimmed: four damped combs and two allpasses a side.
pub(crate) struct Reverb {
    combs: [[Comb; 4]; 2],
    allpasses: [[Allpass; 2]; 2],
}

struct Comb {
    line: Vec<f32>,
    at: usize,
    store: f32,
}

struct Allpass {
    line: Vec<f32>,
    at: usize,
}

impl Reverb {
    pub(crate) fn new(rate: f32) -> Reverb {
        let scale = rate / 44_100.0;
        let len = |n: usize, side: usize| (((n + side * 23) as f32 * scale) as usize).max(1);
        let combs = |side| [1116, 1277, 1356, 1491].map(|n| Comb { line: vec![0.0; len(n, side)], at: 0, store: 0.0 });
        let allpasses = |side| [556, 341].map(|n| Allpass { line: vec![0.0; len(n, side)], at: 0 });
        Reverb { combs: [combs(0), combs(1)], allpasses: [allpasses(0), allpasses(1)] }
    }

    pub(crate) fn memory_bytes(&self) -> usize {
        let combs: usize = self.combs.iter().flatten().map(|c| c.line.capacity()).sum();
        let allpasses: usize = self.allpasses.iter().flatten().map(|a| a.line.capacity()).sum();
        (combs + allpasses) * 4
    }

    pub(crate) fn process(&mut self, input: &[f32], left: &mut [f32], right: &mut [f32]) {
        const FEEDBACK: f32 = 0.84;
        const DAMP: f32 = 0.25;
        for i in 0..input.len() {
            let x = input[i] * 0.06;
            for side in 0..2 {
                let mut out = 0.0;
                for c in &mut self.combs[side] {
                    let y = c.line[c.at];
                    c.store = y * (1.0 - DAMP) + c.store * DAMP;
                    c.line[c.at] = x + c.store * FEEDBACK;
                    c.at = (c.at + 1) % c.line.len();
                    out += y;
                }
                for a in &mut self.allpasses[side] {
                    let y = a.line[a.at];
                    a.line[a.at] = out + y * 0.5;
                    a.at = (a.at + 1) % a.line.len();
                    out = y - out;
                }
                if side == 0 {
                    left[i] += out;
                } else {
                    right[i] += out;
                }
            }
        }
    }
}
