//! coffeemud.sf2, CoffeeMUD's sounds as a SoundFont: each of the game's
//! events on a key of its own, made by ansiapps' Neumetik and dedicated
//! to the public domain (CC0). Shipped in the app's Resources
//! (`coffeemud/coffeemud.sf2`, fetched by scripts/coffeemud-sf2-fetch.py),
//! with `CREDITS.txt` beside it.
//!
//! The slots are `soundbank.tsv`, Neumetik's `coffeemud/map.tsv` at the
//! bank's release (scripts/coffeemud-sf2-sync.py): a program and a key
//! each, what triggers it, the MSP names CoffeeMUD sends for it, and
//! whether it plays once or loops while held. Every sound plays at its
//! own pitch on its key, so the key names the sound. On some keys the
//! velocity also picks the sound (`Tier`).
//!
//! Each sound is an asset by its name in the `bank` folder
//! (`bank/Flesh hit (light)`), so triggers, captions and the buses take
//! it like a file in Assets. `Sounds` makes it a WAV the first time it's
//! asked for and keeps it (`soundbank-cache/<VERSION>/` in app data): a
//! one-shot played through Neumetik MIDI from its note-on to its end, a
//! loop taken straight from the bank at its loop points. Pure but for
//! `Sounds`, and unit-tested.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use neumetik_midi::soundfont::{Link, Sample, SoundFont};
use neumetik_midi::SoundBank;
use serde::Serialize;

/// Where the bank is, in the app's Resources.
pub const FILE: &str = "coffeemud/coffeemud.sf2";
/// The bank's version, as scripts/coffeemud-sf2-fetch.py's `TAG` names
/// it. Its rendered sounds are kept per version.
pub const VERSION: &str = "0.1.0";
/// The folder its sounds are named in, as assets: `bank/Tavern`.
pub const FOLDER: &str = "bank";

const MAP: &str = include_str!("soundbank.tsv");
/// One-shots are rendered at this rate, for at most `LONGEST_SECONDS`.
const RATE: u32 = 44_100;
const LONGEST_SECONDS: usize = 30;
/// Quieter than this (-80 dB) is silence, trimmed from a one-shot's end.
const SILENCE: f32 = 1e-4;

/// A sound in the bank.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// 0 effects, 1 ambience and weather, 2 interface and socials, 3
    /// crafting and foley; 4 and 5 are 0's sub-bass and high detail alone.
    pub program: u8,
    pub key: u8,
    /// Its name, as a caption says it ("Fire impact").
    pub name: &'static str,
    /// What sets it off, in words.
    pub trigger: &'static str,
    /// The MSP sounds CoffeeMUD sends for it (`fireball.wav`).
    pub msp: Vec<&'static str>,
    /// Loops while its key is held, rather than playing once to its end.
    pub looped: bool,
    /// Velocity picks one of three sounds (`Tier`).
    pub tiered: bool,
}

/// On a tiered key, which of its three sounds: combat's light, normal
/// and severe, a creature's idle, aggressive and pain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Light,
    Normal,
    Severe,
}

impl Tier {
    pub const ALL: [Tier; 3] = [Tier::Light, Tier::Normal, Tier::Severe];

    /// The loudest velocity in the tier (of 1–63, 64–107, 108–127). The
    /// bank sets each sound's level in its samples, so at 127 a sound is
    /// as its mix made it; a lighter tier plays quieter, as it was meant to.
    pub fn velocity(self) -> u8 {
        match self {
            Tier::Light => 63,
            Tier::Normal => 107,
            Tier::Severe => 127,
        }
    }
}

impl Slot {
    /// The tiers it has: three, or the one played at every velocity.
    pub fn tiers(&self) -> &'static [Tier] {
        if self.tiered {
            &Tier::ALL
        } else {
            &[Tier::Severe]
        }
    }

    /// A tier's word in its name: light and severe for a blow, idle and
    /// pain for a creature. The middle one, and an untiered sound, has none.
    pub fn tier_word(&self, tier: Tier) -> Option<&'static str> {
        let creature = self.program == 0 && (48..=59).contains(&self.key);
        match (self.tiered, tier, creature) {
            (false, _, _) | (true, Tier::Normal, _) => None,
            (true, Tier::Light, false) => Some("light"),
            (true, Tier::Severe, false) => Some("severe"),
            (true, Tier::Light, true) => Some("idle"),
            (true, Tier::Severe, true) => Some("pain"),
        }
    }

    /// A tier of it as an asset: `bank/Flesh hit (light)`.
    pub fn asset_path(&self, tier: Tier) -> String {
        match self.tier_word(tier) {
            Some(word) => format!("{FOLDER}/{} ({word})", self.name),
            None => format!("{FOLDER}/{}", self.name),
        }
    }
}

/// Every slot in the bank, in the map's order.
pub fn slots() -> &'static [Slot] {
    static SLOTS: OnceLock<Vec<Slot>> = OnceLock::new();
    SLOTS.get_or_init(|| parse(MAP))
}

fn parse(map: &'static str) -> Vec<Slot> {
    let mut lines = map.lines();
    let head: Vec<&str> = lines.next().unwrap_or_default().split('\t').collect();
    let col = |name: &str| head.iter().position(|h| *h == name).unwrap_or_else(|| panic!("soundbank.tsv has no {name} column"));
    let (program, key, name, trigger, msp, play, source) = (col("prog"), col("key"), col("name"), col("trigger"), col("msp"), col("play"), col("source"));
    lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&'static str> = l.split('\t').collect();
            let get = |i: usize| f.get(i).copied().unwrap_or_default();
            Slot {
                program: get(program).parse().expect("a program number"),
                key: get(key).parse().expect("a key number"),
                name: get(name),
                trigger: get(trigger),
                msp: get(msp).split_whitespace().collect(),
                looped: get(play) == "loop",
                tiered: get(source).split(" | ").count() == 3,
            }
        })
        .collect()
}

/// The slot on a program's key.
pub fn slot(program: u8, key: u8) -> Option<&'static Slot> {
    slots().iter().find(|s| s.program == program && s.key == key)
}

/// The slot for an MSP sound, by name, any case. A few names are on two
/// slots (CoffeeMUD sends `dooropen.wav` for closing too): the first, as
/// the map orders them, is the one meant.
pub fn by_msp(name: &str) -> Option<&'static Slot> {
    slots().iter().find(|s| s.msp.iter().any(|m| m.eq_ignore_ascii_case(name)))
}

/// What an MSP sound plays when the player has set no sound on it: the
/// bank's sound for that name, once. Not a weather loop (BGW loops the
/// weather while it lasts, `ambient.rs`). A blow or a creature plays its
/// middle sound, but a scream its pain.
pub fn msp_default(name: &str) -> Option<String> {
    let slot = by_msp(name).filter(|s| !s.looped)?;
    let tier = match (slot.tiered, name.eq_ignore_ascii_case("scream.wav")) {
        (true, false) => Tier::Normal,
        _ => Tier::Severe,
    };
    Some(slot.asset_path(tier))
}

/// Whether an asset path is in the bank's folder (named one of its sounds
/// or not).
pub fn is_bank_path(path: &str) -> bool {
    path.strip_prefix(FOLDER).is_some_and(|rest| rest.starts_with('/'))
}

/// The sound an asset path names, any case: `bank/Flesh hit (light)`.
pub fn find(path: &str) -> Option<(&'static Slot, Tier)> {
    let name = path.strip_prefix(FOLDER)?.strip_prefix('/')?;
    slots().iter().find_map(|s| s.tiers().iter().find(|t| s.asset_path(**t)[FOLDER.len() + 1..].eq_ignore_ascii_case(name)).map(|t| (s, *t)))
}

/// One of the bank's sounds, as the hook editor lists it.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub path: String,
    /// A loop (for BGN and BGW), rather than a sound played once (SFX).
    pub looped: bool,
    /// The MSP sounds that play it when the player has set none
    /// (`msp_default`).
    pub msp: Vec<&'static str>,
}

/// Every sound in the bank, each tier its own, in the map's order.
pub fn list() -> Vec<Sound> {
    slots()
        .iter()
        .flat_map(|s| {
            s.tiers().iter().map(|t| {
                let path = s.asset_path(*t);
                let msp = s.msp.iter().copied().filter(|name| msp_default(name).as_deref() == Some(path.as_str())).collect();
                Sound { path, looped: s.looped, msp }
            })
        })
        .collect()
}

/// The bank's path, in the app's Resources folder.
pub fn path(resources: &Path) -> PathBuf {
    resources.join(FILE)
}

/// Reads the bank, for a synthesizer to play through.
pub fn open(resources: &Path) -> Result<Arc<SoundBank>, String> {
    let bytes = read(resources)?;
    SoundBank::from_bytes(&bytes).map(Arc::new).map_err(|e| format!("CoffeeMUD's sounds couldn't be used. ({e})"))
}

fn read(resources: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path(resources)).map_err(|e| format!("CoffeeMUD's sounds couldn't be read. ({e})"))
}

/// A one-shot played through Neumetik MIDI, dry, from its note-on to the
/// end of its sample, as a WAV (mono if both sides are the same). Its level is the
/// bank's own: nothing's made louder, so the bank's mix holds.
pub fn render_once(bank: &Arc<SoundBank>, slot: &Slot, tier: Tier) -> Vec<u8> {
    let mut synth = neumetik_midi::Synth::new(RATE);
    synth.set_sound_bank(Some(bank.clone()));
    // The bank was mixed without the synthesizer's reverb and chorus.
    synth.control_change(0, 91, 0);
    synth.control_change(0, 93, 0);
    synth.program_change(0, slot.program);
    // Never let go: a one-shot's voice ends with its sample, and a note-off
    // before the first render silences it at once, long release or not.
    synth.note_on(0, slot.key, tier.velocity());
    const BLOCK: usize = 1024;
    let (mut left, mut right) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    let mut out: Vec<f32> = Vec::new();
    while out.len() < LONGEST_SECONDS * RATE as usize * 2 {
        synth.render(&mut left, &mut right);
        out.extend(left.iter().zip(&right).flat_map(|(l, r)| [*l, *r]));
        if synth.active_voices() == 0 {
            break;
        }
    }
    out.truncate(out.iter().rposition(|x| x.abs() > SILENCE).map_or(0, |i| i / 2 * 2 + 2));
    let samples: Vec<i16> = out.iter().map(|x| (x.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
    if samples.as_chunks::<2>().0.iter().all(|[l, r]| l == r) {
        crate::assets::wav(1, RATE, &samples.iter().step_by(2).copied().collect::<Vec<_>>())
    } else {
        crate::assets::wav(2, RATE, &samples)
    }
}

/// The samples a slot plays at a velocity, by their place in the bank.
fn samples_for(font: &SoundFont, slot: &Slot, vel: u8) -> Vec<usize> {
    let Some(preset) = font.presets.iter().find(|p| p.bank == 0 && p.program == u16::from(slot.program)) else {
        return Vec::new();
    };
    preset
        .zones
        .iter()
        .filter(|z| z.covers(slot.key, vel))
        .filter_map(|z| z.target)
        .flat_map(|i| font.instruments[i].zones.iter().filter(|z| z.covers(slot.key, vel)).filter_map(|z| z.target))
        .collect()
}

/// A loop as a WAV, straight from the bank: its loop and nothing else, so
/// it goes round without a seam, at its own rate, a stereo pair as two
/// channels.
pub fn loop_take(font: &SoundFont, slot: &Slot) -> Result<Vec<u8>, String> {
    let missing = || format!("{} isn't in CoffeeMUD's sounds.", slot.name);
    let first = &font.samples[*samples_for(font, slot, 127).first().ok_or_else(missing)?];
    let (left, right) = match first.link {
        Link::Left(other) => (first, Some(&font.samples[other])),
        Link::Right(other) => (&font.samples[other], Some(first)),
        Link::Mono => (first, None),
    };
    let take = |s: &Sample| s.data.get(s.loop_start as usize..s.loop_end as usize).filter(|t| !t.is_empty()).map(<[i16]>::to_vec);
    let left_take = take(left).ok_or_else(|| format!("{} doesn't loop.", slot.name))?;
    match right.map(take) {
        None => Ok(crate::assets::wav(1, left.rate, &left_take)),
        Some(Some(right_take)) if right_take.len() == left_take.len() => {
            Ok(crate::assets::wav(2, left.rate, &left_take.iter().zip(&right_take).flat_map(|(l, r)| [*l, *r]).collect::<Vec<_>>()))
        }
        Some(_) => Err(format!("{}'s two sides don't loop together.", slot.name)),
    }
}

/// The bank's sounds as WAVs, each made once and kept. The bank, once
/// read, stays read: the game's sounds come often.
#[derive(Default)]
pub struct Sounds {
    inner: Mutex<Inner>,
    /// The game's sounds have been made ready this run (`warm`).
    warmed: AtomicBool,
}

#[derive(Default)]
struct Inner {
    resources: Option<PathBuf>,
    cache: Option<PathBuf>,
    bank: Option<Arc<SoundBank>>,
}

impl Sounds {
    /// Where the bank ships (the app's Resources), and the folder its
    /// sounds are kept in (`soundbank-cache`, a folder per version: an
    /// older bank's are deleted).
    pub fn set_dirs(&self, resources: PathBuf, cache_root: &Path) {
        let cache = cache_root.join(VERSION);
        for old in std::fs::read_dir(cache_root).into_iter().flatten().flatten() {
            if old.path() != cache {
                let _ = std::fs::remove_dir_all(old.path());
            }
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.resources = Some(resources);
        inner.cache = Some(cache);
    }

    /// Whether this copy of Coupler has the bank.
    pub fn available(&self) -> bool {
        self.dirs().is_some_and(|(resources, _)| path(&resources).is_file())
    }

    fn dirs(&self) -> Option<(PathBuf, PathBuf)> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Some((inner.resources.clone()?, inner.cache.clone()?))
    }

    /// One of the bank's sounds as a WAV, by its asset path.
    pub fn audio(&self, asset: &str) -> Result<Vec<u8>, String> {
        let (slot, tier) = find(asset).ok_or_else(|| format!("{asset} isn't one of CoffeeMUD's sounds."))?;
        let (resources, cache) = self.dirs().ok_or("CoffeeMUD's sounds aren't ready yet.")?;
        let file = cache.join(file_name(slot, tier));
        if let Ok(bytes) = std::fs::read(&file) {
            return Ok(bytes);
        }
        if !slot.looped {
            let wav = render_once(&self.bank(&resources)?, slot, tier);
            keep(&cache, &file, &wav);
            return Ok(wav);
        }
        // Reading the bank for its loops is the slow part, so the first
        // loop asked for takes them all out.
        let font = SoundFont::from_bytes(&read(&resources)?).map_err(|e| format!("CoffeeMUD's sounds couldn't be used. ({e})"))?;
        let mut asked = None;
        for s in slots().iter().filter(|s| s.looped) {
            let Ok(wav) = loop_take(&font, s) else { continue };
            keep(&cache, &cache.join(file_name(s, Tier::Severe)), &wav);
            if s == slot {
                asked = Some(wav);
            }
        }
        asked.map_or_else(|| loop_take(&font, slot), Ok)
    }

    /// Makes every sound the game's MSP names play ready, once a run, so
    /// the first fireball isn't late: each made now that isn't kept from
    /// before. Slow (the bank's one-shots are rendered), so on a thread of
    /// its own.
    pub fn warm(&self) {
        if !self.available() || self.warmed.swap(true, Ordering::Relaxed) {
            return;
        }
        let mut paths: Vec<String> = slots().iter().flat_map(|s| s.msp.iter()).filter_map(|name| msp_default(name)).collect();
        paths.sort();
        paths.dedup();
        for path in paths {
            if let Err(e) = self.audio(&path) {
                log::warn!("{e}");
            }
        }
    }

    fn bank(&self, resources: &Path) -> Result<Arc<SoundBank>, String> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(bank) = &inner.bank {
            return Ok(bank.clone());
        }
        let bank = open(resources)?;
        inner.bank = Some(bank.clone());
        Ok(bank)
    }
}

/// A sound's file in the cache: its program, key and velocity.
fn file_name(slot: &Slot, tier: Tier) -> String {
    format!("{}-{}-{}.wav", slot.program, slot.key, tier.velocity())
}

/// Keeps a made sound for next time, through a file renamed into place.
/// One that can't be kept costs only another render.
fn keep(cache: &Path, file: &Path, wav: &[u8]) {
    let part = file.with_extension("part");
    if std::fs::create_dir_all(cache).is_ok() && std::fs::write(&part, wav).is_ok() {
        let _ = std::fs::rename(&part, file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_map_reads() {
        let all = slots();
        assert_eq!(all.len(), 210);
        let keys: HashSet<(u8, u8)> = all.iter().map(|s| (s.program, s.key)).collect();
        assert_eq!(keys.len(), all.len(), "a key booked twice");
        assert!(all.iter().all(|s| s.program <= 3 && s.key <= 127 && !s.name.is_empty()));
        // The ambience and weather loops, and the dread drone under them.
        assert_eq!(all.iter().filter(|s| s.looped).count(), 53);
        assert!(all.iter().filter(|s| s.looped).all(|s| s.program == 1 || (s.program, s.key) == (0, 23)));
        // Combat's and the creatures' keys but arrow release, bow draw,
        // miss and critical hit, which have one sound at every velocity.
        assert_eq!(all.iter().filter(|s| s.tiered).count(), 20);
    }

    #[test]
    fn finds_by_msp_name() {
        let names: HashSet<&str> = slots().iter().flat_map(|s| s.msp.iter().copied()).collect();
        assert_eq!(names.len(), 104);
        assert!(names.iter().all(|n| n.ends_with(".wav")));
        assert_eq!(by_msp("fireball.wav").map(|s| s.name), Some("Fire impact"));
        assert_eq!(by_msp("FIREBALL.WAV").map(|s| s.name), Some("Fire impact"));
        assert_eq!(by_msp("dooropen.wav").map(|s| s.name), Some("Door open"));
        assert_eq!(by_msp("rain.wav").map(|s| (s.program, s.key, s.looped)), Some((1, 85, true)));
        assert_eq!(by_msp("nothing.wav"), None);
    }

    #[test]
    fn msp_defaults_are_one_shots() {
        assert_eq!(msp_default("fireball.wav").as_deref(), Some("bank/Fire impact"));
        assert_eq!(msp_default("bashed1.wav").as_deref(), Some("bank/Flesh hit"));
        assert_eq!(msp_default("scream.wav").as_deref(), Some("bank/Humanoid (pain)"));
        assert_eq!(msp_default("rain.wav"), None, "the weather loops on BGW");
        assert_eq!(msp_default("nothing.wav"), None);
        // Each names a sound the bank plays.
        for name in slots().iter().flat_map(|s| s.msp.iter()) {
            if let Some(path) = msp_default(name) {
                let (slot, _) = find(&path).unwrap_or_else(|| panic!("{path} isn't a sound"));
                assert!(!slot.looped);
            }
        }
        assert_eq!(slot(0, 29).map(|s| (s.name, s.tiered)), Some(("Flesh hit", true)));
        assert_eq!(slot(4, 12), None);
    }

    #[test]
    fn tiers_sit_in_their_ranges() {
        let v: Vec<u8> = Tier::ALL.iter().map(|t| t.velocity()).collect();
        assert!((1..=63).contains(&v[0]) && (64..=107).contains(&v[1]) && (108..=127).contains(&v[2]));
    }

    /// The bank scripts/coffeemud-sf2-fetch.py puts in Resources, if it's here.
    fn bank_bytes() -> Option<Vec<u8>> {
        let file = path(&Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"));
        let bytes = std::fs::read(file).ok();
        if bytes.is_none() {
            eprintln!("coffeemud.sf2 isn't here (scripts/coffeemud-sf2-fetch.py): its tests are skipped.");
        }
        bytes
    }

    fn velocities(slot: &Slot) -> Vec<u8> {
        slot.tiers().iter().map(|t| t.velocity()).collect()
    }

    /// Every slot in the map has a sound of its own in the bank, at every
    /// velocity it's played at, looping just when the map says.
    #[test]
    fn every_slot_is_in_the_bank() {
        let Some(bytes) = bank_bytes() else { return };
        let font = SoundFont::from_bytes(&bytes).unwrap();
        for s in slots() {
            for vel in velocities(s) {
                let samples = samples_for(&font, s, vel);
                assert!(!samples.is_empty(), "{} ({} {}) has no sound at velocity {vel}", s.name, s.program, s.key);
                assert!(samples.iter().all(|i| font.samples[*i].has_loop() == s.looped), "{} loops when it shouldn't, or doesn't", s.name);
            }
        }
    }

    /// And each one sounds, played through Neumetik MIDI as Coupler will.
    #[test]
    fn every_slot_plays() {
        let Some(bytes) = bank_bytes() else { return };
        let bank = Arc::new(SoundBank::from_bytes(&bytes).unwrap());
        const RATE: u32 = 22_050;
        for s in slots() {
            for vel in velocities(s) {
                let mut synth = neumetik_midi::Synth::new(RATE);
                synth.set_sound_bank(Some(bank.clone()));
                synth.program_change(0, s.program);
                synth.note_on(0, s.key, vel);
                // Some build from nothing (the implosion): up to 4 s.
                let (mut left, mut right) = (vec![0.0; RATE as usize / 4], vec![0.0; RATE as usize / 4]);
                let sounds = (0..16).any(|_| {
                    synth.render(&mut left, &mut right);
                    left.iter().chain(&right).any(|x| x.abs() > 0.001)
                });
                assert!(sounds, "{} ({} {}) is silent at velocity {vel}", s.name, s.program, s.key);
            }
        }
    }

    #[test]
    fn the_version_is_the_fetched_one() {
        let fetch = include_str!("../../scripts/coffeemud-sf2-fetch.py");
        assert!(fetch.contains(&format!("TAG = \"coffeemud-sf2-v{VERSION}\"")), "VERSION isn't the fetch script's TAG");
    }

    #[test]
    fn every_sound_has_a_path_of_its_own() {
        let all = list();
        assert_eq!(all.len(), 210 + 2 * 20);
        assert_eq!(all.iter().filter(|s| s.looped).count(), 53);
        let paths: HashSet<String> = all.iter().map(|s| s.path.to_lowercase()).collect();
        assert_eq!(paths.len(), all.len());
        for sound in &all {
            let (slot, tier) = find(&sound.path).unwrap_or_else(|| panic!("{} isn't found", sound.path));
            assert_eq!(slot.asset_path(tier), sound.path);
            assert!(is_bank_path(&sound.path));
            // The frontend's title is all after the folder, its extension cut.
            let title = &sound.path[FOLDER.len() + 1..];
            assert!(!title.contains('.') && !title.contains('/'), "{title}");
        }
        assert_eq!(find("bank/flesh hit (LIGHT)").map(|(s, t)| (s.name, t)), Some(("Flesh hit", Tier::Light)));
        assert_eq!(find("bank/Flesh hit").map(|(_, t)| t), Some(Tier::Normal));
        assert_eq!(find("bank/Canine (idle)").map(|(s, t)| (s.name, t)), Some(("Canine", Tier::Light)));
        assert_eq!(find("bank/Canine (pain)").map(|(_, t)| t), Some(Tier::Severe));
        // Only a tiered sound has tiers, each of its own kind.
        assert_eq!(find("bank/Flesh hit (pain)"), None);
        assert_eq!(find("bank/Tavern (light)"), None);
        assert_eq!(find("bank/Tavern").map(|(s, _)| s.looped), Some(true));
        assert_eq!(find("bank/Arrow release").map(|(_, t)| t.velocity()), Some(127));
        assert_eq!(find("wav/Tavern"), None);
        assert!(!is_bank_path("bankrobber/x.wav") && !is_bank_path("wav/bank") && !is_bank_path("bank"));
    }

    /// A WAV's channels, rate and samples.
    fn read_wav(wav: &[u8]) -> (u16, u32, Vec<i16>) {
        assert_eq!(&wav[..4], b"RIFF");
        let channels = u16::from_le_bytes([wav[22], wav[23]]);
        let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        (channels, rate, wav[44..].as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c)).collect())
    }

    #[test]
    fn a_one_shot_plays_to_its_end_at_the_banks_level() {
        let Some(bytes) = bank_bytes() else { return };
        let bank = Arc::new(SoundBank::from_bytes(&bytes).unwrap());
        for name in ["bank/Flesh hit (light)", "bank/Flesh hit (severe)", "bank/Door open", "bank/Implosion", "bank/Level up"] {
            let (slot, tier) = find(name).unwrap();
            let (channels, rate, samples) = read_wav(&render_once(&bank, slot, tier));
            assert_eq!(rate, RATE);
            let seconds = samples.len() as f32 / f32::from(channels) / rate as f32;
            assert!(seconds > 0.05 && seconds < LONGEST_SECONDS as f32, "{name}: {seconds} s");
            // Mixed to peak at -1 dBFS at most, and nothing made louder here.
            let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0) as f32 / 32768.0;
            assert!(peak > 0.02 && peak < 0.95, "{name}: peak {peak}");
            // Trimmed: it ends on sound, not seconds of silence.
            let tail = &samples[samples.len().saturating_sub(usize::from(channels))..];
            assert!(tail.iter().any(|s| s.unsigned_abs() > 0), "{name} ends in silence");
        }
    }

    #[test]
    fn a_loop_is_its_loop_and_nothing_else() {
        let Some(bytes) = bank_bytes() else { return };
        let font = SoundFont::from_bytes(&bytes).unwrap();
        for name in ["bank/Tavern", "bank/Heavy rain", "bank/Dread drone", "bank/Cave"] {
            let (slot, _) = find(name).unwrap();
            let (channels, rate, samples) = read_wav(&loop_take(&font, slot).unwrap());
            assert!(matches!(channels, 1 | 2) && matches!(rate, 22_050 | 32_000), "{name}: {channels} channels at {rate}");
            let seconds = samples.len() as f32 / f32::from(channels) / rate as f32;
            assert!((5.0..=12.0).contains(&seconds), "{name}: {seconds} s");
        }
        assert!(loop_take(&font, find("bank/Door open").unwrap().0).is_err());
    }

    #[test]
    fn sounds_are_made_once_and_kept() {
        if bank_bytes().is_none() {
            return;
        }
        let root = std::env::temp_dir().join(format!("coupler-soundbank-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("0.0.1")).unwrap();
        let sounds = Sounds::default();
        assert!(!sounds.available());
        assert!(sounds.audio("bank/Tavern").is_err());
        sounds.set_dirs(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"), &root);
        assert!(sounds.available());
        // An older bank's sounds are gone.
        assert!(!root.join("0.0.1").exists());
        let cache = root.join(VERSION);
        let tavern = sounds.audio("bank/Tavern").unwrap();
        // The first loop takes out every loop.
        let kept = std::fs::read_dir(&cache).unwrap().count();
        assert_eq!(kept, 53);
        assert_eq!(sounds.audio("bank/tavern").unwrap(), tavern);
        let hit = sounds.audio("bank/Flesh hit (light)").unwrap();
        assert_eq!(std::fs::read(cache.join("0-29-63.wav")).unwrap(), hit);
        assert_eq!(sounds.audio("bank/Flesh hit (light)").unwrap(), hit);
        assert!(sounds.audio("bank/Nothing").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
