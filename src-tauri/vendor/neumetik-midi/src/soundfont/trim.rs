// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! A SoundFont cut down: to the presets chosen, or to what a song plays
//! (its presets, and in them only the zones its notes reach), then
//! everything nothing plays any more taken out and the rest renumbered.

use super::{Link, SoundFont, Zone};
use crate::synth::State;
use crate::Error;

/// The notes a preset plays: a bit for each key and velocity.
struct Notes([u128; 128]);

impl Notes {
    fn any_in(&self, z: &Zone) -> bool {
        let (k, v) = (z.keys.unwrap_or((0, 127)), z.vels.unwrap_or((0, 127)));
        if k.0 > k.1 || v.0 > v.1 {
            return false;
        }
        let vels = (u128::MAX >> (127 - v.1)) & (u128::MAX << v.0);
        (k.0..=k.1).any(|key| self.0[usize::from(key)] & vels != 0)
    }
}

/// A zone with its list's global zone's ranges where it hasn't its own.
fn ranged(z: &Zone, global: &Zone) -> Zone {
    Zone { keys: z.keys.or(global.keys), vels: z.vels.or(global.vels), ..Zone::default() }
}

/// Where two zones' ranges cross.
fn crossed(a: &Zone, b: &Zone) -> Zone {
    let cross = |x: Option<(u8, u8)>, y: Option<(u8, u8)>| {
        let (x, y) = (x.unwrap_or((0, 127)), y.unwrap_or((0, 127)));
        Some((x.0.max(y.0), x.1.min(y.1)))
    };
    Zone { keys: cross(a.keys, b.keys), vels: cross(a.vels, b.vels), ..Zone::default() }
}

impl SoundFont {
    /// Keeps only the presets `keep` says to, and what they play.
    pub fn trim(&mut self, mut keep: impl FnMut(&super::Preset) -> bool) {
        self.presets.retain(|p| keep(p));
        self.prune();
    }

    /// Keeps only what a MIDI file plays through it, as Neumetik plays it:
    /// each channel's bank and program, else that program in bank 0, a
    /// drum part's kit, else kit 0, and in those presets only the zones
    /// the song's notes reach. A song played through what's left sounds
    /// as it did.
    pub fn trim_to_midi(&mut self, midi: &[u8]) -> Result<(), Error> {
        let events = crate::smf::parse(midi)?;
        let mut state = State::new();
        let mut used: Vec<Option<Box<Notes>>> = (0..self.presets.len()).map(|_| None).collect();
        for e in &events {
            if let crate::smf::Message::Channel(status, key, vel) = e.message {
                if status & 0xf0 == 0x90 && vel > 0 {
                    let (drums, bank, program) = state.instrument(usize::from(status & 0x0f));
                    if let Some(p) = self.preset_for(drums, bank, program) {
                        let notes = used[p].get_or_insert_with(|| Box::new(Notes([0; 128])));
                        notes.0[usize::from(key & 0x7f)] |= 1 << (vel & 0x7f);
                    }
                    continue;
                }
            }
            state.chase(&e.message);
        }

        // The instrument zones the notes reach, through the preset zones
        // they reach.
        let mut reached: Vec<Vec<bool>> = self.instruments.iter().map(|i| vec![false; i.zones.len()]).collect();
        for (p, notes) in self.presets.iter_mut().zip(&used) {
            let Some(notes) = notes else {
                p.zones.clear();
                continue;
            };
            let global = p.global.clone();
            p.zones.retain(|pz| {
                let pz_ranges = ranged(pz, &global);
                if !notes.any_in(&pz_ranges) {
                    return false;
                }
                let Some(inst) = pz.target.and_then(|t| self.instruments.get(t)) else { return false };
                let mut any = false;
                for (i, iz) in inst.zones.iter().enumerate() {
                    if notes.any_in(&crossed(&pz_ranges, &ranged(iz, &inst.global))) {
                        reached[pz.target.unwrap_or(0)][i] = true;
                        any = true;
                    }
                }
                any
            });
        }
        for (inst, reached) in self.instruments.iter_mut().zip(&reached) {
            let mut r = reached.iter();
            inst.zones.retain(|_| *r.next().unwrap_or(&false));
        }
        self.presets.retain(|p| !p.zones.is_empty());
        self.prune();
        Ok(())
    }

    /// The preset Neumetik plays for a channel, by its place.
    fn preset_for(&self, drums: bool, bank: u8, program: u8) -> Option<usize> {
        let find = |b: u16, p: u8| self.presets.iter().position(|x| x.bank == b && x.program == u16::from(p));
        if drums {
            return find(128, program).or_else(|| find(128, 0));
        }
        find(u16::from(bank), program).or_else(|| if (80..=82).contains(&bank) { None } else { find(0, program) })
    }

    /// Takes out the instruments no preset plays and the samples no
    /// instrument does (a stereo sample's other half stays while either
    /// half plays), and renumbers what's left.
    pub fn prune(&mut self) {
        let mut inst_used = vec![false; self.instruments.len()];
        for z in self.presets.iter().flat_map(|p| &p.zones) {
            if let Some(u) = z.target.and_then(|t| inst_used.get_mut(t)) {
                *u = true;
            }
        }
        let mut sample_used = vec![false; self.samples.len()];
        for (inst, _) in self.instruments.iter().zip(&inst_used).filter(|(_, u)| **u) {
            for z in &inst.zones {
                if let Some(u) = z.target.and_then(|t| sample_used.get_mut(t)) {
                    *u = true;
                }
            }
        }
        for i in 0..self.samples.len() {
            if let Link::Left(o) | Link::Right(o) = self.samples[i].link {
                if sample_used[i] && o < sample_used.len() {
                    sample_used[o] = true;
                }
            }
        }

        let renumber = |used: &[bool]| {
            let mut next = 0;
            used.iter()
                .map(|&u| {
                    let n = u.then_some(next);
                    next += usize::from(u);
                    n
                })
                .collect::<Vec<Option<usize>>>()
        };
        let inst_at = renumber(&inst_used);
        let sample_at = renumber(&sample_used);
        let mut i = 0;
        self.instruments.retain(|_| {
            i += 1;
            inst_used[i - 1]
        });
        let mut i = 0;
        self.samples.retain(|_| {
            i += 1;
            sample_used[i - 1]
        });
        for p in &mut self.presets {
            p.zones.retain_mut(|z| {
                z.target = z.target.and_then(|t| inst_at.get(t).copied().flatten());
                z.target.is_some()
            });
        }
        for inst in &mut self.instruments {
            inst.zones.retain_mut(|z| {
                z.target = z.target.and_then(|t| sample_at.get(t).copied().flatten());
                z.target.is_some()
            });
        }
        for s in &mut self.samples {
            s.link = match s.link {
                Link::Left(o) => sample_at.get(o).copied().flatten().map_or(Link::Mono, Link::Left),
                Link::Right(o) => sample_at.get(o).copied().flatten().map_or(Link::Mono, Link::Right),
                Link::Mono => Link::Mono,
            };
        }
    }
}
