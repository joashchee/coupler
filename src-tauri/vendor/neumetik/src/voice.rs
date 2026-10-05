// Copyright 2026 ansiapps. Neumetik: see README.md for its license.

//! One sounding note: a patch's engine, filter, envelopes and LFO, or a
//! drum. Everything slow (pitch, the filter's coefficients, envelopes,
//! the LFO) is worked out once a block of `BLOCK` samples; only the
//! oscillators and filters run every sample.

use std::f32::consts::PI;

use super::drums::{Drum, METAL_RATIOS};
use super::patch::{Adsr, Engine, Filter, Lfo, Patch, Wave, BAR_HARMONICS};

/// A radian as a fraction of a cycle: FM's indexes are in radians, as
/// the textbooks' (and the DX7's, near enough) are.
const RADIAN: f32 = 1.0 / (2.0 * PI);

/// Samples between control updates.
pub const BLOCK: usize = 16;

const SINE_SIZE: usize = 4096;
/// One cycle of a sine and its first point again, made when Coupler is
/// compiled (a Taylor series, in f64, accurate far past f32's needs).
static SINE: [f32; SINE_SIZE + 1] = sine_table();

const fn sine_table() -> [f32; SINE_SIZE + 1] {
    let mut table = [0f32; SINE_SIZE + 1];
    let mut i = 0;
    while i <= SINE_SIZE {
        // Into -pi..pi, where the series converges fast.
        let mut x = i as f64 / SINE_SIZE as f64 * 2.0 * std::f64::consts::PI;
        if x > std::f64::consts::PI {
            x -= 2.0 * std::f64::consts::PI;
        }
        let (mut term, mut sum, mut n) = (x, x, 1.0);
        while n < 40.0 {
            term = -term * x * x / ((n + 1.0) * (n + 2.0));
            sum += term;
            n += 2.0;
        }
        table[i] = sum as f32;
        i += 1;
    }
    table
}

/// sin(2 pi `phase`), any phase, from a table.
#[inline]
fn sine(phase: f32) -> f32 {
    let p = (phase - phase.floor()) * SINE_SIZE as f32;
    let i = p as usize;
    let f = p - i as f32;
    SINE[i] + (SINE[i + 1] - SINE[i]) * f
}

/// The band-limiting correction at a saw's or pulse's jump.
#[inline]
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

#[inline]
fn pulse(t: f32, dt: f32, width: f32) -> f32 {
    let mut s = if t < width { 1.0 } else { -1.0 };
    s += blep(t, dt);
    let u = t - width;
    s -= blep(u - u.floor(), dt);
    s
}

#[inline]
fn wrap(p: &mut f32) -> bool {
    if *p >= 1.0 {
        *p -= p.floor();
        true
    } else {
        false
    }
}

#[inline]
fn osc(noise: &mut Noise, width: f32, wave: Wave, t: f32, dt: f32) -> f32 {
    match wave {
        Wave::Sine | Wave::Organ => sine(t),
        Wave::Triangle => 4.0 * (t - 0.5).abs() - 1.0,
        Wave::Saw => 2.0 * t - 1.0 - blep(t, dt),
        Wave::Square => pulse(t, dt, 0.5),
        Wave::Pulse => pulse(t, dt, width),
        Wave::Noise => noise.next(),
    }
}

/// xorshift: noise without a crate.
#[derive(Clone, Copy)]
pub struct Noise(u32);

impl Noise {
    pub fn new(seed: u32) -> Self {
        Noise(seed | 1)
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// A state-variable filter (Cytomic's trapezoidal one): stable at any
/// cutoff, low, band and high from the same two states.
#[derive(Clone, Copy, Default)]
struct Svf {
    ic1: f32,
    ic2: f32,
}

#[derive(Clone, Copy, Default)]
struct Coef {
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Coef {
    fn new(cutoff: f32, reso: f32, rate: f32) -> Coef {
        let g = (PI * cutoff.clamp(20.0, rate * 0.45) / rate).tan();
        let k = 2.0 - 1.96 * reso.clamp(0.0, 1.0);
        let a1 = 1.0 / (1.0 + g * (g + k));
        Coef { a1, a2: g * a1, a3: g * g * a1, k }
    }
}

impl Svf {
    #[inline]
    fn tick(&mut self, c: &Coef, kind: Filter, v0: f32) -> f32 {
        let v3 = v0 - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        match kind {
            Filter::Off => v0,
            Filter::Low | Filter::Low24 => v2,
            Filter::Band => v1,
            Filter::High => v0 - c.k * v1 - v2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

/// -60 dB, as a natural log: the decay and release times reach it.
const FALL: f32 = 6.9;

#[derive(Clone, Copy)]
struct Env {
    stage: Stage,
    level: f32,
}

impl Env {
    const fn new() -> Env {
        Env { stage: Stage::Attack, level: 0.0 }
    }

    fn release(&mut self) {
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }

    fn step(&mut self, adsr: &Adsr, dt: f32) -> f32 {
        match self.stage {
            Stage::Attack => {
                if adsr.a <= dt {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                } else {
                    self.level += dt / adsr.a;
                    if self.level >= 1.0 {
                        self.level = 1.0;
                        self.stage = Stage::Decay;
                    }
                }
            }
            Stage::Decay => {
                let s = adsr.s;
                self.level = s + (self.level - s) * (-dt * FALL / adsr.d.max(0.001)).exp();
                if (self.level - s).abs() < 1e-4 {
                    self.level = s;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = adsr.s,
            Stage::Release => {
                self.level *= (-dt * FALL / adsr.r.max(0.003)).exp();
                if self.level < 1e-4 {
                    self.level = 0.0;
                    self.stage = Stage::Off;
                }
            }
            Stage::Off => self.level = 0.0,
        }
        self.level
    }

    fn silent(&self) -> bool {
        self.stage == Stage::Off || (self.stage == Stage::Sustain && self.level < 1e-4)
    }
}

/// What the channel adds to a note, worked out by the synth each block.
#[derive(Clone, Copy, Default)]
pub struct Controls {
    /// Semitones: pitch bend and the channel's tuning.
    pub bend: f32,
    /// 0 to 1, the mod wheel: vibrato.
    pub modulation: f32,
    /// Octaves: CC 74.
    pub brightness: f32,
    /// 0 to 1: CC 71 (resonance) added.
    pub resonance: f32,
}

pub struct Voice {
    pub active: bool,
    pub channel: u8,
    pub key: u8,
    /// Its key is up (it may still sound, held by the pedal).
    pub released: bool,
    /// Its key is up but the sustain pedal holds it.
    pub sustained: bool,
    pub started: u64,
    pub choke: u8,
    choked: bool,
    gate: f32,
    pub patch: Patch,
    drum: Option<Drum>,
    rate: f32,
    vel: f32,
    t: f32,
    phase: [f32; 7],
    phase2: f32,
    tine_phase: f32,
    tine_mod_phase: f32,
    fb: [f32; 2],
    amp: Env,
    fenv: Env,
    amp_now: f32,
    lfo_phase: f32,
    lfo_held: f32,
    noise: Noise,
    f1: Svf,
    f2: Svf,
    coef: Coef,
    // Per block.
    inc: f32,
    /// Each unison copy's step.
    incs: [f32; 7],
    inc2: f32,
    width: f32,
    index: f32,
    tine_env: f32,
    // Pluck: the string.
    string: Vec<f32>,
    write: usize,
    delay: f32,
    loop_gain: f32,
    z1: f32,
    peak: f32,
    // Drums.
    metal_hp: Coef,
    metal: Svf,
    noise_coef: Coef,
}

impl Voice {
    pub fn new(rate: f32, seed: u32) -> Voice {
        // The longest string: about 15 Hz.
        let len = ((rate / 15.0) as usize).next_power_of_two();
        Voice {
            active: false,
            channel: 0,
            key: 0,
            released: false,
            sustained: false,
            started: 0,
            choke: 0,
            choked: false,
            gate: 1.0,
            patch: super::patch::BASE,
            drum: None,
            rate,
            vel: 0.0,
            t: 0.0,
            phase: [0.0; 7],
            phase2: 0.0,
            tine_phase: 0.0,
            tine_mod_phase: 0.0,
            fb: [0.0; 2],
            amp: Env::new(),
            fenv: Env::new(),
            amp_now: 0.0,
            lfo_phase: 0.0,
            lfo_held: 0.0,
            noise: Noise::new(seed.wrapping_mul(2_654_435_761)),
            f1: Svf::default(),
            f2: Svf::default(),
            coef: Coef::default(),
            inc: 0.0,
            incs: [0.0; 7],
            inc2: 0.0,
            width: 0.5,
            index: 0.0,
            tine_env: 0.0,
            string: vec![0.0; len],
            write: 0,
            delay: 1.0,
            loop_gain: 0.99,
            z1: 0.0,
            peak: 0.0,
            metal_hp: Coef::default(),
            metal: Svf::default(),
            noise_coef: Coef::default(),
        }
    }

    /// How loud it is now, for choosing which to steal.
    pub fn loudness(&self) -> f32 {
        self.amp_now * self.vel
    }

    fn reset(&mut self, channel: u8, key: u8, vel: u8, started: u64) {
        self.active = true;
        self.channel = channel;
        self.key = key;
        self.released = false;
        self.sustained = false;
        self.started = started;
        self.choked = false;
        self.gate = 1.0;
        self.vel = f32::from(vel) / 127.0;
        self.t = 0.0;
        self.amp = Env::new();
        self.fenv = Env::new();
        self.amp_now = 0.0;
        self.f1 = Svf::default();
        self.f2 = Svf::default();
        self.metal = Svf::default();
        self.fb = [0.0; 2];
        self.peak = 1.0;
        self.z1 = 0.0;
        // Free-running phases, staggered so unison copies don't start
        // together.
        for (i, p) in self.phase.iter_mut().enumerate() {
            *p = if i == 0 { 0.0 } else { (self.noise.next() + 1.0) * 0.5 };
        }
        self.phase2 = 0.0;
        self.tine_phase = 0.0;
        self.tine_mod_phase = 0.0;
        self.lfo_phase = 0.0;
        self.lfo_held = self.noise.next();
    }

    pub fn start(&mut self, channel: u8, key: u8, vel: u8, patch: &Patch, started: u64) {
        self.reset(channel, key, vel, started);
        self.patch = *patch;
        self.drum = None;
        self.choke = 0;
        if patch.engine == Engine::Pluck {
            self.pluck(key);
        }
    }

    pub fn start_drum(&mut self, channel: u8, key: u8, vel: u8, drum: Drum, started: u64) {
        self.reset(channel, key, vel, started);
        self.drum = Some(drum);
        self.choke = drum.choke;
        self.patch = super::patch::BASE;
        self.metal_hp = Coef::new(drum.metal_hp, 0.2, self.rate);
        self.noise_coef = Coef::new(drum.noise_cut, drum.noise_reso, self.rate);
        for p in &mut self.phase[..6] {
            *p = (self.noise.next() + 1.0) * 0.5;
        }
    }

    /// Fills the string with a burst of noise, as bright as the patch and
    /// the velocity make it.
    fn pluck(&mut self, key: u8) {
        let hz = 440.0 * 2f32.powf((f32::from(key) + self.patch.transpose - 69.0) / 12.0);
        let len = ((self.rate / hz) as usize + 2).min(self.string.len());
        let c = (0.1 + 0.9 * self.patch.bright * (0.4 + 0.6 * self.vel)).clamp(0.05, 1.0);
        self.string.fill(0.0);
        let mut lp = 0.0;
        for i in 0..len {
            lp += (self.noise.next() - lp) * c;
            self.string[i] = lp;
        }
        // Without its offset, the burst rings around zero.
        let mean = self.string[..len].iter().sum::<f32>() / len as f32;
        let mut most: f32 = 1e-6;
        for s in &mut self.string[..len] {
            *s -= mean;
            most = most.max(s.abs());
        }
        for s in &mut self.string[..len] {
            *s /= most;
        }
        self.write = len & (self.string.len() - 1);
    }

    /// The key's up: it starts its release (or waits for the pedal).
    pub fn note_off(&mut self, pedal: bool) {
        self.released = true;
        if pedal {
            self.sustained = true;
        } else {
            self.release();
        }
    }

    pub fn release(&mut self) {
        self.sustained = false;
        // A drum plays out whatever the key does.
        if self.drum.is_none() {
            self.amp.release();
            self.fenv.release();
        }
    }

    /// Cut off at once, quickly enough not to click: a choked hi-hat, a
    /// stolen voice, All Sound Off.
    pub fn choke_now(&mut self) {
        self.choked = true;
    }

    /// A choked voice fades out over a few milliseconds, then stops.
    fn fade_out(&mut self, out: &mut [f32]) {
        let step = 1.0 / (0.005 * self.rate);
        for o in out.iter_mut() {
            *o *= self.gate;
            self.gate = (self.gate - step).max(0.0);
        }
        if self.gate <= 0.0 {
            self.active = false;
        }
    }

    /// Renders `n` samples (at most `BLOCK`) into `out`, which is zeroed.
    pub fn render(&mut self, out: &mut [f32], ctl: &Controls) {
        if let Some(drum) = self.drum {
            self.render_drum(&drum, out);
        } else {
            self.render_note(out, ctl);
        }
        if self.choked {
            self.fade_out(out);
        }
    }

    fn render_note(&mut self, out: &mut [f32], ctl: &Controls) {
        let n = out.len();
        let rate = self.rate;
        let dt = n as f32 / rate;
        let p = self.patch;
        let key = f32::from(self.key);

        // The LFO.
        self.lfo_phase += p.lfo_rate * dt;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= self.lfo_phase.floor();
            self.lfo_held = self.noise.next();
        }
        let shape = match p.lfo {
            Lfo::Sine => sine(self.lfo_phase),
            Lfo::Square => {
                if self.lfo_phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Lfo::Saw => 1.0 - 2.0 * self.lfo_phase,
            Lfo::Random => self.lfo_held,
        };
        let fade = if p.lfo_delay > 0.0 { (self.t / p.lfo_delay).min(1.0) } else { 1.0 };
        let lfo = shape * fade;
        // The mod wheel's vibrato, at its own rate.
        let wheel = ctl.modulation * sine(self.t * 5.5) * 0.5;

        // Higher keys decay sooner.
        let scale = 2f32.powf(-p.key_decay * (key - 60.0) / 24.0);
        let mut aenv = p.aenv;
        aenv.d *= scale;
        if p.engine == Engine::Pluck {
            // The string decays itself; the envelope only ends it.
            aenv = Adsr { a: 0.0, d: 1.0, s: 1.0, r: p.aenv.r };
        }
        let fenv = self.fenv.step(&p.fenv, dt);
        let before = self.amp_now;
        let mut after = self.amp.step(&aenv, dt);
        if p.tremolo > 0.0 {
            after *= 1.0 - p.tremolo * (0.5 + 0.5 * lfo);
        }
        self.amp_now = after;

        // Pitch.
        let sweep = if p.sweep != 0.0 { p.sweep * (-self.t * FALL / p.fenv.d.max(0.001)).exp() } else { 0.0 };
        let note = key + p.transpose + ctl.bend + sweep + (p.vibrato * lfo) / 100.0 + wheel;
        let hz = 440.0 * 2f32.powf((note - 69.0) / 12.0);
        self.inc = hz / rate;
        let copies = usize::from(p.unison.clamp(1, 7));
        for (u, inc) in self.incs.iter_mut().take(copies).enumerate() {
            // Spread evenly across, -spread/2 to +spread/2.
            let cents = if copies > 1 { p.spread * (u as f32 / (copies - 1) as f32 - 0.5) } else { 0.0 };
            *inc = self.inc * 2f32.powf(cents / 1200.0);
        }
        let detune = 2f32.powf(p.detune / 1200.0);
        self.inc2 = self.inc * p.ratio2 * detune;
        if let Some(up) = p.sync {
            self.inc2 *= 2f32.powf(up * fenv / 12.0);
        }
        self.width = (p.pw + p.pwm * lfo * 0.4).clamp(0.05, 0.95);
        self.index = p.index * (p.index_sus + (1.0 - p.index_sus) * (-self.t * FALL / p.index_decay.max(0.001)).exp()) * (0.4 + 0.6 * self.vel);
        self.tine_env = if p.tine > 0.0 { p.tine * (-self.t * FALL / (p.tine_decay * scale).max(0.001)).exp() * (0.3 + 0.7 * self.vel) } else { 0.0 };

        if p.filter != Filter::Off {
            let octaves = p.key_track * (key - 60.0) / 12.0 + p.env_amt * fenv + p.wobble * lfo - p.vel_filter * (1.0 - self.vel) + ctl.brightness;
            self.coef = Coef::new(p.cutoff * 2f32.powf(octaves), (p.reso + ctl.resonance).min(1.0), rate);
        }

        if p.engine == Engine::Pluck {
            let ring = (p.aenv.d * scale).max(0.01);
            self.loop_gain = 0.001f32.powf(1.0 / (hz * ring)).min(0.99999);
            // The loop's filter delays it by part of a sample too.
            let filtered = 0.5 - 0.5 * p.bright;
            self.delay = (rate / hz - filtered).clamp(1.0, (self.string.len() - 2) as f32);
        }

        let level = p.gain * self.vel * self.vel;
        let step = (after - before) / n as f32;
        let mut amp = before;
        let mut peak: f32 = 0.0;
        for o in out.iter_mut() {
            amp += step;
            let mut s = match p.engine {
                Engine::Analog => self.analog(),
                Engine::Fm => self.fm(),
                Engine::Pluck => {
                    let s = self.string();
                    peak = peak.max(s.abs());
                    // A string loses its burst's top at once: made up, so
                    // it stands as loud as the other engines.
                    s * 2.5
                }
            };
            if self.tine_env > 1e-4 {
                self.tine_mod_phase += self.inc * p.tine_mod;
                wrap(&mut self.tine_mod_phase);
                self.tine_phase += self.inc * p.tine_ratio * detune;
                wrap(&mut self.tine_phase);
                s += self.tine_env * sine(self.tine_phase + p.tine_index * RADIAN * sine(self.tine_mod_phase));
            }
            if p.drive > 0.0 {
                let x = s * (1.0 + p.drive);
                s = x / (1.0 + x.abs());
            }
            if p.filter != Filter::Off {
                s = self.f1.tick(&self.coef, p.filter, s);
                if p.filter == Filter::Low24 {
                    s = self.f2.tick(&self.coef, Filter::Low, s);
                }
            }
            *o += s * amp * level;
        }
        self.t += dt;
        if p.engine == Engine::Pluck {
            self.peak = peak;
        }
        if self.amp.silent() || (p.engine == Engine::Pluck && self.t > 0.05 && self.peak < 1e-4) {
            self.active = false;
        }
    }

    #[inline]
    fn analog(&mut self) -> f32 {
        let p = &self.patch;
        let inc = self.inc;
        let mut s = 0.0;
        let mut wrapped = false;
        if p.wave1 == Wave::Organ {
            for (i, h) in BAR_HARMONICS.iter().enumerate() {
                if p.bars[i] > 0.0 {
                    self.phase[i] += inc * h;
                    wrap(&mut self.phase[i]);
                    s += p.bars[i] * sine(self.phase[i]);
                }
            }
            s *= 0.35;
        } else {
            let copies = usize::from(p.unison.clamp(1, 7));
            for u in 0..copies {
                let dt = self.incs[u];
                self.phase[u] += dt;
                let w = wrap(&mut self.phase[u]);
                if u == 0 {
                    wrapped = w;
                }
                s += osc(&mut self.noise, self.width, p.wave1, self.phase[u], dt);
            }
            if copies > 1 {
                s /= (copies as f32).sqrt();
            }
        }
        if p.mix2 > 0.0 {
            self.phase2 += self.inc2;
            wrap(&mut self.phase2);
            if p.sync.is_some() && wrapped {
                self.phase2 = self.phase[0] * self.inc2 / inc;
            }
            s += p.mix2 * osc(&mut self.noise, self.width, p.wave2, self.phase2, self.inc2);
        }
        if p.noise > 0.0 {
            s += p.noise * self.noise.next();
        }
        s
    }

    #[inline]
    fn fm(&mut self) -> f32 {
        let p = &self.patch;
        self.phase2 += self.inc2;
        wrap(&mut self.phase2);
        self.phase[0] += self.inc;
        wrap(&mut self.phase[0]);
        // Feedback from the average of the last two, as the DX7 did, so
        // it doesn't ring at half the sample rate.
        let m = sine(self.phase2 + p.feedback * RADIAN * 0.5 * (self.fb[0] + self.fb[1]));
        self.fb = [m, self.fb[0]];
        let mut s = sine(self.phase[0] + self.index * RADIAN * m);
        if p.noise > 0.0 {
            s += p.noise * self.noise.next();
        }
        s
    }

    #[inline]
    fn string(&mut self) -> f32 {
        let mask = self.string.len() - 1;
        let read = self.write as f32 - self.delay;
        let i = read.floor();
        let f = read - i;
        // Two's complement and a power of two: the mask wraps negatives too.
        let i = i as isize as usize;
        let y = self.string[i & mask] + (self.string[i.wrapping_add(1) & mask] - self.string[i & mask]) * f;
        // The loop's filter: brighter strings lose less of their top.
        let a = 0.5 + 0.5 * self.patch.bright;
        let s = a * y + (1.0 - a) * self.z1;
        self.z1 = y;
        self.string[self.write] = s * self.loop_gain;
        self.write = (self.write + 1) & mask;
        y
    }

    fn render_drum(&mut self, d: &Drum, out: &mut [f32]) {
        let rate = self.rate;
        let level = d.gain * self.vel * self.vel;
        let dt = 1.0 / rate;
        // Each part's envelope steps once a sample, by a factor worked
        // out once a block.
        let t0 = self.t;
        let tone_fall = (-dt * FALL / d.tone_decay.max(0.001)).exp();
        let metal_fall = (-dt * FALL / d.metal_decay.max(0.001)).exp();
        let mut tone_env = if d.tone_level > 0.0 { d.tone_level * (-t0 * FALL / d.tone_decay.max(0.001)).exp() } else { 0.0 };
        let mut metal_env = if d.metal_level > 0.0 { d.metal_level * (-t0 * FALL / d.metal_decay.max(0.001)).exp() } else { 0.0 };
        let hz = d.tone_end + (d.tone - d.tone_end) * (-t0 / d.sweep.max(0.0005)).exp();
        let inc = hz / rate;
        let mut t = t0;
        for o in out.iter_mut() {
            let mut s = 0.0;
            if tone_env > 1e-5 {
                self.phase2 += inc;
                wrap(&mut self.phase2);
                s += tone_env * sine(self.phase2);
                tone_env *= tone_fall;
            }
            if d.noise_level > 0.0 {
                let env = burst_env(d, t);
                if env > 1e-5 {
                    let x = self.noise.next();
                    s += d.noise_level * env * self.f1.tick(&self.noise_coef, d.noise_filter, x);
                }
            }
            if metal_env > 1e-5 {
                let mut m = 0.0;
                for (i, r) in METAL_RATIOS.iter().take(usize::from(d.metal_count)).enumerate() {
                    self.phase[i] += d.metal_freq * r / rate;
                    wrap(&mut self.phase[i]);
                    m += if self.phase[i] < 0.5 { 1.0 } else { -1.0 };
                }
                m /= f32::from(d.metal_count.max(1));
                s += metal_env * self.metal.tick(&self.metal_hp, Filter::High, m);
                metal_env *= metal_fall;
            }
            *o += s * level;
            t += dt;
        }
        self.amp_now = tone_env.max(metal_env).max(burst_env(d, t));
        self.t = t;
        if self.t > d.length() * 1.05 + 0.01 {
            self.active = false;
        }
    }

    /// Pans a drum by its place in the kit.
    pub fn drum_pan(&self) -> Option<f32> {
        self.drum.map(|d| d.pan)
    }

    pub fn drum_reverb(&self) -> f32 {
        self.drum.map_or(0.0, |d| d.reverb)
    }
}

/// The noise's level at `t`: each burst of a clap or guiro a short hit,
/// the last one its full decay.
fn burst_env(d: &Drum, t: f32) -> f32 {
    let n = d.bursts.max(1);
    let last = d.gap * f32::from(n - 1);
    if t >= last {
        return (-(t - last) * FALL / d.noise_decay.max(0.001)).exp();
    }
    let since = t - d.gap * (t / d.gap).floor();
    (-since * FALL / (d.gap * 1.2).max(0.001)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_sine_is_close() {
        for i in 0..1000 {
            let p = i as f32 / 997.0 * 3.0 - 1.0;
            assert!((sine(p) - (p * 2.0 * PI).sin()).abs() < 1e-3, "{p}");
        }
    }

    #[test]
    fn an_envelope_attacks_decays_and_releases() {
        let adsr = Adsr { a: 0.01, d: 0.1, s: 0.5, r: 0.1 };
        let mut env = Env::new();
        let dt = 0.001;
        for _ in 0..10 {
            env.step(&adsr, dt);
        }
        assert!(env.level > 0.95);
        for _ in 0..200 {
            env.step(&adsr, dt);
        }
        assert!((env.level - 0.5).abs() < 0.01);
        env.release();
        for _ in 0..150 {
            env.step(&adsr, dt);
        }
        assert!(env.silent());
    }

    fn sound(patch: &Patch, key: u8, seconds: f32) -> Vec<f32> {
        let mut v = Voice::new(44_100.0, 1);
        v.start(0, key, 100, patch, 0);
        let mut out = vec![0.0; (seconds * 44_100.0) as usize];
        for chunk in out.chunks_mut(BLOCK) {
            v.render(chunk, &Controls::default());
        }
        out
    }

    /// The pitch, by autocorrelation: the lag the sound best repeats at,
    /// between 100 and 1000 Hz.
    fn pitch(s: &[f32]) -> f32 {
        let s = &s[..8192];
        let score = |lag: usize| s.iter().zip(&s[lag..]).map(|(a, b)| a * b).sum::<f32>();
        let best = (44..441).max_by(|a, b| score(*a).total_cmp(&score(*b))).unwrap();
        // Between whole samples: the peak of a parabola through three.
        let (l, c, r) = (score(best - 1), score(best), score(best + 1));
        let shift = 0.5 * (l - r) / (l - 2.0 * c + r);
        44_100.0 / (best as f32 + shift)
    }

    #[test]
    fn every_engine_plays_in_tune() {
        let a440 = [
            Patch::analog("saw", Wave::Saw).lp(1200.0, 0.0),
            Patch::fm("fm", 1.0, 0.5),
            Patch::pluck("pluck", 3.0, 0.3),
            Patch::analog("sine", Wave::Sine),
        ];
        for p in &a440 {
            let s = sound(p, 69, 0.5);
            let hz = pitch(&s[4410..]);
            assert!((hz - 440.0).abs() < 3.0, "{}: {hz}", p.name);
        }
    }

    #[test]
    fn a_plucked_string_dies_away() {
        let s = sound(&Patch::pluck("pluck", 0.3, 0.5), 60, 1.0);
        let start = s[..4410].iter().fold(0f32, |m, x| m.max(x.abs()));
        let end = s[40_000..].iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!(start > 0.1 && end < start * 0.01, "{start} {end}");
    }

    #[test]
    fn a_voice_with_no_sustain_ends_itself() {
        let mut v = Voice::new(44_100.0, 1);
        v.start(0, 60, 100, &Patch::fm("bell", 3.5, 2.0).amp(0.0, 0.2, 0.0, 0.1), 0);
        let mut out = [0.0; BLOCK];
        for _ in 0..(44_100 / BLOCK) {
            v.render(&mut out, &Controls::default());
        }
        assert!(!v.active);
    }

    #[test]
    fn a_drum_ends_itself() {
        let mut v = Voice::new(44_100.0, 1);
        v.start_drum(9, 36, 100, super::super::drums::drum(0, 36).unwrap(), 0);
        let mut out = [0.0; BLOCK];
        let mut loud: f32 = 0.0;
        for _ in 0..(44_100 / BLOCK) {
            out.fill(0.0);
            v.render(&mut out, &Controls::default());
            loud = loud.max(out.iter().fold(0.0, |m, x| m.max(x.abs())));
        }
        assert!(loud > 0.1);
        assert!(!v.active);
    }
}
