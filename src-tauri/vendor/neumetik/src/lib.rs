// Copyright 2026 ansiapps. Neumetik: see README.md for its license.

//! **Neumetik**, a synthesizer for General MIDI and Roland GS music
//! played without a SoundFont. Every instrument is a few numbers
//! (`patch.rs`) for three small engines (subtractive, FM and plucked
//! string) and every drum a recipe (`drums.rs`): no samples, nothing to
//! download, and it renders far faster than real time. Pure and
//! unit-tested.
//!
//! - `smf.rs` reads the MIDI file (formats 0, 1, 2 and RMID).
//! - `gm.rs`: GM's 128 instruments; `gs.rs`: the SC-55's variation tones
//!   (Bank Select MSB, CC 0, falling back to GM's as the SC-55 did);
//!   `drums.rs`: GM's and GS's drum map and kits (TR-808, TR-909...).
//! - `classics.rs`: the synth sounds everyone knows, from the DX7's
//!   piano to the dubstep wobble, on banks 80 (the eighties), 81 (the
//!   nineties) and 82 (the 2000s to now).
//! - The synth (here): 16 channels, `POLYPHONY` voices, sustain, pitch
//!   bend and its range, mod wheel, volume, expression, pan, brightness
//!   and resonance (CC 74, 71), reverb and chorus sends (CC 91, 93), GM
//!   and GS resets, GS's drum part on any channel, XG's drum bank (127).
//!
//! The name's the neume, the first written music: marks for how a voice
//! moves, not the sound itself, which is all a MIDI file is too.

mod classics;
pub mod drums;
mod gm;
mod gs;
mod patch;
mod smf;
mod voice;

use std::f32::consts::FRAC_PI_2;

use patch::Patch;
use smf::Message;
use voice::{Controls, Voice, BLOCK};

/// Voices at once; past it, the quietest is stolen.
pub const POLYPHONY: usize = 64;

/// A bank of patches a Bank Select picks.
pub struct Bank {
    pub msb: u8,
    pub patches: &'static [Patch],
}

/// General MIDI, then the classics: the 1980s, the 1990s, the 2000s to
/// now.
pub static BANKS: [Bank; 4] = [
    Bank { msb: 0, patches: &gm::GM },
    Bank { msb: 80, patches: classics::EIGHTIES },
    Bank { msb: 81, patches: classics::NINETIES },
    Bank { msb: 82, patches: classics::NOW },
];

/// The patch a bank and program play: a classic bank's, a GS variation,
/// or else GM's own.
pub fn patch(msb: u8, program: u8) -> &'static Patch {
    let program = program & 0x7f;
    if let Some(bank) = BANKS.iter().find(|b| b.msb == msb && msb != 0) {
        if let Some(p) = bank.patches.get(usize::from(program)) {
            return p;
        }
    }
    if msb != 0 {
        if let Some((_, _, p)) = gs::GS.iter().find(|(m, p, _)| *m == msb && *p == program) {
            return p;
        }
    }
    &gm::GM[usize::from(program)]
}

#[derive(Clone, Copy)]
struct Channel {
    bank: u8,
    program: u8,
    patch: &'static Patch,
    drums: bool,
    kit: u8,
    volume: f32,
    expression: f32,
    pan: f32,
    modulation: f32,
    bend: f32,
    bend_range: f32,
    tuning: f32,
    sustain: bool,
    reverb: f32,
    chorus: f32,
    brightness: f32,
    resonance: f32,
    /// The RPN being set (CC 101, 100), or none.
    rpn: Option<(u8, u8)>,
}

impl Channel {
    fn new(drums: bool) -> Channel {
        Channel {
            bank: 0,
            program: 0,
            patch: &gm::GM[0],
            drums,
            kit: 0,
            volume: 100.0 / 127.0,
            expression: 1.0,
            pan: 0.0,
            modulation: 0.0,
            bend: 0.0,
            bend_range: 2.0,
            tuning: 0.0,
            sustain: false,
            reverb: 40.0 / 127.0,
            chorus: 0.0,
            brightness: 0.0,
            resonance: 0.0,
            rpn: None,
        }
    }

    /// Reset All Controllers (CC 121), as GM says.
    fn reset_controllers(&mut self) {
        self.expression = 1.0;
        self.modulation = 0.0;
        self.bend = 0.0;
        self.sustain = false;
        self.rpn = None;
    }

    fn controls(&self) -> Controls {
        Controls { bend: self.bend * self.bend_range + self.tuning, modulation: self.modulation, brightness: self.brightness, resonance: self.resonance }
    }
}

pub struct Synth {
    channels: [Channel; 16],
    voices: Vec<Voice>,
    clock: u64,
    /// GS mode: bank 127 is a GS bank, not XG's drums.
    gs: bool,
    master: f32,
    chorus: Chorus,
    reverb: Reverb,
}

impl Synth {
    pub fn new(rate: u32) -> Synth {
        let rate = rate as f32;
        Synth {
            channels: std::array::from_fn(|i| Channel::new(i == 9)),
            voices: (0..POLYPHONY).map(|i| Voice::new(rate, i as u32 + 1)).collect(),
            clock: 0,
            gs: false,
            master: 1.0,
            chorus: Chorus::new(rate),
            reverb: Reverb::new(rate),
        }
    }

    fn reset(&mut self) {
        self.channels = std::array::from_fn(|i| Channel::new(i == 9));
        self.master = 1.0;
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.release();
        }
    }

    pub fn message(&mut self, message: &Message) {
        match message {
            Message::Channel(status, a, b) => {
                let ch = usize::from(status & 0x0f);
                match status & 0xf0 {
                    0x80 => self.note_off(ch, *a),
                    0x90 if *b == 0 => self.note_off(ch, *a),
                    0x90 => self.note_on(ch, *a, *b),
                    0xb0 => self.control(ch, *a, *b),
                    0xc0 => self.program(ch, *a),
                    0xe0 => {
                        let raw = (i32::from(*b) << 7 | i32::from(*a)) - 8192;
                        self.channels[ch].bend = raw as f32 / 8192.0;
                    }
                    _ => {}
                }
            }
            Message::SysEx(data) => self.sysex(data),
        }
    }

    fn sysex(&mut self, d: &[u8]) {
        match d {
            // GM System On (and GM2's).
            [0x7e, _, 0x09, 0x01 | 0x03, ..] => {
                self.gs = false;
                self.reset();
            }
            // Master volume.
            [0x7f, _, 0x04, 0x01, _, msb, ..] => self.master = f32::from(*msb) / 127.0,
            // Roland GS: a data set to the Sound Canvas.
            [0x41, _, 0x42, 0x12, 0x40, 0x00, 0x7f, 0x00, ..] => {
                self.gs = true;
                self.reset();
            }
            // Use For Rhythm Part: parts 1 to 9 at 1 to 9, part 10 at 0,
            // 11 to 16 at A to F.
            [0x41, _, 0x42, 0x12, 0x40, addr, 0x15, map, ..] if addr & 0xf0 == 0x10 => {
                let part = usize::from(addr & 0x0f);
                let ch = match part {
                    0 => 9,
                    1..=9 => part - 1,
                    _ => part,
                };
                self.channels[ch].drums = *map != 0;
            }
            _ => {}
        }
    }

    fn program(&mut self, ch: usize, program: u8) {
        let c = &mut self.channels[ch];
        c.program = program;
        if c.bank == 127 && !self.gs {
            // XG: bank 127 is the drum kits.
            c.drums = true;
        }
        if c.drums {
            c.kit = drums::kit_for(program);
        } else {
            c.patch = patch(c.bank, program);
        }
    }

    fn control(&mut self, ch: usize, cc: u8, value: u8) {
        let v = f32::from(value) / 127.0;
        let c = &mut self.channels[ch];
        match cc {
            0 => c.bank = value,
            1 => c.modulation = v,
            6 => match c.rpn {
                Some((0, 0)) => c.bend_range = f32::from(value),
                Some((0, 2)) => c.tuning = f32::from(value) - 64.0,
                _ => {}
            },
            38 => {
                if c.rpn == Some((0, 1)) {
                    c.tuning = c.tuning.trunc() + (f32::from(value) - 64.0) / 64.0;
                }
            }
            7 => c.volume = v,
            10 => c.pan = (f32::from(value) - 64.0) / 63.0,
            11 => c.expression = v,
            64 => {
                c.sustain = value >= 64;
                if !c.sustain {
                    for voice in self.voices.iter_mut().filter(|x| x.active && x.channel as usize == ch && x.sustained) {
                        voice.release();
                    }
                }
            }
            71 => c.resonance = ((f32::from(value) - 64.0) / 64.0 * 0.5).max(0.0),
            74 => c.brightness = (f32::from(value) - 64.0) / 64.0 * 2.0,
            91 => c.reverb = v,
            93 => c.chorus = v,
            // An NRPN: what follows isn't for an RPN.
            98 | 99 => c.rpn = None,
            100 => c.rpn = Some((c.rpn.map_or(127, |r| r.0), value)),
            101 => c.rpn = Some((value, c.rpn.map_or(127, |r| r.1))),
            120 => {
                for voice in self.voices.iter_mut().filter(|x| x.active && x.channel as usize == ch) {
                    voice.choke_now();
                }
            }
            121 => c.reset_controllers(),
            123..=127 => {
                for voice in self.voices.iter_mut().filter(|x| x.active && x.channel as usize == ch && !x.released) {
                    voice.note_off(false);
                }
            }
            _ => {}
        }
    }

    fn note_off(&mut self, ch: usize, key: u8) {
        let pedal = self.channels[ch].sustain;
        for v in self.voices.iter_mut().filter(|v| v.active && v.channel as usize == ch && v.key == key && !v.released) {
            v.note_off(pedal);
        }
    }

    fn note_on(&mut self, ch: usize, key: u8, vel: u8) {
        self.clock += 1;
        let c = self.channels[ch];
        if c.drums {
            let Some(drum) = drums::drum(c.kit, key) else { return };
            if drum.choke != 0 {
                for v in self.voices.iter_mut().filter(|v| v.active && v.channel as usize == ch && v.choke == drum.choke) {
                    v.choke_now();
                }
            }
            let i = self.free_voice();
            self.voices[i].start_drum(ch as u8, key, vel, drum, self.clock);
        } else {
            // The same key again ends the last one.
            for v in self.voices.iter_mut().filter(|v| v.active && v.channel as usize == ch && v.key == key && !v.released) {
                v.note_off(false);
            }
            let i = self.free_voice();
            self.voices[i].start(ch as u8, key, vel, c.patch, self.clock);
        }
    }

    /// A voice to play: a silent one, else the quietest of those let go,
    /// else the quietest.
    fn free_voice(&mut self) -> usize {
        if let Some(i) = self.voices.iter().position(|v| !v.active) {
            return i;
        }
        let quietest = |released: bool| {
            self.voices
                .iter()
                .enumerate()
                .filter(|(_, v)| !released || v.released)
                .min_by(|(_, a), (_, b)| a.loudness().total_cmp(&b.loudness()).then(a.started.cmp(&b.started)))
                .map(|(i, _)| i)
        };
        quietest(true).or_else(|| quietest(false)).unwrap_or(0)
    }

    /// Renders into `left` and `right`, the same length.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let mut done = 0;
        while done < left.len() {
            let n = (left.len() - done).min(BLOCK);
            self.render_block(&mut left[done..done + n], &mut right[done..done + n]);
            done += n;
        }
    }

    fn render_block(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len();
        let mut chorus_in = [0f32; BLOCK];
        let mut reverb_in = [0f32; BLOCK];
        let mut one = [0f32; BLOCK];
        left.fill(0.0);
        right.fill(0.0);
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let c = &self.channels[usize::from(v.channel)];
            one[..n].fill(0.0);
            v.render(&mut one[..n], &c.controls());
            let gain = c.volume * c.volume * c.expression * c.expression * 0.25;
            let pan = (c.pan + v.drum_pan().unwrap_or(0.0)).clamp(-1.0, 1.0);
            let angle = (pan + 1.0) * 0.5 * FRAC_PI_2;
            let (gl, gr) = (angle.cos() * gain, angle.sin() * gain);
            let chorus = (c.chorus + v.patch.chorus).min(1.0) * gain;
            let reverb = (c.reverb + v.patch.reverb + v.drum_reverb()).min(1.0) * gain;
            for i in 0..n {
                let s = one[i];
                left[i] += s * gl;
                right[i] += s * gr;
                chorus_in[i] += s * chorus;
                reverb_in[i] += s * reverb;
            }
        }
        self.chorus.process(&chorus_in[..n], left, right);
        self.reverb.process(&reverb_in[..n], left, right);
        if self.master != 1.0 {
            for i in 0..n {
                left[i] *= self.master;
                right[i] *= self.master;
            }
        }
    }

    /// Voices sounding.
    #[cfg(test)]
    fn sounding(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

/// A stereo chorus: one delay line, read at two slowly moving places.
struct Chorus {
    line: Vec<f32>,
    at: usize,
    phase: f32,
    rate: f32,
}

impl Chorus {
    fn new(rate: f32) -> Chorus {
        Chorus { line: vec![0.0; ((rate * 0.03) as usize).next_power_of_two()], at: 0, phase: 0.0, rate }
    }

    fn read(&self, delay: f32) -> f32 {
        let mask = self.line.len() - 1;
        let pos = self.at as f32 - delay;
        let i = pos.floor();
        let f = pos - i;
        let i = i as isize as usize;
        self.line[i & mask] * (1.0 - f) + self.line[i.wrapping_add(1) & mask] * f
    }

    fn process(&mut self, input: &[f32], left: &mut [f32], right: &mut [f32]) {
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
struct Reverb {
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
    fn new(rate: f32) -> Reverb {
        let scale = rate / 44_100.0;
        let len = |n: usize, side: usize| (((n + side * 23) as f32 * scale) as usize).max(1);
        let combs = |side| [1116, 1277, 1356, 1491].map(|n| Comb { line: vec![0.0; len(n, side)], at: 0, store: 0.0 });
        let allpasses = |side| [556, 341].map(|n| Allpass { line: vec![0.0; len(n, side)], at: 0 });
        Reverb { combs: [combs(0), combs(1)], allpasses: [allpasses(0), allpasses(1)] }
    }

    fn process(&mut self, input: &[f32], left: &mut [f32], right: &mut [f32]) {
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

/// A MIDI file played through Neumetik: interleaved stereo at `rate`,
/// as long as the music and then `tail` seconds for the last notes to
/// ring, `longest` seconds at most.
pub fn render(bytes: &[u8], rate: u32, tail: f64, longest: f64) -> Result<Vec<f32>, String> {
    let events = smf::parse(bytes)?;
    let length = events.last().map_or(0.0, |e| e.seconds);
    let frames = ((length + tail).min(longest) * f64::from(rate)) as usize;
    let mut synth = Synth::new(rate);
    let mut out = Vec::with_capacity(frames * 2);
    let (mut left, mut right) = (vec![0f32; 4096], vec![0f32; 4096]);
    let mut done = 0;
    let mut play_to = |synth: &mut Synth, to: usize, out: &mut Vec<f32>| {
        while done < to {
            let n = (to - done).min(left.len());
            synth.render(&mut left[..n], &mut right[..n]);
            for i in 0..n {
                out.push(left[i]);
                out.push(right[i]);
            }
            done += n;
        }
    };
    for e in &events {
        let at = ((e.seconds * f64::from(rate)) as usize).min(frames);
        play_to(&mut synth, at, &mut out);
        if at >= frames {
            break;
        }
        synth.message(&e.message);
    }
    play_to(&mut synth, frames, &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::smf::tests::file;
    use super::*;

    fn loudest(s: &[f32]) -> f32 {
        s.iter().fold(0.0, |m, x| m.max(x.abs()))
    }

    /// Stereo 16-bit WAV at 44.1 kHz, peaking just under full scale.
    fn wav(samples: &[f32]) -> Vec<u8> {
        let gain = 0.89 / loudest(samples).max(1e-6);
        let data: Vec<u8> = samples.iter().flat_map(|s| (((s * gain).clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
        let mut out = Vec::with_capacity(44 + data.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        for field in [16u32.to_le_bytes(), [1, 0, 2, 0], 44_100u32.to_le_bytes(), (44_100u32 * 4).to_le_bytes(), [4, 0, 16, 0]] {
            out.extend_from_slice(&field);
        }
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn every_patch_in_every_bank_sounds_and_stays_sane() {
        let mut checked = 0;
        let banks = BANKS.iter().flat_map(|b| b.patches.iter()).chain(gs::GS.iter().map(|(_, _, p)| p));
        for p in banks {
            let mut synth = Synth::new(22_050);
            synth.channels[0].patch = p;
            synth.note_on(0, 60, 100);
            let (mut l, mut r) = (vec![0.0; 22_050], vec![0.0; 22_050]);
            synth.render(&mut l, &mut r);
            let peak = loudest(&l).max(loudest(&r));
            assert!(peak.is_finite() && peak > 1e-4, "{} is silent ({peak})", p.name);
            assert!(peak < 4.0, "{} is far too loud ({peak})", p.name);
            checked += 1;
        }
        assert!(checked > 300, "{checked}");
    }

    #[test]
    fn a_bank_select_picks_gs_and_classic_patches_and_falls_back_to_gm() {
        assert_eq!(patch(0, 0).name, "Acoustic Grand Piano");
        assert_eq!(patch(8, 4).name, "Detuned EP 1");
        assert_eq!(patch(80, 0).name, "Solid Tine EP");
        assert_eq!(patch(81, 2).name, "Supersaw");
        // No such variation: the capital tone.
        assert_eq!(patch(5, 0).name, "Acoustic Grand Piano");
        // Past a classic bank's end: GM's.
        assert_eq!(patch(82, 127).name, "Gunshot");
    }

    #[test]
    fn gm_has_its_128_by_their_names() {
        assert_eq!(gm::GM[0].name, "Acoustic Grand Piano");
        assert_eq!(gm::GM[40].name, "Violin");
        assert_eq!(gm::GM[127].name, "Gunshot");
    }

    #[test]
    fn a_midi_file_plays() {
        // A C major chord on a piano, a drum hit on channel 10, then the
        // GS reset and a supersaw from bank 81.
        let track = vec![
            0, 0x90, 60, 100, 0, 0x90, 64, 100, 0, 0x90, 67, 100, 0, 0x99, 36, 120, //
            96, 0x80, 60, 0, 0, 0x80, 64, 0, 0, 0x80, 67, 0, //
            0, 0xb1, 0, 81, 0, 0xc1, 2, 0, 0x91, 60, 100, 96, 0x81, 60, 0,
        ];
        let out = render(&file(0, &[track]), 22_050, 1.0, 60.0).unwrap();
        assert_eq!(out.len(), 2 * 22_050 * 2);
        let first = loudest(&out[..22_050]);
        let second = loudest(&out[22_050..44_100]);
        assert!(first > 0.01 && second > 0.01, "{first} {second}");
    }

    #[test]
    fn the_sustain_pedal_holds_notes() {
        let mut synth = Synth::new(22_050);
        synth.control(0, 64, 127);
        synth.note_on(0, 60, 100);
        synth.note_off(0, 60);
        assert!(synth.voices.iter().any(|v| v.active && v.sustained));
        synth.control(0, 64, 0);
        assert!(synth.voices.iter().all(|v| !v.sustained));
    }

    #[test]
    fn the_gs_rhythm_part_message_makes_a_drum_channel() {
        let mut synth = Synth::new(22_050);
        // Part 11 (channel 11, index 10) as drums.
        synth.message(&Message::SysEx(vec![0x41, 0x10, 0x42, 0x12, 0x40, 0x1a, 0x15, 0x02, 0x0f]));
        assert!(synth.channels[10].drums);
        synth.message(&Message::SysEx(vec![0x41, 0x10, 0x42, 0x12, 0x40, 0x00, 0x7f, 0x00, 0x41]));
        assert!(!synth.channels[10].drums && synth.channels[9].drums);
    }

    #[test]
    fn the_bend_range_rpn_is_kept() {
        let mut synth = Synth::new(22_050);
        for (cc, v) in [(101, 0), (100, 0), (6, 12)] {
            synth.control(0, cc, v);
        }
        assert_eq!(synth.channels[0].bend_range, 12.0);
    }

    #[test]
    fn too_many_notes_steal_rather_than_fail() {
        let mut synth = Synth::new(22_050);
        for key in 0..POLYPHONY as u8 + 20 {
            synth.note_on(0, key % 128, 100);
        }
        assert_eq!(synth.sounding(), POLYPHONY);
    }

    /// A tour of the banks to listen to while tuning: each patch plays a
    /// phrase, then the drum kits a beat. Writes a WAV to `OUT`:
    /// `OUT=demo.wav cargo test --release neumetik_demo -- --ignored`.
    #[test]
    #[ignore]
    fn neumetik_demo() {
        let tour: [(u8, u8); 24] = [
            (0, 0), (0, 4), (0, 16), (0, 24), (0, 33), (0, 40), (0, 48), (0, 56), (0, 65), (0, 73), (0, 88), (0, 14),
            (80, 0), (80, 5), (80, 12), (80, 15), (81, 0), (81, 2), (81, 3), (81, 10), (82, 0), (82, 3), (82, 7), (82, 11),
        ];
        let mut t = Vec::new();
        // Ticks to wait before the next event, 96 a quarter note.
        let mut wait = 0u32;
        let ev = |t: &mut Vec<u8>, wait: &mut u32, bytes: &[u8]| {
            let mut d = *wait;
            let mut v = vec![(d & 0x7f) as u8];
            d >>= 7;
            while d > 0 {
                v.insert(0, (d & 0x7f) as u8 | 0x80);
                d >>= 7;
            }
            t.extend(v);
            t.extend_from_slice(bytes);
            *wait = 0;
        };
        for (msb, program) in tour {
            ev(&mut t, &mut wait, &[0xb0, 0, msb]);
            ev(&mut t, &mut wait, &[0xc0, program]);
            let low = if matches!((msb, program), (0, 33) | (80, 12) | (81, 3) | (82, 0) | (82, 7)) { 36 } else { 60 };
            for k in [0, 4, 7, 12] {
                ev(&mut t, &mut wait, &[0x90, low + k, 100]);
                wait = 48;
                ev(&mut t, &mut wait, &[0x80, low + k, 0]);
            }
            for k in [0, 4, 7] {
                ev(&mut t, &mut wait, &[0x90, low + k, 90]);
            }
            wait = 192;
            for k in [0, 4, 7] {
                ev(&mut t, &mut wait, &[0x80, low + k, 0]);
            }
            wait = 96;
        }
        for kit in [0, 25, 30, 40] {
            ev(&mut t, &mut wait, &[0xc9, kit]);
            for step in 0..16u8 {
                let mut hits = vec![42];
                if step % 4 == 0 {
                    hits.push(36);
                }
                if step % 8 == 4 {
                    hits.push(38);
                }
                if step == 15 {
                    hits.extend([49, 39]);
                }
                for h in hits {
                    ev(&mut t, &mut wait, &[0x99, h, 110]);
                }
                wait = 24;
            }
        }
        ev(&mut t, &mut wait, &[0xb0, 123, 0]);
        let out = render(&file(0, &[t]), 44_100, 2.0, 600.0).unwrap();
        assert!(out.iter().all(|s| s.is_finite()));
        if let Ok(path) = std::env::var("OUT") {
            std::fs::write(path, wav(&out)).unwrap();
        }
    }

    /// Prints how much faster than real time a busy piece renders:
    /// `cargo test --release neumetik_speed -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn neumetik_speed() {
        let mut track = Vec::new();
        for bar in 0..64u8 {
            for (ch, program) in [(0u8, 48u8), (1, 0), (2, 81), (3, 33)] {
                track.extend_from_slice(&[0, 0xc0 | ch, program]);
                for k in [48, 52, 55, 60] {
                    track.extend_from_slice(&[0, 0x90 | ch, k + bar % 5, 90]);
                }
            }
            track.extend_from_slice(&[0, 0x99, 36, 100, 0, 0x99, 42, 80]);
            track.extend_from_slice(&[96, 0x80, 0, 0]);
        }
        let start = std::time::Instant::now();
        let out = render(&file(0, &[track]), 44_100, 2.0, 600.0).unwrap();
        let seconds = out.len() as f64 / 2.0 / 44_100.0;
        let took = start.elapsed().as_secs_f64();
        println!("{seconds:.1} s of music in {took:.2} s: {:.0}x real time", seconds / took);
    }
}
