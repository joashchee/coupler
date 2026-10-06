// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! What a Neumetik instrument is: a `Patch`, a handful of numbers, and
//! the `const fn` steps the banks build them with
//! (`Patch::fm("Tubular Bells", 3.5, 2.5).amp(0.002, 6.0, 0.0, 1.0)`).
//!
//! A voice is one of three engines, then the same filter, envelopes and
//! LFO:
//! - **Analog**: two oscillators (sine, triangle, saw, square, pulse,
//!   noise or organ drawbars), osc 1 as up to seven detuned copies (the
//!   supersaw), hard sync, noise mixed in. The subtractive synths, from
//!   the Minimoog to Serum's saws.
//! - **Fm**: a two-operator pair, a modulator (with feedback) phase-
//!   modulating a sine carrier, its index falling from a peak to a
//!   sustain. The DX7's pianos, basses, bells and brass.
//! - **Pluck**: Karplus-Strong, a burst of noise ringing round a tuned
//!   delay line. Guitars, harps, basses, sitars, kotos.
//!
//! Any engine can add the **tine**, a second FM pair with its own short
//! decay: a Rhodes' bell, a hammer's knock, an organ's percussion, a
//! slap bass's pop.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Analog,
    Fm,
    Pluck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Triangle,
    Saw,
    Square,
    /// A pulse the patch's `pw` wide.
    Pulse,
    Noise,
    /// Sines at the patch's drawbars (`bars`).
    Organ,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Off,
    /// 12 dB an octave.
    Low,
    /// 24 dB an octave, two in a row, the Moog's and the Juno's slope.
    Low24,
    High,
    Band,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lfo {
    Sine,
    Square,
    /// Falling, a ramp down.
    Saw,
    /// A new random level each cycle.
    Random,
}

/// Seconds, but `s`, the sustain, is a level from 0 to 1. Decay and
/// release fall exponentially; their time is the fall to about -60 dB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Adsr {
    pub a: f32,
    pub d: f32,
    pub s: f32,
    pub r: f32,
}

/// The harmonics an organ's drawbars sound, a Hammond's 16', 8', 5 1/3',
/// 4', 2 2/3', 2' and 1'.
pub const BAR_HARMONICS: [f32; 7] = [0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 8.0];

/// An instrument: its name, and the numbers its engine plays it by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Patch {
    pub name: &'static str,
    /// The instrument and preset it's after, for the classic banks;
    /// empty in GM's and GS's.
    pub(crate) after: &'static str,
    pub(crate) engine: Engine,
    pub(crate) wave1: Wave,
    pub(crate) wave2: Wave,
    /// Osc 2's frequency as a multiple of the note's; Fm's modulator's.
    pub(crate) ratio2: f32,
    /// Osc 2's level (osc 1's is 1).
    pub(crate) mix2: f32,
    /// Cents osc 2 (and the tine) is tuned off.
    pub(crate) detune: f32,
    pub(crate) pw: f32,
    /// Osc 2 hard-synced to osc 1, swept this many semitones up by the
    /// filter envelope.
    pub(crate) sync: Option<f32>,
    /// Copies of osc 1, spread `spread` cents across, for a supersaw.
    pub(crate) unison: u8,
    pub(crate) spread: f32,
    pub(crate) noise: f32,
    pub(crate) bars: [f32; 7],
    /// Fm: the index's peak, how long it falls and where it rests.
    pub(crate) index: f32,
    pub(crate) index_decay: f32,
    pub(crate) index_sus: f32,
    pub(crate) feedback: f32,
    /// The tine: its level, the carrier's and modulator's ratios, its
    /// index and decay.
    pub(crate) tine: f32,
    pub(crate) tine_ratio: f32,
    pub(crate) tine_mod: f32,
    pub(crate) tine_index: f32,
    pub(crate) tine_decay: f32,
    /// Pluck: how bright the string is, 0 dull to 1 bright; it rings for
    /// the amp envelope's decay.
    pub(crate) bright: f32,
    pub(crate) filter: Filter,
    /// Hz, at middle C.
    pub(crate) cutoff: f32,
    /// 0 to 1, 1 short of self-oscillating.
    pub(crate) reso: f32,
    /// Octaves the filter envelope opens the filter.
    pub(crate) env_amt: f32,
    /// How far the cutoff follows the key, 0 to 1.
    pub(crate) key_track: f32,
    /// Octaves the filter closes at the softest velocity.
    pub(crate) vel_filter: f32,
    pub(crate) fenv: Adsr,
    pub(crate) aenv: Adsr,
    /// Semitones the pitch starts above the note, falling over the filter
    /// envelope's decay: toms, zaps, the 808's thump.
    pub(crate) sweep: f32,
    /// How much shorter the decay is up the keyboard (a piano's high
    /// strings die sooner), 0 to 1.
    pub(crate) key_decay: f32,
    pub(crate) lfo: Lfo,
    pub(crate) lfo_rate: f32,
    /// Seconds before the LFO fades in.
    pub(crate) lfo_delay: f32,
    /// Cents.
    pub(crate) vibrato: f32,
    /// 0 to 1.
    pub(crate) tremolo: f32,
    /// How far the LFO sweeps a pulse's width.
    pub(crate) pwm: f32,
    /// Octaves the LFO sweeps the cutoff: the wobble.
    pub(crate) wobble: f32,
    /// Soft clipping, 0 for none: overdrive, distortion.
    pub(crate) drive: f32,
    /// The patch's own chorus and reverb, added to the channel's sends.
    pub(crate) chorus: f32,
    pub(crate) reverb: f32,
    pub(crate) gain: f32,
    /// Semitones.
    pub(crate) transpose: f32,
}

pub const BASE: Patch = Patch {
    name: "",
    after: "",
    engine: Engine::Analog,
    wave1: Wave::Saw,
    wave2: Wave::Saw,
    ratio2: 1.0,
    mix2: 0.0,
    detune: 0.0,
    pw: 0.5,
    sync: None,
    unison: 1,
    spread: 0.0,
    noise: 0.0,
    bars: [0.0; 7],
    index: 0.0,
    index_decay: 1.0,
    index_sus: 1.0,
    feedback: 0.0,
    tine: 0.0,
    tine_ratio: 1.0,
    tine_mod: 1.0,
    tine_index: 0.0,
    tine_decay: 0.3,
    bright: 0.6,
    filter: Filter::Off,
    cutoff: 20_000.0,
    reso: 0.0,
    env_amt: 0.0,
    key_track: 0.5,
    vel_filter: 0.0,
    fenv: Adsr { a: 0.0, d: 1.0, s: 1.0, r: 0.2 },
    aenv: Adsr { a: 0.005, d: 1.0, s: 1.0, r: 0.2 },
    sweep: 0.0,
    key_decay: 0.0,
    lfo: Lfo::Sine,
    lfo_rate: 5.0,
    lfo_delay: 0.0,
    vibrato: 0.0,
    tremolo: 0.0,
    pwm: 0.0,
    wobble: 0.0,
    drive: 0.0,
    chorus: 0.0,
    reverb: 0.0,
    gain: 1.0,
    transpose: 0.0,
};

impl Patch {
    /// Two oscillators, osc 2 silent until `osc2` mixes it in.
    pub const fn analog(name: &'static str, wave: Wave) -> Patch {
        Patch { name, wave1: wave, wave2: wave, ..BASE }
    }

    /// A sine carrier under a sine modulator at `ratio` the note,
    /// `index` deep. Its sound falls away like a struck thing's until
    /// `amp` says otherwise.
    pub const fn fm(name: &'static str, ratio: f32, index: f32) -> Patch {
        Patch {
            name,
            engine: Engine::Fm,
            wave1: Wave::Sine,
            wave2: Wave::Sine,
            ratio2: ratio,
            index,
            index_decay: 1.0,
            index_sus: 0.3,
            aenv: Adsr { a: 0.002, d: 2.0, s: 0.0, r: 0.3 },
            ..BASE
        }
    }

    /// A plucked string ringing `ring` seconds, `bright` 0 to 1.
    pub const fn pluck(name: &'static str, ring: f32, bright: f32) -> Patch {
        Patch {
            name,
            engine: Engine::Pluck,
            bright,
            aenv: Adsr { a: 0.0, d: ring, s: 0.0, r: 0.12 },
            key_decay: 0.5,
            ..BASE
        }
    }

    /// An organ with these drawbars (`BAR_HARMONICS`), 0 to 1.
    pub const fn organ(name: &'static str, bars: [f32; 7]) -> Patch {
        Patch { name, wave1: Wave::Organ, bars, aenv: Adsr { a: 0.004, d: 1.0, s: 1.0, r: 0.06 }, ..BASE }
    }

    /// The real instrument and preset a classic patch is after.
    pub const fn after(mut self, after: &'static str) -> Patch {
        self.after = after;
        self
    }

    pub const fn named(mut self, name: &'static str) -> Patch {
        self.name = name;
        self
    }

    pub const fn osc2(mut self, wave: Wave, mix: f32, detune: f32) -> Patch {
        self.wave2 = wave;
        self.mix2 = mix;
        self.detune = detune;
        self
    }

    pub const fn waves(mut self, wave1: Wave, wave2: Wave) -> Patch {
        self.wave1 = wave1;
        self.wave2 = wave2;
        self
    }

    pub const fn bars(mut self, bars: [f32; 7]) -> Patch {
        self.bars = bars;
        self
    }

    /// The FM index's peak.
    pub const fn index(mut self, index: f32) -> Patch {
        self.index = index;
        self
    }

    /// Cents osc 2 and the tine are tuned off.
    pub const fn detune(mut self, cents: f32) -> Patch {
        self.detune = cents;
        self
    }

    pub const fn ratio(mut self, ratio: f32) -> Patch {
        self.ratio2 = ratio;
        self
    }

    pub const fn pw(mut self, pw: f32) -> Patch {
        self.pw = pw;
        self
    }

    pub const fn sync(mut self, sweep: f32) -> Patch {
        self.sync = Some(sweep);
        self
    }

    pub const fn unison(mut self, voices: u8, spread: f32) -> Patch {
        self.unison = voices;
        self.spread = spread;
        self
    }

    pub const fn noise(mut self, level: f32) -> Patch {
        self.noise = level;
        self
    }

    /// How the FM index falls: over `decay` seconds to `sus` of its peak.
    pub const fn index_env(mut self, decay: f32, sus: f32) -> Patch {
        self.index_decay = decay;
        self.index_sus = sus;
        self
    }

    pub const fn feedback(mut self, amount: f32) -> Patch {
        self.feedback = amount;
        self
    }

    pub const fn tine(mut self, level: f32, ratio: f32, modulator: f32, index: f32, decay: f32) -> Patch {
        self.tine = level;
        self.tine_ratio = ratio;
        self.tine_mod = modulator;
        self.tine_index = index;
        self.tine_decay = decay;
        self
    }

    const fn filter(mut self, kind: Filter, cutoff: f32, reso: f32) -> Patch {
        self.filter = kind;
        self.cutoff = cutoff;
        self.reso = reso;
        self
    }

    pub const fn lp(self, cutoff: f32, reso: f32) -> Patch {
        self.filter(Filter::Low, cutoff, reso)
    }

    pub const fn lp24(self, cutoff: f32, reso: f32) -> Patch {
        self.filter(Filter::Low24, cutoff, reso)
    }

    pub const fn hp(self, cutoff: f32, reso: f32) -> Patch {
        self.filter(Filter::High, cutoff, reso)
    }

    pub const fn bp(self, cutoff: f32, reso: f32) -> Patch {
        self.filter(Filter::Band, cutoff, reso)
    }

    /// The filter envelope, opening the filter `amount` octaves.
    pub const fn fenv(mut self, a: f32, d: f32, s: f32, r: f32, amount: f32) -> Patch {
        self.fenv = Adsr { a, d, s, r };
        self.env_amt = amount;
        self
    }

    pub const fn amp(mut self, a: f32, d: f32, s: f32, r: f32) -> Patch {
        self.aenv = Adsr { a, d, s, r };
        self
    }

    pub const fn key_track(mut self, amount: f32) -> Patch {
        self.key_track = amount;
        self
    }

    pub const fn vel_filter(mut self, octaves: f32) -> Patch {
        self.vel_filter = octaves;
        self
    }

    pub const fn sweep(mut self, semitones: f32) -> Patch {
        self.sweep = semitones;
        self
    }

    pub const fn key_decay(mut self, amount: f32) -> Patch {
        self.key_decay = amount;
        self
    }

    /// The LFO's shape and rate, and its vibrato (cents) and tremolo.
    pub const fn lfo(mut self, shape: Lfo, rate: f32, vibrato: f32, tremolo: f32) -> Patch {
        self.lfo = shape;
        self.lfo_rate = rate;
        self.vibrato = vibrato;
        self.tremolo = tremolo;
        self
    }

    /// A sine vibrato, `cents` deep, fading in after `delay` seconds.
    pub const fn vibrato(mut self, rate: f32, cents: f32, delay: f32) -> Patch {
        self.lfo_rate = rate;
        self.vibrato = cents;
        self.lfo_delay = delay;
        self
    }

    pub const fn pwm(mut self, amount: f32) -> Patch {
        self.pwm = amount;
        self
    }

    pub const fn wobble(mut self, octaves: f32) -> Patch {
        self.wobble = octaves;
        self
    }

    pub const fn drive(mut self, amount: f32) -> Patch {
        self.drive = amount;
        self
    }

    pub const fn fx(mut self, chorus: f32, reverb: f32) -> Patch {
        self.chorus = chorus;
        self.reverb = reverb;
        self
    }

    pub const fn gain(mut self, gain: f32) -> Patch {
        self.gain = gain;
        self
    }

    pub const fn transpose(mut self, semitones: f32) -> Patch {
        self.transpose = semitones;
        self
    }
}
