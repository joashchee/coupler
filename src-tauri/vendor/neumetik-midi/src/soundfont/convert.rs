// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Any sound bank Neumetik reads (a DLS file, above all) as a SoundFont:
//! each preset an instrument, each region a zone, its sample's part of
//! the bank's samples a sample of its own (once, however many regions
//! play it), and what shapes it as generators, worked back from the
//! units the synthesizer keeps them in. What generators can't say goes
//! in modulators: velocity and key moving the cutoff, velocity the
//! attack, the mod wheel's vibrato through either LFO.

use super::generator as g;
use super::{Instrument, Modulator, Preset, Sample, SoundFont, Zone};
use crate::bank::{Envelope, Looping, Region, SoundBank, SoundBankKind};

// Modulator sources: linear, unipolar, rising (the specification's 8.2.1).
const SRC_VELOCITY: u16 = 2;
const SRC_KEY: u16 = 3;
const SRC_CC1: u16 = 0x0081;

fn tc(t: f32) -> i16 {
    if t == f32::NEG_INFINITY {
        -12_000
    } else {
        t.round().clamp(-12_000.0, 8_000.0) as i16
    }
}

fn clamp16(v: f32) -> i16 {
    v.round().clamp(-32_768.0, 32_767.0) as i16
}

/// Absolute cents from Hz.
fn cents(hz: f32) -> i16 {
    clamp16(1200.0 * (hz.max(0.001) / 8.175_799).log2())
}

/// An envelope's generators, from `first` (delay) on. A DLS envelope's
/// decay and release fall 96 dB in their time, a SoundFont's 100: the
/// same slope is a little longer.
fn envelope(z: &mut Zone, e: &Envelope, first: u16, volume: bool) {
    let longer = if volume { 1200.0 * (1000.0 / e.range).log2() } else { 0.0 };
    let fall = |t: f32| if t == f32::NEG_INFINITY { t } else { t + longer };
    // Per key below the envelope's centre, against the SoundFont's key 60.
    let hold = e.hold + e.key_hold * (e.key_center - 60.0);
    let decay = fall(e.decay) + e.key_decay * (e.key_center - 60.0);
    // The volume envelope's sustain is centibels in either format.
    let sustain = if volume { e.sustain } else { (1.0 - e.sustain) * 1000.0 };
    for (i, v) in [tc(e.delay), tc(e.attack), tc(hold), tc(decay), clamp16(sustain.clamp(0.0, 1440.0)), tc(fall(e.release)), clamp16(e.key_hold), clamp16(e.key_decay)].into_iter().enumerate() {
        z.set(first + i as u16, v);
    }
}

fn modulator(source: u16, destination: u16, amount: f32) -> Modulator {
    Modulator { source, destination, amount: clamp16(amount), amount_source: 0, transform: 0 }
}

/// A region's zone, playing sample `sample`.
fn zone(r: &Region, sample: usize) -> Zone {
    let mut z = Zone::new(sample).keys(r.keys.0, r.keys.1).vels(r.vels.0, r.vels.1);
    let root = r.root.round();
    let tune = r.tune + (r.root - root) * -r.scale;
    z.set(g::ROOT_KEY, root.clamp(0.0, 127.0) as i16);
    z.set(g::COARSE_TUNE, (tune / 100.0).trunc() as i16);
    z.set(g::FINE_TUNE, (tune % 100.0).round() as i16);
    if r.scale != 100.0 {
        z.set(g::SCALE_TUNING, clamp16(r.scale));
    }
    z.set(
        g::SAMPLE_MODES,
        match r.looping {
            Looping::Off => 0,
            Looping::On => 1,
            Looping::UntilRelease => 3,
        },
    );
    if let Some(k) = r.fixed_key {
        z.set(g::KEYNUM, i16::from(k));
    }
    if let Some(v) = r.fixed_vel {
        z.set(g::VELOCITY, i16::from(v));
    }
    // The SoundFont's attenuation counts at 40%, and the resonance's
    // make-up is the player's own.
    let attenuation = (r.attenuation - 5.0 * r.q) / 0.4;
    z.set(g::ATTENUATION, clamp16(attenuation.clamp(0.0, 1440.0)));
    z.set(g::PAN, clamp16(r.pan * 500.0));
    z.set(g::REVERB_SEND, clamp16(r.reverb * 1000.0));
    z.set(g::CHORUS_SEND, clamp16(r.chorus * 1000.0));
    if r.exclusive != 0 {
        z.set(g::EXCLUSIVE_CLASS, r.exclusive.min(127) as i16);
    }
    envelope(&mut z, &r.vol_env, g::VOL_ENV_DELAY, true);
    envelope(&mut z, &r.mod_env, g::MOD_ENV_DELAY, false);
    z.set(g::MOD_ENV_TO_PITCH, clamp16(r.mod_env_pitch));
    z.set(g::MOD_ENV_TO_CUTOFF, clamp16(r.mod_env_cutoff));
    z.set(g::MOD_LFO_DELAY, tc(r.mod_lfo.0));
    z.set(g::MOD_LFO_FREQ, cents(r.mod_lfo.1));
    z.set(g::MOD_LFO_TO_PITCH, clamp16(r.mod_lfo_pitch));
    z.set(g::MOD_LFO_TO_CUTOFF, clamp16(r.mod_lfo_cutoff));
    z.set(g::MOD_LFO_TO_VOLUME, clamp16(r.mod_lfo_volume));
    z.set(g::VIB_LFO_DELAY, tc(r.vib_lfo.0));
    z.set(g::VIB_LFO_FREQ, cents(r.vib_lfo.1));
    z.set(g::VIB_LFO_TO_PITCH, clamp16(r.vib_lfo_pitch));
    // The cutoff, less what the key adds by key 0 (a modulator adds it).
    let cutoff = if r.filtered { r.cutoff } else { 13_500.0 };
    z.set(g::FILTER_CUTOFF, clamp16(cutoff.clamp(1500.0, 13_500.0)));
    z.set(g::FILTER_Q, clamp16(r.q * 10.0));
    if r.vel_cutoff != 0.0 {
        z.modulators.push(modulator(SRC_VELOCITY, g::FILTER_CUTOFF, r.vel_cutoff));
    }
    if r.key_cutoff != 0.0 {
        z.modulators.push(modulator(SRC_KEY, g::FILTER_CUTOFF, r.key_cutoff * 128.0));
    }
    if r.vol_env.vel_attack != 0.0 {
        z.modulators.push(modulator(SRC_VELOCITY, g::VOL_ENV_ATTACK, r.vol_env.vel_attack));
    }
    // The mod wheel: the standard's own modulator is 50 cents of vibrato.
    if r.wheel_vib_lfo != 50.0 {
        z.modulators.push(modulator(SRC_CC1, g::VIB_LFO_TO_PITCH, r.wheel_vib_lfo));
    }
    if r.wheel_mod_lfo != 0.0 {
        z.modulators.push(modulator(SRC_CC1, g::MOD_LFO_TO_PITCH, r.wheel_mod_lfo));
    }
    z
}

pub fn from_bank(bank: &SoundBank) -> SoundFont {
    let mut sf = SoundFont::new(bank.name());
    sf.info.software = format!("Neumetik MIDI {}", crate::VERSION);
    if bank.kind() == SoundBankKind::Dls {
        sf.info.comment = "Made from a DLS bank.".into();
    }
    // Each piece of the bank's samples once: (start, end, loop, rate).
    let mut made: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
    for p in bank.presets() {
        let mut inst = Instrument::new(&p.name);
        for r in &bank.regions[p.regions.clone()] {
            let looped = r.looping != Looping::Off;
            let (ls, le) = if looped { (r.loop_start, r.loop_end) } else { (0, 0) };
            let id = (r.start, r.end, ls, le, r.rate as u32);
            let sample = match made.iter().position(|m| *m == id) {
                Some(i) => i,
                None => {
                    let data = bank.samples.get(r.start as usize..r.end as usize).unwrap_or(&[]).to_vec();
                    let mut s = Sample::new(&format!("{} {}", p.name, made.len() + 1), data, r.rate as u32, r.root.round().clamp(0.0, 127.0) as u8);
                    if looped {
                        s = s.looped(ls - r.start, le - r.start);
                    }
                    made.push(id);
                    sf.add_sample(s)
                }
            };
            inst.zones.push(zone(r, sample));
        }
        let i = sf.add_instrument(inst);
        let bank_number = if p.drums { 128 } else { u16::from(p.bank) };
        sf.add_preset(Preset::new(&p.name, bank_number, u16::from(p.program), i));
    }
    sf
}
