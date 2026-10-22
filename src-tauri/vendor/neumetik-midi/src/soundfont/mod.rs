// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! SoundFonts to make, change and save: a SoundFont 2 as its parts
//! (samples, instruments, presets, and the zones and generators between
//! them), written out as an `.sf2` any SoundFont player reads.
//!
//! - [`SoundFont::new`] starts an empty one; add [`Sample`]s (from your
//!   own data, or a WAV file's with [`Sample::from_wav`]), [`Instrument`]s
//!   whose zones play them, and [`Preset`]s whose zones play those.
//! - [`SoundFont::from_bytes`] opens an `.sf2` as it is, every generator
//!   and modulator kept, or a DLS bank (`.dls`) turned into one.
//! - [`SoundFont::neumetik`] packs Neumetik's own instruments and drum
//!   kits, sampled, so any SoundFont player sounds like Neumetik.
//! - [`SoundFont::trim_to_midi`] keeps only what a song plays, and
//!   [`SoundFont::trim`] the presets you choose.
//! - [`SoundFont::to_bytes`] writes it: SoundFont 2.04, 16-bit.
//!
//! [`SoundBank::from_bytes`](crate::SoundBank::from_bytes) plays what it
//! writes.
//!
//! Written from the SoundFont 2.04 specification (E-mu, 2006).

mod convert;
mod pack;
mod read;
#[cfg(test)]
mod tests;
mod trim;
mod wav;
mod write;

pub use pack::{Pack, PackSet};

use crate::Error;

/// The generators, by the numbers the specification gives them: what a
/// [`Zone`] sets with [`Zone::set`]. Times are timecents (1200 × log₂
/// seconds; -12000 is none), pitches cents, levels centibels, sends and
/// the modulation envelope's sustain tenths of a percent, pan -500 (left)
/// to 500 (right), frequencies absolute cents (8.176 Hz at 0).
pub mod generator {
    pub const START_OFFSET: u16 = 0;
    pub const END_OFFSET: u16 = 1;
    pub const LOOP_START_OFFSET: u16 = 2;
    pub const LOOP_END_OFFSET: u16 = 3;
    pub const START_COARSE_OFFSET: u16 = 4;
    pub const MOD_LFO_TO_PITCH: u16 = 5;
    pub const VIB_LFO_TO_PITCH: u16 = 6;
    pub const MOD_ENV_TO_PITCH: u16 = 7;
    pub const FILTER_CUTOFF: u16 = 8;
    pub const FILTER_Q: u16 = 9;
    pub const MOD_LFO_TO_CUTOFF: u16 = 10;
    pub const MOD_ENV_TO_CUTOFF: u16 = 11;
    pub const END_COARSE_OFFSET: u16 = 12;
    pub const MOD_LFO_TO_VOLUME: u16 = 13;
    pub const CHORUS_SEND: u16 = 15;
    pub const REVERB_SEND: u16 = 16;
    pub const PAN: u16 = 17;
    pub const MOD_LFO_DELAY: u16 = 21;
    pub const MOD_LFO_FREQ: u16 = 22;
    pub const VIB_LFO_DELAY: u16 = 23;
    pub const VIB_LFO_FREQ: u16 = 24;
    pub const MOD_ENV_DELAY: u16 = 25;
    pub const MOD_ENV_ATTACK: u16 = 26;
    pub const MOD_ENV_HOLD: u16 = 27;
    pub const MOD_ENV_DECAY: u16 = 28;
    pub const MOD_ENV_SUSTAIN: u16 = 29;
    pub const MOD_ENV_RELEASE: u16 = 30;
    pub const KEY_TO_MOD_ENV_HOLD: u16 = 31;
    pub const KEY_TO_MOD_ENV_DECAY: u16 = 32;
    pub const VOL_ENV_DELAY: u16 = 33;
    pub const VOL_ENV_ATTACK: u16 = 34;
    pub const VOL_ENV_HOLD: u16 = 35;
    pub const VOL_ENV_DECAY: u16 = 36;
    pub const VOL_ENV_SUSTAIN: u16 = 37;
    pub const VOL_ENV_RELEASE: u16 = 38;
    pub const KEY_TO_VOL_ENV_HOLD: u16 = 39;
    pub const KEY_TO_VOL_ENV_DECAY: u16 = 40;
    /// A preset zone's instrument: [`Zone::target`](super::Zone::target).
    pub const INSTRUMENT: u16 = 41;
    /// [`Zone::keys`](super::Zone::keys).
    pub const KEY_RANGE: u16 = 43;
    /// [`Zone::vels`](super::Zone::vels).
    pub const VEL_RANGE: u16 = 44;
    pub const LOOP_START_COARSE_OFFSET: u16 = 45;
    /// Plays every note at this key.
    pub const KEYNUM: u16 = 46;
    /// Plays every note at this velocity.
    pub const VELOCITY: u16 = 47;
    pub const ATTENUATION: u16 = 48;
    pub const LOOP_END_COARSE_OFFSET: u16 = 50;
    pub const COARSE_TUNE: u16 = 51;
    pub const FINE_TUNE: u16 = 52;
    /// An instrument zone's sample: [`Zone::target`](super::Zone::target).
    pub const SAMPLE: u16 = 53;
    /// 0 plays once, 1 loops, 3 loops until the key's let go.
    pub const SAMPLE_MODES: u16 = 54;
    pub const SCALE_TUNING: u16 = 56;
    pub const EXCLUSIVE_CLASS: u16 = 57;
    pub const ROOT_KEY: u16 = 58;
}

/// A SoundFont, as its parts. Change any of them freely: [`to_bytes`]
/// checks it all fits together when it writes it.
///
/// [`to_bytes`]: SoundFont::to_bytes
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SoundFont {
    pub info: Info,
    pub samples: Vec<Sample>,
    pub instruments: Vec<Instrument>,
    pub presets: Vec<Preset>,
}

/// What a SoundFont says about itself (its `INFO`). Empty strings aren't
/// written.
#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub name: String,
    /// The sound engine it's for: "EMU8000" unless it's for something
    /// else.
    pub engine: String,
    pub date: String,
    /// Who made it.
    pub author: String,
    /// The product it's for.
    pub product: String,
    pub copyright: String,
    pub comment: String,
    /// The tools it was made and changed with.
    pub software: String,
}

impl Default for Info {
    fn default() -> Info {
        Info {
            name: String::new(),
            engine: "EMU8000".into(),
            date: String::new(),
            author: String::new(),
            product: String::new(),
            copyright: String::new(),
            comment: String::new(),
            software: String::new(),
        }
    }
}

/// A sample: 16-bit mono, at its own rate, with the key it sounds at and
/// its loop (`loop_start` to just before `loop_end`, in its own data;
/// both 0 if it hasn't one).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub name: String,
    pub data: Vec<i16>,
    pub rate: u32,
    /// The key it sounds at its own rate, and how many cents it's off.
    pub root: u8,
    pub correction: i8,
    pub loop_start: u32,
    pub loop_end: u32,
    /// The other half of a stereo pair.
    pub link: Link,
}

/// Which side of a stereo pair a sample is, and where its other half is
/// in [`SoundFont::samples`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Mono,
    Left(usize),
    Right(usize),
}

impl Sample {
    /// A mono sample without a loop.
    pub fn new(name: &str, data: Vec<i16>, rate: u32, root: u8) -> Sample {
        Sample { name: name.into(), data, rate, root, correction: 0, loop_start: 0, loop_end: 0, link: Link::Mono }
    }

    /// The same, looping from `start` to just before `end`.
    pub fn looped(mut self, start: u32, end: u32) -> Sample {
        self.loop_start = start;
        self.loop_end = end;
        self
    }

    /// Whether it has a loop.
    pub fn has_loop(&self) -> bool {
        self.loop_end > self.loop_start
    }
}

/// A modulator, as the file has it: a source (and an amount source)
/// moving a generator `amount` at its full range. See the specification's
/// section 8.2 for what the numbers mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modulator {
    pub source: u16,
    pub destination: u16,
    pub amount: i16,
    pub amount_source: u16,
    pub transform: u16,
}

/// A zone: the keys and velocities it plays, its generators, its
/// modulators, and what it plays (an instrument zone a sample, a preset
/// zone an instrument, by their place in the SoundFont's lists).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Zone {
    pub keys: Option<(u8, u8)>,
    pub vels: Option<(u8, u8)>,
    /// Every generator but the ranges and the target, once each.
    pub generators: Vec<(u16, i16)>,
    pub modulators: Vec<Modulator>,
    pub target: Option<usize>,
}

impl Zone {
    /// A zone that plays `target`, everywhere.
    pub fn new(target: usize) -> Zone {
        Zone { target: Some(target), ..Zone::default() }
    }

    /// The same, on keys `lo` to `hi`.
    pub fn keys(mut self, lo: u8, hi: u8) -> Zone {
        self.keys = Some((lo.min(hi), hi.max(lo)));
        self
    }

    /// The same, at velocities `lo` to `hi`.
    pub fn vels(mut self, lo: u8, hi: u8) -> Zone {
        self.vels = Some((lo.min(hi), hi.max(lo)));
        self
    }

    /// The same, a generator set.
    pub fn with(mut self, generator: u16, amount: i16) -> Zone {
        self.set(generator, amount);
        self
    }

    /// Sets a generator (replacing it if it's set). The ranges and target
    /// have fields of their own and are kept there.
    pub fn set(&mut self, generator: u16, amount: i16) {
        use generator::{INSTRUMENT, KEY_RANGE, SAMPLE, VEL_RANGE};
        let range = |a: i16| {
            let [lo, hi] = a.to_le_bytes();
            Some((lo.min(127), hi.min(127)))
        };
        match generator {
            KEY_RANGE => self.keys = range(amount),
            VEL_RANGE => self.vels = range(amount),
            INSTRUMENT | SAMPLE => self.target = Some(amount as u16 as usize),
            _ => match self.generators.iter_mut().find(|(g, _)| *g == generator) {
                Some(g) => g.1 = amount,
                None => self.generators.push((generator, amount)),
            },
        }
    }

    /// A generator's amount, if it's set.
    pub fn get(&self, generator: u16) -> Option<i16> {
        self.generators.iter().find(|(g, _)| *g == generator).map(|(_, a)| *a)
    }

    pub fn remove(&mut self, generator: u16) {
        self.generators.retain(|(g, _)| *g != generator);
    }

    /// Whether it sets nothing at all (a global zone that isn't there).
    pub fn is_empty(&self) -> bool {
        self.keys.is_none() && self.vels.is_none() && self.generators.is_empty() && self.modulators.is_empty() && self.target.is_none()
    }

    /// Whether a note at `key` and `vel` falls in it.
    pub fn covers(&self, key: u8, vel: u8) -> bool {
        let (k, v) = (self.keys.unwrap_or((0, 127)), self.vels.unwrap_or((0, 127)));
        (k.0..=k.1).contains(&key) && (v.0..=v.1).contains(&vel)
    }
}

/// An instrument: zones of samples, and a global zone of what they share
/// (empty if they share nothing).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Instrument {
    pub name: String,
    pub global: Zone,
    pub zones: Vec<Zone>,
}

impl Instrument {
    pub fn new(name: &str) -> Instrument {
        Instrument { name: name.into(), ..Instrument::default() }
    }
}

/// A preset, what a bank and program pick: zones of instruments. Bank 128
/// is the drum kits, `program` the kit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preset {
    pub name: String,
    pub bank: u16,
    pub program: u16,
    pub global: Zone,
    pub zones: Vec<Zone>,
}

impl Preset {
    /// A preset playing one instrument everywhere.
    pub fn new(name: &str, bank: u16, program: u16, instrument: usize) -> Preset {
        Preset { name: name.into(), bank, program, global: Zone::default(), zones: vec![Zone::new(instrument)] }
    }

    /// Whether it's a drum kit.
    pub fn is_drums(&self) -> bool {
        self.bank == 128
    }
}

impl SoundFont {
    /// An empty SoundFont named `name`.
    pub fn new(name: &str) -> SoundFont {
        SoundFont { info: Info { name: name.into(), ..Info::default() }, ..SoundFont::default() }
    }

    /// Opens a SoundFont 2 (`.sf2`) as it is, or a DLS bank (`.dls`)
    /// turned into one (as Neumetik plays it: its regions' articulation
    /// as generators).
    ///
    /// Samples a SoundFont keeps in a sound card's ROM aren't in the file
    /// and are left out, with the zones that play them; 24-bit samples
    /// keep their top 16 bits. A compressed SoundFont (`.sf3`) is
    /// refused: [`Error::CompressedSoundFont`].
    pub fn from_bytes(bytes: &[u8]) -> Result<SoundFont, Error> {
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" {
            return Err(Error::NotSoundBank);
        }
        match &bytes[8..12] {
            b"sfbk" => read::read(bytes),
            b"DLS " => Ok(convert::from_bank(&crate::SoundBank::from_bytes(bytes)?)),
            _ => Err(Error::NotSoundBank),
        }
    }

    /// Writes it: a SoundFont 2.04 file.
    ///
    /// [`Error::Unwritable`] if a zone plays a sample or instrument it
    /// hasn't, a stereo sample's other half isn't there, a loop is outside
    /// its sample, or it's past what the format holds (65,535 zones,
    /// generators or modulators of each kind, 4 GB of samples).
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        write::write(self)
    }

    /// Adds a sample; its place in [`samples`](SoundFont::samples).
    pub fn add_sample(&mut self, sample: Sample) -> usize {
        self.samples.push(sample);
        self.samples.len() - 1
    }

    /// Adds a left and right pair, linked; their places.
    pub fn add_stereo(&mut self, mut left: Sample, mut right: Sample) -> (usize, usize) {
        let l = self.samples.len();
        left.link = Link::Left(l + 1);
        right.link = Link::Right(l);
        self.samples.push(left);
        self.samples.push(right);
        (l, l + 1)
    }

    pub fn add_instrument(&mut self, instrument: Instrument) -> usize {
        self.instruments.push(instrument);
        self.instruments.len() - 1
    }

    pub fn add_preset(&mut self, preset: Preset) -> usize {
        self.presets.push(preset);
        self.presets.len() - 1
    }

    /// The preset a bank and program pick, if it has it.
    pub fn preset(&self, bank: u16, program: u16) -> Option<&Preset> {
        self.presets.iter().find(|p| p.bank == bank && p.program == program)
    }

    /// The bytes its samples take.
    pub fn sample_bytes(&self) -> usize {
        self.samples.iter().map(|s| s.data.len() * 2).sum()
    }
}
