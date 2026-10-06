// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! **Neumetik MIDI**: General MIDI and Roland GS music for games, played
//! without a SoundFont or any samples. Every instrument is a few numbers
//! for three small synthesis engines (subtractive, FM and plucked string)
//! and every drum a recipe, so there's nothing to download: the whole
//! synthesizer is about 60 KB of WebAssembly, gzipped. No dependencies, no
//! audio API: it turns MIDI into samples and your engine plays them.
//!
//! ```
//! use neumetik_midi::Player;
//! # let midi = neumetik_midi::doc_song();
//!
//! let mut player = Player::new(48_000);
//! player.load(&midi)?;          // a .mid file's bytes
//! player.set_looping(true);     // between its loop markers, if it has them
//! player.play();
//!
//! // In your audio callback, or each frame as your engine asks:
//! let mut buffer = [0f32; 1024]; // 512 frames, left and right interleaved
//! player.render_interleaved(&mut buffer);
//! # Ok::<(), neumetik_midi::Error>(())
//! ```
//!
//! - [`Player`]: a song with play, pause, stop, seek, looping (the file's
//!   loop markers, or your own points), volume and fades, speed, and
//!   channel mutes.
//! - [`Synth`]: the synthesizer alone, to play live (notes, controllers,
//!   pitch bend, raw MIDI bytes). Every player has one: [`Player::synth`].
//! - [`Sequencer`]: MIDI files queued back to back, for music written as
//!   it plays.
//! - [`render`]: a whole file to samples at once; [`wav`] makes a WAV.
//!
//! Banks (Bank Select, CC 0): General MIDI's 128 instruments on 0, the
//! Roland SC-55's variation tones on 1 to 63 (falling back to GM's, as the
//! SC-55 did), and classic synth sounds on 80 (the eighties), 81 (the
//! nineties) and 82 (the 2000s on). Drums on channel 9 (MIDI's 10, or any
//! channel GS makes a drum part): every key 27 to 87, and GS's kits at
//! their program numbers.
//!
//! Rendering never allocates, locks or panics, and the same MIDI always
//! makes the same samples.

mod classics;
pub mod drums;
mod effects;
mod error;
mod gm;
mod gs;
mod output;
mod patch;
mod player;
mod sequencer;
mod smf;
mod synth;
mod voice;

pub use error::Error;
pub use output::wav;
pub use patch::Patch;
pub use player::{PlayState, Player, Song};
pub use sequencer::Sequencer;
pub use synth::{Synth, MAX_RATE, MIN_RATE};

/// Voices at once; past it, the quietest is stolen.
pub const POLYPHONY: usize = 64;

/// The crate's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A bank of patches a Bank Select picks.
pub struct Bank {
    pub msb: u8,
    pub patches: &'static [Patch],
}

/// General MIDI, then the classics: the 1980s, the 1990s, the 2000s to
/// now.
pub static BANKS: [Bank; 4] = [
    Bank { msb: 0, patches: &gm::GM },
    Bank { msb: 80, patches: classics::EIGHTIES },
    Bank { msb: 81, patches: classics::NINETIES },
    Bank { msb: 82, patches: classics::NOW },
];

/// The patch a bank and program play: a classic bank's, a GS variation,
/// or else GM's own.
pub fn patch(msb: u8, program: u8) -> &'static Patch {
    let program = program & 0x7f;
    if let Some(bank) = BANKS.iter().find(|b| b.msb == msb && msb != 0) {
        if let Some(p) = bank.patches.get(usize::from(program)) {
            return p;
        }
    }
    if msb != 0 {
        if let Some((_, _, p)) = gs::GS.iter().find(|(m, p, _)| *m == msb && *p == program) {
            return p;
        }
    }
    &gm::GM[usize::from(program)]
}

/// A MIDI file played through the synthesizer, all at once: interleaved
/// stereo at `sample_rate`, as long as the music and then `tail` seconds
/// for the last notes to ring, `longest` seconds at most.
pub fn render(bytes: &[u8], sample_rate: u32, tail: f64, longest: f64) -> Result<Vec<f32>, Error> {
    let events = smf::parse(bytes)?;
    let mut synth = Synth::new(sample_rate);
    let rate = f64::from(synth.sample_rate());
    let length = events.last().map_or(0.0, |e| e.seconds);
    let frames = ((length + tail).min(longest).max(0.0) * rate) as usize;
    let mut out = Vec::with_capacity(frames * 2);
    let (mut left, mut right) = (vec![0f32; 4096], vec![0f32; 4096]);
    let mut done = 0;
    let mut play_to = |synth: &mut Synth, to: usize, out: &mut Vec<f32>| {
        while done < to {
            let n = (to - done).min(left.len());
            synth.render(&mut left[..n], &mut right[..n]);
            for i in 0..n {
                out.push(left[i]);
                out.push(right[i]);
            }
            done += n;
        }
    };
    for e in &events {
        let at = ((e.seconds * rate) as usize).min(frames);
        play_to(&mut synth, at, &mut out);
        if at >= frames {
            break;
        }
        synth.message(&e.message);
    }
    play_to(&mut synth, frames, &mut out);
    Ok(out)
}

/// A little song for the examples in these docs.
#[doc(hidden)]
pub fn doc_song() -> Vec<u8> {
    let track = [0, 0x90, 60, 100, 96, 0x80, 60, 0, 0, 0xff, 0x2f, 0];
    let mut f = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk".to_vec();
    f.extend_from_slice(&(track.len() as u32).to_be_bytes());
    f.extend_from_slice(&track);
    f
}

#[cfg(test)]
mod tests {
    use super::smf::tests::file;
    use super::smf::Message;
    use super::*;

    fn loudest(s: &[f32]) -> f32 {
        s.iter().fold(0.0, |m, x| m.max(x.abs()))
    }

    /// Stereo 16-bit WAV, peaking just under full scale.
    fn normalized_wav(samples: &[f32]) -> Vec<u8> {
        let gain = 0.89 / loudest(samples).max(1e-6);
        let scaled: Vec<f32> = samples.iter().map(|s| s * gain).collect();
        wav(&scaled, 44_100)
    }

    fn render_player(p: &mut Player, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        p.render_interleaved(&mut out);
        out
    }

    #[test]
    fn every_patch_in_every_bank_sounds_and_stays_sane() {
        let mut checked = 0;
        let banks = BANKS.iter().flat_map(|b| b.patches.iter()).chain(gs::GS.iter().map(|(_, _, p)| p));
        for p in banks {
            let mut synth = Synth::new(22_050);
            synth.play_patch(p, 60);
            let (mut l, mut r) = (vec![0.0; 22_050], vec![0.0; 22_050]);
            synth.render(&mut l, &mut r);
            let peak = loudest(&l).max(loudest(&r));
            assert!(peak.is_finite() && peak > 1e-4, "{} is silent ({peak})", p.name);
            assert!(peak < 4.0, "{} is far too loud ({peak})", p.name);
            checked += 1;
        }
        assert!(checked > 300, "{checked}");
    }

    #[test]
    fn a_bank_select_picks_gs_and_classic_patches_and_falls_back_to_gm() {
        assert_eq!(patch(0, 0).name, "Acoustic Grand Piano");
        assert_eq!(patch(8, 4).name, "Detuned EP 1");
        assert_eq!(patch(80, 0).name, "Solid Tine EP");
        assert_eq!(patch(81, 2).name, "Supersaw");
        // No such variation: the capital tone.
        assert_eq!(patch(5, 0).name, "Acoustic Grand Piano");
        // Past a classic bank's end: GM's.
        assert_eq!(patch(82, 127).name, "Gunshot");
    }

    #[test]
    fn gm_has_its_128_by_their_names() {
        assert_eq!(gm::GM[0].name, "Acoustic Grand Piano");
        assert_eq!(gm::GM[40].name, "Violin");
        assert_eq!(gm::GM[127].name, "Gunshot");
    }

    #[test]
    fn a_sequencer_plays_files_back_to_back_as_one() {
        // Two files of a note each, 96 ticks (half a second at 120 bpm),
        // and one file of both; in pieces of odd sizes, they sound the same
        // (to within the control-rate block shifting, far below hearing).
        let one = vec![0, 0x90, 60, 100, 96, 0x80, 60, 0];
        let two = vec![0, 0x90, 67, 100, 96, 0x80, 67, 0];
        let both = vec![0, 0x90, 60, 100, 96, 0x80, 60, 0, 0, 0x90, 67, 100, 96, 0x80, 67, 0];
        let whole = render(&file(0, &[both]), 22_050, 0.5, 60.0).unwrap();
        let mut seq = Sequencer::new(22_050);
        assert_eq!(seq.queue(&file(0, &[one])).unwrap(), 11_025);
        assert_eq!(seq.queue(&file(0, &[two])).unwrap(), 11_025);
        assert_eq!(seq.ahead(), 22_050);
        let mut out = Vec::new();
        for size in [1_000, 7, 12_345, 20_000].iter().cycle() {
            if out.len() >= whole.len() {
                break;
            }
            let (mut l, mut r) = (vec![0.0; *size], vec![0.0; *size]);
            seq.render(&mut l, &mut r);
            out.extend(l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]));
        }
        out.truncate(whole.len());
        assert!(loudest(&whole) > 0.01);
        let off = whole.iter().zip(&out).fold(0f32, |m, (a, b)| m.max((a - b).abs()));
        assert!(off < loudest(&whole) * 0.01, "{off}");
        assert_eq!(seq.ahead(), 0);
    }

    #[test]
    fn a_sequencer_plays_sysex_from_its_pool_and_drops_what_it_played() {
        // Part 11 made a drum part, then a note on it.
        let rhythm = vec![0, 0xf0, 10, 0x41, 0x10, 0x42, 0x12, 0x40, 0x1a, 0x15, 0x02, 0x0f, 0xf7, 0, 0x9a, 36, 100];
        let mut seq = Sequencer::new(22_050);
        seq.queue(&file(0, std::slice::from_ref(&rhythm))).unwrap();
        let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
        seq.render(&mut l, &mut r);
        assert!(seq.synth().channel_state(10).0);
        assert_eq!(seq.pool_len(), 9);
        seq.queue(&file(0, &[rhythm])).unwrap();
        assert_eq!(seq.pool_len(), 9, "the played message is dropped at the next queue");
    }

    #[test]
    fn a_midi_file_plays() {
        // A C major chord on a piano, a drum hit on channel 10, then the
        // GS reset and a supersaw from bank 81.
        let track = vec![
            0, 0x90, 60, 100, 0, 0x90, 64, 100, 0, 0x90, 67, 100, 0, 0x99, 36, 120, //
            96, 0x80, 60, 0, 0, 0x80, 64, 0, 0, 0x80, 67, 0, //
            0, 0xb1, 0, 81, 0, 0xc1, 2, 0, 0x91, 60, 100, 96, 0x81, 60, 0,
        ];
        let out = render(&file(0, &[track]), 22_050, 1.0, 60.0).unwrap();
        assert_eq!(out.len(), 2 * 22_050 * 2);
        let first = loudest(&out[..22_050]);
        let second = loudest(&out[22_050..44_100]);
        assert!(first > 0.01 && second > 0.01, "{first} {second}");
    }

    #[test]
    fn the_sustain_pedal_holds_notes() {
        let mut synth = Synth::new(22_050);
        synth.control_change(0, 64, 127);
        synth.note_on(0, 60, 100);
        synth.note_off(0, 60);
        assert!(synth.voices().iter().any(|v| v.active && v.sustained));
        synth.control_change(0, 64, 0);
        assert!(synth.voices().iter().all(|v| !v.sustained));
    }

    #[test]
    fn the_gs_rhythm_part_message_makes_a_drum_channel() {
        let mut synth = Synth::new(22_050);
        // Part 11 (channel 11, index 10) as drums.
        synth.send(&[0xf0, 0x41, 0x10, 0x42, 0x12, 0x40, 0x1a, 0x15, 0x02, 0x0f, 0xf7]);
        assert!(synth.channel_state(10).0);
        synth.message(&Message::SysEx(vec![0x41, 0x10, 0x42, 0x12, 0x40, 0x00, 0x7f, 0x00, 0x41]));
        assert!(!synth.channel_state(10).0 && synth.channel_state(9).0);
    }

    #[test]
    fn the_bend_range_rpn_is_kept() {
        let mut synth = Synth::new(22_050);
        for (cc, v) in [(101, 0), (100, 0), (6, 12)] {
            synth.control_change(0, cc, v);
        }
        assert_eq!(synth.channel_state(0).1, 12.0);
    }

    #[test]
    fn too_many_notes_steal_rather_than_fail() {
        let mut synth = Synth::new(22_050);
        for key in 0..POLYPHONY as u8 + 20 {
            synth.note_on(0, key % 128, 100);
        }
        assert_eq!(synth.active_voices(), POLYPHONY);
    }

    #[test]
    fn raw_bytes_play_with_running_status_and_skip_what_isnt_for_a_synth() {
        let mut synth = Synth::new(22_050);
        // A clock tick, two notes on in running status, a stray data byte.
        synth.send(&[0xf8, 0x90, 60, 100, 64, 100, 0xfe]);
        assert_eq!(synth.active_voices(), 2);
        synth.send(&[0x80, 60, 0, 64, 0]);
        assert!(synth.voices().iter().all(|v| !v.active || v.released));
        // A message cut short, and nonsense: ignored.
        synth.send(&[0x90, 61]);
        synth.send(&[1, 2, 3, 0xf0, 1, 2]);
        assert_eq!(synth.instrument_name(0), "Acoustic Grand Piano");
        synth.set_instrument(0, 81, 2);
        assert_eq!(synth.instrument_name(0), "Supersaw");
        assert_eq!(synth.instrument_name(9), "Standard");
        assert_eq!(synth.instrument_name(99), "");
    }

    #[test]
    fn out_of_range_input_never_panics() {
        let mut synth = Synth::new(0);
        assert_eq!(synth.sample_rate(), MIN_RATE);
        synth.note_on(200, 200, 200);
        synth.note_off(16, 255);
        synth.control_change(17, 255, 255);
        synth.pitch_bend(0, i16::MIN);
        synth.set_channel_muted(99, true);
        synth.set_volume(f32::NAN);
        synth.fade_to(f32::INFINITY, -1.0);
        let mut out = [0f32; 64];
        synth.render_interleaved(&mut out);
        assert!(out.iter().all(|s| s.is_finite()));
        let mut p = Player::new(u32::MAX);
        assert_eq!(p.synth().sample_rate(), MAX_RATE);
        p.play();
        p.seek(f64::NAN);
        p.set_speed(f64::INFINITY);
        p.set_loop_points(Some(-1.0), Some(f64::NAN));
        assert_eq!(p.load(b"nope"), Err(Error::NotMidi));
        p.render_interleaved(&mut out);
    }

    /// A song of four quarter notes at 120 bpm, a note a half second,
    /// keys 60, 62, 64, 65, and a program change to the organ before the
    /// third: two seconds long.
    fn four_notes() -> Vec<u8> {
        let mut t = Vec::new();
        for (i, key) in [60u8, 62, 64, 65].into_iter().enumerate() {
            if i == 2 {
                t.extend_from_slice(&[0, 0xc0, 16]);
            }
            t.extend_from_slice(&[0, 0x90, key, 100, 96, 0x80, key, 0]);
        }
        file(0, &[t])
    }

    #[test]
    fn a_player_plays_pauses_and_stops() {
        let mut p = Player::new(22_050);
        p.load(&four_notes()).unwrap();
        assert!((p.duration() - 2.0).abs() < 1e-9);
        assert_eq!(p.state(), PlayState::Stopped);
        // Stopped: silence.
        assert_eq!(loudest(&render_player(&mut p, 2_000)), 0.0);
        p.play();
        assert!(loudest(&render_player(&mut p, 11_025)) > 0.01);
        assert!((p.position() - 0.5).abs() < 1e-3);
        p.pause();
        assert_eq!(loudest(&render_player(&mut p, 2_000)), 0.0);
        assert!((p.position() - 0.5).abs() < 1e-3, "paused, it holds");
        p.play();
        render_player(&mut p, 2 * 22_050);
        assert!(p.is_finished() && p.state() == PlayState::Stopped);
        p.play();
        assert!(p.is_playing() && p.position() == 0.0, "played out, play starts again");
        p.stop();
        assert_eq!(p.position(), 0.0);
    }

    #[test]
    fn a_player_seeks_with_the_instruments_set_as_the_song_has_them() {
        let mut p = Player::new(22_050);
        p.load(&four_notes()).unwrap();
        p.seek(1.25);
        assert_eq!(p.synth().instrument_name(0), "Drawbar Organ");
        p.seek(0.25);
        assert_eq!(p.synth().instrument_name(0), "Acoustic Grand Piano");
        p.play();
        render_player(&mut p, 22_050 / 2);
        assert!((p.position() - 0.75).abs() < 1e-3);
    }

    #[test]
    fn a_player_loops_with_no_gap_and_from_the_right_place() {
        let mut p = Player::new(22_050);
        p.load(&four_notes()).unwrap();
        p.set_looping(true);
        p.set_loop_points(Some(1.0), None);
        assert_eq!(p.loop_points(), (1.0, 2.0));
        p.play();
        // Two seconds in, then a quarter of the way into the loop again.
        render_player(&mut p, 2 * 22_050 + 22_050 / 4);
        assert!(p.is_playing());
        assert!((p.position() - 1.25).abs() < 1e-3, "{}", p.position());
        assert_eq!(p.synth().instrument_name(0), "Drawbar Organ");
        // Never silent across the loop.
        p.seek(1.9);
        let out = render_player(&mut p, 22_050 / 5);
        for window in out.chunks(2 * 441) {
            assert!(loudest(window) > 1e-3);
        }
    }

    #[test]
    fn a_files_loop_markers_are_used() {
        let t = vec![0, 0x90, 60, 100, 96, 0xff, 0x06, 9, b'l', b'o', b'o', b'p', b'S', b't', b'a', b'r', b't', 0, 0x80, 60, 0, 0, 0x90, 62, 100, 96, 0x80, 62, 0];
        let song = Song::from_midi(&file(0, &[t])).unwrap();
        assert_eq!(song.loop_points(), Some((0.5, 1.0)));
        let mut p = Player::new(22_050);
        p.load_song(song);
        assert_eq!(p.loop_points(), (0.5, 1.0));
        p.set_loop_points(Some(0.75), Some(0.0));
        assert_eq!(p.loop_points(), (0.75, 1.0), "an end of 0 isn't one");
    }

    #[test]
    fn speed_volume_fades_and_mutes() {
        let mut p = Player::new(22_050);
        p.load(&four_notes()).unwrap();
        p.set_speed(2.0);
        p.play();
        render_player(&mut p, 22_050 / 2);
        assert!((p.position() - 1.0).abs() < 1e-3);
        p.set_volume(0.5);
        assert_eq!(p.volume(), 0.5);
        p.fade_to(0.0, 0.1);
        render_player(&mut p, 22_050 / 5);
        assert_eq!(p.volume(), 0.0);
        assert_eq!(loudest(&render_player(&mut p, 1_000)), 0.0);
        // A muted channel plays on, unheard.
        let mut p = Player::new(22_050);
        p.load(&four_notes()).unwrap();
        p.set_channel_muted(0, true);
        assert!(p.channel_muted(0));
        p.play();
        assert_eq!(loudest(&render_player(&mut p, 5_000)), 0.0);
        assert_eq!(p.synth().active_voices(), 1);
    }

    #[test]
    fn the_same_midi_makes_the_same_samples() {
        let a = render(&four_notes(), 22_050, 1.0, 60.0).unwrap();
        let b = render(&four_notes(), 22_050, 1.0, 60.0).unwrap();
        assert_eq!(a, b);
    }

    /// A tour of the banks to listen to while tuning: each patch plays a
    /// phrase, then the drum kits a beat. Writes a WAV to `OUT`:
    /// `OUT=demo.wav cargo test --release neumetik_demo -- --ignored`.
    #[test]
    #[ignore]
    fn neumetik_demo() {
        let tour: [(u8, u8); 24] = [
            (0, 0), (0, 4), (0, 16), (0, 24), (0, 33), (0, 40), (0, 48), (0, 56), (0, 65), (0, 73), (0, 88), (0, 14),
            (80, 0), (80, 5), (80, 12), (80, 15), (81, 0), (81, 2), (81, 3), (81, 10), (82, 0), (82, 3), (82, 7), (82, 11),
        ];
        let mut t = Vec::new();
        // Ticks to wait before the next event, 96 a quarter note.
        let mut wait = 0u32;
        let ev = |t: &mut Vec<u8>, wait: &mut u32, bytes: &[u8]| {
            let mut d = *wait;
            let mut v = vec![(d & 0x7f) as u8];
            d >>= 7;
            while d > 0 {
                v.insert(0, (d & 0x7f) as u8 | 0x80);
                d >>= 7;
            }
            t.extend(v);
            t.extend_from_slice(bytes);
            *wait = 0;
        };
        for (msb, program) in tour {
            ev(&mut t, &mut wait, &[0xb0, 0, msb]);
            ev(&mut t, &mut wait, &[0xc0, program]);
            let low = if matches!((msb, program), (0, 33) | (80, 12) | (81, 3) | (82, 0) | (82, 7)) { 36 } else { 60 };
            for k in [0, 4, 7, 12] {
                ev(&mut t, &mut wait, &[0x90, low + k, 100]);
                wait = 48;
                ev(&mut t, &mut wait, &[0x80, low + k, 0]);
            }
            for k in [0, 4, 7] {
                ev(&mut t, &mut wait, &[0x90, low + k, 90]);
            }
            wait = 192;
            for k in [0, 4, 7] {
                ev(&mut t, &mut wait, &[0x80, low + k, 0]);
            }
            wait = 96;
        }
        for kit in [0, 25, 30, 40] {
            ev(&mut t, &mut wait, &[0xc9, kit]);
            for step in 0..16u8 {
                let mut hits = vec![42];
                if step % 4 == 0 {
                    hits.push(36);
                }
                if step % 8 == 4 {
                    hits.push(38);
                }
                if step == 15 {
                    hits.extend([49, 39]);
                }
                for h in hits {
                    ev(&mut t, &mut wait, &[0x99, h, 110]);
                }
                wait = 24;
            }
        }
        ev(&mut t, &mut wait, &[0xb0, 123, 0]);
        let out = render(&file(0, &[t]), 44_100, 2.0, 600.0).unwrap();
        assert!(out.iter().all(|s| s.is_finite()));
        if let Ok(path) = std::env::var("OUT") {
            std::fs::write(path, normalized_wav(&out)).unwrap();
        }
    }

    /// Prints how much faster than real time a busy piece renders:
    /// `cargo test --release neumetik_speed -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn neumetik_speed() {
        let mut track = Vec::new();
        for bar in 0..64u8 {
            for (ch, program) in [(0u8, 48u8), (1, 0), (2, 81), (3, 33)] {
                track.extend_from_slice(&[0, 0xc0 | ch, program]);
                for k in [48, 52, 55, 60] {
                    track.extend_from_slice(&[0, 0x90 | ch, k + bar % 5, 90]);
                }
            }
            track.extend_from_slice(&[0, 0x99, 36, 100, 0, 0x99, 42, 80]);
            track.extend_from_slice(&[96, 0x80, 0, 0]);
        }
        let start = std::time::Instant::now();
        let out = render(&file(0, &[track]), 44_100, 2.0, 600.0).unwrap();
        let seconds = out.len() as f64 / 2.0 / 44_100.0;
        let took = start.elapsed().as_secs_f64();
        println!("{seconds:.1} s of music in {took:.2} s: {:.0}x real time", seconds / took);
    }
}
