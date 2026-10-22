// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

use std::sync::Arc;

use super::generator as g;
use super::*;
use crate::bank::make;
use crate::smf::tests::file;
use crate::{SoundBank, Synth};

const RATE: u32 = 44_100;

fn sine_sample(name: &str, root: u8) -> Sample {
    let s = make::sine(100, 20, 44_000, root);
    let (a, b) = s.looped.unwrap();
    Sample::new(name, s.data, s.rate, root).looped(a, b)
}

/// A SoundFont of one looped sine (440 Hz at key 69) on program 0.
fn one_sine() -> SoundFont {
    let mut sf = SoundFont::new("Sines");
    let s = sf.add_sample(sine_sample("A440", 69));
    let mut inst = Instrument::new("Sine");
    inst.zones.push(Zone::new(s).with(g::SAMPLE_MODES, 1));
    let i = sf.add_instrument(inst);
    sf.add_preset(Preset::new("Sine", 0, 0, i));
    sf
}

fn play(bank: &Arc<SoundBank>, channel: u8, program: u8, key: u8, vel: u8, seconds: f32) -> Vec<f32> {
    let mut s = Synth::new(RATE);
    s.set_sound_bank(Some(bank.clone()));
    s.program_change(channel, program);
    s.note_on(channel, key, vel);
    let n = (seconds * RATE as f32) as usize;
    let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
    s.render(&mut l, &mut r);
    l.iter().zip(&r).map(|(a, b)| a + b).collect()
}

fn rms(s: &[f32]) -> f32 {
    (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
}

/// Hz, by counting the upward zero crossings.
fn pitch(s: &[f32]) -> f32 {
    let ups: Vec<usize> = (1..s.len()).filter(|&i| s[i - 1] < 0.0 && s[i] >= 0.0).collect();
    let (first, last) = (ups[0], ups[ups.len() - 1]);
    (ups.len() - 1) as f32 * RATE as f32 / (last - first) as f32
}

fn bank(sf: &SoundFont) -> Arc<SoundBank> {
    Arc::new(SoundBank::from_bytes(&sf.to_bytes().unwrap()).unwrap())
}

#[test]
fn a_soundfont_made_from_scratch_plays_in_tune() {
    let b = bank(&one_sine());
    assert_eq!(b.name(), "Sines");
    let out = play(&b, 0, 0, 69, 100, 0.5);
    assert!((pitch(&out[4410..]) - 440.0).abs() < 2.0, "{}", pitch(&out[4410..]));
    let out = play(&b, 0, 0, 81, 100, 0.5);
    assert!((pitch(&out[4410..]) - 880.0).abs() < 4.0);
}

#[test]
fn writing_and_opening_again_changes_nothing() {
    let mut sf = one_sine();
    sf.info.author = "Somebody".into();
    sf.info.comment = "A comment, with an odd length".into();
    let (l, r) = sf.add_stereo(Sample::new("L", vec![1, 2, 3], 22_050, 60), Sample::new("R", vec![4, 5, 6], 22_050, 60));
    let mut inst = Instrument::new("Stereo");
    inst.global = Zone::default().with(g::PAN, 0).with(g::VOL_ENV_RELEASE, -1200);
    inst.global.modulators.push(Modulator { source: 0x0081, destination: g::VIB_LFO_TO_PITCH, amount: 20, amount_source: 0, transform: 0 });
    inst.zones.push(Zone::new(l).keys(0, 63).vels(10, 127).with(g::PAN, -500));
    inst.zones.push(Zone::new(r).keys(0, 63).with(g::PAN, 500));
    let i = sf.add_instrument(inst);
    let mut preset = Preset::new("Kit", 128, 0, i);
    preset.global = Zone::default().with(g::ATTENUATION, 30);
    sf.add_preset(preset);
    let bytes = sf.to_bytes().unwrap();
    let again = SoundFont::from_bytes(&bytes).unwrap();
    assert_eq!(again, sf);
    assert_eq!(again.to_bytes().unwrap(), bytes);
}

#[test]
fn an_existing_soundfont_rewritten_plays_the_same() {
    let original = make::sf2(
        &[(0, 0, "Sine", vec![(vec![(54, 1), (8, 6000), (9, 60), (36, 3000), (37, 200)], 0)]), (0, 1, "Low", vec![(vec![(43, 0x3c00), (54, 1), (51, -12)], 0)])],
        &[make::sine(100, 20, 44_000, 69)],
    );
    let rewritten = SoundFont::from_bytes(&original).unwrap().to_bytes().unwrap();
    let (a, b) = (Arc::new(SoundBank::from_bytes(&original).unwrap()), Arc::new(SoundBank::from_bytes(&rewritten).unwrap()));
    for (program, key) in [(0, 69), (1, 40)] {
        assert_eq!(play(&a, 0, program, key, 100, 0.5), play(&b, 0, program, key, 100, 0.5));
    }
}

#[test]
fn a_dls_bank_becomes_a_soundfont_that_sounds_alike() {
    // EG1: a 0.5 s decay to 90%; a filter at 1.4 kHz; a little vibrato.
    let art = vec![(0, 0, 0x0207, (1200.0 * 0.5f32.log2() * 65_536.0) as i32), (0, 0, 0x020a, 900 * 65_536), (0, 0, 0x0500, 9000 * 65_536), (1, 0, 3, 10 * 65_536)];
    let dls = make::dls(&[(0, 0, false, "Sine", vec![((0, 127), 0, vec![])], art)], &[make::sine(100, 20, 44_000, 69)]);
    let sf = SoundFont::from_bytes(&dls).unwrap();
    assert_eq!(sf.presets.len(), 1);
    assert_eq!(sf.samples.len(), 1);
    let (a, b) = (Arc::new(SoundBank::from_bytes(&dls).unwrap()), bank(&sf));
    let (x, y) = (play(&a, 0, 0, 69, 100, 1.0), play(&b, 0, 0, 69, 100, 1.0));
    for part in [2205..8820, 22_050..44_100] {
        let (rx, ry) = (rms(&x[part.clone()]), rms(&y[part.clone()]));
        // A SoundFont's shortest time is a millisecond, not none: its
        // delay, attack and hold put the decay 3 ms later.
        assert!((20.0 * (ry / rx).log10()).abs() < 1.0, "{rx} {ry}");
    }
    let (px, py) = (period_hz(&x[4410..]), period_hz(&y[4410..]));
    assert!((px - py).abs() < 0.5, "{px} {py}");
}

#[test]
fn trimming_to_a_song_keeps_only_what_it_plays() {
    let mut sf = SoundFont::new("Three");
    for (n, name) in ["Low", "High", "Unused"].iter().enumerate() {
        sf.add_sample(sine_sample(name, 60));
        let _ = n;
    }
    let mut piano = Instrument::new("Split");
    piano.zones.push(Zone::new(0).keys(0, 59));
    piano.zones.push(Zone::new(1).keys(60, 127));
    let piano = sf.add_instrument(piano);
    let mut other = Instrument::new("Other");
    other.zones.push(Zone::new(2));
    let other = sf.add_instrument(other);
    sf.add_preset(Preset::new("Piano", 0, 0, piano));
    sf.add_preset(Preset::new("Other", 0, 5, other));
    sf.add_preset(Preset::new("Kit", 128, 0, other));

    // Program 0 on channel 0, high notes only; bank 3 falls back to bank 0.
    let song = file(0, &[vec![0, 0xb0, 0, 3, 0, 0xc0, 0, 0, 0x90, 72, 100, 10, 0x80, 72, 0]]);
    let mut trimmed = sf.clone();
    trimmed.trim_to_midi(&song).unwrap();
    assert_eq!(trimmed.presets.len(), 1);
    assert_eq!(trimmed.presets[0].name, "Piano");
    assert_eq!(trimmed.instruments.len(), 1);
    assert_eq!(trimmed.instruments[0].zones.len(), 1);
    assert_eq!(trimmed.samples.len(), 1);
    assert_eq!(trimmed.samples[0].name, "High");
    assert_eq!(trimmed.instruments[0].zones[0].target, Some(0));
    trimmed.to_bytes().unwrap();

    // A drum note keeps the kit, and what it plays.
    let drums = file(0, &[vec![0, 0x99, 36, 100, 10, 0x89, 36, 0]]);
    let mut kit = sf.clone();
    kit.trim_to_midi(&drums).unwrap();
    assert_eq!(kit.presets.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Kit"]);
    assert_eq!(kit.samples.len(), 1);

    let mut chosen = sf;
    chosen.trim(|p| p.program == 5);
    assert_eq!((chosen.presets.len(), chosen.instruments.len(), chosen.samples.len()), (1, 1, 1));
}

#[test]
fn what_doesnt_fit_together_isnt_written() {
    let mut sf = one_sine();
    sf.instruments[0].zones[0].target = Some(7);
    assert_eq!(sf.to_bytes(), Err(crate::Error::Unwritable));
    let mut sf = one_sine();
    sf.samples[0].loop_end = 1_000_000;
    assert_eq!(sf.to_bytes(), Err(crate::Error::Unwritable));
    let mut sf = one_sine();
    sf.samples[0].link = Link::Left(0);
    assert_eq!(sf.to_bytes(), Err(crate::Error::Unwritable));
}

#[test]
fn a_wav_file_becomes_samples() {
    let frames: Vec<f32> = (0..200).flat_map(|i| [i as f32 / 400.0, -(i as f32) / 400.0]).collect();
    let wav = crate::wav(&frames, 22_050);
    let s = Sample::from_wav("Ramp", &wav, 64).unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!((s[0].name.as_str(), s[1].name.as_str()), ("Ramp L", "Ramp R"));
    assert_eq!((s[0].rate, s[0].root, s[0].data.len()), (22_050, 64, 200));
    assert!(s[0].data[100] > 8000 && s[1].data[100] < -8000);
    assert_eq!(Sample::from_wav("x", b"RIFF\0\0\0\0WAVEjunk", 60), Err(crate::Error::NotWav));
}

/// Neumetik's own instrument and the packed one, at a note: their
/// loudness in dB apart and their pitches.
fn against_own(sf: &Arc<SoundBank>, program: u8, key: u8, vel: u8) -> (f32, f32, f32) {
    // Neumetik's own, a few times over: a plucked string's noise burst
    // makes each note a few dB louder or softer than the last.
    let mut s = Synth::new(RATE);
    s.program_change(0, program);
    s.control_change(0, 91, 0);
    let mut power = 0.0;
    let mut own = Vec::new();
    for _ in 0..8 {
        s.note_on(0, key, vel);
        let n = RATE as usize / 2;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        s.render(&mut l, &mut r);
        s.all_sound_off();
        let (mut ql, mut qr) = (vec![0.0; 4410], vec![0.0; 4410]);
        s.render(&mut ql, &mut qr);
        own = l.iter().zip(&r).map(|(a, b)| a + b).collect::<Vec<f32>>();
        power += rms(&own[2205..22_050]).powi(2) / 8.0;
    }
    let packed = {
        let mut s = Synth::new(RATE);
        s.set_sound_bank(Some(sf.clone()));
        s.program_change(0, program);
        s.control_change(0, 91, 0);
        s.note_on(0, key, vel);
        let n = RATE as usize / 2;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        s.render(&mut l, &mut r);
        l.iter().zip(&r).map(|(a, b)| a + b).collect::<Vec<f32>>()
    };
    let part = 2205..22_050;
    (20.0 * (rms(&packed[part.clone()]) / power.sqrt()).log10(), period_hz(&own[part.clone()]), period_hz(&packed[part]))
}

/// Hz, by autocorrelation: the first lag (from 40 Hz to 2 kHz) nearly as
/// alike as the best, so harmonics in any phase don't fool it.
fn period_hz(s: &[f32]) -> f32 {
    let n = s.len() / 2;
    let corr = |lag: usize| {
        let (mut xy, mut xx, mut yy) = (0f32, 0f32, 0f32);
        for i in 0..n {
            xy += s[i] * s[i + lag];
            xx += s[i] * s[i];
            yy += s[i + lag] * s[i + lag];
        }
        xy / (xx * yy).sqrt().max(1e-12)
    };
    let lags: Vec<(usize, f32)> = (RATE as usize / 2000..RATE as usize / 40).map(|l| (l, corr(l))).collect();
    let best = lags.iter().fold(0f32, |m, (_, c)| m.max(*c));
    let at = lags.windows(3).find(|w| w[1].1 >= best * 0.95 && w[1].1 >= w[0].1 && w[1].1 >= w[2].1).map_or(lags[0], |w| w[1]);
    // A parabola through the peak for the fraction of a sample.
    let (a, b, c) = (corr(at.0 - 1), at.1, corr(at.0 + 1));
    let shift = 0.5 * (a - c) / (a - 2.0 * b + c);
    RATE as f32 / (at.0 as f32 + if shift.is_finite() { shift } else { 0.0 })
}

#[test]
fn neumetiks_own_instruments_pack_into_a_soundfont_that_sounds_like_them() {
    let mut made = 0;
    let sf = SoundFont::neumetik_with_progress(&Pack::default(), |done, total| {
        assert!(done <= total);
        made = done;
    });
    assert_eq!(made, 128 + crate::drums::KITS.len());
    assert_eq!(sf.presets.len(), 128 + crate::drums::KITS.len());
    let bytes = sf.to_bytes().unwrap();
    let b = Arc::new(SoundBank::from_bytes(&bytes).unwrap());
    assert_eq!(b.instruments().filter(|i| i.drums).count(), crate::drums::KITS.len());
    // Piano, electric piano, organ, guitar, trumpet, flute, a lead, a
    // pad: as loud, in tune.
    for program in [0u8, 4, 16, 24, 56, 73, 80, 88] {
        for key in [48u8, 69] {
            let (db, own, packed) = against_own(&b, program, key, 100);
            assert!(db.abs() < 2.0, "program {program} key {key}: {db:.1} dB");
            assert!((packed / own - 1.0).abs() < 0.02, "program {program} key {key}: {own} Hz, {packed} Hz");
        }
    }
}

/// Every GM instrument packed against Neumetik's own, at three keys and
/// two velocities: dB apart and the ratio of their pitches. For tuning
/// the packing: `cargo test --lib packed_against_own -- --ignored --nocapture`.
#[test]
#[ignore]
fn packed_against_own() {
    let sf = SoundFont::neumetik(&Pack { set: PackSet { drums: false, ..PackSet::GM }, ..Pack::default() });
    let b = Arc::new(SoundBank::from_bytes(&sf.to_bytes().unwrap()).unwrap());
    println!("{:.1} MB", sf.sample_bytes() as f32 / 1e6);
    for program in 0..128u8 {
        let mut line = format!("{program:3} {:24}", crate::patch(0, program).name);
        for key in [36u8, 60, 84] {
            for vel in [40u8, 110] {
                let (db, own, packed) = against_own(&b, program, key, vel);
                line += &format!(" {db:6.1}/{:5.2}", packed / own);
            }
        }
        println!("{line}");
    }
}

/// Real banks (`NEUMETIK_TEST_BANKS=a.sf2:b.dls`): a SoundFont written
/// again plays exactly as it did, a DLS bank turned into one nearly so, and
/// writing what's read back again changes nothing.
#[test]
fn real_banks_write_and_play_the_same() {
    let Ok(paths) = std::env::var("NEUMETIK_TEST_BANKS") else { return };
    // A note on every program and a drum kit.
    let mut track = Vec::new();
    for p in 0..128u8 {
        track.extend_from_slice(&[0, 0xc0, p, 0, 0x90, 48 + p % 24, 100, 0, 0x99, 35 + p % 46, 100, 20, 0x80, 48 + p % 24, 0]);
    }
    let song = file(0, &[track]);
    for path in paths.split(':') {
        let bytes = std::fs::read(path).unwrap();
        let sf = SoundFont::from_bytes(&bytes).unwrap();
        let written = sf.to_bytes().unwrap();
        // Names past 19 characters lose the rest (the specification
        // keeps the twentieth byte for its NUL): from then on, the same.
        assert!(SoundFont::from_bytes(&written).unwrap().to_bytes().unwrap() == written, "{path}");
        let (a, b) = (Arc::new(SoundBank::from_bytes(&bytes).unwrap()), Arc::new(SoundBank::from_bytes(&written).unwrap()));
        assert_eq!(a.instruments().count(), b.instruments().count(), "{path}");
        let x = crate::render_through(&song, Some(a), RATE, 1.0, 60.0).unwrap();
        let y = crate::render_through(&song, Some(b), RATE, 1.0, 60.0).unwrap();
        if &bytes[8..12] == b"sfbk" {
            assert!(x == y, "{path}");
        } else {
            let db = 20.0 * (rms(&y) / rms(&x)).log10();
            assert!(db.abs() < 1.0, "{path}: {db:.2} dB");
        }
        println!("{path}: {} presets, {} samples, {:.1} MB written", sf.presets.len(), sf.samples.len(), written.len() as f32 / 1e6);
    }
}
