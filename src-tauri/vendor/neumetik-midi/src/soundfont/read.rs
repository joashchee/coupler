// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! A SoundFont 2 opened as its parts, everything kept: every generator
//! and modulator of every zone, as the file has them, so writing it again
//! changes nothing that matters (the sample data is laid out afresh).

use super::generator::{INSTRUMENT, KEY_RANGE, SAMPLE, VEL_RANGE};
use super::{Info, Instrument, Link, Modulator, Preset, Sample, SoundFont, Zone};
use crate::bank::riff::{chunks, find, i16_at, name, u16_at, u32_at};
use crate::Error;

/// The zones of each preset or instrument: records of `size` bytes in
/// `headers`, each pointing at its first bag at `bag_at`; bags pointing
/// at their first generator and modulator. Each list's global zone comes
/// out first (empty if it hasn't one), then its zones; a zone that plays
/// nothing past the first is dropped, as the specification says.
fn zones(headers: &[u8], size: usize, bag_at: usize, bags: &[u8], gens: &[u8], mods: &[u8], target: u16) -> Vec<(Zone, Vec<Zone>)> {
    let count = headers.len() / size;
    let bag_count = bags.len() / 4;
    let (gen_count, mod_count) = (gens.len() / 4, mods.len() / 10);
    let mut out = Vec::with_capacity(count.saturating_sub(1));
    for h in 0..count.saturating_sub(1) {
        let first = usize::from(u16_at(headers, h * size + bag_at));
        let last = usize::from(u16_at(headers, (h + 1) * size + bag_at)).min(bag_count.saturating_sub(1));
        let mut global = Zone::default();
        let mut list = Vec::new();
        for b in first..last.max(first) {
            let (g0, g1) = (usize::from(u16_at(bags, b * 4)), usize::from(u16_at(bags, (b + 1) * 4)).min(gen_count));
            let (m0, m1) = (usize::from(u16_at(bags, b * 4 + 2)), usize::from(u16_at(bags, (b + 1) * 4 + 2)).min(mod_count));
            let mut z = Zone::default();
            for g in g0..g1.max(g0) {
                let op = u16_at(gens, g * 4);
                let at = g * 4 + 2;
                match op {
                    KEY_RANGE => z.keys = Some((gens[at].min(127), gens[at + 1].min(127))),
                    VEL_RANGE => z.vels = Some((gens[at].min(127), gens[at + 1].min(127))),
                    // Nothing after the target counts.
                    _ if op == target => {
                        z.target = Some(usize::from(u16_at(gens, at)));
                        break;
                    }
                    INSTRUMENT | SAMPLE => {}
                    _ if op < 60 => z.set(op, i16_at(gens, at)),
                    _ => {}
                }
            }
            for m in m0..m1.max(m0) {
                let d = &mods[m * 10..m * 10 + 10];
                let mo = Modulator { source: u16_at(d, 0), destination: u16_at(d, 2), amount: i16_at(d, 4), amount_source: u16_at(d, 6), transform: u16_at(d, 8) };
                // A later modulator the same as an earlier one replaces it.
                match z.modulators.iter_mut().find(|x| (x.source, x.destination, x.amount_source, x.transform) == (mo.source, mo.destination, mo.amount_source, mo.transform)) {
                    Some(x) => *x = mo,
                    None => z.modulators.push(mo),
                }
            }
            if z.target.is_some() {
                list.push(z);
            } else if b == first {
                global = z;
            }
        }
        out.push((global, list));
    }
    out
}

pub fn read(bytes: &[u8]) -> Result<SoundFont, Error> {
    let body = &bytes[12..(8 + u32_at(bytes, 4) as usize).clamp(12, bytes.len())];
    let info = find(body, b"INFO").unwrap_or(&[]);
    let text = |id: &[u8; 4]| find(info, id).map(name).unwrap_or_default();
    let sdta = find(body, b"sdta").ok_or(Error::BadSoundBank)?;
    let pdta = find(body, b"pdta").ok_or(Error::BadSoundBank)?;
    let smpl = find(sdta, b"smpl").unwrap_or(&[]);
    let part = |id: &[u8; 4]| chunks(pdta).find(|c| &c.id == id).map(|c| c.data).ok_or(Error::BadSoundBank);
    let (phdr, pbag, pgen) = (part(b"phdr")?, part(b"pbag")?, part(b"pgen")?);
    let (inst, ibag, igen, shdr) = (part(b"inst")?, part(b"ibag")?, part(b"igen")?, part(b"shdr")?);
    let pmod = part(b"pmod").unwrap_or(&[]);
    let imod = part(b"imod").unwrap_or(&[]);

    let words = smpl.len() / 2;
    let word = |i: usize| i16::from_le_bytes([smpl[i * 2], smpl[i * 2 + 1]]);
    // The headers, but the terminal one; each one's place once ROM
    // samples (not in the file) are dropped.
    let headers: Vec<&[u8]> = shdr.chunks_exact(46).collect();
    let headers = &headers[..headers.len().saturating_sub(1)];
    if headers.iter().any(|h| u16_at(h, 44) & 0x10 != 0 && u16_at(h, 44) & 0x8000 == 0) {
        return Err(Error::CompressedSoundFont);
    }
    let mut place = vec![None; headers.len()];
    let mut samples = Vec::new();
    for (i, h) in headers.iter().enumerate() {
        let kind = u16_at(h, 44);
        if kind & 0x8000 != 0 {
            continue;
        }
        let start = (u32_at(h, 20) as usize).min(words);
        let end = (u32_at(h, 24) as usize).clamp(start, words);
        let (ls, le) = (u32_at(h, 28) as usize, u32_at(h, 32) as usize);
        let (ls, le) = if ls >= start && le > ls && le <= end { ((ls - start) as u32, (le - start) as u32) } else { (0, 0) };
        place[i] = Some(samples.len());
        samples.push((
            Sample { name: name(&h[..20]), data: (start..end).map(word).collect(), rate: u32_at(h, 36), root: h[40], correction: h[41] as i8, loop_start: ls, loop_end: le, link: Link::Mono },
            kind,
            usize::from(u16_at(h, 42)),
        ));
    }
    // Stereo pairs, where both halves are there and point at each other.
    let links: Vec<Link> = samples
        .iter()
        .enumerate()
        .map(|(i, (_, kind, other))| {
            let other_at = place.get(*other).copied().flatten();
            let back = other_at.and_then(|o| samples.get(o)).and_then(|(_, _, b)| place.get(*b).copied().flatten());
            match (kind & 0x7fff, other_at) {
                (2, Some(o)) if back == Some(i) && samples[o].1 & 0x7fff == 4 => Link::Right(o),
                (4, Some(o)) if back == Some(i) && samples[o].1 & 0x7fff == 2 => Link::Left(o),
                _ => Link::Mono,
            }
        })
        .collect();
    let samples: Vec<Sample> = samples
        .into_iter()
        .zip(links)
        .map(|((mut s, _, _), link)| {
            s.link = link;
            s
        })
        .collect();

    let instrument_zones = zones(inst, 22, 20, ibag, igen, imod, SAMPLE);
    let instruments: Vec<Instrument> = instrument_zones
        .into_iter()
        .enumerate()
        .map(|(i, (global, zones))| Instrument {
            name: name(&inst[i * 22..i * 22 + 20]),
            global,
            // A zone of a ROM sample, or none, plays nothing here.
            zones: zones
                .into_iter()
                .filter_map(|mut z| {
                    z.target = place.get(z.target?).copied().flatten();
                    z.target.map(|_| z)
                })
                .collect(),
        })
        .collect();
    let count = instruments.len();
    let presets = zones(phdr, 38, 24, pbag, pgen, pmod, INSTRUMENT)
        .into_iter()
        .enumerate()
        .map(|(p, (global, zones))| {
            let h = &phdr[p * 38..];
            Preset { name: name(&h[..20]), bank: u16_at(h, 22), program: u16_at(h, 20), global, zones: zones.into_iter().filter(|z| z.target.is_some_and(|t| t < count)).collect() }
        })
        .collect();

    Ok(SoundFont {
        info: Info {
            name: text(b"INAM"),
            engine: Some(text(b"isng")).filter(|s| !s.is_empty()).unwrap_or_else(|| "EMU8000".into()),
            date: text(b"ICRD"),
            author: text(b"IENG"),
            product: text(b"IPRD"),
            copyright: text(b"ICOP"),
            comment: text(b"ICMT"),
            software: text(b"ISFT"),
        },
        samples,
        instruments,
        presets,
    })
}
