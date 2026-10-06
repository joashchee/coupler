// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! The synthesizer: 16 MIDI channels, `POLYPHONY` voices, the chorus and
//! the reverb. Driven by MIDI messages, live or from a file, and rendered
//! a buffer at a time.

use std::f32::consts::FRAC_PI_2;

use crate::effects::{Chorus, Reverb};
use crate::output;
use crate::patch::Patch;
use crate::smf::Message;
use crate::voice::{Controls, Voice, BLOCK};
use crate::{drums, gm, patch, POLYPHONY};

/// The lowest and highest sample rates the synthesizer runs at; others
/// are clamped to them.
pub const MIN_RATE: u32 = 8_000;
pub const MAX_RATE: u32 = 192_000;

#[derive(Clone, Copy)]
pub(crate) struct Channel {
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

/// What the channels are set to: everything a MIDI file changes but its
/// notes. Kept apart from the voices so a player can work out where a
/// file stands at any moment (to seek, or to loop) without sounding it.
#[derive(Clone, Copy)]
pub(crate) struct State {
    channels: [Channel; 16],
    /// GS mode: bank 127 is a GS bank, not XG's drums.
    gs: bool,
    master: f32,
}

/// What a message asks of the voices, once the channels have taken it.
enum Effect {
    None,
    NoteOn(usize, u8, u8),
    NoteOff(usize, u8),
    /// The pedal let up: release what it held.
    PedalUp(usize),
    /// All Sound Off (CC 120).
    Cut(usize),
    /// All Notes Off and the modes that imply it (CC 123 to 127).
    Release(usize),
    /// A GM or GS reset: let everything go.
    ReleaseAll,
}

impl State {
    pub(crate) fn new() -> State {
        State { channels: std::array::from_fn(|i| Channel::new(i == 9)), gs: false, master: 1.0 }
    }

    fn reset(&mut self, gs: bool) {
        *self = State::new();
        self.gs = gs;
    }

    /// Takes a message into the channels; says what it asks of the voices.
    fn apply(&mut self, message: &Message) -> Effect {
        match message {
            Message::Channel(status, a, b) => self.channel(*status, *a, *b),
            Message::SysEx(data) => self.sysex(data),
        }
    }

    /// A message for where a file stands, never its notes: for seeking.
    pub(crate) fn chase(&mut self, message: &Message) {
        if let Message::Channel(status, _, _) = message {
            if matches!(status & 0xf0, 0x80 | 0x90) {
                return;
            }
        }
        let _ = self.apply(message);
    }

    fn channel(&mut self, status: u8, a: u8, b: u8) -> Effect {
        let ch = usize::from(status & 0x0f);
        let (a, b) = (a & 0x7f, b & 0x7f);
        match status & 0xf0 {
            0x80 => Effect::NoteOff(ch, a),
            0x90 if b == 0 => Effect::NoteOff(ch, a),
            0x90 => Effect::NoteOn(ch, a, b),
            0xb0 => self.control(ch, a, b),
            0xc0 => {
                self.program(ch, a);
                Effect::None
            }
            0xe0 => {
                let raw = (i32::from(b) << 7 | i32::from(a)) - 8192;
                self.channels[ch].bend = raw as f32 / 8192.0;
                Effect::None
            }
            _ => Effect::None,
        }
    }

    fn sysex(&mut self, d: &[u8]) -> Effect {
        match d {
            // GM System On (and GM2's).
            [0x7e, _, 0x09, 0x01 | 0x03, ..] => {
                self.reset(false);
                Effect::ReleaseAll
            }
            // Master volume.
            [0x7f, _, 0x04, 0x01, _, msb, ..] => {
                self.master = f32::from(*msb) / 127.0;
                Effect::None
            }
            // Roland GS: a data set to the Sound Canvas.
            [0x41, _, 0x42, 0x12, 0x40, 0x00, 0x7f, 0x00, ..] => {
                self.reset(true);
                Effect::ReleaseAll
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
                Effect::None
            }
            _ => Effect::None,
        }
    }

    fn program(&mut self, ch: usize, program: u8) {
        let gs = self.gs;
        let c = &mut self.channels[ch];
        c.program = program;
        if c.bank == 127 && !gs {
            // XG: bank 127 is the drum kits.
            c.drums = true;
        }
        if c.drums {
            c.kit = drums::kit_for(program);
        } else {
            c.patch = patch(c.bank, program);
        }
    }

    fn control(&mut self, ch: usize, cc: u8, value: u8) -> Effect {
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
                    return Effect::PedalUp(ch);
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
            120 => return Effect::Cut(ch),
            121 => c.reset_controllers(),
            123..=127 => return Effect::Release(ch),
            _ => {}
        }
        Effect::None
    }
}

/// The synthesizer. Play it live with [`note_on`](Synth::note_on) and the
/// rest, or feed it a MIDI file's messages; call
/// [`render`](Synth::render) (or [`render_interleaved`](Synth::render_interleaved),
/// [`render_i16`](Synth::render_i16)) for every buffer of sound.
///
/// Channels count from 0: channel 9 (MIDI's channel 10) is the drums.
/// Out-of-range channels, keys and values are ignored or clamped, never a
/// panic. Nothing allocates after [`Synth::new`].
pub struct Synth {
    state: State,
    voices: Vec<Voice>,
    clock: u64,
    rate: u32,
    chorus: Chorus,
    reverb: Reverb,
    muted: [bool; 16],
    /// The player's volume, now and where a fade is taking it, a step a
    /// frame.
    volume: f32,
    fade_to: f32,
    fade_step: f32,
}

impl Synth {
    /// A synthesizer at `sample_rate` frames a second (clamped to
    /// [`MIN_RATE`]..=[`MAX_RATE`]). This allocates its voices and effects,
    /// about 1 MB at 48 kHz, once.
    pub fn new(sample_rate: u32) -> Synth {
        let rate = sample_rate.clamp(MIN_RATE, MAX_RATE);
        let r = rate as f32;
        Synth {
            state: State::new(),
            voices: (0..POLYPHONY).map(|i| Voice::new(r, i as u32 + 1)).collect(),
            clock: 0,
            rate,
            chorus: Chorus::new(r),
            reverb: Reverb::new(r),
            muted: [false; 16],
            volume: 1.0,
            fade_to: 1.0,
            fade_step: 0.0,
        }
    }

    /// The sample rate it runs at.
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Starts a note. A velocity of 0 is a note off, as in MIDI.
    pub fn note_on(&mut self, channel: u8, key: u8, velocity: u8) {
        if channel < 16 {
            self.message(&Message::Channel(0x90 | channel, key, velocity));
        }
    }

    /// Ends a note (it rings on while the sustain pedal is down).
    pub fn note_off(&mut self, channel: u8, key: u8) {
        if channel < 16 {
            self.message(&Message::Channel(0x80 | channel, key, 0));
        }
    }

    /// A control change: 0 bank select, 1 modulation, 7 volume, 10 pan,
    /// 11 expression, 64 sustain, 71 resonance, 74 brightness, 91 reverb,
    /// 93 chorus, 100/101 and 6/38 RPNs (bend range, tuning), 120 all sound
    /// off, 121 reset controllers, 123 all notes off.
    pub fn control_change(&mut self, channel: u8, controller: u8, value: u8) {
        if channel < 16 {
            self.message(&Message::Channel(0xb0 | channel, controller, value));
        }
    }

    /// Picks the instrument a channel plays from its bank (CC 0).
    pub fn program_change(&mut self, channel: u8, program: u8) {
        if channel < 16 {
            self.message(&Message::Channel(0xc0 | channel, program, 0));
        }
    }

    /// Bank select and program change in one: bank 0 is General MIDI, 1 to
    /// 63 the GS variations, 80 to 82 the classic synth banks. On the drum
    /// channel the program picks the kit.
    pub fn set_instrument(&mut self, channel: u8, bank: u8, program: u8) {
        self.control_change(channel, 0, bank);
        self.program_change(channel, program);
    }

    /// Bends a channel's pitch: -8192 to 8191, 0 in tune; by default the
    /// whole range is two semitones each way (RPN 0 changes it).
    pub fn pitch_bend(&mut self, channel: u8, value: i16) {
        if channel < 16 {
            let raw = (i32::from(value).clamp(-8192, 8191) + 8192) as u16;
            self.message(&Message::Channel(0xe0 | channel, (raw & 0x7f) as u8, (raw >> 7) as u8));
        }
    }

    /// Raw MIDI bytes: one message or many, running status allowed, system
    /// exclusive from `F0` to `F7`. What it doesn't use it skips.
    pub fn send(&mut self, bytes: &[u8]) {
        let mut i = 0;
        let mut running = 0u8;
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                0xf0 => {
                    let rest = &bytes[i + 1..];
                    let len = rest.iter().position(|&x| x == 0xf7).unwrap_or(rest.len());
                    self.send_sysex(&rest[..len]);
                    running = 0;
                    i += len + 2;
                    continue;
                }
                0xff => {
                    // System Reset.
                    self.reset();
                    running = 0;
                    i += 1;
                    continue;
                }
                0xf1..=0xfe => {
                    // System common and real-time messages: not for a synth.
                    i += match b {
                        0xf1 | 0xf3 => 2,
                        0xf2 => 3,
                        _ => 1,
                    };
                    if b < 0xf8 {
                        running = 0;
                    }
                    continue;
                }
                _ => {}
            }
            let status = if b >= 0x80 {
                i += 1;
                running = b;
                b
            } else if running != 0 {
                running
            } else {
                i += 1;
                continue;
            };
            let wants = if matches!(status & 0xf0, 0xc0 | 0xd0) { 1 } else { 2 };
            let mut d = [0u8; 2];
            for slot in d.iter_mut().take(wants) {
                match bytes.get(i) {
                    Some(&x) if x < 0x80 => {
                        *slot = x;
                        i += 1;
                    }
                    _ => return,
                }
            }
            self.message(&Message::Channel(status, d[0], d[1]));
        }
    }

    /// Lets every note go, as if every key were lifted and the pedal up.
    pub fn all_notes_off(&mut self) {
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.release();
        }
    }

    /// Silences everything at once, reverb and chorus left to fade.
    pub fn all_sound_off(&mut self) {
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.choke_now();
        }
    }

    /// Back to how it started: every channel's instrument and controllers
    /// as General MIDI resets them, and silence. Mutes and the volume stay.
    pub fn reset(&mut self) {
        self.state = State::new();
        self.all_sound_off();
    }

    /// Mutes or unmutes a channel; its notes play on, unheard, so it comes
    /// back in time. For music in layers that come and go.
    pub fn set_channel_muted(&mut self, channel: u8, muted: bool) {
        if let Some(m) = self.muted.get_mut(usize::from(channel)) {
            *m = muted;
        }
    }

    pub fn channel_muted(&self, channel: u8) -> bool {
        self.muted.get(usize::from(channel)).copied().unwrap_or(false)
    }

    /// The overall volume, 0 to 1 (up to 4 for quiet music), at once.
    pub fn set_volume(&mut self, volume: f32) {
        let v = if volume.is_finite() { volume.clamp(0.0, 4.0) } else { 1.0 };
        self.volume = v;
        self.fade_to = v;
        self.fade_step = 0.0;
    }

    /// The volume now (partway through a fade, where it's got to).
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Fades the volume to `volume` over `seconds`, smoothly.
    pub fn fade_to(&mut self, volume: f32, seconds: f32) {
        let v = if volume.is_finite() { volume.clamp(0.0, 4.0) } else { 1.0 };
        let frames = if seconds.is_finite() { seconds.max(0.0) * self.rate as f32 } else { 0.0 };
        if frames < 1.0 {
            self.set_volume(v);
        } else {
            self.fade_to = v;
            self.fade_step = (v - self.volume) / frames;
        }
    }

    /// Voices sounding now (at most [`POLYPHONY`]).
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// The name of the instrument (or drum kit) a channel plays.
    pub fn instrument_name(&self, channel: u8) -> &'static str {
        let Some(c) = self.state.channels.get(usize::from(channel)) else { return "" };
        if c.drums {
            drums::KITS.iter().find(|(p, _)| *p == c.kit).map_or("Standard", |(_, name)| name)
        } else {
            c.patch.name
        }
    }

    /// Roughly how much memory it holds, in bytes.
    pub fn memory_bytes(&self) -> usize {
        std::mem::size_of::<Synth>() + self.voices.iter().map(Voice::memory_bytes).sum::<usize>() + self.chorus.memory_bytes() + self.reverb.memory_bytes()
    }

    /// Renders the next frames into `left` and `right` (the shorter's
    /// length).
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let mut done = 0;
        while done < n {
            let step = (n - done).min(BLOCK);
            self.render_block(&mut left[done..done + step], &mut right[done..done + step]);
            done += step;
        }
    }

    /// Renders interleaved stereo, left then right: `out.len() / 2` frames.
    pub fn render_interleaved(&mut self, out: &mut [f32]) {
        output::interleaved(out, |l, r| self.render(l, r));
    }

    /// Renders interleaved stereo as 16-bit samples, clipped at full scale.
    pub fn render_i16(&mut self, out: &mut [i16]) {
        output::interleaved_i16(out, |l, r| self.render(l, r));
    }

    pub(crate) fn message(&mut self, message: &Message) {
        let effect = self.state.apply(message);
        self.effect(effect);
    }

    pub(crate) fn send_sysex(&mut self, data: &[u8]) {
        let effect = self.state.sysex(data);
        self.effect(effect);
    }

    /// Sets the channels to `state`: where a file stands.
    pub(crate) fn set_state(&mut self, state: State) {
        self.state = state;
    }

    fn effect(&mut self, effect: Effect) {
        let voices = |ch: usize| move |v: &&mut Voice| v.active && usize::from(v.channel) == ch;
        match effect {
            Effect::None => {}
            Effect::NoteOn(ch, key, vel) => self.note_start(ch, key, vel),
            Effect::NoteOff(ch, key) => {
                let pedal = self.state.channels[ch].sustain;
                for v in self.voices.iter_mut().filter(voices(ch)).filter(|v| v.key == key && !v.released) {
                    v.note_off(pedal);
                }
            }
            Effect::PedalUp(ch) => {
                for v in self.voices.iter_mut().filter(voices(ch)).filter(|v| v.sustained) {
                    v.release();
                }
            }
            Effect::Cut(ch) => {
                for v in self.voices.iter_mut().filter(voices(ch)) {
                    v.choke_now();
                }
            }
            Effect::Release(ch) => {
                for v in self.voices.iter_mut().filter(voices(ch)).filter(|v| !v.released) {
                    v.note_off(false);
                }
            }
            Effect::ReleaseAll => self.all_notes_off(),
        }
    }

    fn note_start(&mut self, ch: usize, key: u8, vel: u8) {
        self.clock += 1;
        let c = self.state.channels[ch];
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

    fn render_block(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len();
        let mut chorus_in = [0f32; BLOCK];
        let mut reverb_in = [0f32; BLOCK];
        let mut one = [0f32; BLOCK];
        left.fill(0.0);
        right.fill(0.0);
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let c = &self.state.channels[usize::from(v.channel)];
            one[..n].fill(0.0);
            v.render(&mut one[..n], &c.controls());
            let mute = if self.muted[usize::from(v.channel)] { 0.0 } else { 1.0 };
            let gain = c.volume * c.volume * c.expression * c.expression * 0.25 * mute;
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
        let master = self.state.master;
        if master != 1.0 || self.volume != 1.0 || self.fade_step != 0.0 {
            for i in 0..n {
                if self.fade_step != 0.0 {
                    self.volume += self.fade_step;
                    if (self.fade_step > 0.0) == (self.volume >= self.fade_to) {
                        self.volume = self.fade_to;
                        self.fade_step = 0.0;
                    }
                }
                let g = master * self.volume;
                left[i] *= g;
                right[i] *= g;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn channel_state(&self, ch: usize) -> (bool, f32, bool) {
        let c = &self.state.channels[ch];
        (c.drums, c.bend_range, c.sustain)
    }

    #[cfg(test)]
    pub(crate) fn voices(&self) -> &[Voice] {
        &self.voices
    }

    #[cfg(test)]
    pub(crate) fn play_patch(&mut self, p: &'static Patch, key: u8) {
        self.state.channels[0].patch = p;
        self.note_on(0, key, 100);
    }
}
