// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Neumetik's own instruments and drum kits as a SoundFont: each patch
//! sampled at a root every few keys (and at a few velocities, where the
//! velocity changes its sound and not only its level), each drum once.
//!
//! What a SoundFont can do itself, it's left to: a patch is played with
//! its amplitude envelope held open after its attack, and the envelope's
//! decay, sustain and release go in the volume envelope's generators (a
//! plucked string's decay is taken back out of its sample to go there
//! too). A low-pass filter goes in the SoundFont's filter, its envelope
//! in the modulation envelope, its key tracking in each root's cutoff and
//! its velocity in a modulator; vibrato goes to the vibrato LFO, tremolo
//! and the filter's wobble to the modulation LFO. What changes on its own
//! (an FM index falling, a tine dying away, a pitch sweep, a high-pass or
//! band-pass filter's envelope) is sampled until it has settled, and the
//! sample then loops round a whole number of cycles, crossfaded. So most
//! samples are a fraction of a second long.
//!
//! Each instrument's samples are as loud as 16 bits hold, its attenuation
//! set so it plays (through Neumetik MIDI) as loud as Neumetik's own.

use super::generator as g;
use super::{Instrument, Modulator, Preset, Sample, SoundFont, Zone};
use crate::drums::{self, Drum, KITS};
use crate::patch::{Adsr, Engine, Filter, Patch, Wave};
use crate::voice::{Controls, Voice, BLOCK};

/// Which of Neumetik's instruments [`SoundFont::neumetik`] packs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackSet {
    /// General MIDI's 128 instruments, bank 0.
    pub gm: bool,
    /// Roland GS's variation tones, banks 1 to 63.
    pub gs: bool,
    /// The classic synth banks: 80, 81 and 82.
    pub classics: bool,
    /// The drum kits, bank 128.
    pub drums: bool,
}

impl PackSet {
    /// General MIDI and the drum kits.
    pub const GM: PackSet = PackSet { gm: true, gs: false, classics: false, drums: true };
    /// Everything Neumetik plays.
    pub const ALL: PackSet = PackSet { gm: true, gs: true, classics: true, drums: true };
}

/// How [`SoundFont::neumetik`] samples Neumetik's instruments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pack {
    pub set: PackSet,
    /// The samples' rate.
    pub rate: u32,
    /// Keys between one sampled root and the next (1 samples every key).
    pub step: u8,
    /// Velocity layers, for the patches whose sound changes with
    /// velocity (FM, tines, plucked strings, a filter it moves): 1 to 4.
    pub layers: u8,
    /// The longest a sample runs before it loops (or, for a drum, ends).
    pub seconds: f32,
}

impl Pack {
    /// General MIDI and the drums, small: 22 kHz, a root an octave, one
    /// velocity layer, half a second at most before a loop.
    pub const SMALL: Pack = Pack { set: PackSet::GM, rate: 22_050, step: 12, layers: 1, seconds: 0.5 };
}

impl Default for Pack {
    /// General MIDI and the drums at 32 kHz, a root every 6 keys, 2
    /// velocity layers, a second at most before a loop.
    fn default() -> Pack {
        Pack { set: PackSet::GM, rate: 32_000, step: 6, layers: 2, seconds: 1.0 }
    }
}

/// The lowest and highest keys a root is sampled at; the outer ones
/// stretch to the ends of the keyboard.
const LOWEST: u8 = 21;
const HIGHEST: u8 = 108;

/// A sampled voice through Neumetik MIDI against Neumetik's own: the
/// synthesizer's `SAMPLED_LEVEL`.
const SAMPLED_LEVEL: f32 = 2.0;

/// A Neumetik envelope's decay falls 60 dB in its time; a SoundFont's 100.
const SF2_FALL: f32 = 100.0 / 60.0;

/// How far into a fall (60 dB in its time) a sound counts as settled:
/// 30 dB down, a thirtieth of the way left to go.
const SETTLED: f32 = 0.5;

/// The time constant of a Neumetik envelope's fall, in its time.
const FALL: f32 = 6.9;

fn tc(seconds: f32) -> i16 {
    if seconds <= 0.001 {
        -12_000
    } else {
        (1200.0 * seconds.log2()).round().clamp(-12_000.0, 8_000.0) as i16
    }
}

fn hz_cents(hz: f32) -> i16 {
    (1200.0 * (hz.max(0.01) / 8.175_799).log2()).round().clamp(-16_000.0, 13_500.0) as i16
}

fn key_hz(key: f32) -> f32 {
    440.0 * 2f32.powf((key - 69.0) / 12.0)
}

/// How a patch is sampled.
struct Plan {
    /// The patch as it's rendered: amplitude envelope held, its LFO's
    /// pitch and level out, and its filter out if the SoundFont does it.
    render: Patch,
    /// The SoundFont's filter plays the patch's low-pass.
    filter: bool,
    /// The velocity changes the sound, not only its level.
    layered: bool,
    /// Seconds before it's settled, but for its tine (which depends on
    /// the key).
    settle: f32,
    /// The LFO moves what's sampled (a pulse's width, a wobble the
    /// SoundFont isn't doing): the loop holds whole cycles of it.
    lfo_sampled: bool,
    noisy: bool,
}

fn plan(p: &Patch) -> Plan {
    // A 24 dB filter is sampled: the SoundFont's is 12 dB.
    let filter = p.filter == Filter::Low;
    let mut render = *p;
    render.vibrato = 0.0;
    if p.engine != Engine::Pluck {
        // A string's envelope is its own decay; it's taken out afterwards.
        render.aenv = Adsr { a: p.aenv.a, d: 1.0, s: 1.0, r: p.aenv.r };
    }
    if filter {
        render.filter = Filter::Off;
        render.wobble = 0.0;
    }
    let fenv_time = p.fenv.a + SETTLED * p.fenv.d;
    let mut settle = p.aenv.a;
    if (!filter && p.filter != Filter::Off && p.env_amt != 0.0) || p.sync.is_some() || p.sweep != 0.0 {
        settle = settle.max(fenv_time);
    }
    if p.engine == Engine::Fm && p.index_sus != 1.0 {
        settle = settle.max(SETTLED * p.index_decay);
    }
    // A SoundFont's tremolo is a triangle in decibels, Neumetik's a
    // shape in level: it's sampled.
    let lfo_sampled = p.pwm != 0.0 || p.tremolo != 0.0 || (!filter && p.wobble != 0.0);
    if lfo_sampled {
        settle = settle.max(p.lfo_delay);
    }
    Plan {
        render,
        filter,
        layered: p.engine != Engine::Analog || p.tine > 0.0 || (!filter && p.vel_filter != 0.0),
        settle,
        lfo_sampled,
        noisy: p.noise > 0.0 || p.wave1 == Wave::Noise || p.wave2 == Wave::Noise && p.mix2 > 0.0,
    }
}

/// How much a patch's decays shorten at `key` (a piano's high strings).
fn key_scale(p: &Patch, key: u8) -> f32 {
    2f32.powf(-p.key_decay * (f32::from(key) - 60.0) / 24.0)
}

/// Renders a voice: `seconds` of it, or until it ends.
fn render_voice(voice: &mut Voice, seconds: f32, rate: u32) -> Vec<f32> {
    let n = (seconds * rate as f32).ceil() as usize;
    let mut out = vec![0f32; n];
    let ctl = Controls::default();
    for block in out.chunks_mut(BLOCK) {
        if !voice.active {
            break;
        }
        voice.render(block, &ctl, &[]);
    }
    out
}

/// Where a loop goes: `n` whole periods of `period` samples that come
/// closest to a whole number of samples, about `target` samples long.
fn loop_length(period: f32, target: f32) -> usize {
    if period <= 1.0 {
        return target.max(32.0) as usize;
    }
    let base = (target / period).round().max(1.0) as usize;
    let mut best = (f32::MAX, base);
    for n in (base * 3 / 4).max(1)..=(base * 5 / 4 + 1) {
        let len = n as f32 * period;
        let miss = (len - len.round()).abs() / n as f32;
        if miss < best.0 {
            best = (miss, n);
        }
    }
    ((best.1 as f32 * period).round() as usize).max(32)
}

/// A sample's sound, looped from `loop_start` to `loop_end` (or not),
/// and for a string the decay taken out of it, as a SoundFont's decay
/// time (seconds for 100 dB).
struct Take {
    data: Vec<f32>,
    rate: u32,
    loop_start: usize,
    loop_end: usize,
    decay: Option<f32>,
}

/// Loops `data` round its last `len` samples after `start`, the loop's
/// end crossfaded into what led up to its start, with eight samples of
/// what follows the loop after it (for players that read past it).
fn make_loop(mut data: Vec<f32>, start: usize, len: usize, equal_power: bool) -> Take {
    let fade = (len / 3).min(start).max(1);
    let end = start + len;
    data.resize(end.max(data.len()), 0.0);
    for j in 0..fade {
        let w = (j + 1) as f32 / fade as f32;
        let (a, b) = if equal_power { ((w * std::f32::consts::FRAC_PI_2).cos(), (w * std::f32::consts::FRAC_PI_2).sin()) } else { (1.0 - w, w) };
        let at = end - fade + j;
        data[at] = data[at] * a + data[start - fade + j] * b;
    }
    data.truncate(end);
    for i in 0..8 {
        data.push(data[start + i % len]);
    }
    Take { data, rate: 0, loop_start: start, loop_end: end, decay: None }
}

/// A patch played at `key` and `vel`, sampled.
fn take(p: &Patch, plan: &Plan, key: u8, vel: u8, pack: &Pack) -> Take {
    let pluck = p.engine == Engine::Pluck;
    // A plucked string sounds as the rate it's played at has it: at
    // least CD's, where it's heard.
    let rate_hz = if pluck { pack.rate.max(44_100) } else { pack.rate };
    let rate = rate_hz as f32;
    let scale = key_scale(p, key);
    let hz = key_hz(f32::from(key) + p.transpose);
    let period = rate / hz;

    let mut settle = plan.settle;
    if p.tine > 0.0 {
        settle = settle.max(SETTLED * p.tine_decay * scale);
    }
    let ring = (p.aenv.d * scale).max(0.01);
    if pluck {
        // The string's timbre goes on changing: half its ring, up to a
        // half second, then it's held.
        settle = settle.max((0.5 * ring).min(0.5));
    }
    let settle = settle.min(pack.seconds) + 0.01;

    // A loop long enough to hide its seam: the beating of detuned copies,
    // the LFO's cycle if it's sampled, a noise's grain.
    let mut target: f32 = 0.12;
    if p.unison > 1 && p.spread > 0.0 {
        target = target.max(1.0 / (hz * (2f32.powf(p.spread / 2400.0) - 1.0)).max(1.0));
    }
    if p.mix2 > 0.0 && p.detune != 0.0 {
        target = target.max(1.0 / (hz * (2f32.powf(p.detune.abs() / 1200.0) - 1.0)).max(1.0));
    }
    if plan.noisy {
        target = target.max(0.3);
    }
    let mut target = target.min(0.6) * rate;
    if plan.lfo_sampled && p.lfo_rate > 0.0 {
        let cycle = rate / p.lfo_rate;
        target = (target / cycle).ceil().max(1.0) * cycle;
    }
    let len = loop_length(period, target);
    let mut start = ((settle * rate) as usize).max(len / 3 + 1);
    let seconds = (start + len) as f32 / rate + 0.01;
    let render = |seed: u32| {
        let mut voice = Voice::new(rate, seed);
        voice.start(0, key, vel, &plan.render, 0);
        render_voice(&mut voice, seconds, rate_hz)
    };
    let mut data = if pluck {
        // A string starts from a burst of noise, a few dB louder or
        // softer each time: of five, the one in the middle.
        let early = (0.1 * rate) as usize;
        let mut tries: Vec<(f32, Vec<f32>)> = (0..5u32)
            .map(|i| {
                let d = render(0x4e55_4d45 ^ i.wrapping_mul(0x9e37_79b9));
                (d.iter().take(early).map(|x| x * x).sum::<f32>(), d)
            })
            .collect();
        tries.sort_by(|a, b| a.0.total_cmp(&b.0));
        tries.swap_remove(2).1
    } else {
        render(0x4e55_4d45)
    };
    // The level velocity gives it, out: the SoundFont puts it back.
    let v = f32::from(vel) / 127.0;
    for x in &mut data {
        *x /= v * v;
    }

    let mut decay = None;
    if pluck {
        // The string's decay comes out, to go in the volume envelope. It
        // falls fastest at first, as its top dies, and its loop's filter
        // takes more than its ring says: so it's measured where the loop
        // goes, and taken out at that rate, which leaves the faster start
        // in the sample. The loop goes no later than where the string is
        // 40 dB down.
        let window = ((4.0 * period) as usize).max((0.01 * rate) as usize);
        let level = |s: &[f32]| (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt();
        let peak = data.chunks(window).map(level).fold(0f32, f32::max);
        let alive = data.chunks(window).rposition(|c| level(c) > peak * 0.01).map_or(window, |i| (i + 1) * window);
        start = start.min(alive.saturating_sub(len)).max(len / 3 + 1);
        let (a, b) = (start.saturating_sub(len), (start + len).min(data.len()));
        let mut db_per_s = 60.0 / ring;
        if b >= a + 2 * window {
            let (first, last) = (level(&data[a..a + window]), level(&data[b - window..b]));
            if first > 1e-6 && last > 1e-6 && first > last {
                db_per_s = 20.0 * (first / last).log10() / ((b - window - a) as f32 / rate);
            }
        }
        let db_per_s = db_per_s.clamp(1.0, 2000.0);
        let k = db_per_s / 20.0 * std::f32::consts::LN_10 / rate;
        // Never more than 60 dB up, past which it's the noise floor.
        let most = 60.0 / 20.0 * std::f32::consts::LN_10;
        for (i, x) in data.iter_mut().enumerate() {
            *x *= (k * i as f32).min(most).exp();
        }
        decay = Some(100.0 / db_per_s);
    }
    Take { decay, rate: rate_hz, ..make_loop(data, start, len, plan.noisy) }
}

/// What every zone of a patch's instrument shares.
fn global_zone(p: &Patch, plan: &Plan) -> Zone {
    let mut z = Zone::default();
    let a = &p.aenv;
    if p.engine == Engine::Pluck {
        // Each zone's decay is its string's own.
        z.set(g::VOL_ENV_SUSTAIN, 1000);
    } else {
        z.set(g::VOL_ENV_HOLD, tc(a.a));
        if a.s < 0.999 {
            // The same fall at its start, in decibels.
            z.set(g::VOL_ENV_DECAY, tc(a.d * SF2_FALL / (1.0 - a.s).max(0.01)));
            z.set(g::VOL_ENV_SUSTAIN, if a.s <= 0.0 { 1000 } else { (-200.0 * a.s.log10()).round().min(1000.0) as i16 });
        }
    }
    if p.key_decay != 0.0 && p.engine != Engine::Pluck {
        z.set(g::KEY_TO_VOL_ENV_DECAY, (50.0 * p.key_decay).round() as i16);
    }
    z.set(g::VOL_ENV_RELEASE, tc(a.r * SF2_FALL));
    if p.reverb > 0.0 {
        z.set(g::REVERB_SEND, (p.reverb * 1000.0).round() as i16);
    }
    if p.chorus > 0.0 {
        z.set(g::CHORUS_SEND, (p.chorus * 1000.0).round() as i16);
    }
    let lfo_delay = tc(p.lfo_delay * 0.5);
    if p.vibrato != 0.0 {
        z.set(g::VIB_LFO_FREQ, hz_cents(p.lfo_rate));
        z.set(g::VIB_LFO_DELAY, lfo_delay);
        z.set(g::VIB_LFO_TO_PITCH, p.vibrato.round() as i16);
    }
    let wobble = plan.filter && p.wobble != 0.0;
    if wobble {
        z.set(g::MOD_LFO_FREQ, hz_cents(p.lfo_rate));
        z.set(g::MOD_LFO_DELAY, lfo_delay);
        if wobble {
            z.set(g::MOD_LFO_TO_CUTOFF, (p.wobble * 1200.0).round() as i16);
        }
    }
    if plan.filter {
        z.set(g::FILTER_Q, ((-200.0 * damping(p).log10()).max(0.0)).round() as i16);
        if p.env_amt != 0.0 {
            // A straight line, where Neumetik's falls: about as long to
            // get most of the way.
            let f = &p.fenv;
            z.set(g::MOD_ENV_TO_CUTOFF, (p.env_amt * 1200.0).round().clamp(-12_000.0, 12_000.0) as i16);
            z.set(g::MOD_ENV_ATTACK, tc(f.a));
            z.set(g::MOD_ENV_DECAY, tc(f.d * 2.0 / FALL / (1.0 - f.s).max(0.01)));
            z.set(g::MOD_ENV_SUSTAIN, ((1.0 - f.s.clamp(0.0, 1.0)) * 1000.0).round() as i16);
            z.set(g::MOD_ENV_RELEASE, tc(f.r * 2.0 / FALL));
        }
        if p.vel_filter != 0.0 {
            // Closing as the velocity falls: the cutoff's set for the
            // softest, and velocity opens it.
            z.modulators.push(Modulator { source: 2, destination: g::FILTER_CUTOFF, amount: (p.vel_filter * 1200.0).round() as i16, amount_source: 0, transform: 0 });
        }
    }
    z
}

/// The patch's filter's damping (1/Q), as Neumetik's filter has it.
fn damping(p: &Patch) -> f32 {
    2.0 - 1.96 * p.reso.clamp(0.0, 1.0)
}

/// The cutoff a root's zone has: the patch's at middle C, moved by the
/// key as far as it follows it (and down for the softest velocity, which
/// a modulator brings back up). A SoundFont's filter is no gentler than a
/// Butterworth's; Neumetik's, without resonance, is, so its cutoff comes
/// down to where the gentler one is 3 dB down.
fn cutoff(p: &Patch, key: u8) -> i16 {
    let octaves = p.key_track * (f32::from(key) - 60.0) / 12.0 - p.vel_filter;
    let k = damping(p);
    let b = k * k - 2.0;
    let gentler = if b > 0.0 { ((-b + (b * b + 4.0).sqrt()) / 2.0).sqrt() } else { 1.0 };
    hz_cents(p.cutoff * 2f32.powf(octaves) * gentler).clamp(1500, 13_500)
}

/// An instrument being made: its zones and their sounds, before they're
/// set to one level.
struct Made {
    inst: Instrument,
    takes: Vec<(String, u8, Take)>,
    /// The resonance's make-up Neumetik MIDI takes off a filtered sample,
    /// in centibels.
    q: f32,
}

fn note_name(key: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[usize::from(key % 12)], i32::from(key / 12) - 1)
}

fn short(name: &str, most: usize) -> String {
    name.chars().take(most).collect::<String>().trim_end().to_string()
}

/// The roots a patch is sampled at, and the keys each plays.
fn roots(step: u8) -> Vec<(u8, u8, u8)> {
    let step = step.clamp(1, 24);
    let mut out = Vec::new();
    let mut root = LOWEST + step / 2;
    while root <= HIGHEST {
        let lo = root.saturating_sub(step / 2);
        out.push((root, lo, (lo + step - 1).min(127)));
        root += step;
    }
    if let Some(first) = out.first_mut() {
        first.1 = 0;
    }
    if let Some(last) = out.last_mut() {
        last.2 = 127;
    }
    out
}

fn patch_instrument(p: &Patch, pack: &Pack) -> Made {
    let plan = plan(p);
    let mut inst = Instrument::new(p.name);
    inst.global = global_zone(p, &plan);
    let layers = if plan.layered { pack.layers.clamp(1, 4) } else { 1 };
    let mut takes = Vec::new();
    for (root, lo, hi) in roots(pack.step) {
        for layer in 0..layers {
            let vlo = if layer == 0 { 0 } else { (128 * u32::from(layer) / u32::from(layers)) as u8 };
            let vhi = (128 * (u32::from(layer) + 1) / u32::from(layers) - 1) as u8;
            // Played a little above the layer's middle.
            let t = take(p, &plan, root, (vlo as u32 + (vhi as u32 - vlo as u32) * 2 / 3).max(1) as u8, pack);
            let mut z = Zone::new(0).keys(lo, hi);
            if layers > 1 {
                z = z.vels(vlo, vhi);
            }
            z.set(g::ROOT_KEY, i16::from(root));
            z.set(g::SAMPLE_MODES, 1);
            if let Some(d) = t.decay {
                z.set(g::VOL_ENV_DECAY, tc(d));
            }
            if plan.filter {
                z.set(g::FILTER_CUTOFF, cutoff(p, root));
            }
            inst.zones.push(z);
            let name = if layers > 1 { format!("{} {}{}", short(p.name, 13), note_name(root), layer + 1) } else { format!("{} {}", short(p.name, 15), note_name(root)) };
            takes.push((name, root, t));
        }
    }
    let q = if plan.filter { f32::from(inst.global.get(g::FILTER_Q).unwrap_or(0)) / 10.0 } else { 0.0 };
    Made { inst, takes, q: 5.0 * q }
}

/// A drum, sampled to its end.
fn drum_take(d: &Drum, key: u8, pack: &Pack) -> Take {
    let mut voice = Voice::new(pack.rate as f32, 0x4e55_4d45);
    voice.start_drum(9, key, 127, *d, 0);
    let mut data = render_voice(&mut voice, (d.length() * 1.05 + 0.02).min(pack.seconds.max(0.5)), pack.rate);
    // The last few milliseconds faded, should it be cut short.
    let fade = (pack.rate as usize / 200).min(data.len());
    let n = data.len();
    for (i, x) in data[n - fade..].iter_mut().enumerate() {
        *x *= 1.0 - i as f32 / fade as f32;
    }
    Take { data, rate: pack.rate, loop_start: 0, loop_end: 0, decay: None }
}

/// The loudest a sample can be and still play through Neumetik MIDI as
/// loud as Neumetik's own (a SoundFont's attenuation can't be below 0).
const CEILING: f32 = 0.98 * SAMPLED_LEVEL;

/// Brings peaks above `ceiling` down to it, smoothly: the gain falls over
/// 2 ms before a peak and comes back over 50 ms after. A plucked string's
/// first few milliseconds are its only peaks that high.
fn limit(data: &mut [f32], ceiling: f32, rate: f32) {
    if !data.iter().any(|x| x.abs() > ceiling) {
        return;
    }
    let mut gain: Vec<f32> = data.iter().map(|x| (ceiling / x.abs().max(1e-9)).min(1.0)).collect();
    let back = 1.0 - (-1.0 / (0.05 * rate)).exp();
    for i in 1..gain.len() {
        gain[i] = gain[i].min(gain[i - 1] + (1.0 - gain[i - 1]) * back);
    }
    let ahead = 1.0 / (0.002 * rate);
    for i in (0..gain.len().saturating_sub(1)).rev() {
        gain[i] = gain[i].min(gain[i + 1] + ahead);
    }
    for (x, g) in data.iter_mut().zip(gain) {
        *x *= g;
    }
}

/// Sets an instrument's samples to one level: as loud as 16 bits hold,
/// its attenuation making up for it. Returns the samples, in its zones'
/// order, and sets the attenuation in its global zone.
fn level(made: &mut Made) -> Vec<Sample> {
    for (_, _, t) in &mut made.takes {
        limit(&mut t.data, CEILING, t.rate as f32);
    }
    let peak = made.takes.iter().flat_map(|(_, _, t)| &t.data).fold(1e-6f32, |m, x| m.max(x.abs()));
    // A sample at full scale plays at SAMPLED_LEVEL × its gain.
    let scale = (0.98 / peak).min(1.0 / SAMPLED_LEVEL * 10f32.powf(1440.0 * 0.4 / 200.0));
    let total = 200.0 * (SAMPLED_LEVEL * scale).log10();
    let attenuation = ((total - made.q) / 0.4).clamp(0.0, 1440.0);
    made.inst.global.set(g::ATTENUATION, attenuation.round() as i16);
    made.takes
        .iter()
        .map(|(name, root, t)| {
            let data = t.data.iter().map(|x| (x * scale * 32_767.0).round().clamp(-32_768.0, 32_767.0) as i16).collect();
            let s = Sample::new(name, data, t.rate, *root);
            if t.loop_end > t.loop_start {
                s.looped(t.loop_start as u32, t.loop_end as u32)
            } else {
                s
            }
        })
        .collect()
}

/// Adds a made instrument and its samples; its place.
fn add(sf: &mut SoundFont, mut made: Made) -> usize {
    let samples = level(&mut made);
    for (z, s) in made.inst.zones.iter_mut().zip(samples) {
        z.target = Some(sf.add_sample(s));
    }
    sf.add_instrument(made.inst)
}

impl SoundFont {
    /// Neumetik's own instruments and drum kits as a SoundFont, sampled
    /// as `pack` says: any SoundFont player then sounds like Neumetik.
    /// It takes a few seconds; [`neumetik_with_progress`] says how far it's
    /// got.
    ///
    /// The instruments are sampled, so they're close to Neumetik's own
    /// rather than the same: a SoundFont's filter is 12 dB an octave (a
    /// 24 dB patch's is gentler), its envelopes fall in straight lines,
    /// and what a sample holds once it loops doesn't change any more.
    ///
    /// [`neumetik_with_progress`]: SoundFont::neumetik_with_progress
    pub fn neumetik(pack: &Pack) -> SoundFont {
        SoundFont::neumetik_with_progress(pack, |_, _| {})
    }

    /// The same, calling `progress` with the instruments made so far and
    /// how many there are, after each.
    pub fn neumetik_with_progress(pack: &Pack, mut progress: impl FnMut(usize, usize)) -> SoundFont {
        let pack = Pack { rate: pack.rate.clamp(8_000, 96_000), seconds: pack.seconds.clamp(0.1, 30.0), ..*pack };
        let mut sf = SoundFont::new("Neumetik");
        sf.info.author = "Neumetik MIDI".into();
        sf.info.software = format!("Neumetik MIDI {}", crate::VERSION);
        sf.info.comment = "Neumetik MIDI's own instruments and drum kits, sampled.".into();

        // Every preset: (bank, program, patch), each patch made once.
        let mut wanted: Vec<(u16, u16, &'static Patch)> = Vec::new();
        if pack.set.gm {
            wanted.extend(crate::gm::GM.iter().enumerate().map(|(i, p)| (0, i as u16, p)));
        }
        if pack.set.gs {
            wanted.extend(crate::gs::GS.iter().map(|(m, prog, p)| (u16::from(*m), u16::from(*prog), p)));
        }
        if pack.set.classics {
            for bank in crate::BANKS.iter().filter(|b| b.msb != 0) {
                wanted.extend(bank.patches.iter().enumerate().map(|(i, p)| (u16::from(bank.msb), i as u16, p)));
            }
        }
        let total = wanted.len() + if pack.set.drums { KITS.len() } else { 0 };
        let mut made: Vec<(&'static Patch, usize)> = Vec::new();
        for (i, (bank, program, p)) in wanted.iter().enumerate() {
            let inst = match made.iter().find(|(m, _)| *m == *p) {
                Some((_, at)) => *at,
                None => {
                    let at = add(&mut sf, patch_instrument(p, &pack));
                    made.push((p, at));
                    at
                }
            };
            sf.add_preset(Preset::new(p.name, *bank, *program, inst));
            progress(i + 1, total);
        }

        if pack.set.drums {
            // Every drum once, however many kits play it, all at one level.
            let mut drum_takes: Vec<(Drum, u8, Take)> = Vec::new();
            let mut kits: Vec<Vec<(u8, usize)>> = Vec::new();
            for (k, (kit, _)) in KITS.iter().enumerate() {
                let mut keys = Vec::new();
                for key in 27..=87u8 {
                    let Some(d) = drums::drum(*kit, key) else { continue };
                    let at = match drum_takes.iter().position(|(x, _, _)| *x == d) {
                        Some(at) => at,
                        None => {
                            drum_takes.push((d, key, drum_take(&d, key, &pack)));
                            drum_takes.len() - 1
                        }
                    };
                    keys.push((key, at));
                }
                kits.push(keys);
                progress(wanted.len() + k + 1, total);
            }
            for (_, _, t) in &mut drum_takes {
                limit(&mut t.data, CEILING, pack.rate as f32);
            }
            let peak = drum_takes.iter().flat_map(|(_, _, t)| &t.data).fold(1e-6f32, |m, x| m.max(x.abs()));
            let scale = (0.98 / peak).min(1.0 / SAMPLED_LEVEL * 10f32.powf(1440.0 * 0.4 / 200.0));
            let attenuation = (200.0 * (SAMPLED_LEVEL * scale).log10() / 0.4).clamp(0.0, 1440.0).round() as i16;
            let first = sf.samples.len();
            for (n, (_, key, t)) in drum_takes.iter().enumerate() {
                let data = t.data.iter().map(|x| (x * scale * 32_767.0).round().clamp(-32_768.0, 32_767.0) as i16).collect();
                sf.add_sample(Sample::new(&format!("Drum {} {}", n + 1, note_name(*key)), data, pack.rate, *key));
            }
            for ((kit, name), keys) in KITS.iter().zip(kits) {
                let mut inst = Instrument::new(&format!("{name} Kit"));
                inst.global.set(g::ATTENUATION, attenuation);
                inst.global.set(g::SAMPLE_MODES, 0);
                for (key, at) in keys {
                    let (d, _, t) = &drum_takes[at];
                    // A drum plays out whatever the key does.
                    let mut z = Zone::new(first + at).keys(key, key).with(g::ROOT_KEY, i16::from(key)).with(g::VOL_ENV_RELEASE, tc(t.data.len() as f32 / pack.rate as f32 + 1.0));
                    if d.pan != 0.0 {
                        z.set(g::PAN, (d.pan * 500.0).round() as i16);
                    }
                    if d.reverb > 0.0 {
                        z.set(g::REVERB_SEND, (d.reverb * 1000.0).round() as i16);
                    }
                    if d.choke != 0 {
                        z.set(g::EXCLUSIVE_CLASS, i16::from(d.choke));
                    }
                    inst.zones.push(z);
                }
                let i = sf.add_instrument(inst);
                sf.add_preset(Preset::new(&format!("{name} Kit"), 128, u16::from(*kit), i));
            }
        }
        sf
    }
}
