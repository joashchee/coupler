// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! SoundFont 2, from the SoundFont 2.04 specification: `INFO` (the
//! name), `sdta` (the 16-bit samples) and `pdta` (presets, instruments
//! and samples, the zones between them and their generators).
//!
//! Each preset zone and each instrument zone inside it becomes a region:
//! the instrument's generators (its global zone's, then the zone's own)
//! over the defaults, the preset's (likewise) added, key and velocity
//! ranges crossed. Modulators aren't read: the standard's default ones
//! are built in (velocity to level, the mod wheel to vibrato), as most
//! players do. Like E-mu's own hardware, and Polyphone, FluidSynth and
//! the rest after it, the initial attenuation counts at 40%.

use super::riff::{chunks, find, i16_at, name, u16_at, u32_at};
use super::{build, Envelope, Looping, Region, SoundBank, SoundBankKind};
use crate::Error;

const GENERATORS: usize = 61;

// The generators read, by number.
const START: usize = 0;
const END: usize = 1;
const LOOP_START: usize = 2;
const LOOP_END: usize = 3;
const START_COARSE: usize = 4;
const MOD_LFO_PITCH: usize = 5;
const VIB_LFO_PITCH: usize = 6;
const MOD_ENV_PITCH: usize = 7;
const CUTOFF: usize = 8;
const Q: usize = 9;
const MOD_LFO_CUTOFF: usize = 10;
const MOD_ENV_CUTOFF: usize = 11;
const END_COARSE: usize = 12;
const MOD_LFO_VOLUME: usize = 13;
const CHORUS: usize = 15;
const REVERB: usize = 16;
const PAN: usize = 17;
const MOD_LFO_DELAY: usize = 21;
const MOD_LFO_FREQ: usize = 22;
const VIB_LFO_DELAY: usize = 23;
const VIB_LFO_FREQ: usize = 24;
/// The modulation envelope's delay, attack, hold, decay, sustain,
/// release, key to hold and key to decay, one after another; the volume
/// envelope's are the same eight from `VOL_ENV`.
const MOD_ENV: usize = 25;
const VOL_ENV: usize = 33;
const INSTRUMENT: usize = 41;
const KEY_RANGE: usize = 43;
const VEL_RANGE: usize = 44;
const LOOP_START_COARSE: usize = 45;
const KEYNUM: usize = 46;
const VELOCITY: usize = 47;
const ATTENUATION: usize = 48;
const LOOP_END_COARSE: usize = 50;
const COARSE_TUNE: usize = 51;
const FINE_TUNE: usize = 52;
const SAMPLE: usize = 53;
const SAMPLE_MODES: usize = 54;
const SCALE_TUNING: usize = 56;
const EXCLUSIVE: usize = 57;
const ROOT_KEY: usize = 58;

/// Generators a preset zone can't set (they're an instrument's alone).
const INSTRUMENT_ONLY: [usize; 13] = [START, END, LOOP_START, LOOP_END, START_COARSE, END_COARSE, LOOP_START_COARSE, LOOP_END_COARSE, KEYNUM, VELOCITY, SAMPLE_MODES, EXCLUSIVE, ROOT_KEY];

fn defaults() -> [i32; GENERATORS] {
    let mut g = [0; GENERATORS];
    g[CUTOFF] = 13_500;
    for i in [MOD_LFO_DELAY, VIB_LFO_DELAY, MOD_ENV, MOD_ENV + 1, MOD_ENV + 2, MOD_ENV + 3, MOD_ENV + 5, VOL_ENV, VOL_ENV + 1, VOL_ENV + 2, VOL_ENV + 3, VOL_ENV + 5] {
        g[i] = -12_000;
    }
    g[KEYNUM] = -1;
    g[VELOCITY] = -1;
    g[SCALE_TUNING] = 100;
    g[ROOT_KEY] = -1;
    g
}

/// A zone: the generators it sets, and its key and velocity ranges.
#[derive(Clone, Copy)]
struct Zone {
    set: [Option<i32>; GENERATORS],
    keys: (u8, u8),
    vels: (u8, u8),
}

impl Zone {
    fn empty() -> Zone {
        Zone { set: [None; GENERATORS], keys: (0, 127), vels: (0, 127) }
    }

    /// Another zone's settings over this one's (a zone over its global zone).
    fn over(mut self, z: &Zone) -> Zone {
        for (mine, theirs) in self.set.iter_mut().zip(&z.set) {
            if theirs.is_some() {
                *mine = *theirs;
            }
        }
        if z.set[KEY_RANGE].is_some() {
            self.keys = z.keys;
        }
        if z.set[VEL_RANGE].is_some() {
            self.vels = z.vels;
        }
        self
    }
}

/// The zones of each preset or instrument: records of `size` bytes in
/// `headers`, each pointing at its first bag at `bag_at`; bags pointing at
/// their first generator.
fn zones(headers: &[u8], size: usize, bag_at: usize, bags: &[u8], gens: &[u8]) -> Vec<Vec<Zone>> {
    let count = headers.len() / size;
    let bag_count = bags.len() / 4;
    let gen_count = gens.len() / 4;
    let mut out = Vec::with_capacity(count.saturating_sub(1));
    // The last header only marks the end.
    for h in 0..count.saturating_sub(1) {
        let first = usize::from(u16_at(headers, h * size + bag_at));
        let last = usize::from(u16_at(headers, (h + 1) * size + bag_at)).min(bag_count.saturating_sub(1));
        let mut list = Vec::new();
        for b in first..last {
            let g0 = usize::from(u16_at(bags, b * 4));
            let g1 = usize::from(u16_at(bags, (b + 1) * 4)).min(gen_count);
            let mut z = Zone::empty();
            for g in g0..g1.max(g0) {
                let op = usize::from(u16_at(gens, g * 4));
                let at = g * 4 + 2;
                match op {
                    KEY_RANGE => z.keys = (gens[at].min(127), gens[at + 1].min(127)),
                    VEL_RANGE => z.vels = (gens[at].min(127), gens[at + 1].min(127)),
                    _ => {}
                }
                if op < GENERATORS {
                    // The sample and instrument numbers are unsigned.
                    let v = if op == SAMPLE || op == INSTRUMENT { i32::from(u16_at(gens, at)) } else { i32::from(i16_at(gens, at)) };
                    z.set[op] = Some(v);
                }
            }
            list.push(z);
        }
        out.push(list);
    }
    out
}

/// A zone list's global zone, if its first zone is one (it hasn't the
/// generator that ends a real zone), and the real zones.
fn split(list: &[Zone], last: usize) -> (Zone, impl Iterator<Item = &Zone>) {
    let global = list.first().filter(|z| z.set[last].is_none()).copied();
    let skip = usize::from(global.is_some());
    (global.unwrap_or_else(Zone::empty), list.iter().skip(skip).filter(move |z| z.set[last].is_some()))
}

fn cross(a: (u8, u8), b: (u8, u8)) -> Option<(u8, u8)> {
    let lo = a.0.max(b.0);
    let hi = a.1.min(b.1);
    (lo <= hi).then_some((lo, hi))
}

struct SampleHeader {
    start: u32,
    end: u32,
    loop_start: u32,
    loop_end: u32,
    rate: u32,
    pitch: u8,
    correction: i8,
    kind: u16,
}

pub fn read(body: &[u8]) -> Result<SoundBank, Error> {
    let info = find(body, b"INFO").unwrap_or(&[]);
    let bank_name = find(info, b"INAM").map(name).unwrap_or_default();
    let sdta = find(body, b"sdta").ok_or(Error::BadSoundBank)?;
    let pdta = find(body, b"pdta").ok_or(Error::BadSoundBank)?;
    let smpl = find(sdta, b"smpl").unwrap_or(&[]);
    let samples: Vec<i16> = smpl.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();

    let part = |id: &[u8; 4]| chunks(pdta).find(|c| &c.id == id).map(|c| c.data).ok_or(Error::BadSoundBank);
    let (phdr, pbag, pgen) = (part(b"phdr")?, part(b"pbag")?, part(b"pgen")?);
    let (inst, ibag, igen, shdr) = (part(b"inst")?, part(b"ibag")?, part(b"igen")?, part(b"shdr")?);

    let headers: Vec<SampleHeader> = shdr
        .chunks_exact(46)
        .map(|s| SampleHeader {
            start: u32_at(s, 20),
            end: u32_at(s, 24),
            loop_start: u32_at(s, 28),
            loop_end: u32_at(s, 32),
            rate: u32_at(s, 36),
            pitch: s[40],
            correction: s[41] as i8,
            kind: u16_at(s, 44),
        })
        .collect();
    // Ogg Vorbis samples: a SoundFont 3.
    if headers.iter().any(|h| h.kind & 0x10 != 0 && h.kind & 0x8000 == 0) {
        return Err(Error::CompressedSoundFont);
    }

    let instruments = zones(inst, 22, 20, ibag, igen);
    let presets = zones(phdr, 38, 24, pbag, pgen);
    let mut found = Vec::with_capacity(presets.len());
    for (p, list) in presets.iter().enumerate() {
        let h = &phdr[p * 38..];
        let program = u16_at(h, 20);
        let bank = u16_at(h, 22);
        // Bank 128 is the drum kits; others past MIDI's banks aren't for it.
        let drums = bank == 128;
        if program > 127 || (bank > 127 && !drums) {
            continue;
        }
        let (pglobal, pzones) = split(list, INSTRUMENT);
        let mut regions = Vec::new();
        for pz in pzones {
            let pz = pglobal.over(pz);
            let Some(izones) = pz.set[INSTRUMENT].and_then(|i| instruments.get(i as usize)) else { continue };
            let (iglobal, izones) = split(izones, SAMPLE);
            for iz in izones {
                let iz = iglobal.over(iz);
                let (Some(keys), Some(vels)) = (cross(pz.keys, iz.keys), cross(pz.vels, iz.vels)) else { continue };
                let mut g = defaults();
                for (i, v) in iz.set.iter().enumerate() {
                    if let Some(v) = v {
                        g[i] = *v;
                    }
                }
                for (i, v) in pz.set.iter().enumerate() {
                    if let Some(v) = v {
                        if !INSTRUMENT_ONLY.contains(&i) && ![INSTRUMENT, SAMPLE, KEY_RANGE, VEL_RANGE].contains(&i) {
                            g[i] += v;
                        }
                    }
                }
                let Some(s) = headers.get(g[SAMPLE] as usize) else { continue };
                if s.kind & 0x8000 != 0 {
                    // In a sound card's ROM, not the file.
                    continue;
                }
                if let Some(r) = region(&g, s, keys, vels, samples.len()) {
                    regions.push(r);
                }
            }
        }
        found.push((drums, if drums { 0 } else { bank as u8 }, program as u8, name(&h[..20]), regions));
    }
    build(bank_name, SoundBankKind::SoundFont, samples, found)
}

fn envelope(g: &[i32; GENERATORS], at: usize, volume: bool) -> Envelope {
    let tc = |i: usize| g[at + i] as f32;
    Envelope {
        delay: tc(0),
        attack: tc(1),
        hold: tc(2),
        decay: tc(3),
        sustain: if volume { tc(4).clamp(0.0, 1440.0) } else { 1.0 - tc(4).clamp(0.0, 1000.0) / 1000.0 },
        release: tc(5),
        key_hold: tc(6),
        key_decay: tc(7),
        key_center: 60.0,
        vel_attack: 0.0,
        range: if volume { 1000.0 } else { 1.0 },
    }
}

fn region(g: &[i32; GENERATORS], s: &SampleHeader, keys: (u8, u8), vels: (u8, u8), len: usize) -> Option<Region> {
    let len = len as i64;
    let at = |base: u32, fine: usize, coarse: usize| (i64::from(base) + i64::from(g[fine]) + 32_768 * i64::from(g[coarse])).clamp(0, len) as u32;
    let start = at(s.start, START, START_COARSE);
    let end = at(s.end, END, END_COARSE);
    if end <= start + 1 {
        return None;
    }
    let loop_start = at(s.loop_start, LOOP_START, LOOP_START_COARSE).clamp(start, end);
    let loop_end = at(s.loop_end, LOOP_END, LOOP_END_COARSE).clamp(start, end);
    let looping = match g[SAMPLE_MODES] & 3 {
        _ if loop_end <= loop_start + 1 => Looping::Off,
        1 => Looping::On,
        3 => Looping::UntilRelease,
        _ => Looping::Off,
    };
    let root = match g[ROOT_KEY] {
        k @ 0..=127 => k as f32,
        _ if s.pitch <= 127 => f32::from(s.pitch),
        _ => 60.0,
    };
    let fixed = |v: i32| (0..=127).contains(&v).then_some(v as u8);
    let q = (g[Q] as f32 / 10.0).clamp(0.0, 96.0);
    let filtered = g[CUTOFF] < 13_500 || g[MOD_LFO_CUTOFF] != 0 || g[MOD_ENV_CUTOFF] != 0;
    Some(Region {
        keys,
        vels,
        start,
        end,
        loop_start,
        loop_end,
        looping,
        root,
        tune: (g[COARSE_TUNE] * 100 + g[FINE_TUNE] + i32::from(s.correction)) as f32,
        scale: g[SCALE_TUNING] as f32,
        rate: if s.rate == 0 { 44_100.0 } else { s.rate as f32 },
        fixed_key: fixed(g[KEYNUM]),
        fixed_vel: fixed(g[VELOCITY]),
        // The resonance's peak is made up for by turning the level down
        // by half of it, so a resonant filter isn't louder than the rest;
        // as FluidSynth does, even with the filter open.
        attenuation: 0.4 * g[ATTENUATION].clamp(0, 1440) as f32 + 5.0 * q,
        vel_curve: 2.0,
        pan: (g[PAN] as f32 / 500.0).clamp(-1.0, 1.0),
        reverb: (g[REVERB] as f32 / 1000.0).clamp(0.0, 1.0),
        chorus: (g[CHORUS] as f32 / 1000.0).clamp(0.0, 1.0),
        exclusive: g[EXCLUSIVE].clamp(0, 127) as u16,
        vol_env: envelope(g, VOL_ENV, true),
        mod_env: envelope(g, MOD_ENV, false),
        mod_env_pitch: g[MOD_ENV_PITCH] as f32,
        mod_env_cutoff: g[MOD_ENV_CUTOFF] as f32,
        mod_lfo: (g[MOD_LFO_DELAY] as f32, super::hz(g[MOD_LFO_FREQ] as f32)),
        mod_lfo_pitch: g[MOD_LFO_PITCH] as f32,
        mod_lfo_cutoff: g[MOD_LFO_CUTOFF] as f32,
        mod_lfo_volume: g[MOD_LFO_VOLUME] as f32,
        vib_lfo: (g[VIB_LFO_DELAY] as f32, super::hz(g[VIB_LFO_FREQ] as f32)),
        vib_lfo_pitch: g[VIB_LFO_PITCH] as f32,
        wheel_mod_lfo: 0.0,
        wheel_vib_lfo: 50.0,
        cutoff: g[CUTOFF] as f32,
        q,
        filtered,
        vel_cutoff: 0.0,
        key_cutoff: 0.0,
    })
}
