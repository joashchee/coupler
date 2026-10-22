// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Downloadable Sounds, levels 1 and 2 (the MMA's DLS specifications):
//! `lins`, the instruments (each `ins ` a header, regions and
//! articulation), `ptbl`, the pool table, and `wvpl`, the wave pool of
//! WAV-like samples.
//!
//! An articulation is a list of connections (a source, maybe a control,
//! a destination, a scale), DLS's modulators. The ones a General MIDI
//! bank uses are read into the same region a SoundFont's become: the
//! envelopes (EG1 the volume, EG2 the modulation), the LFOs (DLS 1's one
//! and DLS 2's vibrato), the filter, tuning, level, pan and sends, and
//! how velocity and key move them. A region's own articulation goes over
//! its instrument's. Controller connections (volume, expression, pan,
//! pitch bend) are the synthesizer's own, as General MIDI has them, and
//! aren't read.

use super::riff::{chunks, find, i16_at, i32_at, name, u16_at, u32_at};
use super::{build, Envelope, Looping, Region, SoundBank, SoundBankKind};
use crate::Error;

// Sources.
const SRC_NONE: u16 = 0x0000;
const SRC_LFO: u16 = 0x0001;
const SRC_VELOCITY: u16 = 0x0002;
const SRC_KEY: u16 = 0x0003;
const SRC_EG2: u16 = 0x0005;
const SRC_VIBRATO: u16 = 0x0009;
const SRC_CC1: u16 = 0x0081;

// Destinations.
const DST_GAIN: u16 = 0x0001;
const DST_PITCH: u16 = 0x0003;
const DST_PAN: u16 = 0x0004;
const DST_CHORUS: u16 = 0x0080;
const DST_REVERB: u16 = 0x0081;
const DST_LFO_FREQ: u16 = 0x0104;
const DST_LFO_DELAY: u16 = 0x0105;
const DST_VIB_FREQ: u16 = 0x0114;
const DST_VIB_DELAY: u16 = 0x0115;
/// EG1's attack, decay, (reserved), release, sustain, delay and hold; EG2's
/// are the same, from `EG2`.
const EG1: u16 = 0x0206;
const EG2: u16 = 0x030a;
const DST_CUTOFF: u16 = 0x0500;
const DST_Q: u16 = 0x0501;

/// No time at all, as an absolute timecent value.
const NO_TIME: i32 = i32::MIN;
/// The filter's cutoff when it's off.
const FILTER_OFF: i32 = 0x7fff_ffff;

/// A DLS envelope as it starts, before the articulation: no times, a full
/// sustain.
fn envelope(volume: bool) -> Envelope {
    let none = f32::NEG_INFINITY;
    Envelope {
        delay: none,
        attack: none,
        hold: none,
        decay: none,
        sustain: if volume { 0.0 } else { 1.0 },
        release: none,
        key_hold: 0.0,
        key_decay: 0.0,
        key_center: 0.0,
        vel_attack: 0.0,
        range: if volume { 960.0 } else { 1.0 },
    }
}

/// What a region starts as: DLS's defaults.
fn defaults() -> Region {
    // An LFO at 5 Hz after 10 ms.
    let lfo = (1200.0 * 0.01f32.log2(), 5.0);
    Region {
        keys: (0, 127),
        vels: (0, 127),
        start: 0,
        end: 0,
        loop_start: 0,
        loop_end: 0,
        looping: Looping::Off,
        root: 60.0,
        tune: 0.0,
        scale: 100.0,
        rate: 22_050.0,
        fixed_key: None,
        fixed_vel: None,
        attenuation: 0.0,
        vel_curve: 2.0,
        pan: 0.0,
        reverb: 0.0,
        chorus: 0.0,
        exclusive: 0,
        vol_env: envelope(true),
        mod_env: envelope(false),
        mod_env_pitch: 0.0,
        mod_env_cutoff: 0.0,
        mod_lfo: lfo,
        mod_lfo_pitch: 0.0,
        mod_lfo_cutoff: 0.0,
        mod_lfo_volume: 0.0,
        vib_lfo: lfo,
        vib_lfo_pitch: 0.0,
        wheel_mod_lfo: 50.0,
        wheel_vib_lfo: 0.0,
        cutoff: 13_500.0,
        q: 0.0,
        filtered: false,
        vel_cutoff: 0.0,
        key_cutoff: 0.0,
    }
}

/// An articulation's connections, applied to a region; a region's go over
/// its instrument's, each replacing the same connection's. `gain` is the
/// level it sets, in centibels, apart from the sample's own.
fn articulate(r: &mut Region, gain: &mut f32, art: &[u8]) {
    let size = u32_at(art, 0) as usize;
    let count = u32_at(art, 4) as usize;
    for i in 0..count {
        let at = size + i * 12;
        if at + 12 > art.len() {
            break;
        }
        let (src, ctl, dst) = (u16_at(art, at), u16_at(art, at + 2), u16_at(art, at + 4));
        let raw = i32_at(art, at + 8);
        let v = raw as f32 / 65_536.0;
        // An absolute time, which may be none at all.
        let tc = if raw == NO_TIME { f32::NEG_INFINITY } else { v };
        let eg = |base: u16| dst.checked_sub(base).filter(|d| *d <= 6);
        match (src, ctl, dst) {
            (SRC_NONE, SRC_NONE, DST_GAIN) => *gain = v,
            (SRC_NONE, SRC_NONE, DST_PITCH) => r.tune += v,
            (SRC_NONE, SRC_NONE, DST_PAN) => r.pan = (v / 500.0).clamp(-1.0, 1.0),
            (SRC_NONE, SRC_NONE, DST_CHORUS) => r.chorus = (v / 1000.0).clamp(0.0, 1.0),
            (SRC_NONE, SRC_NONE, DST_REVERB) => r.reverb = (v / 1000.0).clamp(0.0, 1.0),
            (SRC_NONE, SRC_NONE, DST_LFO_FREQ) => r.mod_lfo.1 = super::hz(v).clamp(0.01, 100.0),
            (SRC_NONE, SRC_NONE, DST_LFO_DELAY) => r.mod_lfo.0 = tc,
            (SRC_NONE, SRC_NONE, DST_VIB_FREQ) => r.vib_lfo.1 = super::hz(v).clamp(0.01, 100.0),
            (SRC_NONE, SRC_NONE, DST_VIB_DELAY) => r.vib_lfo.0 = tc,
            (SRC_NONE, SRC_NONE, DST_CUTOFF) => {
                r.filtered = raw != FILTER_OFF;
                r.cutoff = if r.filtered { v } else { 13_500.0 };
            }
            (SRC_NONE, SRC_NONE, DST_Q) => r.q = (v / 10.0).clamp(0.0, 96.0),
            (SRC_NONE, SRC_NONE, _) if eg(EG1).is_some() || eg(EG2).is_some() => {
                let (env, d) = match eg(EG1) {
                    Some(d) => (&mut r.vol_env, d),
                    None => (&mut r.mod_env, eg(EG2).unwrap_or(0)),
                };
                let volume = env.range > 1.0;
                match d {
                    0 => env.attack = tc,
                    1 => env.decay = tc,
                    3 => env.release = tc,
                    // In tenths of a percent of the peak; the volume
                    // envelope's as the attenuation it comes to. Some
                    // banks (Apple's QuickTime one among them) leave the
                    // level unscaled, 1000 for 100%: no bank means a
                    // sustain of a thousandth of a percent, so a value
                    // that small is read that way.
                    4 => {
                        let tenths = if (1..=1000).contains(&raw) { raw as f32 } else { v };
                        let level = (tenths / 1000.0).clamp(0.0, 1.0);
                        env.sustain = if volume { (1.0 - level) * 960.0 } else { level };
                    }
                    5 => env.delay = tc,
                    6 => env.hold = tc,
                    _ => {}
                }
            }
            (SRC_LFO, SRC_NONE, DST_PITCH) => r.mod_lfo_pitch = v,
            (SRC_LFO, SRC_CC1, DST_PITCH) => r.wheel_mod_lfo = v,
            (SRC_LFO, SRC_NONE, DST_GAIN) => r.mod_lfo_volume = v,
            (SRC_LFO, SRC_NONE, DST_CUTOFF) => r.mod_lfo_cutoff = v,
            (SRC_VIBRATO, SRC_NONE, DST_PITCH) => r.vib_lfo_pitch = v,
            (SRC_VIBRATO, SRC_CC1, DST_PITCH) => r.wheel_vib_lfo = v,
            (SRC_EG2, SRC_NONE, DST_PITCH) => r.mod_env_pitch = v,
            (SRC_EG2, SRC_NONE, DST_CUTOFF) => r.mod_env_cutoff = v,
            (SRC_VELOCITY, SRC_NONE, DST_GAIN) => r.vel_curve = (v.abs() / 480.0).min(8.0),
            (SRC_VELOCITY, SRC_NONE, DST_CUTOFF) => r.vel_cutoff = v,
            (SRC_KEY, SRC_NONE, DST_PITCH) => r.scale = v / 128.0,
            (SRC_KEY, SRC_NONE, DST_CUTOFF) => r.key_cutoff = v / 128.0,
            (SRC_VELOCITY | SRC_KEY, SRC_NONE, _) if eg(EG1).is_some() || eg(EG2).is_some() => {
                let (env, d) = match eg(EG1) {
                    Some(d) => (&mut r.vol_env, d),
                    None => (&mut r.mod_env, eg(EG2).unwrap_or(0)),
                };
                // Timecents a whole range of velocity or keys adds; per
                // key down from key 0, as the envelope keeps it.
                match (src, d) {
                    (SRC_VELOCITY, 0) => env.vel_attack = v,
                    (SRC_KEY, 1) => env.key_decay = -v / 128.0,
                    (SRC_KEY, 6) => env.key_hold = -v / 128.0,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// Each `art1` or `art2` in the lists of articulations (`lart`, `lar2`)
/// in an instrument or region.
fn articulations(parts: &[u8]) -> impl Iterator<Item = &[u8]> {
    chunks(parts)
        .filter_map(|c| match c.list() {
            Some((kind, body)) if &kind == b"lart" || &kind == b"lar2" => Some(body),
            _ => None,
        })
        .flat_map(|list| chunks(list).filter(|c| &c.id == b"art1" || &c.id == b"art2").map(|c| c.data))
}

/// A sample's playing details (`wsmp`): its unity note, fine tune, gain
/// and loop.
#[derive(Clone, Copy)]
struct Wsmp {
    unity: u16,
    fine: i16,
    gain: f32,
    looping: Option<(Looping, u32, u32)>,
}

fn wsmp(d: &[u8]) -> Wsmp {
    let size = u32_at(d, 0) as usize;
    let looping = (u32_at(d, 16) > 0 && d.len() >= size + 16).then(|| {
        let at = size;
        let kind = if u32_at(d, at + 4) == 1 { Looping::UntilRelease } else { Looping::On };
        (kind, u32_at(d, at + 8), u32_at(d, at + 12))
    });
    Wsmp { unity: u16_at(d, 4), fine: i16_at(d, 6), gain: i32_at(d, 8) as f32 / 65_536.0, looping }
}

/// A wave, decoded: where it is in the samples, its rate and details.
struct Wave {
    start: u32,
    len: u32,
    rate: u32,
    wsmp: Option<Wsmp>,
}

/// Decodes a wave's `data` to 16-bit mono at the end of `samples`: PCM of
/// 8, 16, 24 or 32 bits, or 32-bit float, channels mixed. `None` for a
/// format it doesn't play.
fn decode(fmt: &[u8], data: &[u8], samples: &mut Vec<i16>) -> Option<u32> {
    let mut format = u16_at(fmt, 0);
    let channels = usize::from(u16_at(fmt, 2).max(1));
    let bits = u16_at(fmt, 14);
    if format == 0xfffe && fmt.len() >= 26 {
        // WAVE_FORMAT_EXTENSIBLE: the format is its subformat's.
        format = u16_at(fmt, 24);
    }
    let width = match (format, bits) {
        (1, 8) => 1,
        (1, 16) => 2,
        (1, 24) => 3,
        (1, 32) | (3, 32) => 4,
        _ => return None,
    };
    let frame = width * channels;
    let before = samples.len();
    for f in data.chunks_exact(frame) {
        let mut sum = 0f32;
        for c in f.chunks_exact(width) {
            sum += match (format, width) {
                (1, 1) => (f32::from(c[0]) - 128.0) / 128.0,
                (1, 2) => f32::from(i16::from_le_bytes([c[0], c[1]])) / 32_768.0,
                (1, 3) => (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0,
                (1, _) => i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0,
                _ => f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
            };
        }
        let s = sum / channels as f32;
        samples.push((s.clamp(-1.0, 1.0) * 32_767.0).round() as i16);
    }
    Some((samples.len() - before) as u32)
}

pub fn read(body: &[u8]) -> Result<SoundBank, Error> {
    let lins = find(body, b"lins").ok_or(Error::BadSoundBank)?;
    let wvpl = find(body, b"wvpl").ok_or(Error::BadSoundBank)?;
    let bank_name = find(body, b"INFO").and_then(|i| find(i, b"INAM")).map(name).unwrap_or_default();

    // The waves, by their offset in the pool, as the pool table lists them.
    let mut samples = Vec::new();
    let mut waves: Vec<(usize, Option<Wave>)> = Vec::new();
    for c in chunks(wvpl) {
        let offset = c.data.as_ptr() as usize - wvpl.as_ptr() as usize - 8;
        let wave = c.list().filter(|(kind, _)| kind == b"wave").and_then(|(_, w)| {
            let fmt = find(w, b"fmt ")?;
            let data = find(w, b"data")?;
            let start = samples.len() as u32;
            let len = decode(fmt, data, &mut samples)?;
            Some(Wave { start, len, rate: u32_at(fmt, 4), wsmp: find(w, b"wsmp").map(wsmp) })
        });
        waves.push((offset, wave));
    }
    let table: Vec<usize> = match find(body, b"ptbl") {
        Some(p) => {
            let size = u32_at(p, 0) as usize;
            (0..u32_at(p, 4) as usize).map(|i| u32_at(p, size + i * 4) as usize).collect()
        }
        None => waves.iter().map(|(o, _)| *o).collect(),
    };
    let wave = |cue: u32| {
        let offset = *table.get(cue as usize)?;
        waves.iter().find(|(o, _)| *o == offset)?.1.as_ref()
    };

    let mut found = Vec::new();
    for c in chunks(lins) {
        let Some((kind, ins)) = c.list() else { continue };
        if &kind != b"ins " {
            continue;
        }
        let insh = find(ins, b"insh").unwrap_or(&[]);
        let locale = u32_at(insh, 4);
        let drums = locale & 0x8000_0000 != 0;
        let bank = ((locale >> 8) & 0x7f) as u8;
        let program = (u32_at(insh, 8) & 0x7f) as u8;
        let ins_name = find(ins, b"INFO").and_then(|i| find(i, b"INAM")).map(name).unwrap_or_default();
        let mut base = defaults();
        let mut base_gain = 0.0;
        for art in articulations(ins) {
            articulate(&mut base, &mut base_gain, art);
        }
        let mut regions = Vec::new();
        for rc in chunks(find(ins, b"lrgn").unwrap_or(&[])) {
            let Some((kind, rgn)) = rc.list() else { continue };
            if &kind != b"rgn " && &kind != b"rgn2" {
                continue;
            }
            let (Some(rgnh), Some(wlnk)) = (find(rgn, b"rgnh"), find(rgn, b"wlnk")) else { continue };
            let Some(w) = wave(u32_at(wlnk, 8)) else { continue };
            let mut r = base;
            let mut gain = base_gain;
            for art in articulations(rgn) {
                articulate(&mut r, &mut gain, art);
            }
            let range = |at: usize| {
                let (lo, hi) = (u16_at(rgnh, at).min(127) as u8, u16_at(rgnh, at + 2).min(127) as u8);
                // DLS 1 had no use for the velocity range, and some
                // banks leave it 0 to 0.
                if hi < lo || (lo == 0 && hi == 0) {
                    (0, 127)
                } else {
                    (lo, hi)
                }
            };
            r.keys = range(0);
            r.vels = range(4);
            r.exclusive = u16_at(rgnh, 10);
            let s = find(rgn, b"wsmp").map(wsmp).or(w.wsmp).unwrap_or(Wsmp { unity: 60, fine: 0, gain: 0.0, looping: None });
            r.start = w.start;
            r.end = w.start + w.len;
            r.rate = if w.rate == 0 { 22_050.0 } else { w.rate as f32 };
            r.root = f32::from(s.unity.min(127));
            r.tune += f32::from(s.fine);
            r.attenuation = -(gain + s.gain);
            if let Some((kind, from, len)) = s.looping {
                let ls = from.min(w.len);
                let le = from.saturating_add(len).min(w.len);
                if le > ls + 1 {
                    r.looping = kind;
                    r.loop_start = w.start + ls;
                    r.loop_end = w.start + le;
                }
            }
            regions.push(r);
        }
        found.push((drums, if drums { 0 } else { bank }, program, ins_name, regions));
    }
    build(bank_name, SoundBankKind::Dls, samples, found)
}
