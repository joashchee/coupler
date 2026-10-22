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
//! velocity also picks the sound (`Tier`). Pure and unit-tested; nothing
//! plays it yet (the soundbank phases in the roadmap).
#![allow(dead_code)] // Read from the next phase, which plays it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Where the bank is, in the app's Resources.
pub const FILE: &str = "coffeemud/coffeemud.sf2";

const MAP: &str = include_str!("soundbank.tsv");

/// A sound in the bank.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// 0 effects, 1 ambience and weather, 2 interface and socials, 3
    /// crafting and foley; 4 and 5 are 0's sub-bass and high detail alone.
    pub program: u8,
    pub key: u8,
    /// Its name, as a caption could say it ("Fire impact").
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

    /// A velocity inside the tier (1–63, 64–107, 108–127), the ones the
    /// bank's mix was checked at.
    pub fn velocity(self) -> u8 {
        match self {
            Tier::Light => 40,
            Tier::Normal => 90,
            Tier::Severe => 120,
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

/// The bank's path, in the app's Resources folder.
pub fn path(resources: &Path) -> PathBuf {
    resources.join(FILE)
}

/// Reads the bank, for a synthesizer to play through.
pub fn open(resources: &Path) -> Result<Arc<neumetik_midi::SoundBank>, String> {
    let file = path(resources);
    let bytes = std::fs::read(&file).map_err(|e| format!("CoffeeMUD's sounds couldn't be read: {e}"))?;
    neumetik_midi::SoundBank::from_bytes(&bytes).map(Arc::new).map_err(|e| format!("CoffeeMUD's sounds couldn't be used: {e}"))
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
        if slot.tiered {
            Tier::ALL.iter().map(|t| t.velocity()).collect()
        } else {
            vec![Tier::Normal.velocity()]
        }
    }

    /// Every slot in the map has a sound of its own in the bank, at every
    /// velocity it's played at, looping just when the map says.
    #[test]
    fn every_slot_is_in_the_bank() {
        let Some(bytes) = bank_bytes() else { return };
        let font = neumetik_midi::soundfont::SoundFont::from_bytes(&bytes).unwrap();
        for s in slots() {
            let preset = font.presets.iter().find(|p| p.bank == 0 && p.program == u16::from(s.program)).unwrap_or_else(|| panic!("no program {}", s.program));
            for vel in velocities(s) {
                let samples: Vec<&neumetik_midi::soundfont::Sample> = preset
                    .zones
                    .iter()
                    .filter(|z| z.covers(s.key, vel))
                    .filter_map(|z| z.target)
                    .flat_map(|i| font.instruments[i].zones.iter().filter(|z| z.covers(s.key, vel)).filter_map(|z| z.target))
                    .map(|i| &font.samples[i])
                    .collect();
                assert!(!samples.is_empty(), "{} ({} {}) has no sound at velocity {vel}", s.name, s.program, s.key);
                assert!(samples.iter().all(|x| x.has_loop() == s.looped), "{} loops when it shouldn't, or doesn't", s.name);
            }
        }
    }

    /// And each one sounds, played through Neumetik MIDI as Coupler will.
    #[test]
    fn every_slot_plays() {
        let Some(bytes) = bank_bytes() else { return };
        let bank = Arc::new(neumetik_midi::SoundBank::from_bytes(&bytes).unwrap());
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
}
