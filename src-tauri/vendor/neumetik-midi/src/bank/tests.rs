// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

use std::sync::Arc;

use super::make::{dls, sf2, sine, Sample};
use super::*;
use crate::Synth;

const RATE: u32 = 44_100;

/// A sine of 440 Hz at key 69 (100 samples a cycle at 44 kHz), looped.
fn a440() -> Sample {
    sine(100, 20, 44_000, 69)
}

fn bank(bytes: &[u8]) -> Arc<SoundBank> {
    Arc::new(SoundBank::from_bytes(bytes).unwrap())
}

fn synth(bank: &Arc<SoundBank>) -> Synth {
    let mut s = Synth::new(RATE);
    s.set_sound_bank(Some(bank.clone()));
    s
}

fn play(s: &mut Synth, seconds: f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
    s.render(&mut l, &mut r);
    l.iter().zip(&r).map(|(a, b)| a + b).collect()
}

fn loudest(s: &[f32]) -> f32 {
    s.iter().fold(0.0, |m, x| m.max(x.abs()))
}

/// Hz, by counting the upward zero crossings.
fn pitch(s: &[f32]) -> f32 {
    let ups: Vec<usize> = (1..s.len()).filter(|&i| s[i - 1] < 0.0 && s[i] >= 0.0).collect();
    let (first, last) = (ups[0], ups[ups.len() - 1]);
    (ups.len() - 1) as f32 * RATE as f32 / (last - first) as f32
}

/// A SoundFont of one sine, its zone's generators as given.
fn font(gens: Vec<(u16, i16)>) -> Vec<u8> {
    sf2(&[(0, 0, "Sine", vec![(gens, 0)])], &[a440()])
}

/// The generator that loops a sample.
const LOOP: (u16, i16) = (54, 1);

#[test]
fn a_soundfont_is_read_with_its_names() {
    let b = bank(&font(vec![LOOP]));
    assert_eq!(b.kind(), SoundBankKind::SoundFont);
    assert_eq!(b.name(), "Test Font");
    let all: Vec<Instrument> = b.instruments().collect();
    assert_eq!(all, [Instrument { bank: 0, program: 0, drums: false, name: "Sine" }]);
    let s = synth(&b);
    assert_eq!(s.instrument_name(0), "Sine");
}

#[test]
fn a_soundfont_note_plays_in_tune_and_follows_the_key_and_the_bend() {
    let b = bank(&font(vec![LOOP]));
    let mut s = synth(&b);
    s.note_on(0, 69, 127);
    let out = play(&mut s, 0.5);
    assert!((pitch(&out[4_410..]) - 440.0).abs() < 1.0, "{}", pitch(&out[4_410..]));
    s.note_off(0, 69);
    play(&mut s, 0.5);
    s.note_on(0, 81, 127);
    assert!((pitch(&play(&mut s, 0.5)[4_410..]) - 880.0).abs() < 2.0);
    // Two semitones up.
    s.pitch_bend(0, 8191);
    assert!((pitch(&play(&mut s, 0.5)[4_410..]) - 880.0 * 2f32.powf(2.0 / 12.0)).abs() < 3.0);
}

#[test]
fn tuning_generators_move_the_pitch() {
    // Coarse tune up 12 semitones, fine tune down 100 cents: 11 up.
    let b = bank(&font(vec![LOOP, (51, 12), (52, -100)]));
    let mut s = synth(&b);
    s.note_on(0, 57, 127);
    let hz = pitch(&play(&mut s, 0.5)[4_410..]);
    assert!((hz - 440.0 * 2f32.powf(-1.0 / 12.0)).abs() < 1.0, "{hz}");
    // Scale tuning 0: every key plays the root's pitch.
    let b = bank(&font(vec![LOOP, (56, 0)]));
    let mut s = synth(&b);
    s.note_on(0, 30, 127);
    assert!((pitch(&play(&mut s, 0.5)[4_410..]) - 440.0).abs() < 1.0);
}

#[test]
fn key_ranges_pick_the_sample() {
    // Below 60, a sine an octave down (200 samples a cycle).
    let low = sine(200, 10, 44_000, 69);
    let bytes = sf2(&[(0, 0, "Split", vec![(vec![(43, i16::from_le_bytes([0, 59])), LOOP], 1), (vec![(43, i16::from_le_bytes([60, 127])), LOOP], 0)])], &[a440(), low]);
    let b = bank(&bytes);
    let mut s = synth(&b);
    s.note_on(0, 69, 127);
    assert!((pitch(&play(&mut s, 0.5)[4_410..]) - 440.0).abs() < 1.0);
    s.all_sound_off();
    play(&mut s, 0.1);
    s.note_on(0, 57, 127);
    // Key 57 on the low sample, a cycle of 220 Hz at key 69, an octave down.
    assert!((pitch(&play(&mut s, 0.5)[4_410..]) - 110.0).abs() < 1.0);
}

#[test]
fn a_looped_sample_sounds_while_held_and_an_unlooped_one_ends() {
    let b = bank(&font(vec![LOOP]));
    let mut s = synth(&b);
    s.note_on(0, 69, 127);
    let out = play(&mut s, 2.0);
    assert!(loudest(&out[out.len() - 4_410..]) > 0.05);
    let b = bank(&font(vec![]));
    let mut s = synth(&b);
    s.note_on(0, 69, 127);
    // 2000 samples at 44 kHz: gone in a twentieth of a second.
    play(&mut s, 0.1);
    assert_eq!(s.active_voices(), 0);
}

#[test]
fn the_volume_envelope_releases_and_ends_the_voice() {
    // A release of a tenth of a second (-3986 timecents).
    let b = bank(&font(vec![LOOP, (38, -3986)]));
    let mut s = synth(&b);
    s.note_on(0, 69, 127);
    play(&mut s, 0.2);
    s.note_off(0, 69);
    let out = play(&mut s, 0.05);
    assert!(loudest(&out) > 0.01, "still ringing");
    play(&mut s, 0.1);
    assert_eq!(s.active_voices(), 0);
}

#[test]
fn attenuation_and_velocity_set_the_level() {
    let level = |gens: Vec<(u16, i16)>, vel: u8| {
        let b = bank(&font(gens));
        let mut s = synth(&b);
        s.note_on(0, 69, vel);
        loudest(&play(&mut s, 0.3)[4_410..])
    };
    let full = level(vec![LOOP], 127);
    // 250 cB, at 40% as E-mu's players count it: -10 dB.
    let down = level(vec![LOOP, (48, 250)], 127);
    assert!((20.0 * (down / full).log10() + 10.0).abs() < 0.2, "{full} {down}");
    // Half velocity: a quarter of the level, -12 dB.
    let soft = level(vec![LOOP], 64);
    assert!((soft / full - (64.0f32 / 127.0).powi(2)).abs() < 0.01, "{soft} {full}");
}

#[test]
fn a_low_pass_filter_darkens() {
    let level = |gens: Vec<(u16, i16)>| {
        let b = bank(&font(gens));
        let mut s = synth(&b);
        s.note_on(0, 81, 127);
        loudest(&play(&mut s, 0.3)[4_410..])
    };
    // 880 Hz through a filter at about 220 Hz (absolute cents 5700).
    let open = level(vec![LOOP]);
    let shut = level(vec![LOOP, (8, 5700)]);
    assert!(shut < open * 0.15, "{open} {shut}");
}

#[test]
fn an_exclusive_class_cuts_the_last_note() {
    let b = bank(&font(vec![LOOP, (57, 1)]));
    let mut s = synth(&b);
    s.note_on(0, 60, 127);
    play(&mut s, 0.1);
    s.note_on(0, 64, 127);
    play(&mut s, 0.1);
    assert_eq!(s.active_voices(), 1);
}

#[test]
fn drum_kits_play_on_drum_channels_and_what_the_bank_lacks_is_neumetiks() {
    // Bank 128 is the drum kits.
    let bytes = sf2(&[(0, 0, "Sine", vec![(vec![LOOP], 0)]), (128, 0, "Kit", vec![(vec![LOOP], 0)])], &[a440()]);
    let b = bank(&bytes);
    assert!(b.instruments().any(|i| i.drums && i.name == "Kit"));
    let mut s = synth(&b);
    assert_eq!(s.instrument_name(9), "Kit");
    // Kit 25 isn't there: kit 0 plays.
    s.program_change(9, 25);
    assert_eq!(s.instrument_name(9), "Kit");
    // No program 40 in the font: Neumetik's violin.
    s.program_change(0, 40);
    assert_eq!(s.instrument_name(0), "Violin");
    s.note_on(0, 60, 100);
    assert!(loudest(&play(&mut s, 0.2)) > 0.01);
    // A GS variation the font hasn't falls back to its own bank 0.
    s.set_instrument(1, 8, 0);
    assert_eq!(s.instrument_name(1), "Sine");
    // But a classic bank of Neumetik's is Neumetik's.
    s.set_instrument(2, 81, 2);
    assert_eq!(s.instrument_name(2), "Supersaw");
}

#[test]
fn changing_the_bank_stops_its_notes_and_none_is_neumetik_again() {
    let b = bank(&font(vec![LOOP]));
    let mut s = synth(&b);
    s.note_on(0, 60, 100);
    play(&mut s, 0.05);
    assert_eq!(s.active_voices(), 1);
    s.set_sound_bank(None);
    assert_eq!(s.active_voices(), 0);
    assert_eq!(s.instrument_name(0), "Acoustic Grand Piano");
    assert!(s.sound_bank().is_none());
}

#[test]
fn the_same_midi_through_a_bank_makes_the_same_samples() {
    let b = bank(&font(vec![LOOP]));
    let song = crate::doc_song();
    let one = crate::render_through(&song, Some(b.clone()), RATE, 0.5, 10.0).unwrap();
    let two = crate::render_through(&song, Some(b), RATE, 0.5, 10.0).unwrap();
    assert_eq!(one, two);
    assert!(loudest(&one) > 0.01);
}

#[test]
fn what_isnt_a_bank_is_refused() {
    assert_eq!(SoundBank::from_bytes(b"nope").unwrap_err(), Error::NotSoundBank);
    assert_eq!(SoundBank::from_bytes(b"RIFF\x04\0\0\0WAVE").unwrap_err(), Error::NotSoundBank);
    assert_eq!(SoundBank::from_bytes(b"RIFF\x04\0\0\0sfbk").unwrap_err(), Error::BadSoundBank);
    // A sample marked Ogg Vorbis: an SF3.
    let mut bytes = font(vec![]);
    let shdr = bytes.windows(4).position(|w| w == b"shdr").unwrap() + 8;
    bytes[shdr + 44] = 0x11;
    assert_eq!(SoundBank::from_bytes(&bytes).unwrap_err(), Error::CompressedSoundFont);
}

#[test]
fn a_bank_cut_short_or_scrambled_never_panics() {
    let files = [font(vec![LOOP, (43, 0x7f00)]), dls(&[(0, 0, false, "Sine", vec![((0, 127), 0, vec![])], vec![])], &[a440()])];
    for f in &files {
        for cut in 0..f.len() {
            let _ = SoundBank::from_bytes(&f[..cut]);
        }
        let mut x = 0x1234_5678u32;
        for _ in 0..2_000 {
            let mut g = f.clone();
            for _ in 0..4 {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                let i = x as usize % g.len();
                g[i] = (x >> 24) as u8;
            }
            if let Ok(b) = SoundBank::from_bytes(&g) {
                let mut s = synth(&Arc::new(b));
                for key in [0, 60, 69, 127] {
                    s.note_on(0, key, 127);
                    s.note_on(9, key, 127);
                }
                let out = play(&mut s, 0.02);
                assert!(out.iter().all(|v| v.is_finite()));
            }
        }
    }
}

// DLS's connections, by number: source, control, destination.
const EG1_RELEASE: (u16, u16, u16) = (0, 0, 0x0209);
const EG1_SUSTAIN: (u16, u16, u16) = (0, 0, 0x020a);
const GAIN: (u16, u16, u16) = (0, 0, 0x0001);
const KEY_TO_PITCH: (u16, u16, u16) = (3, 0, 0x0003);

fn conn(c: (u16, u16, u16), scale: f32) -> (u16, u16, u16, i32) {
    (c.0, c.1, c.2, (scale * 65_536.0) as i32)
}

#[test]
fn a_dls_file_is_read_and_plays_in_tune() {
    let bytes = dls(&[(0, 0, false, "Sine", vec![((0, 127), 0, vec![])], vec![]), (0, 0, true, "Kit", vec![((35, 81), 0, vec![])], vec![])], &[a440()]);
    let b = bank(&bytes);
    assert_eq!(b.kind(), SoundBankKind::Dls);
    assert_eq!(b.name(), "Test DLS");
    assert_eq!(b.instruments().count(), 2);
    let mut s = synth(&b);
    assert_eq!(s.instrument_name(0), "Sine");
    assert_eq!(s.instrument_name(9), "Kit");
    s.note_on(0, 69, 127);
    assert!((pitch(&play(&mut s, 0.5)[4_410..]) - 440.0).abs() < 1.0);
    s.note_on(9, 40, 127);
    play(&mut s, 0.1);
    assert_eq!(s.active_voices(), 2);
}

#[test]
fn dls_articulation_sets_the_envelope_level_and_tuning() {
    let level = |art: Vec<(u16, u16, u16, i32)>, region: Vec<(u16, u16, u16, i32)>| {
        let b = bank(&dls(&[(0, 0, false, "Sine", vec![((0, 127), 0, region)], art)], &[a440()]));
        let mut s = synth(&b);
        s.note_on(0, 69, 127);
        let out = play(&mut s, 0.3);
        (loudest(&out[4_410..]), pitch(&out[4_410..]))
    };
    let (full, hz) = level(vec![], vec![]);
    assert!((hz - 440.0).abs() < 1.0);
    // -6 dB on the instrument, -12 on the region, which wins.
    let (down, _) = level(vec![conn(GAIN, -60.0)], vec![conn(GAIN, -120.0)]);
    assert!((20.0 * (down / full).log10() + 12.0).abs() < 0.3, "{full} {down}");
    let (down, _) = level(vec![conn(GAIN, -60.0)], vec![]);
    assert!((20.0 * (down / full).log10() + 6.0).abs() < 0.3, "{full} {down}");
    // A sustain of 50%: half of 96 dB down.
    let (half, _) = level(vec![conn(EG1_SUSTAIN, 500.0)], vec![]);
    assert!((20.0 * (half / full).log10() + 48.0).abs() < 1.0, "{half}");
    // Keys at 50 cents a key: key 69 is still the root, key 81 six up.
    let b = bank(&dls(&[(0, 0, false, "Sine", vec![((0, 127), 0, vec![conn(KEY_TO_PITCH, 6400.0)])], vec![])], &[a440()]));
    let mut s = synth(&b);
    s.note_on(0, 81, 127);
    let hz = pitch(&play(&mut s, 0.3)[4_410..]);
    assert!((hz - 440.0 * 2f32.powf(0.5)).abs() < 1.5, "{hz}");
    // A release of a tenth of a second (-3986 timecents) ends the voice.
    let b = bank(&dls(&[(0, 0, false, "Sine", vec![((0, 127), 0, vec![])], vec![conn(EG1_RELEASE, -3986.0)])], &[a440()]));
    let mut s = synth(&b);
    s.note_on(0, 69, 127);
    play(&mut s, 0.1);
    s.note_off(0, 69);
    play(&mut s, 0.15);
    assert_eq!(s.active_voices(), 0);
}

/// Real banks, named in `NEUMETIK_TEST_BANKS` (paths, `:` between): each
/// reads, and a song through it plays, finite and heard.
#[test]
fn real_banks_read_and_play() {
    let Ok(paths) = std::env::var("NEUMETIK_TEST_BANKS") else { return };
    let song = crate::doc_song();
    for path in paths.split(':') {
        let b = bank(&std::fs::read(path).unwrap());
        assert!(b.instruments().count() > 0, "{path}");
        let out = crate::render_through(&song, Some(b), RATE, 1.0, 10.0).unwrap();
        assert!(out.iter().all(|v| v.is_finite()), "{path}");
        assert!(loudest(&out) > 0.01, "{path}");
    }
}

