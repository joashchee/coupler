// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Sound banks: SoundFonts (`.sf2`) and DLS files (`.dls`, Downloadable
//! Sounds levels 1 and 2), read into one shape, so the synthesizer plays
//! either the same way. Each instrument becomes a list of regions, a
//! sample each with the keys and velocities it plays and everything that
//! shapes it (envelopes, LFOs, filter, tuning, level, pan), worked out
//! once when the bank is read, so a note only looks its regions up.

mod dls;
#[doc(hidden)]
pub mod make;
mod riff;
mod sf2;
#[cfg(test)]
mod tests;

use crate::Error;

/// Which kind of file a [`SoundBank`] was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundBankKind {
    /// A SoundFont 2 (`.sf2`).
    SoundFont,
    /// Downloadable Sounds, level 1 or 2 (`.dls`).
    Dls,
}

/// An instrument in a [`SoundBank`], as [`SoundBank::instruments`] lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instrument<'a> {
    /// The Bank Select (CC 0) that picks it. Drum kits are on their own,
    /// apart from the banks: `drums` says which.
    pub bank: u8,
    pub program: u8,
    /// A drum kit, played on a drum channel, `program` its kit number.
    pub drums: bool,
    pub name: &'a str,
}

/// A SoundFont or DLS file, read once, ready for a [`Synth`](crate::Synth)
/// to play through: [`Synth::set_sound_bank`](crate::Synth::set_sound_bank).
///
/// Reading it allocates (its samples are kept as 16-bit, about the size
/// of the file); playing through it doesn't. It's shared with an `Arc`,
/// so many synthesizers can play through one bank in memory once.
#[derive(Debug)]
pub struct SoundBank {
    name: String,
    kind: SoundBankKind,
    pub(crate) samples: Vec<i16>,
    presets: Vec<Preset>,
    pub(crate) regions: Vec<Region>,
    /// Each preset's key ([`key`]) and its place in `presets`, by key.
    index: Vec<(u32, usize)>,
}

#[derive(Debug)]
struct Preset {
    bank: u8,
    program: u8,
    drums: bool,
    name: String,
    /// Its regions in the bank's `regions`.
    regions: std::ops::Range<usize>,
}

fn key(drums: bool, bank: u8, program: u8) -> u32 {
    u32::from(drums) << 16 | u32::from(bank) << 8 | u32::from(program)
}

/// How a sample loops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Looping {
    /// It plays once, to its end.
    Off,
    /// Round its loop while it sounds.
    On,
    /// Round its loop while the key's down, then on to its end.
    UntilRelease,
}

/// An envelope: times in timecents (a second is 0, each 1200 doubles it;
/// `f32::NEG_INFINITY` is no time at all), as both formats keep them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Envelope {
    pub delay: f32,
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    /// The volume envelope's sustain is an attenuation in centibels; the
    /// modulation envelope's a level, 0 to 1.
    pub sustain: f32,
    pub release: f32,
    /// Timecents added to the hold and the decay each key below
    /// `key_center` (negative to shorten them, as most pianos do).
    pub key_hold: f32,
    pub key_decay: f32,
    pub key_center: f32,
    /// Timecents added to the attack at full velocity (DLS's only).
    pub vel_attack: f32,
    /// How far the decay and release fall in their time: 1000 cB in a
    /// SoundFont (100 dB), 960 in DLS (96 dB); 1 for a modulation envelope.
    pub range: f32,
}

/// A sample and how it plays: what a note starts a voice for. Every
/// field is ready to use, in the units given.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Region {
    pub keys: (u8, u8),
    pub vels: (u8, u8),
    /// The sample, in the bank's `samples`: start, end (just past it),
    /// and the loop (its end just past it, too).
    pub start: u32,
    pub end: u32,
    pub loop_start: u32,
    pub loop_end: u32,
    pub looping: Looping,
    /// The key the sample sounds at its own rate, the cents it's tuned
    /// by, and the cents a key (100, usually).
    pub root: f32,
    pub tune: f32,
    pub scale: f32,
    pub rate: f32,
    /// Plays every note at this key or velocity, if set (a SoundFont's
    /// `keynum` and `velocity`).
    pub fixed_key: Option<u8>,
    pub fixed_vel: Option<u8>,
    /// Attenuation, in centibels.
    pub attenuation: f32,
    /// The level follows the velocity to this power: 2, (velocity/127)²,
    /// in both formats unless a DLS file says otherwise.
    pub vel_curve: f32,
    /// -1 left to 1 right; the sends 0 to 1.
    pub pan: f32,
    pub reverb: f32,
    pub chorus: f32,
    /// A new note in the same class on the channel cuts the others (an
    /// open hi-hat by a closed one); 0 for none.
    pub exclusive: u16,
    pub vol_env: Envelope,
    pub mod_env: Envelope,
    /// Cents (pitch, filter) and centibels (volume) at the modulation
    /// envelope's or an LFO's peak.
    pub mod_env_pitch: f32,
    pub mod_env_cutoff: f32,
    /// The modulation LFO: its delay (timecents) and rate (Hz).
    pub mod_lfo: (f32, f32),
    pub mod_lfo_pitch: f32,
    pub mod_lfo_cutoff: f32,
    pub mod_lfo_volume: f32,
    /// The vibrato LFO.
    pub vib_lfo: (f32, f32),
    pub vib_lfo_pitch: f32,
    /// Cents of vibrato the mod wheel (CC 1) adds at its top, through
    /// either LFO.
    pub wheel_mod_lfo: f32,
    pub wheel_vib_lfo: f32,
    /// The low-pass filter's cutoff (absolute cents, as `hz` takes) and
    /// resonance (dB), and whether it's needed at all. With everything
    /// that moves it, the cutoff stays 1500 to 13500 cents (20 Hz to 20
    /// kHz), as the SoundFont standard has it (and FluidSynth does it).
    pub cutoff: f32,
    pub q: f32,
    pub filtered: bool,
    /// Cents the cutoff moves at full velocity, and for each key above
    /// 60 (DLS's).
    pub vel_cutoff: f32,
    pub key_cutoff: f32,
}

/// Seconds from timecents, at most 100.
pub(crate) fn seconds(timecents: f32) -> f32 {
    if timecents == f32::NEG_INFINITY {
        0.0
    } else {
        2f32.powf(timecents / 1200.0).min(100.0)
    }
}

/// Hz from absolute cents (8.176 Hz, MIDI key 0, at 0).
pub(crate) fn hz(cents: f32) -> f32 {
    8.175_799 * 2f32.powf(cents / 1200.0)
}

impl SoundBank {
    /// Reads a SoundFont 2 (`.sf2`) or DLS (`.dls`) file, whichever it is.
    /// The bytes are copied; drop them when it returns.
    ///
    /// A compressed SoundFont (`.sf3`, Ogg Vorbis samples) is refused:
    /// [`Error::CompressedSoundFont`]. Samples in a format it can't play
    /// (some DLS files' ADPCM) are left out, silent, rather than failing
    /// the whole bank.
    pub fn from_bytes(bytes: &[u8]) -> Result<SoundBank, Error> {
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" {
            return Err(Error::NotSoundBank);
        }
        let body = &bytes[12..(8 + riff::u32_at(bytes, 4) as usize).clamp(12, bytes.len())];
        match &bytes[8..12] {
            b"sfbk" => sf2::read(body),
            b"DLS " => dls::read(body),
            _ => Err(Error::NotSoundBank),
        }
    }

    /// The bank's name, as the file has it (empty if it hasn't one).
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> SoundBankKind {
        self.kind
    }

    /// Its instruments and drum kits, by bank and program.
    pub fn instruments(&self) -> impl Iterator<Item = Instrument<'_>> {
        self.index.iter().map(|(_, i)| {
            let p = &self.presets[*i];
            Instrument { bank: p.bank, program: p.program, drums: p.drums, name: &p.name }
        })
    }

    /// Roughly how much memory it holds, in bytes.
    pub fn memory_bytes(&self) -> usize {
        std::mem::size_of::<SoundBank>()
            + self.samples.capacity() * 2
            + self.regions.capacity() * std::mem::size_of::<Region>()
            + self.presets.iter().map(|p| std::mem::size_of::<Preset>() + p.name.capacity()).sum::<usize>()
            + self.index.capacity() * std::mem::size_of::<(u32, usize)>()
    }

    fn get(&self, drums: bool, bank: u8, program: u8) -> Option<&Preset> {
        let k = key(drums, bank, program);
        self.index.binary_search_by_key(&k, |(k, _)| *k).ok().map(|i| &self.presets[self.index[i].1])
    }

    /// The preset a channel plays: the bank and program it asks for, else
    /// the same program in bank 0, as the Sound Canvas falls back. A drum
    /// channel's kit falls back to kit 0. `None` if the bank hasn't even
    /// that, and Neumetik's own instrument plays instead; so does a
    /// classic bank (80 to 82) the bank hasn't, rather than its bank 0.
    fn preset(&self, drums: bool, bank: u8, program: u8) -> Option<&Preset> {
        if drums {
            return self.get(true, 0, program).or_else(|| self.get(true, 0, 0));
        }
        self.get(false, bank, program).or_else(|| if (80..=82).contains(&bank) { None } else { self.get(false, 0, program) })
    }

    /// Whether it has something for a channel's bank and program.
    pub(crate) fn has(&self, drums: bool, bank: u8, program: u8) -> bool {
        self.preset(drums, bank, program).is_some()
    }

    /// The name of what a channel's bank and program play, if it has it.
    pub(crate) fn preset_name(&self, drums: bool, bank: u8, program: u8) -> Option<&str> {
        self.preset(drums, bank, program).map(|p| p.name.as_str())
    }

    /// The regions a note plays, by their place in `regions`; `None` if
    /// the bank hasn't the preset at all.
    pub(crate) fn regions_for(&self, drums: bool, bank: u8, program: u8, key: u8, vel: u8) -> Option<impl Iterator<Item = usize> + '_> {
        let p = self.preset(drums, bank, program)?;
        Some(p.regions.clone().filter(move |&i| {
            let r = &self.regions[i];
            (r.keys.0..=r.keys.1).contains(&key) && (r.vels.0..=r.vels.1).contains(&vel)
        }))
    }
}

/// Builds a bank from what a reader found: presets as (drums, bank,
/// program, name, regions). A preset number given twice keeps the first,
/// as players do.
fn build(name: String, kind: SoundBankKind, samples: Vec<i16>, found: Vec<(bool, u8, u8, String, Vec<Region>)>) -> Result<SoundBank, Error> {
    let mut presets = Vec::with_capacity(found.len());
    let mut regions = Vec::new();
    let mut index: Vec<(u32, usize)> = Vec::with_capacity(found.len());
    for (drums, bank, program, pname, rs) in found {
        let k = key(drums, bank, program);
        if index.iter().any(|(x, _)| *x == k) || rs.is_empty() {
            continue;
        }
        let from = regions.len();
        regions.extend(rs);
        index.push((k, presets.len()));
        presets.push(Preset { bank, program, drums, name: pname, regions: from..regions.len() });
    }
    if presets.is_empty() {
        return Err(Error::BadSoundBank);
    }
    index.sort_by_key(|(k, _)| *k);
    Ok(SoundBank { name, kind, samples, presets, regions, index })
}
