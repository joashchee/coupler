//! The speech engines bundled in Coupler, for the characters' voices
//! (`cast.rs`): what they can say in, and the audio for a line. The
//! system's own voices are the WebView's (`src/lib/voice.ts`); these
//! are the ones that ship inside the app, so every Mac has them.
//!
//! - **Flite** (`flite-rs`, a pure-Rust rewrite of CMU's Flite,
//!   Apache-2.0, its voice data CMU's): Kal at 8 kHz and at 16 kHz, one
//!   man's diphones, plainly synthetic. Pitch is its F0 shift and speed
//!   its duration stretch, so a feminine character gets a raised pitch.
//!
//! - **Pocket TTS** (Kyutai, CC BY 4.0 weights; Candle's port, MIT,
//!   vendored and trimmed in `vendor/pocket-tts`: no downloader, no
//!   MKL, no voice cloning): 19 English presets, real voices of both
//!   kinds, from recordings under CC0 or CC BY 4.0. Its files are in the
//!   app's Resources (`pocket-tts/`, put there by
//!   scripts/pocket-tts-fetch.py); without them its voices aren't
//!   offered (`Engines::available`). It's loaded the first time it
//!   speaks, each voice the first time it's used, and kept. It has no
//!   pitch or speed of its own: `stretch.rs` gives it both.
//!
//! Every engine here is permissively licensed, its voices too (no GPL
//! phonemizer, no non-commercial voice). A new engine adds its
//! voices to `VOICES` and a branch to `Engines::say`.
//!
//! A line comes back as raw audio: the sample rate (u32), then 16-bit
//! mono samples, all little-endian, which the frontend puts straight in
//! an AudioBuffer. Below `LEAST_RATE` it's raised by a whole factor
//! first: older WebKit refuses a buffer under 22,050 Hz, and Kal is 8 kHz.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use pocket_tts::{ModelState, TTSModel};
use serde::Serialize;

use crate::cast::Gender;
use crate::stretch;

/// One voice of a bundled engine.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledVoice {
    pub engine: &'static str,
    pub id: &'static str,
    /// As the player sees it.
    pub name: &'static str,
    /// The speaker's own.
    pub gender: Gender,
    /// Suits a character of either kind (pitched to suit): Automatic
    /// offers it to everyone. A real voice of one kind isn't.
    pub any_gender: bool,
}

const fn flite(id: &'static str, name: &'static str) -> BundledVoice {
    BundledVoice { engine: "flite", id, name, gender: Gender::Masculine, any_gender: true }
}
const fn pocket(id: &'static str, name: &'static str, gender: Gender) -> BundledVoice {
    BundledVoice { engine: "pocket", id, name, gender, any_gender: false }
}
use Gender::{Feminine as F, Masculine as M};

/// Every bundled voice. Pocket TTS's are the English presets whose
/// recordings are CC0 or CC BY 4.0 (not CC BY-NC); each kind is
/// its speaker's (VCTK's speaker list, or the reader's name).
pub const VOICES: &[BundledVoice] = &[
    flite("kal", "Flite Kal"),
    flite("kal16", "Flite Kal 16 kHz"),
    pocket("alba", "Pocket Alba", F),
    pocket("anna", "Pocket Anna", F),
    pocket("azelma", "Pocket Azelma", F),
    pocket("bill_boerst", "Pocket Bill Boerst", M),
    pocket("caro_davy", "Pocket Caro Davy", F),
    pocket("charles", "Pocket Charles", M),
    pocket("eponine", "Pocket Eponine", F),
    pocket("eve", "Pocket Eve", F),
    pocket("fantine", "Pocket Fantine", F),
    pocket("george", "Pocket George", M),
    pocket("jane", "Pocket Jane", F),
    pocket("javert", "Pocket Javert", M),
    pocket("marius", "Pocket Marius", M),
    pocket("mary", "Pocket Mary", F),
    pocket("michael", "Pocket Michael", M),
    pocket("paul", "Pocket Paul", M),
    pocket("peter_yearsley", "Pocket Peter Yearsley", M),
    pocket("stuart_bell", "Pocket Stuart Bell", M),
    pocket("vera", "Pocket Vera", F),
];

/// Pocket TTS's model description (the vendored crate's, local paths only).
const POCKET_CONFIG: &[u8] = include_bytes!("../vendor/pocket-tts/config/b6369a24.yaml");

/// The lowest sample rate sent.
const LEAST_RATE: u32 = 22_050;

/// The longest line said at once, in characters: talk is a line or two.
const MOST: usize = 2000;
/// How much higher a feminine character speaks in a masculine voice.
const FEMININE_LIFT: f32 = 1.45;

/// Flite's F0 shift and duration stretch for a pitch and speed in
/// percent (speed already the player's times the character's), and
/// whether the character's kind differs from the speaker's.
pub fn flite_params(pitch: u16, rate: u16, lift: Option<f32>) -> (f32, f32) {
    let shift = f32::from(pitch.max(10)) / 100.0 * lift.unwrap_or(1.0);
    let stretch = 100.0 / f32::from(rate.max(10));
    (shift.clamp(0.3, 3.0), stretch.clamp(0.2, 3.0))
}

/// How a character of `gender` is lifted in `voice`.
fn lift(voice: &BundledVoice, gender: Gender) -> Option<f32> {
    match (voice.gender, gender) {
        (Gender::Masculine, Gender::Feminine) => Some(FEMININE_LIFT),
        (Gender::Feminine, Gender::Masculine) => Some(1.0 / FEMININE_LIFT),
        _ => None,
    }
}

/// Pocket TTS, loaded, and each voice once it's been used.
struct Pocket {
    model: TTSModel,
    voices: HashMap<&'static str, ModelState>,
}

/// The engines, each built the first time it speaks.
#[derive(Default)]
pub struct Engines {
    flite: Mutex<Option<flite_rs::Engine>>,
    pocket: Mutex<Option<Pocket>>,
    /// Where Pocket TTS's files are: the app's Resources, `pocket-tts/`.
    pocket_dir: OnceLock<PathBuf>,
}

/// A Pocket TTS voice's file.
fn pocket_voice(dir: &Path, id: &str) -> PathBuf {
    dir.join("voices").join(format!("{id}.safetensors"))
}

impl Engines {
    /// Where Pocket TTS's files are; set once, at launch.
    pub fn set_pocket_dir(&self, dir: PathBuf) {
        let _ = self.pocket_dir.set(dir);
    }

    /// The voices this copy of Coupler can speak in: Pocket TTS's only
    /// when its files came with it.
    pub fn available(&self) -> Vec<BundledVoice> {
        let pocket = self.pocket_dir.get().filter(|d| d.join("model.safetensors").is_file() && d.join("tokenizer.model").is_file());
        VOICES.iter().copied().filter(|v| v.engine != "pocket" || pocket.is_some_and(|d| pocket_voice(d, v.id).is_file())).collect()
    }

    /// `text` in a bundled voice: pitch and speed in percent.
    pub fn say(&self, engine: &str, voice: &str, text: &str, gender: Gender, pitch: u16, rate: u16) -> Result<Vec<u8>, String> {
        let found = VOICES.iter().find(|v| v.engine == engine && v.id == voice).ok_or_else(|| format!("Coupler has no voice {voice} in {engine}."))?;
        let text: String = text.chars().take(MOST).collect();
        match found.engine {
            "flite" => {
                let mut slot = self.flite.lock().unwrap();
                let flite = match slot.as_mut() {
                    Some(f) => f,
                    None => slot.insert(flite_rs::Engine::try_new().map_err(|e| format!("Flite couldn't start. ({e})"))?),
                };
                if !flite.select_voice(found.id) {
                    return Err(format!("Flite has no voice {}.", found.id));
                }
                let (shift, stretch) = flite_params(pitch, rate, lift(found, gender));
                flite.set_f0_shift(shift);
                flite.set_duration_stretch(stretch);
                let audio = flite.synthesize(&text);
                let (rate, samples) = raised(audio.sample_rate, &audio.samples);
                Ok(pcm(rate, &samples))
            }
            "pocket" => {
                let dir = self.pocket_dir.get().ok_or("Pocket TTS didn't come with this copy of Coupler.")?;
                let mut slot = self.pocket.lock().unwrap();
                let pocket = match slot.as_mut() {
                    Some(p) => p,
                    None => {
                        let tokenizer = std::fs::read(dir.join("tokenizer.model")).map_err(|e| format!("Pocket TTS's tokenizer is missing. ({e})"))?;
                        let model = TTSModel::load(POCKET_CONFIG, &dir.join("model.safetensors"), &tokenizer).map_err(|e| format!("Pocket TTS couldn't start. ({e})"))?;
                        slot.insert(Pocket { model, voices: HashMap::new() })
                    }
                };
                if !pocket.voices.contains_key(found.id) {
                    let bytes = std::fs::read(pocket_voice(dir, found.id)).map_err(|e| format!("{}'s voice is missing. ({e})", found.name))?;
                    let state = pocket.model.get_voice_state_from_kv_bytes(&bytes).map_err(|e| format!("{}'s voice couldn't be read. ({e})", found.name))?;
                    pocket.voices.insert(found.id, state);
                }
                let audio = pocket.model.generate(&text, &pocket.voices[found.id]).map_err(|e| format!("{} couldn't speak. ({e})", found.name))?;
                let samples: Vec<f32> = audio.flatten_all().and_then(|a| a.to_vec1()).map_err(|e| e.to_string())?;
                let rate_hz = pocket.model.sample_rate as u32;
                drop(slot);
                let mut shaped = stretch::reshape(&samples, f32::from(pitch) / 100.0, f32::from(rate) / 100.0);
                level(&mut shaped);
                let ints: Vec<i16> = shaped.iter().map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
                Ok(pcm(rate_hz, &ints))
            }
            _ => Err(format!("Coupler can't speak with {engine} yet.")),
        }
    }
}

/// Pocket TTS's voices come out at very different loudness: each line
/// is brought to `LEVEL` (RMS), unless that would push a peak past
/// `PEAK`. Silence stays silence.
const LEVEL: f32 = 0.1;
const PEAK: f32 = 0.95;

fn level(samples: &mut [f32]) {
    let rms = (samples.iter().map(|x| x * x).sum::<f32>() / samples.len().max(1) as f32).sqrt();
    let peak = samples.iter().fold(0f32, |m, x| m.max(x.abs()));
    if rms < 1e-4 || peak < 1e-4 {
        return;
    }
    let gain = (LEVEL / rms).min(PEAK / peak);
    samples.iter_mut().for_each(|x| *x *= gain);
}

/// The samples at a whole multiple of their rate, at least
/// `LEAST_RATE`, each new one on the line between its neighbours.
fn raised(rate: u32, samples: &[i16]) -> (u32, Vec<i16>) {
    let factor = LEAST_RATE.div_ceil(rate.max(1)).max(1);
    if factor == 1 {
        return (rate, samples.to_vec());
    }
    let mut out = Vec::with_capacity(samples.len() * factor as usize);
    for (i, &s) in samples.iter().enumerate() {
        let after = samples.get(i + 1).copied().unwrap_or(s);
        for k in 0..factor {
            let t = k as f32 / factor as f32;
            out.push((f32::from(s) + (f32::from(after) - f32::from(s)) * t).round() as i16);
        }
    }
    (rate * factor, out)
}

/// The sample rate, then the samples, little-endian.
fn pcm(rate: u32, samples: &[i16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + samples.len() * 2);
    out.extend_from_slice(&rate.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seconds(bytes: &[u8]) -> f32 {
        let rate = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        ((bytes.len() - 4) / 2) as f32 / rate as f32
    }

    #[test]
    fn every_flite_voice_speaks() {
        let engines = Engines::default();
        for v in VOICES.iter().filter(|v| v.engine == "flite") {
            let out = engines.say(v.engine, v.id, "Well met, traveller.", Gender::Masculine, 100, 100).unwrap();
            let rate = u32::from_le_bytes(out[..4].try_into().unwrap());
            assert!(rate >= LEAST_RATE, "{}", v.id);
            assert!(seconds(&out) > 0.5, "{}", v.id);
        }
    }

    #[test]
    fn low_rates_are_raised_by_a_whole_factor() {
        assert_eq!(raised(8000, &[0, 300, -300]), (24_000, vec![0, 100, 200, 300, 100, -100, -300, -300, -300]));
        assert_eq!(raised(16_000, &[10]).0, 32_000);
        assert_eq!(raised(44_100, &[1, 2]), (44_100, vec![1, 2]));
    }

    #[test]
    fn faster_is_shorter() {
        let engines = Engines::default();
        let slow = engines.say("flite", "kal", "The orc growls at you.", Gender::Masculine, 100, 70).unwrap();
        let fast = engines.say("flite", "kal", "The orc growls at you.", Gender::Masculine, 100, 200).unwrap();
        assert!(seconds(&fast) < seconds(&slow) * 0.6);
    }

    #[test]
    fn unknown_voices_and_engines_are_refused() {
        let engines = Engines::default();
        assert!(engines.say("flite", "slt", "hi", Gender::Feminine, 100, 100).is_err());
        assert!(engines.say("pocket", "cosette", "hi", Gender::Feminine, 100, 100).is_err());
        assert!(engines.say("espeak", "en", "hi", Gender::Feminine, 100, 100).is_err());
        // Without its files, Pocket TTS isn't offered and can't speak.
        assert!(engines.available().iter().all(|v| v.engine != "pocket"));
        assert!(engines.say("pocket", "alba", "hi", Gender::Feminine, 100, 100).is_err());
    }

    /// The files scripts/pocket-tts-fetch.py prepares, if they're here.
    fn pocket_engines() -> Option<Engines> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/pocket-tts");
        if !dir.join("model.safetensors").is_file() {
            eprintln!("Pocket TTS's files aren't here (scripts/pocket-tts-fetch.py): its tests are skipped.");
            return None;
        }
        let engines = Engines::default();
        engines.set_pocket_dir(dir);
        Some(engines)
    }

    /// Loudness, 0 to 1.
    fn rms(bytes: &[u8]) -> f32 {
        let s: Vec<f32> = bytes[4..].as_chunks::<2>().0.iter().map(|c| f32::from(i16::from_le_bytes(*c)) / 32768.0).collect();
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    #[test]
    fn pocket_voices_speak() {
        let Some(engines) = pocket_engines() else { return };
        assert_eq!(engines.available().iter().filter(|v| v.engine == "pocket").count(), 19);
        for id in ["alba", "george"] {
            let out = engines.say("pocket", id, "Well met, traveller.", Gender::Feminine, 100, 100).unwrap();
            assert_eq!(u32::from_le_bytes(out[..4].try_into().unwrap()), 24_000);
            assert!(seconds(&out) > 0.5 && seconds(&out) < 6.0, "{id}: {} s", seconds(&out));
            assert!(rms(&out) > 0.01, "{id} is silent");
        }
    }

    #[test]
    fn pocket_speed_shortens_the_line() {
        let Some(engines) = pocket_engines() else { return };
        // Generation varies from run to run; speed 200% is still clearly shorter.
        let normal = engines.say("pocket", "paul", "The guard tells you to move along.", Gender::Masculine, 100, 100).unwrap();
        let fast = engines.say("pocket", "paul", "The guard tells you to move along.", Gender::Masculine, 140, 200).unwrap();
        assert!(seconds(&fast) < seconds(&normal) * 0.75, "{} vs {}", seconds(&fast), seconds(&normal));
    }

    /// Every shipped voice, once: slow, so run by name
    /// (`cargo test every_pocket_voice -- --ignored`).
    #[test]
    #[ignore]
    fn every_pocket_voice() {
        let Some(engines) = pocket_engines() else { return };
        for v in VOICES.iter().filter(|v| v.engine == "pocket") {
            // A whole sentence: some voices trail off on a single word.
            let out = engines.say("pocket", v.id, "Greetings, traveller. The road north is closed tonight.", v.gender, 100, 100).unwrap();
            assert!(rms(&out) > 0.01, "{} is silent", v.id);
        }
    }

    #[test]
    fn lines_are_brought_to_one_level() {
        let mut quiet: Vec<f32> = (0..2400).map(|i| (i as f32 / 10.0).sin() * 0.02).collect();
        level(&mut quiet);
        let rms = (quiet.iter().map(|x| x * x).sum::<f32>() / quiet.len() as f32).sqrt();
        assert!((rms - LEVEL).abs() < 0.005);
        // A click isn't made deafening: the peak holds it back.
        let mut click = vec![0.0f32; 2400];
        click[10] = 0.5;
        level(&mut click);
        assert!(click.iter().all(|x| x.abs() <= PEAK + 1e-6));
        let mut silence = vec![0.0f32; 100];
        level(&mut silence);
        assert!(silence.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn params_follow_pitch_speed_and_kind() {
        assert_eq!(flite_params(100, 100, None), (1.0, 1.0));
        let (shift, stretch) = flite_params(120, 200, None);
        assert!((shift - 1.2).abs() < 1e-6 && (stretch - 0.5).abs() < 1e-6);
        let kal = &VOICES[0];
        let (lifted, _) = flite_params(100, 100, lift(kal, Gender::Feminine));
        assert!((lifted - FEMININE_LIFT).abs() < 1e-6);
        assert_eq!(lift(kal, Gender::Masculine), None);
        // Kept where Flite still sounds like speech.
        assert!((flite_params(200, 450, lift(kal, Gender::Feminine)).0 - 2.9).abs() < 1e-5);
        assert_eq!(flite_params(200, 100, Some(2.0)).0, 3.0);
        assert_eq!(flite_params(10, 1000, None).1, 0.2);
    }
}
