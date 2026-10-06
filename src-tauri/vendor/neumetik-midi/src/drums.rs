// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Neumetik's drums: every key of GM's and GS's drum map (27 to 87) as
//! a recipe of three parts, each with its own decay, and the kits a drum
//! part's program change picks: GS's own (Standard, Room, Power,
//! Electronic, TR-808, Jazz, Brush, Orchestra, SFX) and the classic drum
//! machines where the SC-88 put them (Dance, CR-78, TR-606, TR-707,
//! TR-909), plus Neumetik's LinnDrum and Simmons.
//!
//! - **Tone**: a sine falling from `tone` to `tone_end` Hz over `sweep`
//!   seconds, the drum's body (a kick's thump, a tom, a woodblock).
//! - **Noise**: white noise through a filter, the snare's wires, a clap
//!   (`bursts` hits `gap` apart), a shaker, a guiro.
//! - **Metal**: up to six square waves at the TR-808's cymbal ratios
//!   through a high-pass, the hi-hats, cymbals, cowbell and agogo.

use super::patch::Filter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drum {
    pub tone: f32,
    pub tone_end: f32,
    pub sweep: f32,
    pub tone_decay: f32,
    pub tone_level: f32,
    pub noise_level: f32,
    pub noise_filter: Filter,
    pub noise_cut: f32,
    pub noise_reso: f32,
    pub noise_decay: f32,
    pub bursts: u8,
    pub gap: f32,
    pub metal_level: f32,
    pub metal_freq: f32,
    pub metal_count: u8,
    pub metal_hp: f32,
    pub metal_decay: f32,
    /// -1 left to 1 right, as a drummer faces the kit.
    pub pan: f32,
    /// Drums in the same group cut each other off (the hi-hats, 1;
    /// the whistles, the guiros, the cuicas and the triangles their own).
    pub choke: u8,
    pub reverb: f32,
    pub gain: f32,
}

/// The TR-808's six cymbal oscillators, as multiples of the lowest.
pub const METAL_RATIOS: [f32; 6] = [1.0, 1.4471, 1.617, 1.9265, 2.5028, 2.6637];

const SILENT: Drum = Drum {
    tone: 0.0,
    tone_end: 0.0,
    sweep: 0.01,
    tone_decay: 0.1,
    tone_level: 0.0,
    noise_level: 0.0,
    noise_filter: Filter::Off,
    noise_cut: 5000.0,
    noise_reso: 0.0,
    noise_decay: 0.1,
    bursts: 1,
    gap: 0.01,
    metal_level: 0.0,
    metal_freq: 400.0,
    metal_count: 6,
    metal_hp: 6000.0,
    metal_decay: 0.1,
    pan: 0.0,
    choke: 0,
    reverb: 0.0,
    gain: 1.0,
};

/// A body that falls from `from` to `to` Hz.
const fn tone(from: f32, to: f32, sweep: f32, decay: f32) -> Drum {
    Drum { tone: from, tone_end: to, sweep, tone_decay: decay, tone_level: 1.0, ..SILENT }
}

const fn noise(kind: Filter, cut: f32, reso: f32, decay: f32) -> Drum {
    Drum { noise_level: 1.0, noise_filter: kind, noise_cut: cut, noise_reso: reso, noise_decay: decay, ..SILENT }
}

const fn metal(freq: f32, count: u8, hp: f32, decay: f32) -> Drum {
    Drum { metal_level: 1.0, metal_freq: freq, metal_count: count, metal_hp: hp, metal_decay: decay, ..SILENT }
}

impl Drum {
    const fn with_tone(mut self, from: f32, to: f32, sweep: f32, decay: f32, level: f32) -> Drum {
        self.tone = from;
        self.tone_end = to;
        self.sweep = sweep;
        self.tone_decay = decay;
        self.tone_level = level;
        self
    }

    const fn with_noise(mut self, kind: Filter, cut: f32, reso: f32, decay: f32, level: f32) -> Drum {
        self.noise_filter = kind;
        self.noise_cut = cut;
        self.noise_reso = reso;
        self.noise_decay = decay;
        self.noise_level = level;
        self
    }

    const fn with_metal(mut self, freq: f32, count: u8, hp: f32, decay: f32, level: f32) -> Drum {
        self.metal_freq = freq;
        self.metal_count = count;
        self.metal_hp = hp;
        self.metal_decay = decay;
        self.metal_level = level;
        self
    }

    const fn bursts(mut self, count: u8, gap: f32) -> Drum {
        self.bursts = count;
        self.gap = gap;
        self
    }

    const fn pan(mut self, pan: f32) -> Drum {
        self.pan = pan;
        self
    }

    const fn choke(mut self, group: u8) -> Drum {
        self.choke = group;
        self
    }

    const fn gain(mut self, gain: f32) -> Drum {
        self.gain = gain;
        self
    }

    /// How long it sounds, the longest of its parts plus its bursts.
    pub fn length(&self) -> f32 {
        let mut most: f32 = 0.0;
        if self.tone_level > 0.0 {
            most = most.max(self.tone_decay);
        }
        if self.noise_level > 0.0 {
            most = most.max(self.noise_decay + self.gap * f32::from(self.bursts.saturating_sub(1)));
        }
        if self.metal_level > 0.0 {
            most = most.max(self.metal_decay);
        }
        most
    }
}

/// The GM and GS kits, by the program change that picks them on a drum
/// part, and Neumetik's.
pub const KITS: [(u8, &str); 16] = [
    (0, "Standard"),
    (8, "Room"),
    (16, "Power"),
    (24, "Electronic"),
    (25, "Analog Boom"),
    (26, "Dance"),
    (27, "Rhythm Box"),
    (28, "Analog Tick"),
    (29, "Digital Machine"),
    (30, "Analog Punch"),
    (32, "Jazz"),
    (40, "Brush"),
    (48, "Orchestra"),
    (56, "SFX"),
    (64, "Studio Machine"),
    (72, "Hex Pads"),
];

/// The kit a program change picks: its own, or the nearest below it,
/// as the Sound Canvas does (so 1 to 7 are Standard's variations).
pub fn kit_for(program: u8) -> u8 {
    KITS.iter().rev().find(|(p, _)| *p <= program).map_or(0, |(p, _)| *p)
}

/// The drum `key` plays in `kit`, if any.
pub fn drum(kit: u8, key: u8) -> Option<Drum> {
    let base = standard(key)?;
    let mut d = match kit {
        8 => room(key, base),
        16 => power(key, base),
        24 | 72 => electronic(key, base),
        25 => tr808(key).unwrap_or(base),
        26 | 30 => tr909(key).unwrap_or(base),
        27 => cr78(key).unwrap_or(base),
        28 => tr606(key).unwrap_or(base),
        29 | 64 => linn(key).unwrap_or(base),
        32 => jazz(key, base),
        40 => brush(key, base),
        48 => orchestra(key, base),
        _ => base,
    };
    if kit == 26 {
        // Dance: the 909, punchier.
        d.gain *= 1.15;
    }
    if kit == 72 {
        // Simmons: the hexagonal pads' long, falling toms.
        if is_tom(key) {
            d.sweep *= 1.6;
            d.tone *= 1.3;
            d.tone_decay *= 1.3;
        }
    }
    d.reverb += match kit {
        8 => 0.35,
        16 => 0.15,
        48 => 0.4,
        _ => 0.0,
    };
    Some(d)
}

fn is_tom(key: u8) -> bool {
    matches!(key, 41 | 43 | 45 | 47 | 48 | 50)
}

/// The six toms' pitches, low floor to high.
fn tom_hz(key: u8) -> f32 {
    match key {
        41 => 82.0,
        43 => 98.0,
        45 => 117.0,
        47 => 139.0,
        48 => 165.0,
        _ => 196.0,
    }
}

/// Toms spread across, low on the right as a drummer sees them.
fn tom_pan(key: u8) -> f32 {
    match key {
        41 => 0.5,
        43 => 0.35,
        45 => 0.2,
        47 => 0.0,
        48 => -0.2,
        _ => -0.35,
    }
}

/// GM's Standard kit, with GS's extra keys (27 to 34 and 82 to 87).
fn standard(key: u8) -> Option<Drum> {
    use Filter::*;
    Some(match key {
        27 => tone(2400.0, 500.0, 0.008, 0.07).gain(0.6),
        28 => noise(Band, 2200.0, 0.3, 0.05).bursts(2, 0.006).gain(0.8),
        29 => noise(Band, 1400.0, 0.6, 0.15).with_tone(500.0, 900.0, 0.1, 0.12, 0.3),
        30 => noise(Band, 1100.0, 0.6, 0.15).with_tone(900.0, 450.0, 0.1, 0.12, 0.3),
        31 => tone(2400.0, 2300.0, 0.01, 0.03).with_noise(Band, 4000.0, 0.2, 0.02, 0.5).gain(0.7),
        32 => tone(3000.0, 3000.0, 0.01, 0.012).gain(0.6),
        33 => tone(1800.0, 1800.0, 0.01, 0.025).gain(0.6),
        34 => metal(1250.0, 2, 1000.0, 0.35).with_tone(2500.0, 2500.0, 0.01, 0.4, 0.4).gain(0.5),
        35 => tone(110.0, 50.0, 0.035, 0.35).with_noise(Low, 1500.0, 0.0, 0.02, 0.25).gain(1.2),
        36 => tone(130.0, 55.0, 0.025, 0.4).with_noise(Low, 2500.0, 0.0, 0.015, 0.3).gain(1.2),
        37 => tone(520.0, 480.0, 0.01, 0.04).with_noise(Band, 3000.0, 0.3, 0.03, 0.6).gain(0.7),
        38 => tone(200.0, 175.0, 0.02, 0.12).with_noise(Band, 4200.0, 0.05, 0.22, 0.9),
        39 => noise(Band, 1200.0, 0.35, 0.22).bursts(4, 0.011).gain(1.1),
        40 => tone(230.0, 200.0, 0.015, 0.08).with_noise(High, 2000.0, 0.1, 0.18, 1.0),
        41 | 43 | 45 | 47 | 48 | 50 => {
            let hz = tom_hz(key);
            tone(hz * 1.25, hz, 0.08, 0.5).with_noise(Low, 1200.0, 0.0, 0.04, 0.15).pan(tom_pan(key))
        }
        42 => metal(330.0, 6, 7000.0, 0.06).with_noise(High, 8000.0, 0.0, 0.05, 0.4).choke(1).pan(-0.3).gain(0.55),
        44 => metal(330.0, 6, 7000.0, 0.09).with_noise(High, 7000.0, 0.0, 0.07, 0.3).choke(1).pan(-0.3).gain(0.45),
        46 => metal(330.0, 6, 6500.0, 0.55).with_noise(High, 7500.0, 0.0, 0.45, 0.4).choke(1).pan(-0.3).gain(0.5),
        49 => metal(340.0, 6, 5000.0, 1.8).with_noise(High, 5000.0, 0.0, 1.5, 0.7).pan(-0.45).gain(0.5),
        51 => metal(380.0, 6, 4000.0, 1.6).with_noise(High, 6000.0, 0.0, 0.8, 0.2).pan(0.45).gain(0.4),
        52 => metal(300.0, 6, 3000.0, 1.1).with_noise(Band, 4000.0, 0.4, 0.9, 0.6).pan(0.55).gain(0.45),
        53 => metal(700.0, 3, 1500.0, 1.0).pan(0.45).gain(0.4),
        54 => noise(High, 7000.0, 0.2, 0.2).bursts(2, 0.02).with_metal(2400.0, 6, 6000.0, 0.2, 0.5).pan(0.3).gain(0.5),
        55 => metal(420.0, 6, 6000.0, 0.7).with_noise(High, 7000.0, 0.0, 0.6, 0.6).pan(-0.5).gain(0.45),
        56 => metal(540.0, 2, 400.0, 0.3).pan(0.3).gain(0.5),
        57 => metal(370.0, 6, 5000.0, 2.0).with_noise(High, 5000.0, 0.0, 1.7, 0.7).pan(0.5).gain(0.5),
        58 => noise(Band, 3000.0, 0.7, 0.8).bursts(12, 0.03).gain(0.4),
        59 => metal(360.0, 6, 4500.0, 1.7).with_noise(High, 6000.0, 0.0, 0.8, 0.2).pan(0.5).gain(0.4),
        60 => tone(420.0, 400.0, 0.01, 0.12).pan(-0.25),
        61 => tone(310.0, 290.0, 0.01, 0.16).pan(-0.25),
        62 => tone(340.0, 330.0, 0.01, 0.06).with_noise(Band, 2000.0, 0.2, 0.03, 0.3).pan(0.25),
        63 => tone(340.0, 330.0, 0.01, 0.25).pan(0.25),
        64 => tone(230.0, 220.0, 0.01, 0.3).pan(0.3),
        65 => tone(540.0, 520.0, 0.01, 0.25).with_metal(900.0, 2, 800.0, 0.2, 0.3).pan(-0.3).gain(0.7),
        66 => tone(390.0, 380.0, 0.01, 0.3).with_metal(650.0, 2, 600.0, 0.25, 0.3).pan(-0.3).gain(0.7),
        67 => metal(1050.0, 2, 600.0, 0.3).pan(0.4).gain(0.5),
        68 => metal(700.0, 2, 400.0, 0.35).pan(0.4).gain(0.5),
        69 => noise(High, 6000.0, 0.1, 0.1).bursts(3, 0.02).pan(-0.4).gain(0.5),
        70 => noise(High, 5000.0, 0.1, 0.06).pan(-0.4).gain(0.5),
        71 => tone(2450.0, 2400.0, 0.02, 0.25).choke(2).gain(0.4),
        72 => tone(2450.0, 2400.0, 0.02, 0.9).choke(2).gain(0.4),
        73 => noise(Band, 2500.0, 0.5, 0.06).bursts(6, 0.015).choke(3).pan(0.4).gain(0.6),
        74 => noise(Band, 2500.0, 0.5, 0.08).bursts(14, 0.02).choke(3).pan(0.4).gain(0.6),
        75 => tone(2500.0, 2500.0, 0.01, 0.08).pan(-0.3).gain(0.7),
        76 => tone(1200.0, 1150.0, 0.01, 0.06).with_noise(Band, 2500.0, 0.2, 0.01, 0.2).pan(0.2).gain(0.7),
        77 => tone(800.0, 780.0, 0.01, 0.07).with_noise(Band, 1800.0, 0.2, 0.01, 0.2).pan(0.2).gain(0.7),
        78 => tone(500.0, 750.0, 0.06, 0.12).choke(4).pan(-0.2).gain(0.5),
        79 => tone(380.0, 250.0, 0.15, 0.3).choke(4).pan(-0.2).gain(0.5),
        80 => tone(4200.0, 4200.0, 0.01, 0.15).with_metal(4200.0, 2, 3000.0, 0.1, 0.2).choke(5).pan(0.4).gain(0.35),
        81 => tone(4200.0, 4200.0, 0.01, 1.5).with_metal(4200.0, 2, 3000.0, 0.8, 0.2).choke(5).pan(0.4).gain(0.35),
        82 => noise(High, 5000.0, 0.1, 0.08).bursts(2, 0.03).pan(-0.3).gain(0.5),
        83 => noise(High, 7000.0, 0.5, 0.1).bursts(4, 0.03).with_metal(2000.0, 6, 5000.0, 0.4, 0.5).pan(0.3).gain(0.4),
        84 => metal(3000.0, 3, 2000.0, 1.5).with_tone(5000.0, 3000.0, 0.5, 1.5, 0.4).pan(-0.2).gain(0.35),
        85 => tone(2000.0, 2000.0, 0.01, 0.03).with_noise(Band, 3000.0, 0.3, 0.03, 0.6).bursts(2, 0.012).pan(0.2).gain(0.6),
        86 => tone(85.0, 80.0, 0.01, 0.15).pan(-0.2),
        87 => tone(85.0, 80.0, 0.01, 0.6).pan(-0.2),
        _ => return None,
    })
}

/// Room: a smaller kit in a live room (the reverb's added in `drum`).
fn room(key: u8, mut d: Drum) -> Drum {
    if is_tom(key) {
        d.tone_decay *= 0.7;
        d.tone *= 1.08;
        d.tone_end *= 1.08;
    }
    d
}

/// Power: big, gated eighties drums.
fn power(key: u8, mut d: Drum) -> Drum {
    match key {
        35 | 36 => {
            d.tone_decay = 0.5;
            d.tone_end *= 0.9;
            d.noise_level = 0.45;
            d.gain *= 1.15;
        }
        38 | 40 => {
            d.noise_decay = 0.3;
            d.noise_cut = 3000.0;
            d.tone_level = 1.2;
            d.gain *= 1.2;
        }
        k if is_tom(k) => {
            d.tone_decay *= 1.3;
            d.noise_level = 0.3;
        }
        _ => {}
    }
    d
}

/// Electronic: the Simmons SDS-V's toms and snare.
fn electronic(key: u8, mut d: Drum) -> Drum {
    match key {
        35 | 36 => {
            d = tone(160.0, 50.0, 0.05, 0.5).with_noise(Filter::Low, 800.0, 0.0, 0.03, 0.3).gain(1.2);
        }
        38 | 40 => {
            d = tone(320.0, 180.0, 0.05, 0.2).with_noise(Filter::Band, 2500.0, 0.2, 0.3, 0.9);
        }
        k if is_tom(k) => {
            let hz = tom_hz(k) * 1.4;
            d = tone(hz * 2.0, hz * 0.8, 0.25, 0.7).with_noise(Filter::Low, 1500.0, 0.0, 0.1, 0.25).pan(tom_pan(k));
        }
        _ => {}
    }
    d
}

/// The Roland TR-808 (1980): a boom that rings, a snappy snare, its
/// cymbal circuit, the clap, cowbell, clave, rimshot and congas.
fn tr808(key: u8) -> Option<Drum> {
    use Filter::*;
    Some(match key {
        35 => tone(70.0, 45.0, 0.015, 1.4).gain(1.4),
        36 => tone(80.0, 50.0, 0.012, 0.9).gain(1.4),
        37 => tone(1700.0, 1700.0, 0.01, 0.02).with_noise(Band, 2000.0, 0.3, 0.01, 0.3).gain(0.7),
        38 | 40 => tone(240.0, 180.0, 0.01, 0.1).with_noise(High, 1800.0, 0.0, 0.2, 0.8),
        39 => noise(Band, 1100.0, 0.4, 0.3).bursts(4, 0.01).gain(1.1),
        41 | 43 | 45 | 47 | 48 | 50 => {
            let hz = tom_hz(key) * 1.1;
            tone(hz * 1.15, hz, 0.06, 0.6).with_noise(Low, 1000.0, 0.0, 0.03, 0.08).pan(tom_pan(key))
        }
        42 | 44 => metal(205.0, 6, 8000.0, 0.05).choke(1).pan(-0.3).gain(0.6),
        46 => metal(205.0, 6, 7500.0, 0.45).choke(1).pan(-0.3).gain(0.55),
        49 | 57 => metal(205.0, 6, 4500.0, 2.2).with_noise(High, 6000.0, 0.0, 1.2, 0.3).pan(-0.4).gain(0.5),
        56 => metal(540.0, 2, 500.0, 0.25).gain(0.55),
        62..=64 => {
            let hz = [370.0, 280.0, 185.0][usize::from(key - 62)];
            tone(hz, hz, 0.01, 0.3).pan(0.25)
        }
        70 => noise(High, 6000.0, 0.0, 0.04).gain(0.5),
        75 => tone(2500.0, 2500.0, 0.01, 0.06).gain(0.7),
        _ => return None,
    })
}

/// The Roland TR-909 (1983): a punchy kick with a click, its noisy
/// snare and clap, and (sampled on the real one) hats and cymbals.
fn tr909(key: u8) -> Option<Drum> {
    use Filter::*;
    Some(match key {
        35 | 36 => tone(200.0, 52.0, 0.02, 0.5).with_noise(Low, 4000.0, 0.0, 0.008, 0.6).gain(1.4),
        38 | 40 => tone(220.0, 190.0, 0.015, 0.12).with_noise(Low, 7000.0, 0.0, 0.2, 1.0),
        39 => noise(Band, 1300.0, 0.3, 0.25).bursts(4, 0.009).gain(1.1),
        41 | 43 | 45 | 47 | 48 | 50 => {
            let hz = tom_hz(key) * 1.2;
            tone(hz * 1.5, hz, 0.05, 0.45).with_noise(Low, 2000.0, 0.0, 0.03, 0.2).pan(tom_pan(key))
        }
        42 | 44 => metal(420.0, 6, 9000.0, 0.05).with_noise(High, 9000.0, 0.0, 0.05, 0.6).choke(1).pan(-0.3).gain(0.55),
        46 => metal(420.0, 6, 8500.0, 0.4).with_noise(High, 9000.0, 0.0, 0.35, 0.6).choke(1).pan(-0.3).gain(0.5),
        49 | 57 => metal(450.0, 6, 6000.0, 1.6).with_noise(High, 6000.0, 0.0, 1.4, 0.9).pan(-0.4).gain(0.45),
        51 | 59 => metal(480.0, 6, 5000.0, 1.4).with_noise(High, 7000.0, 0.0, 0.7, 0.3).pan(0.4).gain(0.4),
        _ => return None,
    })
}

/// The Roland CR-78 (1978): soft, round, a little tick on everything.
fn cr78(key: u8) -> Option<Drum> {
    use Filter::*;
    Some(match key {
        35 | 36 => tone(90.0, 70.0, 0.02, 0.25).gain(1.2),
        38 | 40 => tone(400.0, 380.0, 0.01, 0.06).with_noise(Band, 5000.0, 0.2, 0.12, 0.6),
        42 | 44 => noise(High, 9000.0, 0.3, 0.03).choke(1).gain(0.5),
        46 => noise(High, 8000.0, 0.3, 0.25).choke(1).gain(0.45),
        54 => noise(High, 8000.0, 0.4, 0.15).bursts(2, 0.02).gain(0.5),
        56 => metal(800.0, 2, 600.0, 0.15).gain(0.5),
        70 => noise(High, 7000.0, 0.2, 0.05).gain(0.5),
        75 => tone(2600.0, 2600.0, 0.01, 0.05).gain(0.7),
        _ => return None,
    })
}

/// The Roland TR-606 (1981), the TB-303's partner: thin and clicky.
fn tr606(key: u8) -> Option<Drum> {
    use Filter::*;
    Some(match key {
        35 | 36 => tone(110.0, 60.0, 0.01, 0.25).gain(1.2),
        38 | 40 => tone(280.0, 250.0, 0.01, 0.07).with_noise(High, 3000.0, 0.0, 0.12, 0.9),
        42 | 44 => metal(320.0, 6, 9000.0, 0.035).choke(1).gain(0.5),
        46 => metal(320.0, 6, 8500.0, 0.3).choke(1).gain(0.45),
        49 | 57 => metal(320.0, 6, 5000.0, 1.0).gain(0.45),
        _ => return None,
    })
}

/// The Roland TR-707 and the LinnDrum (both sampled, 1984 and 1982):
/// tight kicks, snappy snares and dry claps.
fn linn(key: u8) -> Option<Drum> {
    use Filter::*;
    Some(match key {
        35 | 36 => tone(140.0, 58.0, 0.02, 0.3).with_noise(Low, 3000.0, 0.0, 0.01, 0.4).gain(1.3),
        38 | 40 => tone(210.0, 185.0, 0.01, 0.1).with_noise(Band, 5000.0, 0.1, 0.17, 1.0),
        39 => noise(Band, 1500.0, 0.3, 0.15).bursts(3, 0.008).gain(1.1),
        42 | 44 => noise(High, 9000.0, 0.2, 0.05).with_metal(400.0, 6, 9000.0, 0.04, 0.4).choke(1).gain(0.5),
        46 => noise(High, 8500.0, 0.2, 0.4).with_metal(400.0, 6, 8000.0, 0.35, 0.4).choke(1).gain(0.45),
        _ => return None,
    })
}

/// Jazz: a smaller kick, a ringing snare, everything softer.
fn jazz(key: u8, mut d: Drum) -> Drum {
    match key {
        35 | 36 => {
            d.tone = 90.0;
            d.tone_end = 65.0;
            d.tone_decay = 0.3;
            d.gain *= 0.85;
        }
        38 | 40 => {
            d.noise_decay = 0.3;
            d.gain *= 0.85;
        }
        _ => {}
    }
    d
}

/// Brush: brushes on the snare and toms, swished.
fn brush(key: u8, mut d: Drum) -> Drum {
    match key {
        38 | 40 => {
            d = noise(Filter::Low, 5000.0, 0.0, 0.35).with_tone(190.0, 180.0, 0.02, 0.06, 0.3).bursts(3, 0.03).gain(0.8);
        }
        39 => {
            d = noise(Filter::Low, 4000.0, 0.0, 0.5).bursts(6, 0.04).gain(0.6);
        }
        k if is_tom(k) => {
            d.noise_level = 0.5;
            d.noise_decay = 0.2;
            d.gain *= 0.8;
        }
        _ => {}
    }
    d
}

/// Orchestra: the concert bass drum and snare, the timpani tuned across
/// 41 to 53, cymbals that ring.
fn orchestra(key: u8, mut d: Drum) -> Drum {
    match key {
        35 | 36 => {
            d = tone(60.0, 48.0, 0.1, 1.6).with_noise(Filter::Low, 600.0, 0.0, 0.3, 0.4).gain(1.3);
        }
        38 | 40 => {
            d = tone(220.0, 200.0, 0.01, 0.1).with_noise(Filter::Band, 5000.0, 0.0, 0.35, 0.8);
        }
        41..=53 if !matches!(key, 42 | 44 | 46 | 49 | 51 | 52) => {
            // F to F, as the SC-55's: timpani.
            let hz = 87.31 * 2f32.powf(f32::from(key - 41) / 12.0);
            d = tone(hz * 1.02, hz, 0.1, 1.6).with_noise(Filter::Low, 600.0, 0.0, 0.1, 0.25);
        }
        49 | 57 | 52 | 55 => {
            d.metal_decay *= 1.5;
            d.noise_decay *= 1.5;
        }
        _ => {}
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_of_the_gs_map_has_a_drum_in_every_kit() {
        for (kit, name) in KITS {
            for key in 27..=87 {
                let d = drum(kit, key).unwrap_or_else(|| panic!("{name} has no {key}"));
                assert!(d.length() > 0.0, "{name} {key}");
            }
            assert!(drum(kit, 26).is_none());
            assert!(drum(kit, 88).is_none());
        }
    }

    #[test]
    fn a_program_between_kits_picks_the_one_below() {
        assert_eq!(kit_for(0), 0);
        assert_eq!(kit_for(5), 0);
        assert_eq!(kit_for(25), 25);
        assert_eq!(kit_for(31), 30);
        assert_eq!(kit_for(127), 72);
    }

    #[test]
    fn the_hats_choke_each_other() {
        for key in [42, 44, 46] {
            assert_eq!(drum(0, key).unwrap().choke, 1);
        }
    }
}
